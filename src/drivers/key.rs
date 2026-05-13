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
    last_event_mono: AtomicU64,
    /// Pre-converted from `entry.debounce_ms` so the ISR avoids a divide.
    debounce_ticks: u64,
    /// Set true only after `init_entry` ran to completion. `Key::open_*`
    /// rejects keys whose init started but failed partway, so callers
    /// don't get a handle that would block on `wait()` forever.
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

// One slot per DT entry, sized at compile time so there is no truncation
// case to handle at runtime. `OnceCell::set_or_get` writes in place, so
// the `&'static KeyState` handed to ISRs stays valid for the program
// lifetime.
static SLOTS: [OnceCell<KeyState>; hal::device_tree::KEY_REGISTRY.len()] =
    [const { OnceCell::new() }; hal::device_tree::KEY_REGISTRY.len()];

// Compile-time validation: every DT-derived line must map to a known
// IRQ vector that fits in the u64 dedup mask. If a board adds a key
// with an unsupported line, the build fails here with a clear message
// rather than silently failing at boot.
const _: () = {
    let entries = hal::device_tree::KEY_REGISTRY;
    let mut i = 0;
    while i < entries.len() {
        match hal::gpio::nvic_vector_for_line(entries[i].line) {
            Some(v) => assert!(v < 64, "gpio-key IRQ vector exceeds u64 dedup-mask width"),
            None => panic!("gpio-key DT entry references an unmapped GPIO line"),
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
                        return Err(kerr!(EINVAL, "key node {node} init failed"));
                    }
                    return Ok(Self { state });
                }
            }
        }
        Err(kerr!(EINVAL, "key node {node} not initialized"))
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
        Ok(self.event_from_level(level))
    }

    /// Block until the next edge, then return the event for the current
    /// line level. Returns `EBUSY` if another thread is already waiting
    /// on this key.
    pub fn wait(&self) -> Result<KeyEvent> {
        self.state.waiter.park_current()?;
        let level = hal::gpio::read(self.state.pin())?;
        Ok(self.event_from_level(level))
    }

    fn event_from_level(&self, level: Level) -> KeyEvent {
        let active_low = self.state.entry.active_low != 0;
        let pressed = (level == Level::High) ^ active_low;
        KeyEvent {
            code: self.state.entry.code,
            pressed,
        }
    }
}

extern "C" fn on_edge(_line: u8, ctx: *mut ()) {
    // Defensive: the HAL guarantees a non-null `ctx` for any line whose
    // handler is currently installed, but a stray fire (e.g. against a
    // line in the middle of teardown) should not deref a null.
    if ctx.is_null() {
        return;
    }
    // SAFETY: ctx points into SLOTS (a static OnceCell array).
    let state = unsafe { &*(ctx as *const KeyState) };

    if state.debounce_ticks > 0 {
        let now = hal::Machine::monotonic_now();
        let prev = state.last_event_mono.load(Ordering::Acquire);
        if now.saturating_sub(prev) < state.debounce_ticks {
            return;
        }
        state.last_event_mono.store(now, Ordering::Release);
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

    // Several keys may resolve to the same IRQ vector; install the
    // shared dispatcher exactly once per vector.
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
    // `nvic_vector_for_line` returning Some and `vector < 64` are both
    // enforced at compile time for every DT entry by the const block at
    // the top of this module, so this never errors in practice.
    let vector =
        hal::gpio::nvic_vector_for_line(entry.line).ok_or_else(|| kerr!(EINVAL, "invalid line"))?;

    let state = KeyState {
        entry,
        waiter: ParkedWaiter::new(),
        last_event_mono: AtomicU64::new(0),
        debounce_ticks: debounce_to_ticks(entry.debounce_ms),
        initialized: AtomicBool::new(false),
    };
    let state_ref: &'static KeyState = SLOTS[slot_idx].set_or_get(state);
    let pin = state_ref.pin();

    let bit = 1u64 << vector;
    if *seen_slots & bit == 0 {
        unsafe { crate::irq::register_irq(vector, hal::gpio::dispatch, None) }?;
        *seen_slots |= bit;
    }

    let pull = if entry.active_low != 0 {
        Pull::Up
    } else {
        Pull::Down
    };
    hal::gpio::configure_input(pin, pull)?;

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
