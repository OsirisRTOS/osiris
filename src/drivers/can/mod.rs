use crate::hal;
pub use hal::can::{BusState, BusStatus, Diag, Error, Filter, Frame, Mode};

pub mod wait;

pub type Result<T> = hal::can::Result<T>;

#[derive(Clone, Copy, Default)]
pub struct Config {
    pub mode: Mode,
}

pub struct Device {
    desc: hal::can::Device,
}

impl Device {
    pub fn open(compatible: &str, ordinal: usize, config: Config) -> Result<Self> {
        let desc = hal::can::get(compatible, ordinal)?;

        // Wire IRQ vector before HAL init enables NVIC.
        wait::ensure_registered(&desc)?;

        hal::can::init(&desc, config.mode)?;
        Ok(Self { desc })
    }

    /// Bring the bus online
    pub fn start(&self) -> Result<()> {
        hal::can::start(&self.desc)
    }

    pub fn transmit(&self, frame: &Frame) -> Result<()> {
        hal::can::transmit(&self.desc, frame)
    }

    pub fn receive(&self, out: &mut Frame) -> Result<bool> {
        hal::can::receive(&self.desc, out)
    }

    /// Configure a hardware filter. Prefer calling this before start
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
