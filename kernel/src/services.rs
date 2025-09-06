//! This module initializes and manages the microkernel services.

use crate::{
    sched::{self, thread::Timing},
    uspace,
    utils::KernelError,
};

/// Initialize the microkernel services.
///
/// This function creates the init task and other services.
pub fn init_services() -> Result<(), KernelError> {
    // TODO: These are dummy values for testing.
    let init_timing = Timing {
        period: 8,
        deadline: 8,
        exec_time: 2,
    };

    let init_task = sched::create_task(uspace::core::Init::task_desc())?;
    sched::create_thread(init_task, uspace::core::Init::main, None, init_timing)?;

    // TODO: These are dummy values for testing.
    let dummy_timing = Timing {
        period: 6,
        deadline: 6,
        exec_time: 1,
    };

    let dummy_task = sched::create_task(uspace::core::Dummy::task_desc())?;
    sched::create_thread(dummy_task, uspace::core::Dummy::main, None, dummy_timing)?;

    // TODO: These are dummy values for testing.
    let dummy_timing2 = Timing {
        period: 6,
        deadline: 6,
        exec_time: 1,
    };

    let dummy2_task = sched::create_task(uspace::core::Dummy2::task_desc())?;
    sched::create_thread(dummy2_task, uspace::core::Dummy2::main, None, dummy_timing2)?;

    Ok(())
}
