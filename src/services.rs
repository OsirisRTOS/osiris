use hal::common::sched::ThreadDesc;

use crate::sched::{self, task::TaskDesc};

pub mod init;

pub extern "C" fn finalizer() {
    // Do nothing.
    loop {}
}

pub fn init_services() {
    let task = TaskDesc {
        mem_size: 0,
        stack_size: 4096,
    };

    // Create the init task.
    let thread = ThreadDesc {
        entry: init::InitTask::main,
        argc: 0,
        argv: core::ptr::null(),
        finalizer,
    };

    sched::create_task(task, thread).expect("Failed to create init task");
}

