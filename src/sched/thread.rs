// ----------------------------------- Identifiers -----------------------------------

use core::{borrow::Borrow, ffi::c_void};

use hal::Stack;
use hal::stack::Stacklike;
use macros::TaggedLinks;

use crate::{types::{rbtree::{self, Compare}, traits::{Project, ToIndex}}, sched::task::TaskId, utils::KernelError};

/// Id of a task. This is only unique within a Task.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct Id {
    id: usize,
    owner: TaskId,
}

#[allow(dead_code)]
impl Id {
    pub fn new(id: usize, owner: TaskId) -> Self {
        Self { id, owner }
    }

    pub fn as_usize(&self) -> usize {
        self.id
    }

    pub fn owner(&self) -> TaskId {
        self.owner
    }

    pub fn get_uid(&self, uid: usize) -> UId {
        UId { uid, tid: *self }
    }
}

/// Unique identifier for a thread. Build from TaskId and ThreadId.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct UId {
    uid: usize,
    tid: Id,
}

#[allow(dead_code)]
impl UId {
    pub fn tid(&self) -> Id {
        self.tid
    }
}

impl PartialEq for UId {
    fn eq(&self, other: &Self) -> bool {
        self.uid == other.uid
    }
}

impl Eq for UId {}

impl Into<usize> for UId {
    fn into(self) -> usize {
        self.uid
    }
}

impl Default for UId {
    fn default() -> Self {
        Self {
            uid: 0,
            tid: Id::new(0, TaskId::User(0)),
        }
    }
}

impl PartialOrd for UId {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UId {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.uid.cmp(&other.uid)
    }
}

impl ToIndex for UId {
    fn to_index<Q: Borrow<Self>>(idx: Option<Q>) -> usize {
        idx.as_ref().map_or(0, |k| k.borrow().uid)
    }
}

// -------------------------------------------------------------------------

pub struct Descriptor {
    pub tid: Id,
    pub stack: Stack,
}

/// The state of a thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RunState {
    /// The thread is currently using the cpu.
    Runs,
    /// The thread is ready to run, but is not running.
    Ready,
    /// The thread is waiting for an event/signal to unblock it.
    Waits,
}

#[derive(Debug, Clone, Copy)]
pub struct State {
    run_state: RunState,
    stack: Stack,
}

#[derive(Debug, Clone, Copy)]
#[derive(TaggedLinks)]
pub struct RtServer {
    budget: u64,
    total_budget: u64,

    reservation: u64,
    deadline: u64,

    // Back-reference to the thread uid.
    uid: UId,

    /// Real-time tree links for the server.
    #[rbtree(tag = RtTree, idx = UId)]
    _rt_links: rbtree::Links<RtTree, UId>,
}

impl RtServer {
    pub fn new(budget: u64, reservation: u64, deadline: u64, uid: UId) -> Self {
        Self {
            budget,
            total_budget: budget,
            reservation,
            deadline,
            uid,
            _rt_links: rbtree::Links::new(),
        }
    }

    pub fn budget(&self) -> u64 {
        self.budget
    }

    pub fn replenish(&mut self, now: u64) {
        let next = self.deadline + self.reservation;
        self.deadline = next.max(now + self.reservation);
        self.budget = self.total_budget;
    }

    pub fn consume(&mut self, dt: u64) {
        if self.budget >= dt {
            self.budget -= dt;
        } else {
            self.budget = 0;
        }
    }

    pub fn deadline(&self) -> u64 {
        self.deadline
    }

    pub fn uid(&self) -> UId {
        self.uid
    }
}

impl Compare<RtTree, UId> for RtServer {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        let ord = self.deadline.cmp(&other.deadline);

        if ord == core::cmp::Ordering::Equal {
            self.uid.cmp(&other.uid)
        } else {
            ord
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct WakupTree;
#[derive(Debug, Clone, Copy)]
pub struct RtTree;

/// The struct representing a thread.
#[derive(Debug, Clone, Copy)]
#[derive(TaggedLinks)]
pub struct Thread {
    /// The current state of the thread.
    state: State,
    /// The unique identifier of the thread.
    uid: UId,
    /// If the thread is real-time, its contains a constant bandwidth server.
    rt_server: Option<RtServer>,
    /// Wakup tree links for the thread.
    #[rbtree(tag = WakupTree, idx = UId)]
    _wakeup_links: rbtree::Links<WakupTree, UId>,
}

#[allow(dead_code)]
impl Thread {
    /// Create a new thread.
    ///
    /// `stack` - The stack of the thread.
    ///
    /// Returns a new thread.
    fn new(uid: UId, stack: Stack) -> Self {
        Self {
            state: State {
                run_state: RunState::Ready,
                stack,
            },
            uid,
            rt_server: None,
            _wakeup_links: rbtree::Links::new(),
        }
    }

    pub fn save_ctx(&mut self, ctx: *mut c_void) -> Result<(), KernelError> {
        let sp = self.state.stack.create_sp(ctx)?;
        self.state.stack.set_sp(sp);
        Ok(())
    }

    pub fn set_run_state(&mut self, state: RunState) {
        self.state.run_state = state;
    }

    pub fn rt_server(&self) -> Option<&RtServer> {
        self.rt_server.as_ref()
    }

    pub fn ctx(&self) -> *mut c_void {
        self.state.stack.sp()
    }

    pub fn uid(&self) -> UId {
        self.uid
    }
}

impl Compare<WakupTree, UId> for Thread {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.uid.cmp(&other.uid)
    }
}

impl Project<RtServer> for Thread {
    fn project(&self) -> Option<&RtServer> {
         self.rt_server.as_ref()
    }

    fn project_mut(&mut self) -> Option<&mut RtServer> {
        self.rt_server.as_mut()
    }
}