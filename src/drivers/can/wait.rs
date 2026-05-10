//! Per-CAN-slot wait slot. One consumer thread per controller: the
//! consumer parks its uid in the slot before sleeping, the RX ISR reads
//! the slot and kicks.

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::hal;

pub const CAN_SLOT_COUNT: usize = 2;

/// 0 = no waiter; otherwise raw `UId::as_usize()` of the parked thread.
static WAITER: [AtomicU32; CAN_SLOT_COUNT] = [AtomicU32::new(0), AtomicU32::new(0)];

static REGISTERED: [AtomicBool; CAN_SLOT_COUNT] = [AtomicBool::new(false), AtomicBool::new(false)];

pub fn register_waiter(slot: u8, uid: u32) {
    WAITER[slot as usize].store(uid, Ordering::Release);
}

pub fn unregister_waiter(slot: u8) {
    WAITER[slot as usize].store(0, Ordering::Release);
}

extern "C" fn kernel_dispatch(kind: hal::can::Irq, ctx: *mut ()) {
    if !matches!(kind, hal::can::Irq::Rx0 | hal::can::Irq::Rx1) {
        return;
    }
    let slot = ctx as usize;
    if slot >= CAN_SLOT_COUNT {
        return;
    }
    let uid = WAITER[slot].load(Ordering::Acquire);
    if uid != 0 {
        crate::sched::with(|s| {
            let _ = s.kick_by_uid(uid as usize);
        });
        crate::sched::reschedule();
    }
}

fn rx_kernel_handler(_ctx: *mut u8, _vector: usize, userdata: Option<usize>) {
    let Some(slot) = userdata else { return };
    if slot >= CAN_SLOT_COUNT {
        return;
    }
    hal::can::dispatch_isr(slot as u8);
}

/// Idempotent per-slot IRQ wiring. Must run before `hal::can::init`
/// enables NVIC, otherwise an early frame leaves the bxCAN flag set
/// with no consumer.
pub fn ensure_registered(dev: &hal::can::Device) -> hal::can::Result<()> {
    let slot = dev.index();
    if slot as usize >= CAN_SLOT_COUNT {
        return Err(hal::can::Error::InvalidArgument);
    }
    if REGISTERED[slot as usize].swap(true, Ordering::AcqRel) {
        return Ok(());
    }

    let ctx = slot as usize as *mut ();
    hal::can::register_irq_handler(dev, Some(kernel_dispatch), ctx)?;

    let entry = dev.entry();
    let rx0_vector = entry.rx0_irq.irqn as usize + 16;
    let rx1_vector = entry.rx1_irq.irqn as usize + 16;
    unsafe {
        crate::irq::register_irq(rx0_vector, rx_kernel_handler, Some(slot as usize))
            .map_err(|_| hal::can::Error::NotifyFailed)?;
        crate::irq::register_irq(rx1_vector, rx_kernel_handler, Some(slot as usize))
            .map_err(|_| hal::can::Error::NotifyFailed)?;
    }
    Ok(())
}
