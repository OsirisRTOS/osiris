pub(crate) mod store;
pub use store::{HeapSnapshot, StackSnapshot};

use crate::sched::{task, thread};

/// Returns the latest global kernel heap snapshot, or `None` if the scheduler
/// has not yet run a single reschedule.
pub fn global_heap() -> Option<HeapSnapshot> {
    store::read_global_heap()
}

/// Returns the latest heap snapshot for the task identified by `task_id`, or
/// `None` if no snapshot exists for that slot.
pub fn task_heap(task_id: task::UId) -> Option<HeapSnapshot> {
    store::read_task_heap(task_id.as_usize())
}

/// Returns the latest stack snapshot for the thread identified by `uid`, or
/// `None` if no snapshot exists for that slot.
pub fn thread_stack(uid: thread::UId) -> Option<StackSnapshot> {
    store::read_thread_stack(uid.as_usize())
}
