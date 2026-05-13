//! gpio-keys kernel driver. Hardware-independent — uses only
//! `hal::gpio`, `hal::device_tree`, and the kernel scheduler. IRQ + park
//! logic clones the CAN pattern via [`sync::waiter::ParkedWaiter`].

use core::sync::atomic::Ordering;

use crate::error::Result;
use crate::hal;
use crate::sync::atomic::AtomicU64;
use crate::sync::once::OnceCell;
use crate::sync::waiter::ParkedWaiter;

use hal::Machinelike;
use hal::device_tree::KeyRegistryEntry;
use hal::gpio::{Edges, Level, Pin, Pull};

/// Max keys the kernel tracks; the DT may populate fewer.
const KEY_MAX: usize = 8;

/// NVIC priority for EXTI vectors. Below CAN's typical priority.
const KEY_NVIC_PRIORITY: u8 = 5;

pub struct KeyEvent {
    pub code: u32,
    /// `true` when the line is at the logical "pressed" level.
    pub pressed: bool,
}

struct KeyState {
    entry: &'static KeyRegistryEntry,
    waiter: ParkedWaiter,
    /// Last accepted edge in `Machine::monotonic_now()` units.
    last_event_mono: AtomicU64,
    /// `debounce_ms` pre-converted to monotonic ticks — keeps the ISR division-free.
    debounce_ticks: u64,
}

impl KeyState {
    fn pin(&self) -> Pin {
        Pin {
            port: self.entry.port,
            line: self.entry.line,
        }
    }
}

/// `OnceCell::set_or_get` writes in place, so `&'static KeyState` stays
/// valid for IRQ-context use. Same property the CAN driver relies on.
static SLOTS: [OnceCell<KeyState>; KEY_MAX] = [const { OnceCell::new() }; KEY_MAX];

pub struct Key {
    state: &'static KeyState,
}

impl Key {
    /// Open by `/aliases` entry, e.g. `"sw0"`.
    pub fn open_by_alias(name: &str) -> Result<Self> {
        let entry = hal::device_tree::key_by_alias(name)
            .ok_or_else(|| kerr!(ENODEV, "key alias not found: {name}"))?;
        Self::open_for_node(entry.node)
    }

    /// Open by the DT `label` property.
    pub fn open_by_label(label: &str) -> Result<Self> {
        let entry = hal::device_tree::key_by_label(label)
            .ok_or_else(|| kerr!(ENODEV, "key label not found: {label}"))?;
        Self::open_for_node(entry.node)
    }

    /// Open by `osiris,code` / `zephyr,code` value.
    pub fn open_by_code(code: u32) -> Result<Self> {
        let entry = hal::device_tree::key_by_code(code)
            .ok_or_else(|| kerr!(ENODEV, "key code not found: {code}"))?;
        Self::open_for_node(entry.node)
    }

    fn open_for_node(node: usize) -> Result<Self> {
        for cell in SLOTS.iter() {
            if let Some(state) = cell.get() {
                if state.entry.node == node {
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

    /// Block until the next edge, then return the event built from the
    /// current line level.
    pub fn wait(&self) -> Result<KeyEvent> {
        let uid = crate::sched::with(|s| s.current_uid())
            .ok_or_else(|| kerr!(EINVAL, "key::wait with no current thread"))?
            as u32;
        if uid == 0 {
            return Err(kerr!(EINVAL, "idle thread cannot wait on a key"));
        }

        // Mask IRQs around arm+park: an edge that fires between the two
        // would otherwise kick a thread that's not yet in the wakeup tree.
        crate::sync::atomic::irq_free(|| {
            self.state.waiter.arm(uid);
            crate::sched::with(|s| {
                let _ = s.sleep_until(u64::MAX, crate::time::tick());
            });
        });
        self.state.waiter.disarm();

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
    if ctx.is_null() {
        return;
    }
    // SAFETY: `ctx` is the `&'static KeyState` installed by `init`, backed by SLOTS.
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
    let n = hal::device_tree::KEY_REGISTRY.len();
    kprintln!("Found {n} gpio-key entries");

    // One dispatcher registration per shared NVIC slot — EXTI9_5 and
    // EXTI15_10 are aggregates over multiple lines.
    let mut seen_slots: u64 = 0;

    for (i, entry) in hal::device_tree::KEY_REGISTRY.iter().enumerate() {
        if i >= KEY_MAX {
            kprintln!("    Key registry exceeds KEY_MAX={KEY_MAX}");
            break;
        }

        let state = KeyState {
            entry,
            waiter: ParkedWaiter::new(),
            last_event_mono: AtomicU64::new(0),
            debounce_ticks: debounce_to_ticks(entry.debounce_ms),
        };
        let state_ref: &'static KeyState = SLOTS[i].set_or_get(state);
        let pin = state_ref.pin();

        let Some(vector) = hal::gpio::nvic_vector_for_line(entry.line) else {
            kprintln!("    Key {} has invalid line {}", entry.label, entry.line);
            continue;
        };
        let bit = 1u64 << (vector & 63);
        if seen_slots & bit == 0 {
            seen_slots |= bit;
            if let Err(e) =
                unsafe { crate::irq::register_irq(vector, hal::gpio::dispatch, None) }
            {
                kprintln!("    register_irq({vector}) failed: {:?}", e);
                continue;
            }
        }

        // Pull complements the active level so the idle state is unambiguous.
        let pull = if entry.active_low != 0 {
            Pull::Up
        } else {
            Pull::Down
        };
        if let Err(e) = hal::gpio::configure_input(pin, pull) {
            kprintln!("    configure_input({}): {:?}", entry.label, e);
            continue;
        }

        let ctx = state_ref as *const KeyState as *mut ();
        // Both edges so press and release both surface as events.
        if let Err(e) = hal::gpio::register_edge_handler(
            pin,
            Edges::BOTH,
            on_edge,
            ctx,
            KEY_NVIC_PRIORITY,
        ) {
            kprintln!("    register_edge_handler({}): {:?}", entry.label, e);
            continue;
        }

        kprintln!(
            "    Initialized key {} on port 0x{:x} line {}",
            entry.label,
            entry.port,
            entry.line
        );
    }
}
