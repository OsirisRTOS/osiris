//! ST bxCAN HAL bridge — thin Rust wrappers over `interface/can.c`.
//!
//! Single-FIFO design: the C HAL only enables `CAN_IT_RX_FIFO0_MSG_PENDING`
//! and overrides `HAL_CAN_RxFifo0MsgPendingCallback`, draining frames into
//! a 32-deep SW ring. Consumers pull from the ring via [`receive`] in
//! thread context. `register_irq_handler` lets a thread-side consumer
//! install a brief wake hook (kick a parked thread by uid) that fires
//! after each successful ring push.
//!
//! See `osiris/machine/cortex-m/st/stm32l4/interface/can.c` for the
//! C-side implementation, and `src/drivers/can/{mod,wait}.rs` for the
//! kernel driver that wires this bridge into the IRQ registry at boot.

use core::num::NonZeroU32;

use super::bindings;
use super::device_tree;

/// CAN-specific error. Each variant maps to exactly one `CAN_ERR_*`
/// code in `interface/can.c`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Bad input: null pointer, index out of range, zero TX timeout, DLC
    /// > 8, bank out of range, fifo > 1, or ID/mask exceeds the 11/29-bit
    /// limit. Should be impossible from valid Rust callers.
    InvalidArgument,
    /// `compatible` / `ordinal` matched no entry in the DT registry.
    NoSuchDevice,
    /// Peripheral has not been initialized (or has been deinit'd).
    NotInitialized,
    /// Requested bitrate isn't achievable from the current PCLK with any
    /// of the bit-timing presets (no clean integer division).
    BitrateInfeasible,
    /// Peripheral clock couldn't be enabled — RCC failed or the instance
    /// isn't supported on this part.
    ClockUnavailable,
    /// `HAL_CAN_Init` rejected the configured peripheral (bit timing
    /// register values out of range, peripheral stuck in init mode, …).
    InitFailed,
    /// `HAL_CAN_ConfigFilter` rejected the filter.
    FilterRejected,
    /// `HAL_CAN_Start` failed.
    StartFailed,
    /// `HAL_CAN_ActivateNotification` failed.
    NotifyFailed,
    /// `HAL_CAN_AddTxMessage` rejected the message (peripheral not READY,
    /// param error, …).
    TransmitFailed,
    /// TX mailbox stayed busy past `tx_timeout_iters`.
    MailboxBusy,
}

pub type Result<T> = core::result::Result<T, Error>;

/// Map a C return code to an `Error`. The fallback arm only triggers if
/// `interface/can.c` adds a code without updating this match.
fn from_c_rc(rc: i32) -> Error {
    match rc {
        -1 => Error::InvalidArgument,
        -2 => Error::NotInitialized,
        -3 => Error::BitrateInfeasible,
        -4 => Error::ClockUnavailable,
        -5 => Error::InitFailed,
        -6 => Error::FilterRejected,
        -7 => Error::StartFailed,
        -8 => Error::NotifyFailed,
        -9 => Error::TransmitFailed,
        -10 => Error::MailboxBusy,
        _ => Error::InvalidArgument,
    }
}

#[derive(Clone, Copy, Default)]
pub struct Frame {
    pub id: u32,
    pub data: [u8; 8],
    pub len: u8,
    pub is_extended: bool,
}

/// Peripheral mode. `Loopback` short-circuits TX→RX inside the controller
/// (no bus / transceiver needed) for diagnostics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    #[default]
    Normal,
    Loopback,
}

/// CAN fault-confinement state, decoded from CAN_ESR.
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
pub struct Device(&'static device_tree::CanRegistryEntry);

impl Device {
    /// Bitrate declared in the device tree for this peripheral.
    pub fn bitrate_hz(&self) -> u32 {
        self.0.bitrate_hz
    }

    /// DT-assigned slot index. Stable across reboots; matches the C-side
    /// `s_handles[index]` slot.
    pub fn index(&self) -> u8 {
        self.0.index
    }

    /// Underlying registry entry — mostly for the kernel driver to read
    /// the IRQ vector when registering with `crate::irq::register_irq`.
    pub fn entry(&self) -> &'static device_tree::CanRegistryEntry {
        self.0
    }
}

fn cfg_from_dev(dev: &Device) -> bindings::can_bus_cfg_t {
    cfg_from_dev_full(dev, dev.0.bitrate_hz, Mode::Normal, 0)
}

fn cfg_from_dev_full(
    dev: &Device,
    bitrate_hz: u32,
    mode: Mode,
    tx_timeout_iters: u32,
) -> bindings::can_bus_cfg_t {
    let e = dev.0;
    bindings::can_bus_cfg_t {
        instance: e.instance,
        bitrate_hz,
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
        auto_retransmit: e.auto_retransmit,
        tx_timeout_iters,
    }
}

fn frame_to_c(frame: &Frame) -> bindings::can_frame_t {
    bindings::can_frame_t {
        id: frame.id,
        data: frame.data,
        len: frame.len,
        is_extended: frame.is_extended as u8,
        reserved: 0,
    }
}

fn frame_from_c(c: &bindings::can_frame_t) -> Frame {
    Frame {
        id: c.id,
        data: c.data,
        len: c.len,
        is_extended: c.is_extended != 0,
    }
}

pub fn get(compatible: &str, ordinal: usize) -> Result<Device> {
    let entry = device_tree::can_by_compatible(compatible, ordinal).ok_or(Error::NoSuchDevice)?;
    Ok(Device(entry))
}

/// Look up a CAN device by its DT-assigned `index` (0..CAN_SLOT_COUNT).
pub fn get_by_index(index: u8) -> Result<Device> {
    for entry in device_tree::CAN_REGISTRY {
        if entry.index == index {
            return Ok(Device(entry));
        }
    }
    Err(Error::NoSuchDevice)
}

pub fn init(dev: &Device, bitrate_hz: NonZeroU32, mode: Mode) -> Result<()> {
    let cfg = cfg_from_dev_full(dev, bitrate_hz.get(), mode, 0);
    let rc = unsafe { bindings::can_init(&cfg) };
    if rc == 0 { Ok(()) } else { Err(from_c_rc(rc)) }
}

pub fn deinit(dev: &Device) -> Result<()> {
    let cfg = cfg_from_dev(dev);
    let rc = unsafe { bindings::can_deinit(&cfg) };
    if rc == 0 { Ok(()) } else { Err(from_c_rc(rc)) }
}

pub fn transmit(dev: &Device, frame: &Frame, tx_timeout_iters: NonZeroU32) -> Result<()> {
    if frame.len > 8 {
        return Err(Error::InvalidArgument);
    }
    let cfg = cfg_from_dev_full(dev, dev.0.bitrate_hz, Mode::Normal, tx_timeout_iters.get());
    let cframe = frame_to_c(frame);
    let rc = unsafe { bindings::can_transmit(&cfg, &cframe) };
    if rc == 0 { Ok(()) } else { Err(from_c_rc(rc)) }
}

pub fn receive(dev: &Device, out: &mut Frame) -> Result<bool> {
    let cfg = cfg_from_dev(dev);
    let mut cframe = bindings::can_frame_t {
        id: 0,
        data: [0u8; 8],
        len: 0,
        is_extended: 0,
        reserved: 0,
    };
    let rc = unsafe { bindings::can_receive(&cfg, &mut cframe) };
    match rc {
        0 => Ok(false),
        1 => {
            *out = frame_from_c(&cframe);
            Ok(true)
        }
        other => Err(from_c_rc(other)),
    }
}

/// ID/mask filter. `mask` bits set to 1 must match; 0 = wildcard. Standard
/// IDs use 11 bits (0..=0x7FF), extended IDs 29 bits (0..=0x1FFF_FFFF).
/// `fifo` selects RX FIFO 0 or 1; only FIFO 0 raises an interrupt today.
#[derive(Clone, Copy)]
pub struct Filter {
    pub bank: u8,
    pub id: u32,
    pub mask: u32,
    pub extended: bool,
    pub fifo: u8,
}

/// Master-side filter bank limit (banks 14..=27 belong to the slave CAN).
const FILTER_BANK_MAX: u8 = 13;

fn validate_filter(filter: &Filter) -> Result<()> {
    if filter.bank > FILTER_BANK_MAX || filter.fifo > 1 {
        return Err(Error::InvalidArgument);
    }
    let id_max: u32 = if filter.extended { 0x1FFF_FFFF } else { 0x7FF };
    if filter.id > id_max || filter.mask > id_max {
        return Err(Error::InvalidArgument);
    }
    Ok(())
}

pub fn configure_filter(dev: &Device, filter: &Filter) -> Result<()> {
    validate_filter(filter)?;
    let cfg = cfg_from_dev(dev);
    let c = bindings::can_filter_t {
        id: filter.id,
        mask: filter.mask,
        bank: filter.bank,
        extended: filter.extended as u8,
        fifo: filter.fifo,
        reserved: 0,
    };
    let rc = unsafe { bindings::can_configure_filter(&cfg, &c) };
    if rc == 0 { Ok(()) } else { Err(from_c_rc(rc)) }
}

pub fn disable_filter(dev: &Device, bank: u8) -> Result<()> {
    if bank > FILTER_BANK_MAX {
        return Err(Error::InvalidArgument);
    }
    let cfg = cfg_from_dev(dev);
    let rc = unsafe { bindings::can_disable_filter(&cfg, bank) };
    if rc == 0 { Ok(()) } else { Err(from_c_rc(rc)) }
}

/// IRQ kind passed to a registered handler. Mirrors the bxCAN vector lines.
/// Currently only `Rx0` is wired; the others are reserved.
///
/// `#[repr(u32)]` to match the C `can_irq_kind` enum, which bindgen lays
/// out as `c_uint`.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Irq {
    Tx = 0,
    Rx0 = 1,
    Rx1 = 2,
    Sce = 3,
}

/// IRQ handler signature. The HAL passes through the opaque `ctx` pointer
/// the caller provided to [`register_irq_handler`] — the HAL never
/// dereferences it.
pub type IrqHandler = extern "C" fn(kind: Irq, ctx: *mut ());

/// Register an IRQ handler for this CAN instance. `ctx` is an opaque
/// pointer the HAL stores and passes to `handler` on every IRQ; it must
/// remain valid for as long as the handler is registered. Pass `None`
/// to clear.
pub fn register_irq_handler(
    dev: &Device,
    handler: Option<IrqHandler>,
    ctx: *mut (),
) -> Result<()> {
    // SAFETY: bindgen lowers `can_irq_handler_fn` to
    //   `Option<unsafe extern "C" fn(c_int, *mut c_void)>`.
    // Our `IrqHandler` is `extern "C" fn(Irq, *mut ())`. ABI-compatible:
    // function-pointer ABI is identical regardless of `unsafe`, `Irq` is
    // `#[repr(u32)]` (matches `c_int`/`c_uint` ABI), `*mut ()` and
    // `*mut c_void` are layout-equivalent, `Option<extern "C" fn(..)>`
    // is niche-filled (null = None).
    let raw_fn: bindings::can_irq_handler_fn = match handler {
        Some(f) => unsafe { core::mem::transmute(f) },
        None => None,
    };
    let rc = unsafe {
        bindings::can_set_irq_handler(dev.0.index, raw_fn, ctx as *mut core::ffi::c_void)
    };
    if rc == 0 { Ok(()) } else { Err(from_c_rc(rc)) }
}

pub fn last_error(dev: &Device) -> u32 {
    let cfg = cfg_from_dev(dev);
    unsafe { bindings::can_last_error(&cfg) }
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
    let cfg = cfg_from_dev(dev);
    let rc = unsafe { bindings::can_recover(&cfg) };
    if rc == 0 { Ok(()) } else { Err(from_c_rc(rc)) }
}

/// Snapshot of CAN peripheral state for diagnosing "TX silently fails".
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

/// IRQ-vector entry point: forwards to the C HAL's `can_isr(slot)`.
pub fn dispatch_isr(slot: u8) {
    unsafe { bindings::can_isr(slot) }
}

pub fn diag(dev: &Device) -> Diag {
    let cfg = cfg_from_dev(dev);
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
        rx_drops: 0,
        rx_hw_ovr: 0,
    };
    unsafe { bindings::can_diag(&cfg, &mut raw) };
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
        rx_drops: raw.rx_drops,
        rx_hw_ovr: raw.rx_hw_ovr,
    }
}
