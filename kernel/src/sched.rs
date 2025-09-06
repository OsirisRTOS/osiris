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
    scheduler::SCHEDULER.lock().create_task(desc)
}

pub fn create_thread(
    task_id: task::TaskId,
    entry: extern "C" fn(),
    fin: Option<extern "C" fn() -> !>,
    timing: thread::Timing,
) -> Result<thread::ThreadUId, KernelError> {
    scheduler::SCHEDULER
        .lock()
        .create_thread(entry, fin, timing, task_id)
}

pub fn enable_scheduler() {
    scheduler::SCHEDULER.lock().enable();
}

pub fn tick_scheduler() -> bool {
    scheduler::SCHEDULER.lock().tick()
}
