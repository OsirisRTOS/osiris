//! This module provides access to the scheduler.

pub mod scheduler;
pub mod task;

use crate::utils::KernelError;
use hal::common;

/// Reschedule the tasks.
pub fn reschedule() {
    common::sched::reschedule();
}

/// Create a new task.
///
/// `desc` - The task descriptor.
/// `main_desc` - The main thread descriptor.
/// `main_timing` - The timing information for the main thread.
///
/// Returns the task ID if the task was created successfully, or an error if the task could not be created.
pub fn create_task(
    desc: task::TaskDesc,
    main_desc: common::sched::ThreadDesc,
    main_timing: task::Timing,
) -> Result<task::TaskId, KernelError> {
    scheduler::SCHEDULER
        .lock()
        .create_task(desc, main_desc, main_timing)
}
