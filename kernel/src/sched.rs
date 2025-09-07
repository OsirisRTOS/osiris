//! This module provides access to the scheduler.

pub mod scheduler;
pub mod task;
pub mod thread;

use hal::Schedable;

use crate::{sched::task::TaskDescriptor, utils::KernelError};

/// Reschedule the tasks.
pub fn reschedule() {
    hal::Machine::trigger_reschedule();
}

/// Create a new task.
///
/// `desc` - The task descriptor.
/// `main_desc` - The main thread descriptor.
/// `main_timing` - The timing information for the main thread.
///
/// Returns the task ID if the task was created successfully, or an error if the task could not be created.
pub fn create_task(desc: task::TaskDescriptor) -> Result<task::TaskId, KernelError> {
    enable_scheduler(false);
    let res = scheduler::SCHEDULER.lock().create_task(desc);
    enable_scheduler(true);

    res
}

pub fn create_thread(
    task_id: task::TaskId,
    entry: extern "C" fn(),
    fin: Option<extern "C" fn() -> !>,
    timing: thread::Timing,
) -> Result<thread::ThreadUId, KernelError> {
    enable_scheduler(false);
    let res = scheduler::SCHEDULER
        .lock()
        .create_thread(entry, fin, timing, task_id);
    enable_scheduler(true);

    res
}

pub fn enable_scheduler(enable: bool) {
    scheduler::set_enabled(enable);
}

pub fn tick_scheduler() -> bool {
    if !scheduler::enabled() {
        return false;
    }

    scheduler::SCHEDULER.lock().tick()
}
