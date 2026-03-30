// ----------------------------------- Identifiers -----------------------------------

use core::fmt::Display;
use core::{borrow::Borrow, ffi::c_void};

use hal::{Stack, stack::EntryFn};
use hal::stack::{FinFn, Stacklike};
use proc_macros::TaggedLinks;

use crate::error::Result;
use crate::sched::task::{self, KERNEL_TASK};
use crate::time::tick;
use crate::types::list;
use crate::{types::{rbtree::{self, Compare}, traits::{Project, ToIndex}}};

pub const IDLE_THREAD: UId = UId {
    uid: 1,
    tid: Id { id: 0, owner: KERNEL_TASK },
};

/// Id of a task. This is only unique within a Task.
#[proc_macros::fmt]
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct Id {
    id: usize,
    owner: task::UId,
}

#[allow(dead_code)]
impl Id {
    pub fn new(id: usize, owner: task::UId) -> Self {
        Self { id, owner }
    }

    pub fn as_usize(&self) -> usize {
        self.id
    }

    pub fn owner(&self) -> task::UId {
        self.owner
    }

    pub fn get_uid(&self, uid: usize) -> UId {
        UId { uid, tid: *self }
    }
}

/// Unique identifier for a thread. Build from TaskId and ThreadId.
#[proc_macros::fmt]
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct UId {
    /// A globally unique identifier for the thread.
    uid: usize,
    /// The task-local identifier for the thread.
    tid: Id,
}

#[allow(dead_code)]
impl UId {
    pub fn tid(&self) -> Id {
        self.tid
    }

    pub fn as_usize(&self) -> usize {
        self.uid
    }

    pub fn owner(&self) -> task::UId {
        self.tid.owner()
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

impl Display for UId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}-{}", self.tid.owner(), self.tid.as_usize())
    }
}

// -------------------------------------------------------------------------

/// The state of a thread.
#[proc_macros::fmt]
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RunState {
    /// The thread is currently using the cpu.
    Runs,
    /// The thread is ready to run, but is not running.
    Ready,
    /// The thread is waiting for an event/signal to unblock it.
    Waits,
}

#[proc_macros::fmt]
#[derive(Clone, Copy)]
pub struct State {
    run_state: RunState,
    stack: Stack,
}

#[proc_macros::fmt]
#[derive(Clone, Copy)]
#[derive(TaggedLinks)]
pub struct RtServer {
    budget: u64,
    budget_left: u64,
    period: u64,
    deadline: u64,

    // Back-reference to the thread uid.
    uid: UId,

    /// Real-time tree links for the server.
    #[rbtree(tag = RtTree, idx = UId)]
    _rt_links: rbtree::Links<RtTree, UId>,
}

impl RtServer {
    pub fn new(budget: u64, period: u64, uid: UId) -> Self {
        Self {
            budget,
            budget_left: budget,
            period,
            deadline: tick() + period,
            uid,
            _rt_links: rbtree::Links::new(),
        }
    }

    pub fn budget_left(&self) -> u64 {
        self.budget_left
    }

    pub fn budget(&self) -> u64 {
        self.budget
    }

    pub fn replenish(&mut self, now: u64) {
        self.deadline += self.period;
        self.budget_left = self.budget;
    }

    pub fn consume(&mut self, dt: u64) {
        if self.budget_left >= dt {
            self.budget_left -= dt;
        } else {
            self.budget_left = 0;
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

#[proc_macros::fmt]
#[derive(Clone, Copy)]
#[derive(TaggedLinks)]
pub struct Waiter {
    /// The time when the Thread will be awakened.
    until: u64,

    // Back-reference to the thread uid.
    uid: UId,
    /// Wakup tree links for the thread.
    #[rbtree(tag = WakupTree, idx = UId)]
    _wakeup_links: rbtree::Links<WakupTree, UId>,
}

impl Waiter {
    pub fn new(until: u64, uid: UId) -> Self {
        Self {
            until,
            uid,
            _wakeup_links: rbtree::Links::new(),
        }
    }

    pub fn until(&self) -> u64 {
        self.until
    }

    pub fn set_until(&mut self, until: u64) {
        self.until = until;
    }
}

impl Compare<WakupTree, UId> for Waiter {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match self.until.cmp(&other.until) {
            core::cmp::Ordering::Equal => self.uid.cmp(&other.uid),
            ord => ord,
        }
    }
}

#[proc_macros::fmt]
#[derive(Clone, Copy)]
pub struct WakupTree;
#[proc_macros::fmt]
#[derive(Clone, Copy)]
pub struct RtTree;

#[proc_macros::fmt]
#[derive(Clone, Copy)]
pub struct RRList;

#[proc_macros::fmt]
#[derive(Clone, Copy)]
pub struct ThreadList;

pub struct Attributes {
    pub entry: EntryFn,
    pub fin: Option<FinFn>,
}

/// The struct representing a thread.
#[proc_macros::fmt]
#[derive(Clone, Copy)]
#[derive(TaggedLinks)]
pub struct Thread {
    /// The current state of the thread.
    state: State,
    /// The unique identifier of the thread.
    uid: UId,
    /// If the thread is real-time, its contains a constant bandwidth server.
    rt_server: Option<RtServer>,

    waiter: Option<Waiter>,

    #[list(tag = RRList, idx = UId)]
    rr_links: list::Links<RRList, UId>,

    #[list(tag = ThreadList, idx = UId)]
    thread_links: list::Links<ThreadList, UId>,
}

#[allow(dead_code)]
impl Thread {
    /// Create a new thread.
    ///
    /// `stack` - The stack of the thread.
    ///
    /// Returns a new thread.
    pub fn new(uid: UId, stack: Stack) -> Self {
        Self {
            state: State {
                run_state: RunState::Ready,
                stack,
            },
            uid,
            rt_server: None,
            waiter: None,
            rr_links: list::Links::new(),
            thread_links: list::Links::new(),
        }
    }

    pub fn set_waiter(&mut self, waiter: Option<Waiter>) {
        self.waiter = waiter;
    }

    pub fn waiter(&self) -> Option<&Waiter> {
        self.waiter.as_ref()
    }

    pub fn save_ctx(&mut self, ctx: *mut c_void) -> Result<()> {
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

    pub fn task_id(&self) -> task::UId {
        self.uid.tid().owner()
    }
}

impl PartialEq for Thread {
    fn eq(&self, other: &Self) -> bool {
        self.uid == other.uid
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

impl Project<Waiter> for Thread {
    fn project(&self) -> Option<&Waiter> {
         self.waiter.as_ref()
    }

    fn project_mut(&mut self) -> Option<&mut Waiter> {
        self.waiter.as_mut()
    }
}
