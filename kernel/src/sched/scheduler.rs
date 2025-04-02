//! The scheduler module is responsible for managing the tasks and threads in the system.
//! It provides the necessary functions to create tasks and threads, and to switch between them.

use super::task::{Task, TaskDesc, TaskId, Thread, ThreadId, ThreadState, Timing};
use crate::{
    mem::{self, array::IndexMap, heap::BinaryHeap, queue::Queue},
    utils,
};
use hal::common::{
    sched::{CtxPtr, ThreadDesc},
    sync::SpinLocked,
};

/// The global scheduler instance.
pub static SCHEDULER: SpinLocked<Scheduler> = SpinLocked::new(Scheduler::new());

/// The scheduler struct. It keeps track of the tasks and threads in the system.
/// This scheduler is a simple Rate Monotonic Scheduler (RMS) implementation.
pub struct Scheduler {
    /// The current running thread.
    current: Option<ThreadId>,
    /// Fast interval store. This gets updated every time a new thread is selected.
    current_interval: usize,
    /// Stores the tasks in the system.
    tasks: IndexMap<Task, 8>,
    /// Stores the threads in the system.
    threads: IndexMap<Thread, 32>,
    /// The priority queue that yields the next thread to run.
    queue: BinaryHeap<(usize, ThreadId), 32>,
    /// The callbacks queue that stores the threads that need to be fired in the future.
    callbacks: Queue<(ThreadId, usize), 32>,
    /// The current time in the system.
    time: usize,
}

impl Scheduler {
    /// Create a new scheduler instance.
    pub const fn new() -> Self {
        Self {
            current: None,
            current_interval: 0,
            tasks: IndexMap::new(),
            threads: IndexMap::new(),
            queue: BinaryHeap::new(),
            callbacks: Queue::new(),
            time: 0,
        }
    }

    /// Create a new task in the system.
    ///
    /// `desc` - The task descriptor.
    /// `main_desc` - The main thread descriptor.
    /// `main_timing` - The timing information for the main thread.
    ///
    /// Returns the task ID if the task was created successfully, or an error if the task could not be created.
    pub fn create_task(
        &mut self,
        desc: TaskDesc,
        main_desc: ThreadDesc,
        main_timing: Timing,
    ) -> Result<TaskId, utils::KernelError> {
        let size = mem::align_up(desc.mem_size) + mem::align_up(desc.stack_size);
        let mut task = Task::new(size)?;

        let period = main_timing.period;

        let thread_ctx = task.create_thread_ctx(main_desc)?;
        let thread = Thread::new(thread_ctx, main_timing);

        let thread_id = self.threads.insert_next(thread)?;
        task.register_thread(thread_id)?;

        let task_id = self.tasks.insert_next(task)?;

        self.queue.push((period, thread_id))?;

        if let Some(task) = self.tasks.get_mut(task_id) {
            task.id = task_id.into();
            return Ok(task_id.into());
        }

        Err(utils::KernelError::OutOfMemory)
    }

    /// Updates the current thread context with the given context.
    ///
    /// `ctx` - The new context to update the current thread with.
    fn update_current_ctx(&mut self, ctx: CtxPtr) {
        if let Some(id) = self.current {
            if let Some(thread) = self.threads.get_mut(id) {
                thread.context = ctx.into();
            }
        }
    }

    /// Selects a new thread to run, sets the previous thread as ready, and sets the new thread as runs.
    /// The old thread will be added to the queue to be fired in the next period.
    /// The new thread will be selected based on the priority queue.
    ///
    /// Returns the context of the new thread to run, or `None` if no thread is available.
    fn select_new_thread(&mut self) -> Option<CtxPtr> {
        if let Some(id) = self.queue.pop().map(|(_, id)| id) {
            // Set the previous thread as ready. And add a callback from now.
            if let Some(id) = self.current {
                if let Some(thread) = self.threads.get_mut(id) {
                    thread.state = ThreadState::Ready;
                    // The delay that is already in the queue.
                    let delay = self.callbacks.back().map(|(_, delay)| *delay).unwrap_or(0);
                    // Check if the period is already passed.
                    if thread.timing.period > (self.time + delay) {
                        // Add the callback to the queue. If it fails, we can't do much.
                        let _ = self
                            .callbacks
                            .push_back((id, thread.timing.period - (self.time + delay)));
                    } else {
                        // If the period is already passed, add it to the queue immediately.
                        let _ = self.queue.push((thread.timing.exec_time, id));
                    }
                }
            }

            if let Some(thread) = self.threads.get_mut(id) {
                thread.state = ThreadState::Runs;

                // Set the new thread as the current one.
                self.current_interval = thread.timing.exec_time;
                self.current = Some(id);

                // Return the new thread context.
                return Some(thread.context.into());
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
                if let Some(thread) = self.threads.get_mut(id) {
                    thread.state = ThreadState::Ready;
                    let _ = self.queue.push((thread.timing.exec_time, id));
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
    fn tick(&mut self) -> bool {
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

/// cbindgen:ignore
/// cbindgen:no-export
#[unsafe(no_mangle)]
pub extern "C" fn sched_enter(ctx: CtxPtr) -> CtxPtr {
    {
        let mut scheduler = SCHEDULER.lock();

        // Update the current context.
        scheduler.update_current_ctx(ctx);

        // Select a new thread to run, if available.
        scheduler.select_new_thread().unwrap_or(ctx)
    }
}

/// cbindgen:ignore
/// cbindgen:no-export
#[unsafe(no_mangle)]
pub extern "C" fn systick() {
    let resched = {
        let mut scheduler = SCHEDULER.lock();
        scheduler.tick()
    };

    if resched {
        hal::common::sched::reschedule();
    }
}
