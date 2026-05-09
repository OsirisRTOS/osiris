use core::ffi::c_void;

use crate::hal;
use hal::stack::EntryFn;

pub fn sleep(_until: u64) -> isize {
    hal::asm::syscall!(1, (_until >> 32) as u32, _until as u32)
}

pub fn sleep_for(_duration: u64) -> isize {
    hal::asm::syscall!(2, (_duration >> 32) as u32, _duration as u32)
}

/// Voluntarily give up CPU without parking. Triggers PendSV so the
/// scheduler picks the next runnable thread; the caller stays in the
/// run queue and is eligible to be re-picked. Use as a cooperative
/// "step aside" — never as a wait, since nothing wakes a parked
/// caller (a previous incarnation of this function called sleep with
/// `until = u64::MAX`, which left the thread parked indefinitely).
pub fn yield_thread() -> isize {
    crate::sched::reschedule();
    0
}

/// Trigger PendSV so the scheduler picks another runnable thread.
/// Cheaper than a syscall — direct MMIO write to `SCB->ICSR.PENDSVSET`.
/// Use as the relax step of a spin loop to break priority inversion
/// when a higher-priority thread waits on a lock held by a preempted
/// lower-priority thread.
#[inline]
pub fn reschedule() {
    crate::sched::reschedule();
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RtAttrs {
    pub deadline: u64,
    pub period: u32,
    pub budget: u32,
}

/// Spawn a new thread. `ctx` is delivered to `func_ptr` as its first
/// argument (R0); pass `null_mut()` for stateless entries. The caller
/// must ensure the pointee outlives the thread.
pub fn spawn_thread(_func_ptr: EntryFn, _ctx: *mut c_void, attrs: Option<RtAttrs>) -> isize {
    if let Some(attrs) = attrs {
        if attrs.budget == 0 || attrs.period == 0 {
            return -1; // Invalid attributes
        }

        if attrs.budget > attrs.period {
            return -1; // Budget cannot exceed period
        }

        if attrs.budget > u32::MAX / 2 || attrs.period > u32::MAX / 2 {
            return -1; // Prevent potential overflow in calculations
        }

        hal::asm::syscall!(
            3,
            _func_ptr as u32,
            _ctx as usize,
            &attrs as *const RtAttrs as usize
        )
    } else {
        -1
    }
}

pub fn exit(_code: usize) -> ! {
    hal::asm::syscall!(4, _code as u32);
    loop {
        hal::asm::nop!();
    }
}

/// Raw `UId::as_usize()` of the calling thread; same value
/// `spawn_thread` returned. One syscall per call; cache for hot
/// loops. Used to register as a waiter on a queue / wake primitive.
pub fn current_id() -> isize {
    hal::asm::syscall!(6)
}
