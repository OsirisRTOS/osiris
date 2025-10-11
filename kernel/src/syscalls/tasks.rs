//! This module provides task management related syscalls.

use core::ffi::c_int;

use crate::sched;
use macros::syscall_handler;

/// Syscall handler: reschedule.
/// This syscall is used to request a reschedule.
///
/// No arguments are passed to this syscall.
#[syscall_handler(num = 1)]
fn syscall_reschedule() -> c_int {
    sched::reschedule();
    0
}

#[syscall_handler(num = 2)]
fn syscall_exec(entry: usize) -> c_int {
    let entry: extern "C" fn() -> () = unsafe { core::mem::transmute(entry) };

    let timing = sched::thread::Timing {
        period: 8,
        deadline: 8,
        exec_time: 2,
    };

    sched::create_task(sched::task::TaskDescriptor { mem_size: 0 })
        .and_then(|task| sched::create_thread(task, entry, None, timing))
        .map(|_| 0)
        .unwrap_or(-1)
}
