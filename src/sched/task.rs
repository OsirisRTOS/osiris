//! Task management module.

use core::{num::NonZero, ops::Add};

use hal::common::sched::{CtxPtr, ThreadContext, ThreadDesc};

use crate::mem::{alloc::AllocError, array::IndexMap};

pub type ThreadId = u32;

#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TaskId {
    Kernel = 0,
    User(NonZero<u16>),
}

impl From<TaskId> for usize {
    fn from(val: TaskId) -> Self {
        match val {
            TaskId::Kernel => 0,
            TaskId::User(id) => id.get() as usize,
        }
    }
}

impl From<usize> for TaskId {
    fn from(val: usize) -> Self {
        if val == 0 {
            TaskId::Kernel
        } else {
            TaskId::User(NonZero::new(val as u16).unwrap())
        }
    }
}

pub struct TaskDesc {
    pub mem_size: usize,
    pub stack_size: usize,
}

pub struct Task {
    pub id: TaskId,
    memory: TaskMemory,
    active_thread: ThreadId,
    threads: IndexMap<Thread, 4>,
}

impl Task {
    pub fn new(memory: TaskMemory, init_ctx: ThreadContext) -> Self {
        let mut threads = IndexMap::new();

        threads.insert(0, Thread {
            state: ThreadState::Ready,
            context: init_ctx,
        });

        
        Self {
            id: TaskId::Kernel,
            memory,
            active_thread: 0,
            threads,
        }
    }

    pub fn set_id(&mut self, id: TaskId) {
        self.id = id;
    }

    pub fn add_thread(&mut self, thread: Thread) -> Result<ThreadId, AllocError> {
        self.threads.insert_next(thread).map(|id| id as ThreadId)
    }

    pub fn remove_thread(&mut self, index: ThreadId) -> Option<Thread> {
        self.threads.remove(index as usize)
    }

    pub fn set_state(&mut self, index: ThreadId, state: ThreadState) {
        if let Some(thread) = self.threads.get_mut(index as usize) {
            thread.state = state;
        }
    }

    pub fn save_context(&mut self, index: ThreadId, ctx: ThreadContext) {
        if let Some(thread) = self.threads.get_mut(index as usize) {
            thread.context = ctx;
        }
    }

    pub fn get_active_thread(&self) -> ThreadId {
        self.active_thread
    }

    pub fn get_active_ctx(&self) -> Option<&ThreadContext> {
        self.threads.get(self.active_thread as usize).map(|thread| &thread.context)
    }
}

pub struct TaskMemory {
    begin: *mut u8,
    size: usize,
}

impl TaskMemory {
    pub fn new(begin: *mut u8, size: usize) -> Self {
        Self { begin, size }
    }

    pub fn stack(&self) -> *mut u8 {
        unsafe { self.begin.add(self.size) }
    }
}

pub struct Thread {
    state: ThreadState,
    context: ThreadContext,
}

pub enum ThreadState {
    Runs,
    Ready,
    Waits,
}
