//! ST bxCAN HAL bridge — thin Rust wrappers over `interface/can.c`.

use hal_api::{PosixError, Result, ok_or_err};

use super::bindings;
use super::device_tree;

#[derive(Clone, Copy, Default)]
pub struct Frame {
    pub id: u32,
    pub data: [u8; 8],
    pub len: u8,
    pub is_extended: bool,
    /// 64-bit-extended bxCAN SOF hardware timestamp (1 tick = 1 CAN
    /// bit-time, captured at the SOF sample point). RX only; 0 on Tx.
    pub hw_timestamp_rx: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    /// Internal TX→RX short-circuit; no bus needed.
    Loopback,
}

/// Fault-confinement state, decoded from CAN_ESR.
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

pub struct Device(&'static device_tree::CanRegistryEntry);

impl Device {
    pub fn from_entry(entry: &'static device_tree::CanRegistryEntry) -> Self {
        Self(entry)
    }

    pub fn bitrate_hz(&self) -> u32 {
        self.0.bitrate_hz
    }

    pub fn index(&self) -> u8 {
        self.0.index
    }

    pub fn entry(&self) -> &'static device_tree::CanRegistryEntry {
        self.0
    }
}

fn init_cfg(dev: &Device, mode: Mode) -> bindings::can_bus_cfg_t {
    let e = dev.0;
    bindings::can_bus_cfg_t {
        instance: e.instance,
        bitrate_hz: e.bitrate_hz,
        rx: bindings::can_pin_cfg_t {
            port: e.rx.port,
            pin: e.rx.line,
            af: e.rx.af,
            reserved: 0,
        },
        tx: bindings::can_pin_cfg_t {
            port: e.tx.port,
            pin: e.tx.line,
            af: e.tx.af,
            reserved: 0,
        },
        rx0_irqn: e.rx0_irq.irqn,
        rx0_priority: e.rx0_irq.priority,
        rx1_irqn: e.rx1_irq.irqn,
        rx1_priority: e.rx1_irq.priority,
        index: e.index,
        mode: mode as u8,
        tx_open_drain: e.tx_open_drain,
        reserved: 0,
    }
}

fn frame_to_c(frame: &Frame) -> bindings::can_frame_t {
    bindings::can_frame_t {
        id: frame.id,
        data: frame.data,
        len: frame.len,
        is_extended: frame.is_extended as u8,
        // Tx: unused (no Tx mailbox sets TGT).
        hw_timestamp_rx: 0,
    }
}

fn frame_from_c(c: &bindings::can_frame_t) -> Frame {
    Frame {
        id: c.id,
        data: c.data,
        len: c.len,
        is_extended: c.is_extended != 0,
        hw_timestamp_rx: c.hw_timestamp_rx,
    }
}

pub fn get(compatible: &str, ordinal: usize) -> Result<Device> {
    let entry =
        device_tree::can_by_compatible(compatible, ordinal).ok_or(PosixError::ENODEV)?;
    Ok(Device(entry))
}

pub fn init(dev: &Device, mode: Mode) -> Result<()> {
    let cfg = init_cfg(dev, mode);
    let rc = unsafe { bindings::can_init(&cfg) };
    ok_or_err(rc, ())
}

pub fn start(dev: &Device) -> Result<()> {
    let rc = unsafe { bindings::can_start(dev.0.index) };
    ok_or_err(rc, ())
}

pub fn deinit(dev: &Device) -> Result<()> {
    let rc = unsafe { bindings::can_deinit(dev.0.index) };
    ok_or_err(rc, ())
}

pub fn transmit(dev: &Device, frame: &Frame) -> Result<()> {
    let id_max = if frame.is_extended { 0x1FFF_FFFF } else { 0x7FF };
    if frame.len > 8 || frame.id > id_max {
        return Err(PosixError::EINVAL);
    }
    let cframe = frame_to_c(frame);
    let rc = unsafe { bindings::can_transmit(dev.0.index, &cframe) };
    ok_or_err(rc, ())
}

pub fn receive(dev: &Device, out: &mut Frame) -> Result<bool> {
    let mut cframe = bindings::can_frame_t {
        id: 0,
        data: [0u8; 8],
        len: 0,
        is_extended: 0,
        hw_timestamp_rx: 0,
    };
    let rc = unsafe { bindings::can_receive(dev.0.index, &mut cframe) };
    match rc {
        0 => Ok(false),
        1 => {
            *out = frame_from_c(&cframe);
            Ok(true)
        }
        other => Err(PosixError::from_errno(-other)),
    }
}

/// ID/mask filter. Set bits in `mask` must match; 0 = wildcard. Standard
/// IDs use 11 bits (0..=0x7FF), extended IDs 29 bits (0..=0x1FFF_FFFF).
/// `fifo` selects RX FIFO 0 or 1.
#[derive(Clone, Copy)]
pub struct Filter {
    pub bank: u8,
    pub id: u32,
    pub mask: u32,
    pub extended: bool,
    pub fifo: u8,
}

const FILTER_BANK_MAX: u8 = 13;

fn validate_filter(filter: &Filter) -> Result<()> {
    if filter.bank > FILTER_BANK_MAX || filter.fifo > 1 {
        return Err(PosixError::EINVAL);
    }
    let id_max: u32 = if filter.extended { 0x1FFF_FFFF } else { 0x7FF };
    if filter.id > id_max || filter.mask > id_max {
        return Err(PosixError::EINVAL);
    }
    Ok(())
}

pub fn configure_filter(dev: &Device, filter: &Filter) -> Result<()> {
    validate_filter(filter)?;
    let c = bindings::can_filter_t {
        id: filter.id,
        mask: filter.mask,
        bank: filter.bank,
        extended: filter.extended as u8,
        fifo: filter.fifo,
        reserved: 0,
    };
    let rc = unsafe { bindings::can_configure_filter(dev.0.index, &c) };
    ok_or_err(rc, ())
}

/// IRQ kind passed to a registered handler. Mirrors the bxCAN vector lines.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Irq {
    Tx = 0,
    Rx0 = 1,
    Rx1 = 2,
    Sce = 3,
}

pub type IrqHandler = extern "C" fn(kind: Irq, ctx: *mut ());

pub fn register_irq_handler(dev: &Device, handler: Option<IrqHandler>, ctx: *mut ()) -> Result<()> {
    // SAFETY: bindgen lowers `can_irq_handler_fn` to
    // `Option<unsafe extern "C" fn(c_int, *mut c_void)>`. Layout-equivalent
    // to `IrqHandler` (Irq is #[repr(u32)], *mut () == *mut c_void).
    let raw_fn: bindings::can_irq_handler_fn = match handler {
        Some(f) => unsafe { core::mem::transmute(f) },
        None => None,
    };
    let rc = unsafe {
        bindings::can_set_irq_handler(dev.0.index, raw_fn, ctx as *mut core::ffi::c_void)
    };
    ok_or_err(rc, ())
}

fn last_error(dev: &Device) -> u32 {
    unsafe { bindings::can_last_error(dev.0.index) }
}

fn decode_bus_status(esr: u32) -> BusStatus {
    let tec = ((esr >> 16) & 0xFF) as u8;
    let rec = ((esr >> 24) & 0xFF) as u8;
    let state = if esr & (1 << 2) != 0 {
        BusState::BusOff
    } else if esr & (1 << 1) != 0 {
        BusState::ErrorPassive
    } else if esr & 1 != 0 {
        BusState::ErrorWarning
    } else {
        BusState::ErrorActive
    };
    BusStatus { state, tec, rec }
}

pub fn bus_status(dev: &Device) -> BusStatus {
    decode_bus_status(last_error(dev))
}

pub fn recover(dev: &Device) -> Result<()> {
    let rc = unsafe { bindings::can_recover(dev.0.index) };
    ok_or_err(rc, ())
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
    pub rx_frames_fifo0: u32,
    pub rx_frames_fifo1: u32,
    pub rx_drops: u32,
    pub rx_hw_ovr: u32,
    pub rx_hw_ovr_fifo0: u32,
    pub rx_hw_ovr_fifo1: u32,
    pub rx_peak_fmp: u32,
    pub rx_get_fails: u32,
}

#[inline]
pub fn dispatch_isr(slot: u8) {
    unsafe { bindings::can_isr(slot) }
}

pub fn diag(dev: &Device) -> Diag {
    let mut raw = bindings::can_diag_t {
        esr: 0,
        tsr: 0,
        msr: 0,
        mcr: 0,
        btr: 0,
        tx_attempts: 0,
        tx_hal_fails: 0,
        tx_mbx_timeouts: 0,
        rx_irqs: 0,
        rx_frames: 0,
        rx_frames_fifo0: 0,
        rx_frames_fifo1: 0,
        rx_drops: 0,
        rx_hw_ovr: 0,
        rx_hw_ovr_fifo0: 0,
        rx_hw_ovr_fifo1: 0,
        rx_peak_fmp: 0,
        rx_get_fails: 0,
    };
    unsafe { bindings::can_diag(dev.0.index, &mut raw) };
    Diag {
        esr: raw.esr,
        tsr: raw.tsr,
        msr: raw.msr,
        mcr: raw.mcr,
        btr: raw.btr,
        tx_attempts: raw.tx_attempts,
        tx_hal_fails: raw.tx_hal_fails,
        tx_mbx_timeouts: raw.tx_mbx_timeouts,
        rx_irqs: raw.rx_irqs,
        rx_frames: raw.rx_frames,
        rx_frames_fifo0: raw.rx_frames_fifo0,
        rx_frames_fifo1: raw.rx_frames_fifo1,
        rx_drops: raw.rx_drops,
        rx_hw_ovr: raw.rx_hw_ovr,
        rx_hw_ovr_fifo0: raw.rx_hw_ovr_fifo0,
        rx_hw_ovr_fifo1: raw.rx_hw_ovr_fifo1,
        rx_peak_fmp: raw.rx_peak_fmp,
        rx_get_fails: raw.rx_get_fails,
    }
}
