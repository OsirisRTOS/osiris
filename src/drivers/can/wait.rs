//! Per-CAN-slot wait list and ISR-context callback registry.
//!
//! Threads call [`register_waiter`] to park on a slot; the RX ISR
//! kicks everyone on the list when a frame lands in the SW ring.
//! [`ensure_registered`] wires the kernel IRQ vector and the HAL-level
//! per-frame callback to the dispatcher in this module, once per slot.

use core::cell::Cell;
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

use crate::hal;

/// Maximum bxCAN slots on STM32L4 (matches `CAN_SLOT_COUNT` in
/// `interface/can.c`).
pub const CAN_SLOT_COUNT: usize = 2;

/// Per-device kernel state. One static instance per CAN slot; the
/// pointer is stable for the program's lifetime, satisfying the HAL's
/// `ctx` contract.
pub struct CanWaitDevice {
    /// Head of the intrusive wait-queue. Mutated only with IRQs disabled
    /// (in `register_waiter` / `unregister_waiter`) or read in IRQ
    /// context (`kernel_dispatch`).
    waiters: AtomicPtr<Waiter>,
    /// Optional ISR-context fn registered by a non-thread consumer.
    /// Stored as `usize` so we can load it atomically; `0` = none.
    rx_callback: AtomicUsize,
}

impl CanWaitDevice {
    pub const fn new() -> Self {
        Self {
            waiters: AtomicPtr::new(core::ptr::null_mut()),
            rx_callback: AtomicUsize::new(0),
        }
    }
}

/// One static instance per CAN slot; the address of each element is the
/// `ctx` we hand to the HAL.
static CAN_DEVICES: [CanWaitDevice; CAN_SLOT_COUNT] =
    [CanWaitDevice::new(), CanWaitDevice::new()];

/// Per-slot "IRQ handler is registered with the kernel registry" flag,
/// so [`ensure_registered`] is idempotent across multiple `Device::open`
/// calls on the same slot.
static REGISTERED: [AtomicBool; CAN_SLOT_COUNT] =
    [AtomicBool::new(false), AtomicBool::new(false)];

/// Caller-owned wait-queue node. Must stay live and at a fixed address
/// from `register_waiter` until `unregister_waiter` returns; the ISR
/// dereferences it. Typically stack-local in a `recv_blocking` loop.
///
/// Killing the parked thread mid-wait leaves a dangling pointer in the
/// list. Caller must unregister before exiting.
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
// and read in IRQ context (where IRQs are already masked at the same
// priority). `Cell` is normally !Sync; the IRQ-disable discipline here
// substitutes for the locking the type system would otherwise enforce.
unsafe impl Sync for Waiter {}

/// Push `w` onto the head of slot `slot`'s wait list. O(1).
pub fn register_waiter(slot: u8, w: &Waiter) {
    let dev = &CAN_DEVICES[slot as usize];
    let state = hal::asm::disable_irq_save();
    let head = dev.waiters.load(Ordering::Relaxed);
    w.next.set(head);
    dev.waiters
        .store(w as *const _ as *mut _, Ordering::Relaxed);
    hal::asm::enable_irq_restr(state);
}

/// Remove `w` from slot `slot`'s wait list. O(N). No-op if `w` isn't
/// linked (re-entrant safe).
pub fn unregister_waiter(slot: u8, w: &Waiter) {
    let dev = &CAN_DEVICES[slot as usize];
    let state = hal::asm::disable_irq_save();
    let target = w as *const _ as *mut Waiter;
    let mut prev_next: *const Cell<*mut Waiter> = core::ptr::null();
    let mut cur = dev.waiters.load(Ordering::Relaxed);
    while !cur.is_null() {
        if cur == target {
            let next = unsafe { (*cur).next.get() };
            if prev_next.is_null() {
                dev.waiters.store(next, Ordering::Relaxed);
            } else {
                unsafe { (*prev_next).set(next) };
            }
            // Clear our own `next` so accidental reuse without re-register
            // doesn't leave a dangling pointer in the node.
            w.next.set(core::ptr::null_mut());
            break;
        }
        prev_next = unsafe { &(*cur).next };
        cur = unsafe { (*cur).next.get() };
    }
    hal::asm::enable_irq_restr(state);
}

/// ISR-context callback signature. Receives the slot index it was
/// registered for.
pub type RxCallback = extern "C" fn(slot: u8);

/// Install (or clear, with `None`) the ISR-context callback for `slot`.
/// Atomic; safe to call concurrently with the dispatcher.
pub fn set_rx_callback(slot: u8, cb: Option<RxCallback>) {
    let raw = cb.map(|f| f as usize).unwrap_or(0);
    CAN_DEVICES[slot as usize]
        .rx_callback
        .store(raw, Ordering::Release);
}

/// Registered with the HAL once per slot at boot. The HAL hands back the
/// per-device static as `ctx`; we walk its waiter list, kick each parked
/// thread, then invoke the optional ISR-context callback.
extern "C" fn kernel_dispatch(kind: hal::can::Irq, ctx: *mut ()) {
    if !matches!(kind, hal::can::Irq::Rx0) {
        return;
    }
    let dev = unsafe { &*(ctx as *const CanWaitDevice) };

    let mut cur = dev.waiters.load(Ordering::Relaxed);
    while !cur.is_null() {
        let uid = unsafe { (*cur).uid };
        crate::sched::with(|s| {
            let _ = s.kick_by_uid(uid as usize);
        });
        cur = unsafe { (*cur).next.get() };
    }

    let raw = dev.rx_callback.load(Ordering::Acquire);
    if raw != 0 {
        // SAFETY: `raw` was produced by `set_rx_callback` as `f as usize`
        // from a `RxCallback = extern "C" fn(u8)`. Function pointers and
        // `usize` are layout-compatible on every supported target.
        let cb: RxCallback = unsafe { core::mem::transmute(raw) };
        let base = &CAN_DEVICES[0] as *const _ as usize;
        let slot = (ctx as usize - base) / core::mem::size_of::<CanWaitDevice>();
        cb(slot as u8);
    }

    crate::sched::reschedule();
}

/// Kernel IRQ-vector handler. Forwards to the C HAL, which fires the
/// per-frame [`kernel_dispatch`] callback we registered separately.
fn rx0_kernel_handler(_ctx: *mut u8, _vector: usize, userdata: Option<usize>) {
    let Some(slot) = userdata else { return };
    if slot >= CAN_SLOT_COUNT {
        return;
    }
    hal::can::dispatch_isr(slot as u8);
}

/// Idempotent per-slot wiring. Must run BEFORE `hal::can::init` enables
/// the NVIC line: a frame arriving in the gap fires an IRQ that finds
/// no kernel handler, leaving the bxCAN message-pending flag set and
/// NVIC tail-chaining forever.
pub fn ensure_registered(dev: &hal::can::Device) -> hal::can::Result<()> {
    let slot = dev.index();
    if slot as usize >= CAN_SLOT_COUNT {
        return Err(hal::can::Error::InvalidArgument);
    }
    if REGISTERED[slot as usize].swap(true, Ordering::AcqRel) {
        return Ok(());
    }

    let ctx = &CAN_DEVICES[slot as usize] as *const _ as *mut ();
    hal::can::register_irq_handler(dev, Some(kernel_dispatch), ctx)?;

    // Vector index = irqn + 16 (system exceptions occupy 0..15).
    let entry = dev.entry();
    let rx0_vector = entry.rx0_irq.irqn as usize + 16;
    // SAFETY: called from `Device::open` (thread context, not ISR).
    unsafe {
        crate::irq::register_irq(rx0_vector, rx0_kernel_handler, Some(slot as usize))
            .map_err(|_| hal::can::Error::NotifyFailed)?;
    }
    Ok(())
}
