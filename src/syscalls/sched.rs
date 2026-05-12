//! This module provides task management related syscalls.

use core::ffi::c_int;

use proc_macros::syscall_handler;

use crate::{error::PosixError, sched, time, uapi::sched::RtAttrs};

#[syscall_handler(num = 1)]
fn sleep(until_hi: u32, until_lo: u32) -> c_int {
    let until = ((until_hi as u64) << 32) | (until_lo as u64);
    sched::with(|sched| {
        let now = time::tick();
        let uid = sched.current_uid();
        if let Err(e) = sched.sleep_until(until, now) {
            bug!(
                "sleep(until={}, now={}, current={:?}) failed: {:?}",
                until,
                now,
                uid,
                e
            );
        }
        0
    })
}

#[syscall_handler(num = 2)]
fn sleep_for(duration_hi: u32, duration_lo: u32) -> c_int {
    let duration = ((duration_hi as u64) << 32) | (duration_lo as u64);
    sched::with(|sched| {
        let now = time::tick();
        let until = now.saturating_add(duration);
        let uid = sched.current_uid();
        if let Err(e) = sched.sleep_until(until, now) {
            bug!(
                "sleep_for(duration={}, now={}, current={:?}) failed: {:?}",
                duration,
                now,
                uid,
                e
            );
        }
        0
    })
}

fn valid_rt_attrs(attrs: RtAttrs) -> bool {
    attrs.budget != 0
        && attrs.period != 0
        && attrs.deadline != 0
        && attrs.budget as u64 <= attrs.deadline
        && attrs.deadline <= attrs.period as u64
}

#[syscall_handler(num = 3)]
fn spawn_thread(func_ptr: usize, ctx: usize, attrs: *const RtAttrs) -> c_int {
    sched::with(|sched| {
        let attrs = if attrs.is_null() {
            None
        } else {
            let attrs = unsafe { *attrs };
            if !valid_rt_attrs(attrs) {
                return -1;
            }
            Some(attrs)
        };

        let attrs = sched::thread::Attributes {
            entry: unsafe { core::mem::transmute(func_ptr) },
            ctx: ctx as *mut core::ffi::c_void,
            fin: None,
            attrs,
        };
        match sched.create_thread(None, &attrs) {
            Ok(uid) => {
                if let Err(e) = sched.enqueue(time::tick(), uid) {
                    bug!("spawn_thread: failed to enqueue thread {}: {:?}", uid, e);
                }
                uid.as_usize() as c_int
            }
            Err(e) => {
                warn!("spawn_thread: create_thread failed: {:?}", e);
                -1
            }
        }
    })
}

#[syscall_handler(num = 4)]
fn exit(_code: usize) -> c_int {
    sched::with(|sched| {
        if let Err(e) = sched.kill_by_thread(None) {
            bug!("exit: kill_by_thread failed: {:?}", e);
        }
    });
    0
}

#[syscall_handler(num = 5)]
fn kick_thread(uid: usize) -> c_int {
    sched::with(|sched| {
        if let Err(e) = sched.kick_by_uid(uid) {
            // Not in the wakeup tree is expected (target is running / already
            // runnable); any other error means scheduler state is broken.
            bug_on!(
                e.kind != PosixError::ENOENT,
                "kick_thread({}): unexpected error: {:?}",
                uid,
                e
            );
        }
    });
    sched::reschedule();
    0
}

#[syscall_handler(num = 6)]
fn current_id() -> c_int {
    sched::with(|sched| match sched.current_uid() {
        Some(uid) => uid as c_int,
        None => -1,
    })
}
