//! The scheduler module is responsible for managing the tasks and threads in the system.
//! It provides the necessary functions to create tasks and threads, and to switch between them.

use core::{ffi::c_void, sync::atomic::AtomicBool};

use super::task::{Task, TaskId};
use crate::{
    mem::{self, array::IndexMap, heap::BinaryHeap, queue::Queue},
    sched::{
        task::TaskDescriptor,
        thread::{RunState, ThreadMap, ThreadUId, Timing},
    },
    sync::spinlock::SpinLocked,
    utils,
};

/// The global scheduler instance.
pub static SCHEDULER: SpinLocked<Scheduler> = SpinLocked::new(Scheduler::new());
static SCHEDULER_ENABLED: AtomicBool = AtomicBool::new(false);

/// The scheduler struct. It keeps track of the tasks and threads in the system.
/// This scheduler is a simple Rate Monotonic Scheduler (RMS) implementation.
pub struct Scheduler {
    /// The current running thread.
    current: Option<ThreadUId>,
    /// Fast interval store. This gets updated every time a new thread is selected.
    current_interval: usize,
    /// Stores the tasks in the system.
    user_tasks: IndexMap<usize, Task, 8>,
    /// Stores the threads in the system.
    threads: ThreadMap<8>,
    /// The priority queue that yields the next thread to run.
    queue: BinaryHeap<(usize, ThreadUId), 32>,
    /// The callbacks queue that stores the threads that need to be fired in the future.
    callbacks: Queue<(ThreadUId, usize), 32>,
    /// The progression of the time interval of the scheduler.
    time: usize,
}

impl Scheduler {
    /// Create a new scheduler instance.
    pub const fn new() -> Self {
        Self {
            current: None,
            current_interval: 0,
            user_tasks: IndexMap::new(),
            threads: ThreadMap::new(),
            queue: BinaryHeap::new(),
            callbacks: Queue::new(),
            time: 0,
        }
    }

    pub fn create_task(&mut self, desc: TaskDescriptor) -> Result<TaskId, utils::KernelError> {
        let size = mem::align_up(desc.mem_size);
        let idx = self
            .user_tasks
            .find_empty()
            .ok_or(utils::KernelError::OutOfMemory)?;
        let task_id = TaskId::new_user(idx);

        let task = Task::new(size, task_id)?;
        self.user_tasks.insert(&idx, task)?;
        Ok(task_id)
    }

    pub fn create_thread(
        &mut self,
        entry: extern "C" fn(),
        fin: Option<extern "C" fn() -> !>,
        timing: Timing,
        task_id: TaskId,
    ) -> Result<ThreadUId, utils::KernelError> {
        let task_idx: usize = task_id.into();

        if let Some(task) = self.user_tasks.get_mut(&task_idx) {
            let desc = task.create_thread(entry, fin, timing)?;
            let id = self.threads.create(desc)?;
            self.queue.push((timing.period, id))?;
            Ok(id)
        } else {
            Err(utils::KernelError::InvalidArgument)
        }
    }

    /// Updates the current thread context with the given context.
    ///
    /// `ctx` - The new context to update the current thread with.
    fn update_current_ctx(&mut self, ctx: *mut c_void) {
        if let Some(id) = self.current {
            if let Some(thread) = self.threads.get_mut(&id) {
                thread
                    .update_sp(ctx)
                    .expect("Failed to update thread context");
            }
        }
    }

    /// Selects a new thread to run, sets the previous thread as ready, and sets the new thread as runs.
    /// The old thread will be added to the queue to be fired in the next period.
    /// The new thread will be selected based on the priority queue.
    ///
    /// Returns the context of the new thread to run, or `None` if no thread is available.
    fn select_new_thread(&mut self) -> Option<*mut c_void> {
        if let Some(id) = self.queue.pop().map(|(_, id)| id) {
            // Set the previous thread as ready. And add a callback from now.
            if let Some(id) = self.current {
                if let Some(thread) = self.threads.get_mut(&id) {
                    thread.update_run_state(RunState::Ready);
                    // The delay that is already in the queue.
                    let delay = self.callbacks.back().map(|(_, delay)| *delay).unwrap_or(0);
                    // Check if the period is already passed.
                    if thread.timing().period > (self.time + delay) {
                        // Add the callback to the queue. If it fails, we can't do much.
                        let _ = self
                            .callbacks
                            .push_back((id, thread.timing().period - (self.time + delay)));
                    } else {
                        // If the period is already passed, add it to the queue immediately.
                        let _ = self.queue.push((thread.timing().exec_time, id));
                    }
                }
            }

            if let Some(thread) = self.threads.get_mut(&id) {
                thread.update_run_state(RunState::Runs);

                // Set the new thread as the current one.
                self.current_interval = thread.timing().exec_time;
                self.current = Some(id);

                // Return the new thread context.
                return Some(thread.sp());
            }
        }

        None
    }

    /// Fires the thread if necessary.
    ///
    /// Returns `true` if a thread was fired, otherwise `false`.
    fn fire_thread_if_necessary(&mut self) -> bool {
        let mut found = false;
        while let Some((id, cnt)) = self.callbacks.front().cloned() {
            // If the delay is 0, we can fire the thread.
            if cnt - 1 == 0 {
                self.callbacks.pop_front();
                if let Some(thread) = self.threads.get_mut(&id) {
                    thread.update_run_state(RunState::Ready);

                    let _ = self.queue.push((thread.timing().exec_time, id));
                    found = true;
                }
            } else {
                // If the delay is not 0, we need to update the delay and reinsert it.
                let _ = self.callbacks.insert(0, (id, cnt - 1));
                break;
            }
        }

        found
    }

    /// Ticks the scheduler. This function is called every time the system timer ticks.
    pub fn tick(&mut self) -> bool {
        self.time += 1;

        // If a thread was fired, we need to reschedule.
        if self.fire_thread_if_necessary() {
            return true;
        }

        // If the current thread is done, we need to reschedule.
        if self.time >= self.current_interval {
            self.time = 0;
            return true;
        }

        false
    }
}

pub fn enabled() -> bool {
    SCHEDULER_ENABLED.load(core::sync::atomic::Ordering::Acquire)
}

pub fn set_enabled(enabled: bool) {
    SCHEDULER_ENABLED.store(enabled, core::sync::atomic::Ordering::Release);
}

/// cbindgen:ignore
/// cbindgen:no-export
#[unsafe(no_mangle)]
pub extern "C" fn sched_enter(ctx: *mut c_void) -> *mut c_void {
    {
        let mut scheduler = SCHEDULER.lock();
        // Update the current context.
        scheduler.update_current_ctx(ctx);

        // Select a new thread to run, if available.
        scheduler.select_new_thread().unwrap_or(ctx)
    }
}
