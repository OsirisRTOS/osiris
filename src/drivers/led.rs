//! gpio-leds kernel driver.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::error::Result;
use crate::hal;
use crate::sync::once::OnceCell;

use hal::device_tree::{LedDefaultState, LedRegistryEntry};
use hal::gpio::{Level, Pin};

struct LedState {
    entry: &'static LedRegistryEntry,
    /// `Led::open_*` rejects handles whose init failed.
    initialized: AtomicBool,
}

static SLOTS: [OnceCell<LedState>; hal::device_tree::LED_REGISTRY.len()] =
    [const { OnceCell::new() }; hal::device_tree::LED_REGISTRY.len()];

pub struct Led {
    state: &'static LedState,
}

impl Led {
    pub fn open_by_alias(name: &str) -> Result<Self> {
        let entry = hal::device_tree::led_by_alias(name)
            .ok_or_else(|| kerr!(ENODEV, "led alias not found: {name}"))?;
        Self::open_for_node(entry.node)
    }

    pub fn open_by_label(label: &str) -> Result<Self> {
        let entry = hal::device_tree::led_by_label(label)
            .ok_or_else(|| kerr!(ENODEV, "led label not found: {label}"))?;
        Self::open_for_node(entry.node)
    }

    fn open_for_node(node: usize) -> Result<Self> {
        for cell in SLOTS.iter() {
            if let Some(state) = cell.get() {
                if state.entry.node == node {
                    if !state.initialized.load(Ordering::Acquire) {
                        return Err(kerr!(EIO, "LED node {node} init failed"));
                    }
                    return Ok(Self { state });
                }
            }
        }
        Err(kerr!(ENODEV, "LED node {node} not registered"))
    }

    pub fn label(&self) -> &'static str {
        self.state.entry.label
    }

    pub fn on(&self) -> Result<()> {
        self.set(true)
    }

    pub fn off(&self) -> Result<()> {
        self.set(false)
    }

    pub fn set(&self, on: bool) -> Result<()> {
        hal::gpio::write(pin_of(self.state.entry), level_for(self.state.entry, on))
            .map_err(Into::into)
    }

    pub fn toggle(&self) -> Result<()> {
        hal::gpio::toggle(pin_of(self.state.entry)).map_err(Into::into)
    }
}

const fn pin_of(entry: &LedRegistryEntry) -> Pin {
    Pin {
        port: entry.port,
        line: entry.line,
    }
}

const fn level_for(entry: &LedRegistryEntry, on: bool) -> Level {
    let active_low = entry.active_low != 0;
    if on ^ active_low {
        Level::High
    } else {
        Level::Low
    }
}

pub fn init() {
    let entries = hal::device_tree::LED_REGISTRY;
    kprintln!("Found {} gpio-led entries", entries.len());

    for (i, entry) in entries.iter().enumerate() {
        let state = LedState {
            entry,
            initialized: AtomicBool::new(false),
        };
        let state_ref: &'static LedState = SLOTS[i].set_or_get(state);

        let initial = match entry.default_state {
            LedDefaultState::Off => level_for(entry, false),
            LedDefaultState::On => level_for(entry, true),
            LedDefaultState::Keep => {
                // enable clock, as we read before configure_output, which internally turns on clock
                let _ = hal::gpio::enable_port_clock(pin_of(entry)).map_err(|e| {
                    kprintln!("    LED {}: enable_port_clock failed: {:?}", entry.label, e)
                });
                hal::gpio::read(pin_of(entry)).unwrap_or(level_for(entry, false))
            }
        };
        match hal::gpio::configure_output(pin_of(entry), initial) {
            Ok(()) => {
                state_ref.initialized.store(true, Ordering::Release);
                kprintln!(
                    "    Initialized LED {} on port 0x{:x} line {} ({:?})",
                    entry.label,
                    entry.port,
                    entry.line,
                    entry.default_state,
                );
            }
            Err(e) => kprintln!("    LED {}: configure_output failed: {:?}", entry.label, e),
        }
    }
}
