

// ----------------------------------- Identifiers -----------------------------------

use core::{borrow::Borrow, ffi::c_void};

use hal::Stack;
use hal::stack::Stacklike;

use crate::{mem::array::IndexMap, sched::task::TaskId, utils::KernelError};

/// Id of a task. This is only unique within a Task.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct ThreadId {
    id: usize,
    owner: TaskId,
}

impl ThreadId {
    pub fn new(id: usize, owner: TaskId) -> Self {
        Self { id, owner }
    }

    pub fn as_usize(&self) -> usize {
        self.id
    }

    pub fn owner(&self) -> TaskId {
        self.owner
    }

    pub fn into_uid(&self, uid: usize) -> ThreadUId {
        ThreadUId { uid, tid: *self }
    }
}

/// Unique identifier for a thread. Build from TaskId and ThreadId.
#[derive(Clone, Copy, Debug)]
pub struct ThreadUId {
    uid: usize,
    tid: ThreadId,
}

impl PartialEq for ThreadUId {
    fn eq(&self, other: &Self) -> bool {
        self.uid == other.uid
    }
}

impl Eq for ThreadUId {}

impl Borrow<usize> for ThreadUId {
    fn borrow(&self) -> &usize {
        &self.uid
    }
}

impl Default for ThreadUId {
    fn default() -> Self {
        Self {
            uid: 0,
            tid: ThreadId::new(0, TaskId::User(0)),
        }
    }
}

impl PartialOrd for ThreadUId {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}   

impl Ord for ThreadUId {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.uid.cmp(&other.uid)
    }
}

// -------------------------------------------------------------------------

pub struct ThreadDescriptor {
    pub tid: ThreadId,
    pub stack: Stack,
    pub timing: Timing,
}


/// The timing information for a thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// The period of the thread after which it should run again.
    pub period: usize,
    /// The deadline of the thread.
    pub deadline: usize,
    /// The execution time of the thread. (How much cpu time it needs)
    pub exec_time: usize,
}

/// The state of a thread.
pub enum RunState {
    /// The thread is currently using the cpu.
    Runs,
    /// The thread is ready to run, but is not running.
    Ready,
    /// The thread is waiting for an event/signal to unblock it.
    Waits,
}

pub struct ThreadState {
    run_state: RunState,
    stack: Stack,
}

/// The struct representing a thread.
pub struct Thread {
    /// The current state of the thread.
    state: ThreadState,
    /// The timing constraints of the thread.
    timing: Timing,
    /// The unique identifier of the thread.
    tuid: ThreadUId,
}

impl Thread {
    /// Create a new thread.
    ///
    /// `stack` - The stack of the thread.
    /// `timing` - The timing constraints of the thread.
    ///
    /// Returns a new thread.
    fn new(tuid: ThreadUId, stack: Stack, timing: Timing) -> Self {
        Self {
            state: ThreadState {
                run_state: RunState::Ready,
                stack,
            },
            timing,
            tuid,
        }
    }

    pub fn update_sp(&mut self, sp: *mut c_void) -> Result<(), KernelError> {
        let sp = self.state.stack.create_sp(sp)?;
        self.state.stack.set_sp(sp);
        Ok(())
    }

    pub fn update_run_state(&mut self, state: RunState) {
        self.state.run_state = state;
    }

    pub fn timing(&self) -> &Timing {
        &self.timing
    }

    pub fn sp(&self) -> *mut c_void {
        self.state.stack.sp()
    }
}


pub struct ThreadMap<const N: usize> {
    map: IndexMap<ThreadUId, Thread, N>,
}

impl<const N: usize> ThreadMap<N> {
    pub const fn new() -> Self {
        Self {
            map: IndexMap::new(),
        }
    }

    pub fn create(&mut self, desc: ThreadDescriptor) -> Result<ThreadUId, KernelError> {
        let idx = self.map.find_empty().ok_or(KernelError::OutOfMemory)?;
        let tuid = desc.tid.into_uid(idx);
        let thread = Thread::new(tuid, desc.stack, desc.timing);

        self.map.insert(&tuid, thread)?;
        Ok(tuid)
    }

    pub fn get_mut(&mut self, id: &ThreadUId) -> Option<&mut Thread> {
        self.map.get_mut(id)
    }

    pub fn get(&self, id: &ThreadUId) -> Option<&Thread> {
        self.map.get(id)
    }

    pub fn remove(&mut self, id: &ThreadUId) -> Option<Thread> {
        self.map.remove(id)
    }
}