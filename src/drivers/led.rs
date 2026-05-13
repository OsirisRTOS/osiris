//! gpio-leds kernel driver. Hardware-independent — uses only
//! `hal::gpio` and `hal::device_tree`. LEDs are stateless outputs, so
//! there's no per-device kernel state beyond the generated registry entry.

use crate::error::Result;
use crate::hal;

use hal::device_tree::LedRegistryEntry;
use hal::gpio::{Level, Pin};

pub struct Led {
    entry: &'static LedRegistryEntry,
}

impl Led {
    /// Open by `/aliases` entry, e.g. `"led0"`.
    pub fn open_by_alias(name: &str) -> Result<Self> {
        hal::device_tree::led_by_alias(name)
            .map(|entry| Self { entry })
            .ok_or_else(|| kerr!(ENODEV, "led alias not found: {name}"))
    }

    /// Open by the DT `label` property.
    pub fn open_by_label(label: &str) -> Result<Self> {
        hal::device_tree::led_by_label(label)
            .map(|entry| Self { entry })
            .ok_or_else(|| kerr!(ENODEV, "led label not found: {label}"))
    }

    pub fn label(&self) -> &'static str {
        self.entry.label
    }

    pub fn on(&self) -> Result<()> {
        hal::gpio::write(pin_of(self.entry), level_for(self.entry, true))?;
        Ok(())
    }

    pub fn off(&self) -> Result<()> {
        hal::gpio::write(pin_of(self.entry), level_for(self.entry, false))?;
        Ok(())
    }

    pub fn set(&self, on: bool) -> Result<()> {
        hal::gpio::write(pin_of(self.entry), level_for(self.entry, on))?;
        Ok(())
    }

    pub fn toggle(&self) -> Result<()> {
        hal::gpio::toggle(pin_of(self.entry))?;
        Ok(())
    }
}

fn pin_of(entry: &LedRegistryEntry) -> Pin {
    Pin {
        port: entry.port,
        line: entry.line,
    }
}

/// Physical level the line must drive for the requested logical state.
fn level_for(entry: &LedRegistryEntry, on: bool) -> Level {
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

    for entry in entries {
        // Drive the "off" level before switching to output so the line
        // never glitches the active polarity on boot.
        let off = level_for(entry, false);
        match hal::gpio::configure_output(pin_of(entry), off) {
            Ok(()) => kprintln!(
                "    Initialized LED {} on port 0x{:x} line {}",
                entry.label,
                entry.port,
                entry.line
            ),
            Err(e) => kprintln!("    LED {}: configure_output failed: {:?}", entry.label, e),
        }
    }
}
