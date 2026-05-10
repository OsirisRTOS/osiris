//! Per-UART-slot wait lists and ISR-context callbacks. Mirrors
//! `drivers/can/wait.rs` with separate RX and TX channels per slot.

use core::cell::Cell;
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

pub const UART_SLOT_COUNT: usize = 6;

pub struct UartWaitDevice {
    rx_waiters: AtomicPtr<Waiter>,
    tx_waiters: AtomicPtr<Waiter>,
    rx_callback: AtomicUsize,
    tx_callback: AtomicUsize,
}

impl UartWaitDevice {
    pub const fn new() -> Self {
        Self {
            rx_waiters: AtomicPtr::new(core::ptr::null_mut()),
            tx_waiters: AtomicPtr::new(core::ptr::null_mut()),
            rx_callback: AtomicUsize::new(0),
            tx_callback: AtomicUsize::new(0),
        }
    }
}

static UART_DEVICES: [UartWaitDevice; UART_SLOT_COUNT] = [
    UartWaitDevice::new(),
    UartWaitDevice::new(),
    UartWaitDevice::new(),
    UartWaitDevice::new(),
    UartWaitDevice::new(),
    UartWaitDevice::new(),
];

/// Caller-owned wait-queue node. Must stay live and at a fixed address
/// from `register_*_waiter` until `unregister_*_waiter` returns; the
/// ISR dereferences it. Killing the parked thread mid-wait leaves a
/// dangling pointer in the list — caller must unregister before exit.
#[repr(C)]
pub struct Waiter {
    uid: u32,
    next: Cell<*mut Waiter>,
}

impl Waiter {
    pub const fn new(uid: u32) -> Self {
        Self {
            uid,
            next: Cell::new(core::ptr::null_mut()),
        }
    }

    pub fn uid(&self) -> u32 {
        self.uid
    }
}

// SAFETY: `Waiter` is mutated only with IRQs disabled (register/unregister)
// and read in IRQ context. The IRQ-disable discipline substitutes for
// the locking the type system would otherwise enforce on `Cell`.
unsafe impl Sync for Waiter {}

#[derive(Clone, Copy)]
pub enum Channel {
    Rx,
    Tx,
}

fn waiter_head(dev: &UartWaitDevice, ch: Channel) -> &AtomicPtr<Waiter> {
    match ch {
        Channel::Rx => &dev.rx_waiters,
        Channel::Tx => &dev.tx_waiters,
    }
}

fn callback_slot(dev: &UartWaitDevice, ch: Channel) -> &AtomicUsize {
    match ch {
        Channel::Rx => &dev.rx_callback,
        Channel::Tx => &dev.tx_callback,
    }
}

pub fn register_waiter(slot: u8, ch: Channel, w: &Waiter) {
    let dev = &UART_DEVICES[slot as usize];
    let head = waiter_head(dev, ch);
    let state = crate::hal::asm::disable_irq_save();
    let cur = head.load(Ordering::Relaxed);
    w.next.set(cur);
    head.store(w as *const _ as *mut _, Ordering::Relaxed);
    crate::hal::asm::enable_irq_restr(state);
}

pub fn unregister_waiter(slot: u8, ch: Channel, w: &Waiter) {
    let dev = &UART_DEVICES[slot as usize];
    let head = waiter_head(dev, ch);
    let state = crate::hal::asm::disable_irq_save();
    let target = w as *const _ as *mut Waiter;
    let mut prev_next: *const Cell<*mut Waiter> = core::ptr::null();
    let mut cur = head.load(Ordering::Relaxed);
    while !cur.is_null() {
        if cur == target {
            let next = unsafe { (*cur).next.get() };
            if prev_next.is_null() {
                head.store(next, Ordering::Relaxed);
            } else {
                unsafe { (*prev_next).set(next) };
            }
            w.next.set(core::ptr::null_mut());
            break;
        }
        prev_next = unsafe { &(*cur).next };
        cur = unsafe { (*cur).next.get() };
    }
    crate::hal::asm::enable_irq_restr(state);
}

pub fn register_rx_waiter(slot: u8, w: &Waiter) {
    register_waiter(slot, Channel::Rx, w);
}
pub fn unregister_rx_waiter(slot: u8, w: &Waiter) {
    unregister_waiter(slot, Channel::Rx, w);
}
pub fn register_tx_waiter(slot: u8, w: &Waiter) {
    register_waiter(slot, Channel::Tx, w);
}
pub fn unregister_tx_waiter(slot: u8, w: &Waiter) {
    unregister_waiter(slot, Channel::Tx, w);
}

pub type ChannelCallback = extern "C" fn(slot: u8);

pub fn set_rx_callback(slot: u8, cb: Option<ChannelCallback>) {
    let raw = cb.map(|f| f as usize).unwrap_or(0);
    UART_DEVICES[slot as usize]
        .rx_callback
        .store(raw, Ordering::Release);
}

pub fn set_tx_callback(slot: u8, cb: Option<ChannelCallback>) {
    let raw = cb.map(|f| f as usize).unwrap_or(0);
    UART_DEVICES[slot as usize]
        .tx_callback
        .store(raw, Ordering::Release);
}

extern "C" fn kernel_dispatch(kind: crate::hal::uart::Irq, ctx: *mut ()) {
    let dev = unsafe { &*(ctx as *const UartWaitDevice) };

    let ch = match kind {
        crate::hal::uart::Irq::Rx => Channel::Rx,
        crate::hal::uart::Irq::TxDone => Channel::Tx,
    };

    let head = waiter_head(dev, ch);
    let mut cur = head.load(Ordering::Relaxed);
    while !cur.is_null() {
        let uid = unsafe { (*cur).uid };
        crate::sched::with(|s| {
            let _ = s.kick_by_uid(uid as usize);
        });
        cur = unsafe { (*cur).next.get() };
    }

    let raw = callback_slot(dev, ch).load(Ordering::Acquire);
    if raw != 0 {
        // SAFETY: `raw` came from `set_*_callback` as `f as usize` and the
        // `raw != 0` guard excludes the unset sentinel.
        let cb: ChannelCallback = unsafe { core::mem::transmute(raw) };
        let base = &UART_DEVICES[0] as *const _ as usize;
        let slot = (ctx as usize - base) / core::mem::size_of::<UartWaitDevice>();
        cb(slot as u8);
    }

    crate::sched::reschedule();
}

fn vector_dispatch(_ctx: *mut u8, _vector: usize, userdata: Option<usize>) {
    let Some(slot) = userdata else {
        return;
    };
    crate::hal::uart::dispatch_by_slot(slot as u8);
}

/// Call once during osiris boot, after the console has been initialised.
pub fn init() {
    for slot in 0..UART_SLOT_COUNT as u8 {
        let Ok(dev) = crate::hal::uart::get_by_index(slot) else {
            continue;
        };

        // IPSR = NVIC line + 16 on Cortex-M.
        let vector = dev.irqn() as usize + 16;
        unsafe {
            if let Err(e) = crate::irq::register_irq(vector, vector_dispatch, Some(slot as usize))
            {
                panic!(
                    "UART wait dispatcher: failed to register IRQ vector {} for slot {} ({:?})",
                    vector, slot, e
                );
            }
        }

        let ctx = &UART_DEVICES[slot as usize] as *const _ as *mut ();
        match crate::hal::uart::register_irq_handler(&dev, Some(kernel_dispatch), ctx) {
            Ok(()) => {}
            // Console-owned slot — IT mode disabled there by design.
            Err(crate::hal::uart::Error::Busy) => {}
            Err(e) => panic!("UART wait dispatcher: HAL rejected slot {} ({:?})", slot, e),
        }
    }
}
