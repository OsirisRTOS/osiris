
pub mod scheduler;
pub mod task;

use hal::common::{self, sched::ThreadDesc};
use task::{TaskDesc, TaskId, Timing};

use crate::{mem::alloc::AllocError, sched::scheduler::SCHEDULER};

pub fn reschedule() {
    common::sched::reschedule();
}

pub fn create_task(desc: TaskDesc, init_desc: ThreadDesc, timing: Timing) -> Result<TaskId, AllocError> {
    SCHEDULER.lock().create_task(desc, init_desc, timing)
}