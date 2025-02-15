//! This module provides task management related syscalls.

use core::ffi::c_void;

use crate::sched;
use macros::syscall_handler;

/// Syscall handler: reschedule.
/// This syscall is used to request a reschedule.
///
/// No arguments are passed to this syscall.
#[no_mangle]
#[syscall_handler(args = 0, num = 1)]
extern "C" fn syscall_reschedule(_svc_args: *const c_void) {
    let _ = hal::hprintln!("debug: reschedule requested.");

    sched::reschedule();
}
