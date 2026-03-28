//! This module provides access to userspace structures and services.

use crate::{sched, time};

unsafe extern "C" {
    /// The entry point for the userspace application.
    fn app_main() -> ();
}

extern "C" fn app_main_entry() {
    unsafe { app_main() }
}

pub fn init_app() {
    let attrs = sched::thread::Attributes {
        entry: app_main_entry,
        fin: None,
    };
    sched::with(|sched| {
        if let Ok(uid) = sched.create_thread(Some(sched::task::KERNEL_TASK), &attrs) {
            if sched.enqueue(time::tick(), uid).is_err() {
                panic!("failed to enqueue init thread.");
            }
        } else {
            panic!("failed to create init task.");
        }
    })
}
