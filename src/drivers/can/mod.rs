use core::num::NonZeroU32;

use crate::hal;
pub use hal::can::{
    BusState, BusStatus, Diag, Error, Filter, Frame, Mode,
};

pub mod wait;

pub type Result<T> = hal::can::Result<T>;

#[derive(Clone, Copy)]
pub struct Config {
    /// Override bitrate. `None` uses the value declared in the device
    /// tree's `bitrate` property.
    pub bitrate_hz: Option<NonZeroU32>,
    pub mode: Mode,
    /// TX mailbox-free busy-loop limit in iterations. ~80_000 is roughly
    /// 1 ms at 80 MHz; tune for clock speed and how aggressively callers
    /// want to give up on a stuck bus.
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

#[derive(Clone)]
pub struct Device {
    desc: hal::can::Device,
    bitrate_hz: NonZeroU32,
    mode: Mode,
    tx_timeout_iters: NonZeroU32,
}

impl Device {
    pub fn open(compatible: &str, ordinal: usize, config: Config) -> Result<Self> {
        let desc = hal::can::get(compatible, ordinal)?;
        let bitrate_hz = config
            .bitrate_hz
            .or_else(|| NonZeroU32::new(desc.bitrate_hz()))
            .ok_or(Error::BitrateInfeasible)?;

        // Wire the IRQ handler before init enables NVIC; otherwise a
        // frame in the gap stalls the vector. See `wait::ensure_registered`.
        wait::ensure_registered(&desc)?;

        hal::can::init(&desc, bitrate_hz, config.mode)?;
        Ok(Self {
            desc,
            bitrate_hz,
            mode: config.mode,
            tx_timeout_iters: config.tx_timeout_iters,
        })
    }

    pub fn close(self) -> Result<()> {
        hal::can::deinit(&self.desc)
    }

    pub fn transmit(&self, frame: &Frame) -> Result<()> {
        hal::can::transmit(&self.desc, frame, self.tx_timeout_iters)
    }

    pub fn receive(&self, out: &mut Frame) -> Result<bool> {
        hal::can::receive(&self.desc, out)
    }

    pub fn bitrate_hz(&self) -> NonZeroU32 {
        self.bitrate_hz
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn configure_filter(&self, filter: &Filter) -> Result<()> {
        hal::can::configure_filter(&self.desc, filter)
    }

    pub fn disable_filter(&self, bank: u8) -> Result<()> {
        hal::can::disable_filter(&self.desc, bank)
    }

    pub fn last_error(&self) -> u32 {
        hal::can::last_error(&self.desc)
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

    /// DT-assigned slot index. Pass to [`wait`] APIs to address per-slot
    /// state.
    pub fn slot(&self) -> u8 {
        self.desc.index()
    }

    /// Enqueue `w` on this device's wait list. Producer (RX ISR) will
    /// kick `w.uid()` on the next ring push. Caller must keep `w` alive
    /// — i.e. stack-pinned across the sleep — and call
    /// [`Self::unregister_waiter`] before `w` is dropped or moved.
    pub fn register_waiter(&self, w: &wait::Waiter) {
        wait::register_waiter(self.slot(), w);
    }

    pub fn unregister_waiter(&self, w: &wait::Waiter) {
        wait::unregister_waiter(self.slot(), w);
    }

    /// Install (or clear, with `None`) an ISR-context callback that fires
    /// after every successful RX ring push. Independent of the waiter
    /// list; useful for non-thread consumers (e.g. CSP queue producers).
    pub fn set_rx_callback(&self, cb: Option<wait::RxCallback>) {
        wait::set_rx_callback(self.slot(), cb);
    }
}

pub fn init() {
    // Nothing to do at boot — the kernel CAN driver is open-on-demand.
    // `Device::open` will register the per-slot IRQ handler the first
    // time each peripheral is opened.
}
