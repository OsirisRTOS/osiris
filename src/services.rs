use hal::common::sched::ThreadDesc;

use crate::{sched::{self, task::Timing}, uspace};

pub fn init_services() {
    // Create the init task.
    let init = ThreadDesc {
        entry: uspace::core::Init::main,
        argc: 0,
        argv: core::ptr::null(),
        finalizer: uspace::util::thread_finalizer,
    };

    let init_timing = Timing {
        period: 8,
        deadline: 8,
        exec_time: 2,
    };

    sched::create_task(uspace::core::Init::task_desc(), init, init_timing).expect("Failed to create init task");

    // Create the dummy task.
    let dummy = ThreadDesc {
        entry: uspace::core::Dummy::main,
        argc: 0,
        argv: core::ptr::null(),
        finalizer: uspace::util::thread_finalizer,
    };

    let dummy_timing = Timing {
        period: 6,
        deadline: 6,
        exec_time: 1,
    };

    sched::create_task(uspace::core::Dummy::task_desc(), dummy, dummy_timing).expect("Failed to create dummy task");

    // Create the dummy task.
    let dummy2 = ThreadDesc {
        entry: uspace::core::Dummy2::main,
        argc: 0,
        argv: core::ptr::null(),
        finalizer: uspace::util::thread_finalizer,
    };

    let dummy_timing2 = Timing {
        period: 6,
        deadline: 6,
        exec_time: 1,
    };

    sched::create_task(uspace::core::Dummy2::task_desc(), dummy2, dummy_timing2).expect("Failed to create dummy2 task");
}

