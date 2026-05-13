//! gpio-keys kernel driver. Each DT child of a `gpio-keys` node maps to
//! one [`Key`]; consumers `open` by alias, label, or code and block on
//! [`Key::wait`] until an edge arrives.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::error::Result;
use crate::hal;
use crate::sync::atomic::AtomicU64;
use crate::sync::once::OnceCell;
use crate::sync::waiter::ParkedWaiter;

use hal::Machinelike;
use hal::device_tree::KeyRegistryEntry;
use hal::gpio::{Edges, Level, Pin, Pull};

pub struct KeyEvent {
    pub code: u32,
    pub pressed: bool,
}

struct KeyState {
    entry: &'static KeyRegistryEntry,
    waiter: ParkedWaiter,
    /// `0` means "no edge yet" — see `on_edge`.
    last_event_mono: AtomicU64,
    /// Pre-converted from `entry.debounce_ms` so the edge callback avoids a divide.
    debounce_ticks: u64,
    /// State at edge time, so `wait()` survives presses that settle back before the consumer wakes.
    latched_pressed: AtomicBool,
    /// `Key::open_*` rejects handles whose init failed partway.
    initialized: AtomicBool,
}

impl KeyState {
    fn pin(&self) -> Pin {
        Pin {
            port: self.entry.port,
            line: self.entry.line,
        }
    }
}

// One slot per DT entry; the `&'static KeyState` handed to ISRs lives
// for the program.
static SLOTS: [OnceCell<KeyState>; hal::device_tree::KEY_REGISTRY.len()] =
    [const { OnceCell::new() }; hal::device_tree::KEY_REGISTRY.len()];

const _: () = {
    let entries = hal::device_tree::KEY_REGISTRY;
    let mut i = 0;
    while i < entries.len() {
        let slot_i = match hal::gpio::irq_slot_for_line(entries[i].line) {
            Some(v) => v,
            None => panic!("gpio-key DT entry references an unmapped GPIO line"),
        };
        assert!(
            slot_i < 64,
            "gpio-key IRQ slot exceeds u64 dedup-mask width"
        );

        let mut j = 0;
        while j < i {
            if let Some(slot_j) = hal::gpio::irq_slot_for_line(entries[j].line) {
                if slot_i == slot_j {
                    assert!(
                        entries[i].irq_priority == entries[j].irq_priority,
                        "gpio-keys sharing an IRQ slot must declare the same `osiris,irq-priority`"
                    );
                }
            }
            j += 1;
        }
        i += 1;
    }
};

pub struct Key {
    state: &'static KeyState,
}

impl Key {
    pub fn open_by_alias(name: &str) -> Result<Self> {
        let entry = hal::device_tree::key_by_alias(name)
            .ok_or_else(|| kerr!(ENODEV, "key alias not found: {name}"))?;
        Self::open_for_node(entry.node)
    }

    pub fn open_by_label(label: &str) -> Result<Self> {
        let entry = hal::device_tree::key_by_label(label)
            .ok_or_else(|| kerr!(ENODEV, "key label not found: {label}"))?;
        Self::open_for_node(entry.node)
    }

    pub fn open_by_code(code: u32) -> Result<Self> {
        let entry = hal::device_tree::key_by_code(code)
            .ok_or_else(|| kerr!(ENODEV, "key code not found: {code}"))?;
        Self::open_for_node(entry.node)
    }

    fn open_for_node(node: usize) -> Result<Self> {
        for cell in SLOTS.iter() {
            if let Some(state) = cell.get() {
                if state.entry.node == node {
                    if !state.initialized.load(Ordering::Acquire) {
                        return Err(kerr!(EIO, "key node {node} init failed"));
                    }
                    return Ok(Self { state });
                }
            }
        }
        Err(kerr!(ENODEV, "key node {node} not registered"))
    }

    pub fn code(&self) -> u32 {
        self.state.entry.code
    }

    pub fn label(&self) -> &'static str {
        self.state.entry.label
    }

    /// Snapshot the line without blocking.
    pub fn poll(&self) -> Result<KeyEvent> {
        let level = hal::gpio::read(self.state.pin())?;
        Ok(KeyEvent {
            code: self.state.entry.code,
            pressed: pressed_from_level(self.state.entry, level),
        })
    }

    /// Block until the next edge. `EBUSY` if another thread is already
    /// waiting; edges arriving before wake-up coalesce into one event.
    pub fn wait(&self) -> Result<KeyEvent> {
        self.state.waiter.park_current()?;
        Ok(KeyEvent {
            code: self.state.entry.code,
            pressed: self.state.latched_pressed.load(Ordering::Acquire),
        })
    }
}

fn pressed_from_level(entry: &KeyRegistryEntry, level: Level) -> bool {
    let active_low = entry.active_low != 0;
    (level == Level::High) ^ active_low
}

extern "C" fn on_edge(_line: u8, ctx: *mut ()) {
    // Guard against a stray fire during teardown.
    if ctx.is_null() {
        return;
    }
    // SAFETY: ctx points into SLOTS (a static OnceCell array).
    let state = unsafe { &*(ctx as *const KeyState) };

    if state.debounce_ticks > 0 {
        let now = hal::Machine::monotonic_now();
        let prev = state.last_event_mono.load(Ordering::Acquire);
        // Skip debounce on the first edge — `monotonic_now()` may still
        // be smaller than `debounce_ticks`.
        if prev != 0 && now.saturating_sub(prev) < state.debounce_ticks {
            return;
        }
        let stored = if now == 0 { 1 } else { now };
        state.last_event_mono.store(stored, Ordering::Release);
    }

    if let Ok(level) = hal::gpio::read(state.pin()) {
        state
            .latched_pressed
            .store(pressed_from_level(state.entry, level), Ordering::Release);
    }

    state.waiter.wake();
}

fn debounce_to_ticks(debounce_ms: u32) -> u64 {
    if debounce_ms == 0 {
        return 0;
    }
    let freq = hal::Machine::monotonic_freq();
    (debounce_ms as u64).saturating_mul(freq) / 1000
}

pub fn init() {
    let entries = hal::device_tree::KEY_REGISTRY;
    kprintln!("Found {} gpio-key entries", entries.len());

    // Install the shared dispatcher once per slot.
    let mut seen_slots: u64 = 0;

    for (i, entry) in entries.iter().enumerate() {
        if let Err(e) = init_entry(i, entry, &mut seen_slots) {
            kprintln!("    Key {}: init failed: {:?}", entry.label, e);
        }
    }
}

fn init_entry(
    slot_idx: usize,
    entry: &'static KeyRegistryEntry,
    seen_slots: &mut u64,
) -> Result<()> {
    let slot =
        hal::gpio::irq_slot_for_line(entry.line).ok_or_else(|| kerr!(EINVAL, "invalid line"))?;

    let state = KeyState {
        entry,
        waiter: ParkedWaiter::new(),
        last_event_mono: AtomicU64::new(0),
        debounce_ticks: debounce_to_ticks(entry.debounce_ms),
        latched_pressed: AtomicBool::new(false),
        initialized: AtomicBool::new(false),
    };
    let state_ref: &'static KeyState = SLOTS[slot_idx].set_or_get(state);
    let pin = state_ref.pin();

    let bit = 1u64 << slot;
    if *seen_slots & bit == 0 {
        unsafe { crate::irq::register_irq(slot, hal::gpio::dispatch, None) }?;
        *seen_slots |= bit;
    }

    let pull = if entry.active_low != 0 {
        Pull::Up
    } else {
        Pull::Down
    };
    hal::gpio::configure_input(pin, pull)?;

    if let Ok(level) = hal::gpio::read(pin) {
        state_ref
            .latched_pressed
            .store(pressed_from_level(entry, level), Ordering::Release);
    }

    let ctx = state_ref as *const KeyState as *mut ();
    hal::gpio::register_edge_handler(pin, Edges::BOTH, on_edge, ctx, entry.irq_priority)?;

    state_ref.initialized.store(true, Ordering::Release);
    kprintln!(
        "    Initialized key {} on port 0x{:x} line {}",
        entry.label,
        entry.port,
        entry.line
    );
    Ok(())
}
