//! This module provides the basic task and thread structures for the scheduler.

use core::{cmp::Ordering, ptr::NonNull};

use crate::{mem};

use crate::utils::KernelError;

// ----------------------------------- Identifiers -----------------------------------

/// Id of a task. This is unique across all tasks.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum TaskId {
    User(u16),
}

/// Convert TaskId to usize.
impl From<TaskId> for usize {
    fn from(val: TaskId) -> Self {
        match val {
            TaskId::User(id) => id as usize,
        }
    }
}

/// Convert usize to TaskId.
impl From<usize> for TaskId {
    fn from(val: usize) -> Self {
        TaskId::User(val as u16)
    }
}

/// Id of a task. This is only unique within a Task.
pub type ThreadId = usize;

/// Unique identifier for a thread. Build from TaskId and ThreadId.
#[derive(Clone, Copy, Debug)]
pub struct ThreadUID {
    pub task: TaskId,
    pub thread: ThreadId,
}

impl ThreadUID {
    pub fn new(task: TaskId, thread: ThreadId) -> Self {
        Self { task, thread }
    }
}

impl PartialEq for ThreadUID {
    fn eq(&self, other: &Self) -> bool {
        self.task == other.task && self.thread == other.thread
    }
}

impl Eq for ThreadUID {}

impl PartialOrd for ThreadUID {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ThreadUID {
    fn cmp(&self, other: &Self) -> Ordering {
        self.task
            .cmp(&other.task)
            .then(self.thread.cmp(&other.thread))
    }
}

// -------------------------------------------------------------------------

/// Descibes a task.
pub struct TaskDesc {
    /// The size of the memory that the task requires.
    pub mem_size: usize,
    /// The size of the stack that the task requires.
    pub stack_size: usize,
}

/// The struct representing a task.
pub struct Task {
    /// The unique identifier of the task.
    pub id: TaskId,
    /// The memory of the task.
    memory: TaskMemory,
    /// The threads associated with the task.
    threads: mem::array::Vec<ThreadId, 4>,
}

impl Task {
    /// Create a new task.
    ///
    /// `memory_size` - The size of the memory that the task requires.
    ///
    /// Returns a new task if the task was created successfully, or an error if the task could not be created.
    pub fn new(memory_size: usize) -> Result<Self, KernelError> {
        let memory = TaskMemory::new(memory_size)?;
        let threads = mem::array::Vec::new();

        Ok(Self {
            id: TaskId::User(0),
            memory,
            threads,
        })
    }

    /// Create a new thread context for the task.
    ///
    /// `desc` - The descriptor for the thread.
    ///
    /// Returns the thread context if the thread was created successfully, or an error if the thread could not be created. TODO: Check if stack is sufficient
    pub fn create_thread_ctx(
        &self,
        desc: hal::sched::ThreadDesc,
    ) -> Result<hal::sched::ThreadContext, KernelError> {
        let stack = self.memory.stack();

        // TODO: Check if stack is sufficient
        let ctx = unsafe { hal::sched::ThreadContext::from_empty(stack.as_ptr(), desc) };
        Ok(ctx)
    }

    /// Register a thread with the task.
    ///
    /// `thread_id` - The id of the thread to register.
    ///
    /// Returns `Ok(())` if the thread was registered successfully, or an error if the thread could not be registered. TODO: Check if the thread is using the same memory as the task.
    pub fn register_thread(&mut self, thread_id: ThreadId) -> Result<(), KernelError> {
        self.threads.push(thread_id)
    }
}

/// The memory of a task.
pub struct TaskMemory {
    /// The beginning of the memory.
    begin: NonNull<u8>,
    /// The size of the memory.
    size: usize,
}

impl TaskMemory {
    /// Create a new task memory.
    ///
    /// `size` - The size of the memory.
    ///
    /// Returns a new task memory if the memory was created successfully, or an error if the memory could not be created.
    pub fn new(size: usize) -> Result<Self, KernelError> {
        let begin = mem::malloc(size, align_of::<u128>()).ok_or(KernelError::OutOfMemory)?;
        Ok(Self { begin, size })
    }

    /// Get the stack of the task.
    pub fn stack(&self) -> NonNull<u8> {
        unsafe { self.begin.add(self.size) }
    }
}

/// The timing information for a thread.
pub struct Timing {
    /// The period of the thread after which it should run again.
    pub period: usize,
    /// The deadline of the thread.
    pub deadline: usize,
    /// The execution time of the thread. (How much cpu time it needs)
    pub exec_time: usize,
}

/// The state of a thread.
pub enum ThreadState {
    /// The thread is currently using the cpu.
    Runs,
    /// The thread is ready to run, but is not running.
    Ready,
    /// The thread is waiting for an event/signal to unblock it.
    Waits,
}

/// The struct representing a thread.
pub struct Thread {
    /// The state of the thread.
    pub state: ThreadState,
    /// The context of the thread.
    pub context: hal::sched::ThreadContext,
    /// The timing constraints of the thread.
    pub timing: Timing,
}

impl Thread {
    /// Create a new thread.
    ///
    /// `ctx` - The context of the thread.
    /// `timing` - The timing constraints of the thread.
    ///
    /// Returns a new thread.
    pub fn new(ctx: hal::sched::ThreadContext, timing: Timing) -> Self {
        Self {
            state: ThreadState::Ready,
            context: ctx,
            timing,
        }
    }
}
