//! This module provides task management related syscalls.

use core::ffi::c_int;

use proc_macros::syscall_handler;

use crate::{sched, time, uapi::sched::RtAttrs};

#[syscall_handler(num = 1)]
fn sleep(until_hi: u32, until_lo: u32) -> c_int {
    let until = ((until_hi as u64) << 32) | (until_lo as u64);
    sched::with(|sched| {
        if sched.sleep_until(until, time::tick()).is_err() {
            bug!("no current thread set.");
        }
    });
    0
}

#[syscall_handler(num = 2)]
fn sleep_for(duration_hi: u32, duration_lo: u32) -> c_int {
    let duration = ((duration_hi as u64) << 32) | (duration_lo as u64);
    sched::with(|sched| {
        let now = time::tick();
        if sched.sleep_until(now + duration, now).is_err() {
            bug!("no current thread set.");
        }
    });
    0
}

#[syscall_handler(num = 3)]
fn spawn_thread(func_ptr: usize, attrs: *const RtAttrs) -> c_int {
    sched::with(|sched| {
        let attrs = if attrs.is_null() {
            None
        } else {
            Some(unsafe { *attrs })
        };

        let attrs = sched::thread::Attributes {
            entry: unsafe { core::mem::transmute(func_ptr) },
            fin: None,
            attrs,
        };
        match sched.create_thread(None, &attrs) {
            Ok(uid) => {
                if sched.enqueue(time::tick(), uid).is_err() {
                    bug!("failed to enqueue thread.");
                }
                uid.as_usize() as c_int
            }
            Err(_) => -1,
        }
    })
}

#[syscall_handler(num = 4)]
fn exit(code: usize) -> c_int {
    sched::with(|sched| {
        if sched.kill_thread(None).is_err() {
            bug!("failed to terminate thread.");
        }
    });
    0
}

#[syscall_handler(num = 5)]
fn kick_thread(uid: usize) -> c_int {
    0
}
