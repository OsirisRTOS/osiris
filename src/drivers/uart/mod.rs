pub mod wait;

pub use crate::hal::uart::{Error, Overrides};

use core::time::Duration;

use crate::sched;
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
    /// Default timeout for [`Device::read_blocking`]. `None` blocks
    /// forever; per-call timeouts override.
    pub read_timeout: Option<Duration>,
    /// Default timeout for [`Device::write_blocking`]. `None` blocks
    /// forever; per-call timeouts override.
    pub write_timeout: Option<Duration>,
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
        if let Err(e) = wait::ensure_registered(&desc) {
            let _ = crate::hal::uart::deinit(&desc);
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

    /// Block until at least one byte is available or the device's
    /// configured `read_timeout` expires.
    pub fn read_blocking(&self, buf: &mut [u8]) -> Result<usize, Error> {
        self.read_with_timeout(buf, self.read_timeout)
    }

    /// `None` blocks forever; `Some(d)` returns `Err(TimedOut)` if no
    /// byte arrives within `d`.
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
        let waiter = wait::Waiter::new(uid);
        wait::register_rx_waiter(self.slot(), &waiter);
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
        wait::unregister_rx_waiter(self.slot(), &waiter);
        result
    }

    /// Block until the entire buffer is enqueued or the device's
    /// configured `write_timeout` expires.
    pub fn write_blocking(&self, buf: &[u8]) -> Result<(), Error> {
        self.write_with_timeout(buf, self.write_timeout)
    }

    /// `None` blocks forever; `Some(d)` returns `Err(TimedOut)` if the
    /// buffer can't be fully enqueued within `d`. Partial progress is
    /// not surfaced; use [`Self::write_nb`] if you need that.
    pub fn write_with_timeout(&self, buf: &[u8], timeout: Option<Duration>) -> Result<(), Error> {
        if buf.is_empty() {
            return Ok(());
        }
        let uid = match sched::with(|s| s.current_uid()) {
            Some(u) => u as u32,
            None => return Err(Error::Io),
        };
        let deadline = timeout.map(|d| time::tick().saturating_add(time::duration_to_ticks(d)));
        let waiter = wait::Waiter::new(uid);
        wait::register_tx_waiter(self.slot(), &waiter);
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
        wait::unregister_tx_waiter(self.slot(), &waiter);
        result
    }

    pub fn register_rx_waiter(&self, w: &wait::Waiter) {
        wait::register_rx_waiter(self.slot(), w);
    }
    pub fn unregister_rx_waiter(&self, w: &wait::Waiter) {
        wait::unregister_rx_waiter(self.slot(), w);
    }
    pub fn register_tx_waiter(&self, w: &wait::Waiter) {
        wait::register_tx_waiter(self.slot(), w);
    }
    pub fn unregister_tx_waiter(&self, w: &wait::Waiter) {
        wait::unregister_tx_waiter(self.slot(), w);
    }

    pub fn set_rx_callback(&self, cb: Option<wait::ChannelCallback>) {
        wait::set_rx_callback(self.slot(), cb);
    }
    pub fn set_tx_callback(&self, cb: Option<wait::ChannelCallback>) {
        wait::set_tx_callback(self.slot(), cb);
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = crate::hal::uart::deinit(&self.desc);
    }
}
