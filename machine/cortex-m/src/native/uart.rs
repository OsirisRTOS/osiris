use super::bindings;
use super::device_tree;

pub(crate) fn console_entry() -> Option<&'static device_tree::UartRegistryEntry> {
    device_tree::CONSOLE_UART.map(|i| &device_tree::UART_REGISTRY[i])
}

pub(crate) fn init_console_from_dt() -> Result<()> {
    let Some(entry) = console_entry() else {
        return Ok(());
    };
    let cfg = cfg_from_entry(entry, &Overrides::default());
    let rc = unsafe { bindings::uart_init_console(&cfg as *const _) };
    if rc < 0 {
        return Err(from_c_rc(rc));
    }
    Ok(())
}

/// Length-proportional console TX timeout: slack over the line-rate
/// transmit time, capped so the PendSV-masked window stays bounded
/// (caller masks PendSV across this). Beyond the cap, lines may truncate.
const CONSOLE_TX_SLACK_MS: u32 = 50;
const CONSOLE_TX_MAX_MS: u32 = 100;

pub(crate) fn console_write(buf: &[u8]) -> Result<()> {
    let Some(entry) = console_entry() else {
        return Ok(());
    };
    if buf.is_empty() {
        return Ok(());
    }

    // 10 bits/byte (8N1).
    let per_byte_us = 10_000_000 / entry.baud.max(1);
    let budget_ms = (buf.len() as u32).saturating_mul(per_byte_us) / 1000;
    let timeout_ms = CONSOLE_TX_SLACK_MS
        .saturating_add(budget_ms)
        .min(CONSOLE_TX_MAX_MS);
    let rc = unsafe {
        bindings::uart_transmit_blocking(
            entry.instance,
            buf.as_ptr(),
            buf.len() as i32,
            timeout_ms,
        )
    };
    if rc < 0 {
        return Err(from_c_rc(rc));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidArgument,
    NoSuchDevice,
    NotInitialized,
    OutOfMemory,
    Busy,
    WouldBlock,
    TimedOut,
    Io,
}

pub type Result<T> = core::result::Result<T, Error>;

fn from_c_rc(rc: i32) -> Error {
    match rc {
        -1 => Error::InvalidArgument,
        -2 => Error::OutOfMemory,
        -3 => Error::Busy,
        -4 => Error::WouldBlock,
        -5 => Error::Io,
        _ => Error::Io,
    }
}

#[derive(Clone, Copy)]
pub struct Device(&'static device_tree::UartRegistryEntry);

impl Device {
    pub fn entry(&self) -> &'static device_tree::UartRegistryEntry {
        self.0
    }

    pub fn index(&self) -> u8 {
        self.0.index
    }

    pub fn instance(&self) -> usize {
        self.0.instance
    }

    pub fn irqn(&self) -> u8 {
        self.0.irqn
    }
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Irq {
    Rx = 0,
    TxDone = 1,
}

pub type IrqHandler = extern "C" fn(Irq, *mut ());

pub fn get_by_index(idx: u8) -> Result<Device> {
    device_tree::uart_by_index(idx)
        .map(Device)
        .ok_or(Error::NoSuchDevice)
}

pub fn get(compatible: &str, ordinal: usize) -> Result<Device> {
    device_tree::uart_by_compatible(compatible, ordinal)
        .map(Device)
        .ok_or(Error::NoSuchDevice)
}

fn cfg_from_entry(
    entry: &device_tree::UartRegistryEntry,
    overrides: &Overrides,
) -> bindings::uart_bus_cfg_t {
    let mk_pin = |port: usize, line: u8, af: u8| bindings::uart_pin_cfg_t {
        port,
        pin: line,
        af,
        reserved: 0,
    };
    let zero_pin = bindings::uart_pin_cfg_t {
        port: 0,
        pin: 0,
        af: 0,
        reserved: 0,
    };
    let rts = entry
        .rts
        .map(|p| mk_pin(p.port, p.line, p.af))
        .unwrap_or(zero_pin);
    let cts = entry
        .cts
        .map(|p| mk_pin(p.port, p.line, p.af))
        .unwrap_or(zero_pin);

    bindings::uart_bus_cfg_t {
        instance: entry.instance,
        tx: mk_pin(entry.tx.port, entry.tx.line, entry.tx.af),
        rx: mk_pin(entry.rx.port, entry.rx.line, entry.rx.af),
        rts,
        cts,
        baud: overrides.baud.unwrap_or(entry.baud),
        data_bits: overrides.data_bits.unwrap_or(entry.data_bits),
        stop_bits: overrides.stop_bits.unwrap_or(entry.stop_bits),
        parity: overrides.parity.unwrap_or(entry.parity),
        flow_control: overrides.flow_control.unwrap_or(entry.flow_control),
        irqn: entry.irqn,
        priority: entry.priority,
    }
}

#[derive(Clone, Copy, Default)]
pub struct Overrides {
    pub baud: Option<u32>,
    pub data_bits: Option<u8>,
    pub stop_bits: Option<u8>,
    pub parity: Option<u8>,
    pub flow_control: Option<u8>,
}

pub fn init(dev: &Device, overrides: &Overrides) -> Result<()> {
    let cfg = cfg_from_entry(dev.0, overrides);
    let rc = unsafe { bindings::uart_init(&cfg as *const _) };
    if rc < 0 {
        return Err(from_c_rc(rc));
    }
    Ok(())
}

pub fn deinit(dev: &Device) -> Result<()> {
    let rc = unsafe { bindings::uart_deinit(dev.0.instance) };
    if rc < 0 {
        return Err(from_c_rc(rc));
    }
    Ok(())
}

pub fn transmit_blocking(dev: &Device, buf: &[u8], timeout_ms: u32) -> Result<()> {
    if buf.is_empty() {
        return Ok(());
    }
    let rc = unsafe {
        bindings::uart_transmit_blocking(
            dev.0.instance,
            buf.as_ptr(),
            buf.len() as i32,
            timeout_ms,
        )
    };
    if rc < 0 {
        return Err(from_c_rc(rc));
    }
    Ok(())
}

pub fn transmit_nb(dev: &Device, buf: &[u8]) -> Result<usize> {
    if buf.is_empty() {
        return Ok(0);
    }
    let rc =
        unsafe { bindings::uart_transmit_nb(dev.0.instance, buf.as_ptr(), buf.len() as i32) };
    if rc < 0 {
        return Err(from_c_rc(rc));
    }
    Ok(rc as usize)
}

pub fn receive_nb(dev: &Device, buf: &mut [u8]) -> Result<usize> {
    if buf.is_empty() {
        return Ok(0);
    }
    let rc =
        unsafe { bindings::uart_receive_nb(dev.0.instance, buf.as_mut_ptr(), buf.len() as i32) };
    if rc < 0 {
        return Err(from_c_rc(rc));
    }
    Ok(rc as usize)
}

pub fn register_irq_handler(
    dev: &Device,
    handler: Option<IrqHandler>,
    ctx: *mut (),
) -> Result<()> {
    // SAFETY: `IrqHandler = extern "C" fn(Irq, *mut ())` is ABI-compatible
    // with `uart_irq_handler_fn` because `Irq` is `#[repr(u32)]` and `ctx`
    // is `*mut () ↔ void *`.
    let raw: bindings::uart_irq_handler_fn = unsafe { core::mem::transmute(handler) };
    let rc =
        unsafe { bindings::uart_set_irq_handler(dev.0.instance, raw, ctx as *mut core::ffi::c_void) };
    if rc < 0 {
        return Err(from_c_rc(rc));
    }
    Ok(())
}

pub fn dispatch_by_slot(slot: u8) {
    unsafe {
        bindings::uart_dispatch_by_slot(slot);
    }
}
