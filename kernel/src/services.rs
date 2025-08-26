//! This module initializes and manages the microkernel services.

use crate::{
    hal,
    sched::{self, task::Timing},
    uspace,
    utils::KernelError,
};
use hal::sched::ThreadDesc;

/// Initialize the microkernel services.
///
/// This function creates the init task and other services.
pub fn init_services() -> Result<(), KernelError> {
    // Create the init task.
    let init = ThreadDesc {
        entry: uspace::core::Init::main,
        argc: 0,
        argv: core::ptr::null(),
        finalizer: uspace::util::thread_finalizer,
    };

    // TODO: These are dummy values for testing.
    let init_timing = Timing {
        period: 8,
        deadline: 8,
        exec_time: 2,
    };

    // Create the init task.
    sched::create_task(uspace::core::Init::task_desc(), init, init_timing)?;

    // Create the first dummy task.
    let dummy = ThreadDesc {
        entry: uspace::core::Dummy::main,
        argc: 0,
        argv: core::ptr::null(),
        finalizer: uspace::util::thread_finalizer,
    };

    // TODO: These are dummy values for testing.
    let dummy_timing = Timing {
        period: 6,
        deadline: 6,
        exec_time: 1,
    };

    // Create the dummy task.
    sched::create_task(uspace::core::Dummy::task_desc(), dummy, dummy_timing)?;

    // Create the second dummy task.
    let dummy2 = ThreadDesc {
        entry: uspace::core::Dummy2::main,
        argc: 0,
        argv: core::ptr::null(),
        finalizer: uspace::util::thread_finalizer,
    };

    // TODO: These are dummy values for testing.
    let dummy_timing2 = Timing {
        period: 6,
        deadline: 6,
        exec_time: 1,
    };

    // Create the second dummy task.
    sched::create_task(uspace::core::Dummy2::task_desc(), dummy2, dummy_timing2)?;

    Ok(())
}
