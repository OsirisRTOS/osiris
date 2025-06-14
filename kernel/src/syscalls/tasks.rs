//! This module provides task management related syscalls.

use core::{ffi::c_void, str};

use crate::sched;
use macros::syscall_handler;

/// Syscall handler: reschedule.
/// This syscall is used to request a reschedule.
///
/// No arguments are passed to this syscall.
#[syscall_handler(num = 1)]
fn syscall_reschedule() -> usize {
    sched::reschedule();
    0
}
