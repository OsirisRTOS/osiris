//! This module provides task management related syscalls.

use core::{ffi::c_void, str};

use crate::{kprintln, sched};
use macros::syscall_handler;

/// Syscall handler: reschedule.
/// This syscall is used to request a reschedule.
///
/// No arguments are passed to this syscall.
#[unsafe(no_mangle)]
#[syscall_handler(args = 0, num = 1)]
extern "C" fn syscall_reschedule(_svc_args: *const c_void) {
    //let _ = hal::hprintln!("debug: reschedule requested.");

    sched::reschedule();
}

#[unsafe(no_mangle)]
#[syscall_handler(args = 2, num = 2)]
extern "C" fn syscall_print(svc_args: *const c_void) {
    unsafe {
        // svc_args is really a pointer to two machine‐word arguments:
        // [0] = pointer to the byte buffer
        // [1] = length of the buffer
        let args = svc_args as *const usize;
        let buf_ptr = *args.add(0) as *const u8;
        let buf_len = *args.add(1);

        // Build a byte‐slice from ptr+len
        let bytes = core::slice::from_raw_parts(buf_ptr, buf_len);

        if let Ok(s) = str::from_utf8(&bytes) {
            // If the bytes are valid UTF-8, print them directly
            kprintln!("[uspace] {}", s);
            return;
        }
    }
}
