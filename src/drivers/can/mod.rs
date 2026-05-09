use core::num::NonZeroU32;

use crate::hal;
pub use hal::can::{BusState, BusStatus, Diag, Error, Filter, Frame, Mode};

pub mod wait;

pub type Result<T> = hal::can::Result<T>;

#[derive(Clone, Copy)]
pub struct Config {
    /// Override bitrate. `None` uses the device tree's `bitrate` property.
    pub bitrate_hz: Option<NonZeroU32>,
    pub mode: Mode,
    /// TX mailbox-free busy-loop limit, in iterations. ~80_000 ≈ 1 ms at
    /// 80 MHz.
    pub tx_timeout_iters: NonZeroU32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bitrate_hz: None,
            mode: Mode::Normal,
            tx_timeout_iters: NonZeroU32::new(80_000).unwrap(),
        }
    }
}

pub struct Device {
    desc: hal::can::Device,
    tx_timeout_iters: NonZeroU32,
}

impl Device {
    pub fn open(compatible: &str, ordinal: usize, config: Config) -> Result<Self> {
        let desc = hal::can::get(compatible, ordinal)?;
        let bitrate_hz = config
            .bitrate_hz
            .or_else(|| NonZeroU32::new(desc.bitrate_hz()))
            .ok_or(Error::BitrateInfeasible)?;

        // Wire IRQ vector before HAL init enables NVIC.
        wait::ensure_registered(&desc)?;

        hal::can::init(&desc, bitrate_hz, config.mode)?;
        Ok(Self {
            desc,
            tx_timeout_iters: config.tx_timeout_iters,
        })
    }

    pub fn transmit(&self, frame: &Frame) -> Result<()> {
        hal::can::transmit(&self.desc, frame, self.tx_timeout_iters)
    }

    pub fn receive(&self, out: &mut Frame) -> Result<bool> {
        hal::can::receive(&self.desc, out)
    }

    pub fn configure_filter(&self, filter: &Filter) -> Result<()> {
        hal::can::configure_filter(&self.desc, filter)
    }

    pub fn bus_status(&self) -> BusStatus {
        hal::can::bus_status(&self.desc)
    }

    pub fn recover(&self) -> Result<()> {
        hal::can::recover(&self.desc)
    }

    pub fn diag(&self) -> Diag {
        hal::can::diag(&self.desc)
    }

    pub fn slot(&self) -> u8 {
        self.desc.index()
    }

    /// Park `uid` as the single waiter on this controller. One consumer
    /// per controller; a second `register_waiter` overwrites the first.
    pub fn register_waiter(&self, uid: u32) {
        wait::register_waiter(self.slot(), uid);
    }

    pub fn unregister_waiter(&self) {
        wait::unregister_waiter(self.slot());
    }
}

pub fn init() {}
