//! This module provides access to userspace structures and services.

use crate::sched;

unsafe extern "C" {
    /// The entry point for the userspace application.
    fn app_main() -> ();
}

extern "C" fn app_main_entry() {
    unsafe { app_main() }
}

pub fn init_app() -> Result<(), crate::utils::KernelError> {
    let attrs = sched::thread::Attributes {
        entry: app_main_entry,
        fin: None,
    };
    let uid = sched::create_thread(sched::task::KERNEL_TASK, &attrs)?;
    
    sched::enqueue(uid)
}
