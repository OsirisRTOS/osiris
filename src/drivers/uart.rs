pub use crate::hal::uart::{Error, Overrides};

use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;

use crate::sched;
use crate::sync::waiter::ParkedWaiter;
use crate::time;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DataBits {
    Seven,
    #[default]
    Eight,
    Nine,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StopBits {
    #[default]
    One,
    Two,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Parity {
    #[default]
    None,
    Odd,
    Even,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlowControl {
    #[default]
    None,
    RtsCts,
}

#[derive(Clone, Copy, Default)]
pub struct Config {
    pub baud: Option<u32>,
    pub data_bits: Option<DataBits>,
    pub stop_bits: Option<StopBits>,
    pub parity: Option<Parity>,
    pub flow_control: Option<FlowControl>,
    /// `read_blocking` default; `None` blocks forever.
    pub read_timeout: Option<Duration>,
    /// `write_blocking` default; `None` blocks forever.
    pub write_timeout: Option<Duration>,
}

pub const UART_SLOT_COUNT: usize = 6;

/// Latched once the process-global vector→`vector_dispatch` mapping is
/// live (it outlives every `Device`); set only after `register_irq`
/// succeeds. Per-Device `slot->cb` is re-installed each `open` instead.
static VECTOR_REGISTERED: [AtomicBool; UART_SLOT_COUNT] =
    [const { AtomicBool::new(false) }; UART_SLOT_COUNT];

struct UartSlotWaiters {
    rx: ParkedWaiter,
    tx: ParkedWaiter,
}

impl UartSlotWaiters {
    const fn new() -> Self {
        Self {
            rx: ParkedWaiter::new(),
            tx: ParkedWaiter::new(),
        }
    }
}

// One reader + one writer thread per slot; a second is rejected with `Error::Busy`.
static UART_WAITERS: [UartSlotWaiters; UART_SLOT_COUNT] =
    [const { UartSlotWaiters::new() }; UART_SLOT_COUNT];

pub struct Device {
    desc: crate::hal::uart::Device,
    read_timeout: Option<Duration>,
    write_timeout: Option<Duration>,
}

impl Device {
    pub fn open(compatible: &str, ordinal: usize, cfg: Config) -> Result<Self, Error> {
        let desc = crate::hal::uart::get(compatible, ordinal)?;
        let overrides = Overrides {
            baud: cfg.baud,
            data_bits: cfg.data_bits.map(data_bits_to_u8),
            stop_bits: cfg.stop_bits.map(stop_bits_to_u8),
            parity: cfg.parity.map(parity_to_u8),
            flow_control: cfg.flow_control.map(flow_to_u8),
        };
        crate::hal::uart::init(&desc, &overrides)?;
        if let Err(e) = ensure_registered(&desc) {
            if let Err(de) = crate::hal::uart::deinit(&desc) {
                warn!("uart: deinit during open cleanup failed: {:?}", de);
            }
            return Err(e);
        }
        Ok(Self {
            desc,
            read_timeout: cfg.read_timeout,
            write_timeout: cfg.write_timeout,
        })
    }

    pub fn slot(&self) -> u8 {
        self.desc.index()
    }

    pub fn write_nb(&self, buf: &[u8]) -> Result<usize, Error> {
        match crate::hal::uart::transmit_nb(&self.desc, buf) {
            Ok(n) => Ok(n),
            Err(Error::WouldBlock) => Ok(0),
            Err(e) => Err(e),
        }
    }

    pub fn read_nb(&self, buf: &mut [u8]) -> Result<usize, Error> {
        match crate::hal::uart::receive_nb(&self.desc, buf) {
            Ok(n) => Ok(n),
            Err(Error::WouldBlock) => Ok(0),
            Err(e) => Err(e),
        }
    }

    pub fn read_blocking(&self, buf: &mut [u8]) -> Result<usize, Error> {
        self.read_with_timeout(buf, self.read_timeout)
    }

    /// `None` blocks forever. `Busy` if another thread already reads this UART.
    pub fn read_with_timeout(
        &self,
        buf: &mut [u8],
        timeout: Option<Duration>,
    ) -> Result<usize, Error> {
        if buf.is_empty() {
            return Ok(0);
        }
        let uid = match sched::with(|s| s.current_uid()) {
            Some(u) => u as u32,
            None => return Err(Error::Io),
        };
        let deadline = timeout.map(|d| time::tick().saturating_add(time::duration_to_ticks(d)));
        register_rx_waiter(self.slot(), uid)?;
        let result = loop {
            let mut got: Result<usize, Error> = Ok(0);
            let exit = sched::with(|s| {
                got = self.read_nb(buf);
                match got {
                    Ok(0) => {
                        let now = time::tick();
                        match deadline {
                            Some(d) if now >= d => {
                                got = Err(Error::TimedOut);
                                true
                            }
                            Some(d) => {
                                let _ = s.sleep_until(None, d, now);
                                false
                            }
                            None => {
                                let _ = s.sleep_until(None, u64::MAX, now);
                                false
                            }
                        }
                    }
                    _ => true,
                }
            });
            if exit {
                break got;
            }
        };
        unregister_rx_waiter(self.slot());
        result
    }

    pub fn write_blocking(&self, buf: &[u8]) -> Result<(), Error> {
        self.write_with_timeout(buf, self.write_timeout)
    }

    /// Returns once the whole buffer is enqueued. `None` blocks forever;
    /// on `TimedOut`, partial progress is not reported (use `write_nb`).
    /// `Busy` if another thread already writes this UART.
    pub fn write_with_timeout(&self, buf: &[u8], timeout: Option<Duration>) -> Result<(), Error> {
        if buf.is_empty() {
            return Ok(());
        }
        let uid = match sched::with(|s| s.current_uid()) {
            Some(u) => u as u32,
            None => return Err(Error::Io),
        };
        let deadline = timeout.map(|d| time::tick().saturating_add(time::duration_to_ticks(d)));
        register_tx_waiter(self.slot(), uid)?;
        let mut sent = 0usize;
        let result = loop {
            let mut step: Result<usize, Error> = Ok(0);
            let exit = sched::with(|s| {
                step = self.write_nb(&buf[sent..]);
                match step {
                    Ok(0) => {
                        let now = time::tick();
                        match deadline {
                            Some(d) if now >= d => {
                                step = Err(Error::TimedOut);
                                true
                            }
                            Some(d) => {
                                let _ = s.sleep_until(None, d, now);
                                false
                            }
                            None => {
                                let _ = s.sleep_until(None, u64::MAX, now);
                                false
                            }
                        }
                    }
                    Ok(_) => true,
                    Err(_) => true,
                }
            });
            if exit {
                match step {
                    Ok(n) => {
                        sent += n;
                        if sent >= buf.len() {
                            break Ok(());
                        }
                    }
                    Err(e) => break Err(e),
                }
            }
        };
        unregister_tx_waiter(self.slot());
        result
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        if let Err(e) = crate::hal::uart::deinit(&self.desc) {
            warn!("uart: deinit on drop failed: {:?}", e);
        }
    }
}

fn data_bits_to_u8(d: DataBits) -> u8 {
    match d {
        DataBits::Seven => 7,
        DataBits::Eight => 8,
        DataBits::Nine => 9,
    }
}

fn stop_bits_to_u8(s: StopBits) -> u8 {
    match s {
        StopBits::One => 1,
        StopBits::Two => 2,
    }
}

fn parity_to_u8(p: Parity) -> u8 {
    match p {
        Parity::None => 0,
        Parity::Odd => 1,
        Parity::Even => 2,
    }
}

fn flow_to_u8(f: FlowControl) -> u8 {
    match f {
        FlowControl::None => 0,
        FlowControl::RtsCts => 1,
    }
}

fn register_rx_waiter(slot: u8, uid: u32) -> Result<(), Error> {
    UART_WAITERS[slot as usize]
        .rx
        .arm(uid as usize)
        .map_err(|_| Error::Busy)
}

fn unregister_rx_waiter(slot: u8) {
    UART_WAITERS[slot as usize].rx.disarm();
}

fn register_tx_waiter(slot: u8, uid: u32) -> Result<(), Error> {
    UART_WAITERS[slot as usize]
        .tx
        .arm(uid as usize)
        .map_err(|_| Error::Busy)
}

fn unregister_tx_waiter(slot: u8) {
    UART_WAITERS[slot as usize].tx.disarm();
}

extern "C" fn kernel_dispatch(kind: crate::hal::uart::Irq, ctx: *mut ()) {
    if ctx.is_null() {
        return;
    }
    // SAFETY: `ctx` is the `&'static UartSlotWaiters` installed by
    // `ensure_registered` and round-tripped by the HAL; `ParkedWaiter`
    // is atomic, so concurrent ISR/thread access has no aliasing `&mut`.
    let w = unsafe { &*(ctx as *const UartSlotWaiters) };
    match kind {
        crate::hal::uart::Irq::Rx => w.rx.wake(),
        crate::hal::uart::Irq::TxDone => w.tx.wake(),
    }
}

fn vector_dispatch(_ctx: *mut u8, _vector: usize, userdata: Option<usize>) {
    let Some(slot) = userdata else {
        return;
    };
    crate::hal::uart::dispatch_by_slot(slot as u8);
}

/// Must be called *after* `hal::uart::init` — `uart_set_irq_handler` looks up
/// the slot by `in_use`, which only `uart_init` sets.
pub fn ensure_registered(dev: &crate::hal::uart::Device) -> Result<(), Error> {
    let slot = dev.index();
    if (slot as usize) >= UART_SLOT_COUNT {
        return Err(Error::InvalidArgument);
    }

    // Claim the once-only vector install via CAS; release on failure so
    // a later `open` can retry. Latch true only after success.
    if VECTOR_REGISTERED[slot as usize]
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        // IPSR = NVIC line + 16: kernel `HANDLERS` is IPSR-indexed.
        let vector = dev.irqn() as usize + 16;
        if let Err(e) =
            unsafe { crate::irq::register_irq(vector, vector_dispatch, Some(slot as usize)) }
        {
            VECTOR_REGISTERED[slot as usize].store(false, Ordering::Release);
            warn!("uart: irq vector registration failed: {:?}", e);
            return Err(Error::Io);
        }
    }

    // `deinit` clears the C slot's `cb`, so re-install it on every open
    let ctx = &UART_WAITERS[slot as usize] as *const _ as *mut ();
    crate::hal::uart::register_irq_handler(dev, Some(kernel_dispatch), ctx)
}
