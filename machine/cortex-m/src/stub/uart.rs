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

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Irq {
    Rx = 0,
    TxDone = 1,
}

pub type IrqHandler = extern "C" fn(Irq, *mut ());

#[derive(Clone, Copy, Default)]
pub struct Overrides {
    pub baud: Option<u32>,
    pub data_bits: Option<u8>,
    pub stop_bits: Option<u8>,
    pub parity: Option<u8>,
    pub flow_control: Option<u8>,
}

#[derive(Clone, Copy)]
pub struct Device;

impl Device {
    pub fn index(&self) -> u8 {
        0
    }
    pub fn instance(&self) -> usize {
        0
    }
    pub fn irqn(&self) -> u8 {
        0
    }
}

pub fn get_by_index(_idx: u8) -> Result<Device> {
    Err(Error::NoSuchDevice)
}

pub fn get(_compatible: &str, _ordinal: usize) -> Result<Device> {
    Err(Error::NoSuchDevice)
}

pub fn init(_dev: &Device, _overrides: &Overrides) -> Result<()> {
    Err(Error::NotInitialized)
}

pub fn deinit(_dev: &Device) -> Result<()> {
    Err(Error::NotInitialized)
}

pub fn transmit_blocking(_dev: &Device, _buf: &[u8], _timeout_ms: u32) -> Result<()> {
    Err(Error::NotInitialized)
}

pub fn transmit_nb(_dev: &Device, _buf: &[u8]) -> Result<usize> {
    Err(Error::NotInitialized)
}

pub fn receive_nb(_dev: &Device, _buf: &mut [u8]) -> Result<usize> {
    Err(Error::NotInitialized)
}

pub fn register_irq_handler(
    _dev: &Device,
    _handler: Option<IrqHandler>,
    _ctx: *mut (),
) -> Result<()> {
    Err(Error::NotInitialized)
}

pub fn dispatch_by_slot(_slot: u8) {}

/// Mirror of the `hal_arm::uart::console_entry` API; testing has no
/// device-tree-driven console, so always returns `None`.
pub fn console_entry() -> Option<&'static ()> {
    None
}

pub fn init_console_from_dt() -> Result<()> {
    Ok(())
}

pub fn console_write(_buf: &[u8]) -> Result<()> {
    Ok(())
}
