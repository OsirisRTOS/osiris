//! Stub CAN HAL for host/test builds. Mirrors `native::can` shape.

use core::num::NonZeroU32;

use super::device_tree;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidArgument,
    NoSuchDevice,
    NotInitialized,
    BitrateInfeasible,
    ClockUnavailable,
    InitFailed,
    FilterRejected,
    StartFailed,
    NotifyFailed,
    TransmitFailed,
    MailboxBusy,
}

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Clone, Copy, Default)]
pub struct Frame {
    pub id: u32,
    pub data: [u8; 8],
    pub len: u8,
    pub is_extended: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Loopback,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BusState {
    #[default]
    ErrorActive,
    ErrorWarning,
    ErrorPassive,
    BusOff,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct BusStatus {
    pub state: BusState,
    pub tec: u8,
    pub rec: u8,
}

#[derive(Clone, Copy)]
pub struct Filter {
    pub bank: u8,
    pub id: u32,
    pub mask: u32,
    pub extended: bool,
    pub fifo: u8,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Irq {
    Tx = 0,
    Rx0 = 1,
    Rx1 = 2,
    Sce = 3,
}

pub type IrqHandler = extern "C" fn(kind: Irq, ctx: *mut ());

pub struct Device;

impl Device {
    pub fn bitrate_hz(&self) -> u32 {
        0
    }
    pub fn index(&self) -> u8 {
        0
    }
    pub fn entry(&self) -> &'static device_tree::CanRegistryEntry {
        unimplemented!("stub: no DT registry")
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Diag {
    pub esr: u32,
    pub tsr: u32,
    pub msr: u32,
    pub mcr: u32,
    pub btr: u32,
    pub tx_attempts: u32,
    pub tx_hal_fails: u32,
    pub tx_mbx_timeouts: u32,
    pub rx_irqs: u32,
    pub rx_frames: u32,
    pub rx_drops: u32,
    pub rx_hw_ovr: u32,
}

pub fn get(_compatible: &str, _ordinal: usize) -> Result<Device> {
    Err(Error::NoSuchDevice)
}
pub fn init(_dev: &Device, _bitrate_hz: NonZeroU32, _mode: Mode) -> Result<()> {
    Err(Error::NotInitialized)
}
pub fn deinit(_dev: &Device) -> Result<()> {
    Err(Error::NotInitialized)
}
pub fn transmit(_dev: &Device, _frame: &Frame, _tx_timeout_iters: NonZeroU32) -> Result<()> {
    Err(Error::NotInitialized)
}
pub fn receive(_dev: &Device, _out: &mut Frame) -> Result<bool> {
    Ok(false)
}
pub fn configure_filter(_dev: &Device, _filter: &Filter) -> Result<()> {
    Err(Error::NotInitialized)
}
pub fn register_irq_handler(
    _dev: &Device,
    _handler: Option<IrqHandler>,
    _ctx: *mut (),
) -> Result<()> {
    Err(Error::NotInitialized)
}
pub fn bus_status(_dev: &Device) -> BusStatus {
    BusStatus::default()
}
pub fn recover(_dev: &Device) -> Result<()> {
    Err(Error::NotInitialized)
}
pub fn dispatch_isr(_slot: u8) {}

pub fn diag(_dev: &Device) -> Diag {
    Diag::default()
}
