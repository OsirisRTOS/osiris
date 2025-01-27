
pub mod scheduler;
pub mod task;

use hal::common::{self, sched::ThreadDesc};
use task::{Task, TaskDesc, TaskId};

use crate::{mem::alloc::AllocError, sched::scheduler::SCHEDULER};

pub fn reschedule() {
    common::sched::reschedule();
}

pub fn create_task(desc: TaskDesc, init_desc: ThreadDesc) -> Result<TaskId, AllocError> {
    SCHEDULER.lock().create_task(desc, init_desc)
}

pub fn add_task(task: Task) -> Result<TaskId, AllocError> {
    SCHEDULER.lock().add_task(task)
}