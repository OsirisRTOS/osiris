//! Task management module.

use core::ptr::NonNull;

use hal::common::sched::{ThreadContext, ThreadDesc};

use crate::mem::{self, alloc::AllocError, array::Vec};

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
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ThreadUID {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.task.cmp(&other.task).then(self.thread.cmp(&other.thread))
    }
}

// -------------------------------------------------------------------------

pub struct TaskDesc {
    pub mem_size: usize,
    pub stack_size: usize,
}

pub struct Task {
    pub id: TaskId,
    memory: TaskMemory,
    active_thread: ThreadId,
    threads: Vec<ThreadId, 4>
}

impl Task {
    pub fn new(memory_size: usize) -> Result<Self, AllocError> {
        let memory = TaskMemory::new(memory_size)?;
        let threads = Vec::new();

        Ok(Self {
            id: TaskId::User(0),
            memory,
            active_thread: 0,
            threads,
        })
    }

    pub fn create_thread_ctx(&self, desc: ThreadDesc) -> Result<ThreadContext, AllocError> {
        let stack = self.memory.stack();
        // TODO: Check if stack is sufficient
        let ctx = unsafe { ThreadContext::from_empty(stack.as_ptr(), desc) };
        Ok(ctx)
    }

    pub fn register_thread(&mut self, thread_id: ThreadId) -> Result<(), AllocError> {
        self.threads.push(thread_id)
    }
}

pub struct TaskMemory {
    begin: NonNull<u8>,
    size: usize,
}

impl TaskMemory {
    pub fn new(size: usize) -> Result<Self, AllocError> {
        let begin = mem::malloc(size, align_of::<u128>()).ok_or(AllocError::OutOfMemory)?;
        Ok(Self { begin, size })
    }

    pub fn stack(&self) -> NonNull<u8> {
        unsafe { self.begin.add(self.size) }
    }
}

pub struct Timing {
    pub period: usize,
    pub deadline: usize,
    pub exec_time: usize,
}

pub struct Thread {
    pub state: ThreadState,
    pub context: ThreadContext,
    pub period: usize,
    pub deadline: usize,
    pub exec_time: usize,
}

impl Thread {
    pub fn new(ctx: ThreadContext, timing: Timing) -> Self {
        Self {
            state: ThreadState::Ready,
            context: ctx,
            period: timing.period,
            deadline: timing.deadline,
            exec_time: timing.exec_time,
        }
    }
}

pub enum ThreadState {
    Runs,
    Ready,
    Waits,
}
