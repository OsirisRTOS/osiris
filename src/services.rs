use hal::common::sched::ThreadDesc;

use crate::{sched, uspace};

pub fn init_services() {
    // Create the init task.
    let init = ThreadDesc {
        entry: uspace::core::Init::main,
        argc: 0,
        argv: core::ptr::null(),
        finalizer: uspace::util::thread_finalizer,
    };

    sched::create_task(uspace::core::Init::task_desc(), init).expect("Failed to create init task");

    // Create the dummy task.
    let dummy = ThreadDesc {
        entry: uspace::core::Dummy::main,
        argc: 0,
        argv: core::ptr::null(),
        finalizer: uspace::util::thread_finalizer,
    };

    sched::create_task(uspace::core::Dummy::task_desc(), dummy).expect("Failed to create dummy task");
}

