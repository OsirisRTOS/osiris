//! This module provides the basic task and thread structures for the scheduler.
use core::num::NonZero;
use core::ops::Range;
use core::{ptr::NonNull};

use hal::Stack;

use hal::stack::Stacklike;

use crate::mem;

use crate::mem::alloc::{Allocator, BestFitAllocator};
use crate::sched::thread::{ThreadDescriptor, ThreadId, Timing};
use crate::utils::KernelError;

/// Id of a task. This is unique across all tasks.
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum TaskId {
    // Task with normal user privileges in user mode.
    User(usize),
    // Task with kernel privileges in user mode.
    Kernel(usize),
}

impl TaskId {
    /// Check if the task is a user task.
    pub fn is_user(&self) -> bool {
        matches!(self, TaskId::User(_))
    }

    /// Check if the task is a kernel task.
    pub fn is_kernel(&self) -> bool {
        matches!(self, TaskId::Kernel(_))
    }

    pub fn new_user(id: usize) -> Self {
        TaskId::User(id)
    }

    pub fn new_kernel(id: usize) -> Self {
        TaskId::Kernel(id)
    }
}

impl Into<usize> for TaskId {
    fn into(self) -> usize {
        match self {
            TaskId::User(id) => id,
            TaskId::Kernel(id) => id,
        }
    }
}


/// Descibes a task.
pub struct TaskDescriptor {
    /// The size of the memory that the task requires.
    pub mem_size: usize,
}

/// The struct representing a task.
pub struct Task {
    /// The unique identifier of the task.
    pub id: TaskId,
    /// The memory of the task.
    memory: TaskMemory,
    /// The allocator for the task's memory.
    alloc: BestFitAllocator,
    /// The counter for the thread ids.
    tid_cntr: usize,
    /// The threads associated with the task.
    threads: mem::array::Vec<ThreadId, 4>,
}

impl Task {
    /// Create a new task.
    ///
    /// `memory_size` - The size of the memory that the task requires.
    ///
    /// Returns a new task if the task was created successfully, or an error if the task could not be created.
    pub fn new(memory_size: usize, id: TaskId) -> Result<Self, KernelError> {
        let memory = TaskMemory::new(memory_size)?;
        let threads = mem::array::Vec::new();

        let mut alloc = BestFitAllocator::new();
        let range = Range {
            start: memory.begin.as_ptr() as usize,
            end: memory.begin.as_ptr() as usize + memory.size,
        };

        unsafe { alloc.add_range(range) }?;

        Ok(Self {
            id,
            memory,
            alloc,
            tid_cntr: 0,
            threads,
        })
    }

    fn allocate_tid(&mut self) -> ThreadId {
        let tid = self.tid_cntr;
        self.tid_cntr += 1;

        ThreadId::new(tid, self.id)
    }

    pub fn create_thread(
        &mut self,
        entry: extern "C" fn(),
        fin: Option<extern "C" fn() -> !>,
        timing: Timing,
    ) -> Result<ThreadDescriptor, KernelError> {
        // Safe unwrap because stack size is non zero.
        // TODO: Make this configurable
        let stack_size = NonZero::new(4096usize).unwrap();
        // TODO: Revert if error occurs
        let stack_mem = self.alloc.malloc(stack_size.into(), align_of::<u128>())?;
        let stack_top = unsafe { stack_mem.byte_add(stack_size.get()) };

        let stack = hal::stack::StackDescriptor {
            top: stack_top,
            size: stack_size,
            entry,
            fin,
        };

        let stack = unsafe { Stack::new(stack) }?;

        let tid = self.allocate_tid();

        // TODO: Revert if error occurs
        self.register_thread(tid)?;

        Ok(ThreadDescriptor { tid, stack, timing })
    }

    /// Register a thread with the task.
    ///
    /// `thread_id` - The id of the thread to register.
    ///
    /// Returns `Ok(())` if the thread was registered successfully, or an error if the thread could not be registered. TODO: Check if the thread is using the same memory as the task.
    fn register_thread(&mut self, thread_id: ThreadId) -> Result<(), KernelError> {
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
}
