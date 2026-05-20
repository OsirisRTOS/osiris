//! gpio-leds kernel driver.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::error::Result;
use crate::hal;
use crate::sync::once::OnceCell;

use hal::device_tree::{LedDefaultState, LedOutputMode, LedPull, LedRegistryEntry};
use hal::gpio::{Level, Pin, Pull};

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

fn pin_of(entry: &LedRegistryEntry) -> Pin {
    Pin {
        port: entry.port,
        line: entry.line,
    }
}

fn level_for(entry: &LedRegistryEntry, on: bool) -> Level {
    let active_low = entry.active_low != 0;
    if on ^ active_low {
        Level::High
    } else {
        Level::Low
    }
}

fn keep_initial_level(entry: &LedRegistryEntry) -> Level {
    let pin = pin_of(entry);
    if let Err(e) = hal::gpio::enable_port_clock(pin) {
        kprintln!(
            "    LED {}: keep: enable_port_clock failed ({:?}); falling back to Off",
            entry.label,
            e,
        );
        return level_for(entry, false);
    }
    match hal::gpio::read_odr(pin) {
        Ok(Level::High) => Level::High,
        Ok(Level::Low) => level_for(entry, false),
        Err(e) => {
            kprintln!(
                "    LED {}: keep: read_odr failed ({:?}); falling back to Off",
                entry.label,
                e,
            );
            level_for(entry, false)
        }
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
            LedDefaultState::Keep => keep_initial_level(entry),
        };
        let pull = match entry.pull {
            LedPull::None => Pull::None,
            LedPull::Up => Pull::Up,
            LedPull::Down => Pull::Down,
        };
        let res = match entry.output_mode {
            LedOutputMode::OpenDrain => {
                hal::gpio::configure_output_od(pin_of(entry), initial, pull)
            }
            LedOutputMode::PushPull => hal::gpio::configure_output(pin_of(entry), initial, pull),
        };
        match res {
            Ok(()) => {
                state_ref.initialized.store(true, Ordering::Release);
                kprintln!(
                    "    Initialized LED {} on port 0x{:x} line {} ({:?}, {:?}, {:?})",
                    entry.label,
                    entry.port,
                    entry.line,
                    entry.default_state,
                    entry.output_mode,
                    entry.pull,
                );
            }
            Err(e) => kprintln!("    LED {}: configure_output failed: {:?}", entry.label, e),
        }
    }
}
