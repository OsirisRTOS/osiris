use core::ffi::c_void;

use hal::common::sched::ThreadDesc;
use macros::syscall_handler;

use crate::sched::{create_task, reschedule, task::TaskDesc};

#[no_mangle]
#[syscall_handler(args = 0, num = 1)]
extern "C" fn syscall_reschedule(_svc_args: *const c_void) {
    let _ = hal::hprintln!("debug: reschedule requested.");

    reschedule();
}