//! This module provides access to the scheduler.

pub mod rt;
//pub mod scheduler;
pub mod task;
pub mod thread;

use hal::Schedable;

use crate::mem::{array::IndexMap, rbtree::RbTree, view::ViewMut};

type ThreadMap<const N: usize> = IndexMap<thread::UId, thread::Thread, N>;

pub struct Scheduler<const N: usize> {
    threads: ThreadMap<N>,
    rt_scheduler: rt::Scheduler<N>,

    wakeup: RbTree<thread::WakupTree, thread::UId>,

    last_tick: u64,
}

impl<const N: usize> Scheduler<N> {
    pub fn new() -> Self {
        Self {
            threads: IndexMap::new(),
            rt_scheduler: rt::Scheduler::new(),
            wakeup: RbTree::new(),
            last_tick: 0,
        }
    }

    pub fn enqueue(&mut self, thread: thread::Thread) {
        let uid = thread.uid();
        let rt = thread.rt_server().is_some();
        self.threads.insert(&thread.uid(), thread);

        if rt {
            let mut view = ViewMut::<thread::UId, thread::RtServer, ThreadMap<N>>::new(
                &mut self.threads,
            );
            self.rt_scheduler.enqueue(uid, &mut view);
        }
    }

    pub fn do_sched(&mut self, now: u64, old: Option<thread::UId>) -> Option<thread::UId> {
        let dt = now - self.last_tick;
        self.last_tick = now;

        if let Some(old) = old {
            let mut view = rt::ServerView::<N>::new(&mut self.threads);
            // If this is not a real-time thread, this will just do nothing.
            self.rt_scheduler.put(old, dt, &mut view);

            // TODO: thread is still enqueued. Dequeue if blocked or sleeping and put to the respective tree/list.
            // If it exited remove it completely.
        }

        let mut view = rt::ServerView::<N>::new(&mut self.threads);
        self.rt_scheduler.pick(now, &mut view)
    }

    pub fn dequeue(&mut self, uid: thread::UId) -> Option<thread::Thread> {
        let mut view = rt::ServerView::<N>::new(&mut self.threads);
        // If this is not a real-time thread, this will just do nothing.
        self.rt_scheduler.dequeue(uid, &mut view);

        self.threads.remove(&uid)
    }
}

/// Reschedule the tasks.
pub fn reschedule() {
    hal::Machine::trigger_reschedule();
}

/* 



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

    */
