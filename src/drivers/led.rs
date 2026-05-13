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
        let entry = hal::device_tree::led_by_alias(name)
            .ok_or_else(|| kerr!(ENODEV, "led alias not found: {name}"))?;
        Ok(Self { entry })
    }

    /// Open by the DT `label` property.
    pub fn open_by_label(label: &str) -> Result<Self> {
        let entry = hal::device_tree::led_by_label(label)
            .ok_or_else(|| kerr!(ENODEV, "led label not found: {label}"))?;
        Ok(Self { entry })
    }

    fn pin(&self) -> Pin {
        Pin {
            port: self.entry.port,
            line: self.entry.line,
        }
    }

    fn level_for(&self, on: bool) -> Level {
        let active_low = self.entry.active_low != 0;
        if on ^ active_low {
            Level::High
        } else {
            Level::Low
        }
    }

    pub fn on(&self) -> Result<()> {
        Ok(hal::gpio::write(self.pin(), self.level_for(true))?)
    }

    pub fn off(&self) -> Result<()> {
        Ok(hal::gpio::write(self.pin(), self.level_for(false))?)
    }

    pub fn set(&self, on: bool) -> Result<()> {
        Ok(hal::gpio::write(self.pin(), self.level_for(on))?)
    }

    pub fn toggle(&self) -> Result<()> {
        Ok(hal::gpio::toggle(self.pin())?)
    }

    pub fn label(&self) -> &'static str {
        self.entry.label
    }
}

pub fn init() {
    kprintln!(
        "Found {} gpio-led entries",
        hal::device_tree::LED_REGISTRY.len()
    );
    for entry in hal::device_tree::LED_REGISTRY {
        let pin = Pin {
            port: entry.port,
            line: entry.line,
        };
        // Drive to logical "off" before init so the line never glitches on boot.
        let initial = if entry.active_low != 0 {
            Level::High
        } else {
            Level::Low
        };
        match hal::gpio::configure_output(pin, initial) {
            Ok(()) => kprintln!(
                "    Initialized LED {} on port 0x{:x} line {}",
                entry.label,
                entry.port,
                entry.line
            ),
            Err(e) => kprintln!(
                "    Failed to initialize LED {}: {:?}",
                entry.label,
                e
            ),
        }
    }
}
