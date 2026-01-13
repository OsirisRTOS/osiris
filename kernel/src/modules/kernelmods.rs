mod sample_module;

use std::ffi::c_int;
use macros::syscall_handler;
use crate::modules::KernelModule;
use crate::sync::spinlock::SpinLock;
use crate::utils::KernelError;

//Lock to guarantee race condition free access
static LOCK: SpinLock = SpinLock::new();

#[syscall_handler[num=3]]
pub fn dispatch(call: usize, args: *mut u8) -> c_int {
    0
}

pub(super) fn init_modules() -> Result<(), KernelError> {
    //SAFETY: All kernel modules are private to this generated file and are secured using a common lock, therefor no race conditions can appear
    unsafe {
        LOCK.lock();
        LOCK.unlock();
    }
    Ok(())
}

pub(super) fn exit_modules() -> Result<(), KernelError> {
    //SAFETY: All kernel modules are private to this generated file and are secured using a common lock, therefor no race conditions can appear
    unsafe {
        LOCK.lock();
        LOCK.unlock();
    }
    Ok(())
}