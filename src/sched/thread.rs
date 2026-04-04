// ----------------------------------- Identifiers -----------------------------------

use core::fmt::Display;
use core::{borrow::Borrow, ffi::c_void};

use hal::stack::{FinFn, Stacklike};
use hal::{Stack, stack::EntryFn};
use proc_macros::TaggedLinks;

use crate::error::Result;
use crate::sched::task::{self, KERNEL_TASK};
use crate::types::list;
use crate::types::{
    rbtree::{self, Compare},
    traits::{Project, ToIndex},
};
use crate::uapi;

pub const IDLE_THREAD: UId = UId {
    uid: 1,
    tid: Id {
        id: 0,
        owner: KERNEL_TASK,
    },
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
#[derive(Clone, Copy, TaggedLinks)]
pub struct RtServer {
    budget: u32,
    budget_left: u32,
    period: u32,
    deadline: u64,

    // Back-reference to the thread uid.
    uid: UId,

    /// Real-time tree links for the server.
    #[rbtree(tag = RtTree, idx = UId)]
    _rt_links: rbtree::Links<RtTree, UId>,
}

impl RtServer {
    pub fn new(budget: u32, period: u32, deadline: u64, uid: UId) -> Self {
        Self {
            budget,
            budget_left: budget,
            period,
            deadline,
            uid,
            _rt_links: rbtree::Links::new(),
        }
    }

    pub fn budget_left(&self) -> u32 {
        self.budget_left
    }

    pub fn budget(&self) -> u32 {
        self.budget
    }

    fn violates_sched(&self, now: u64) -> bool {
        self.budget_left as u64 * self.period as u64
            > self.budget as u64 * (self.deadline.saturating_sub(now))
    }

    pub fn on_wakeup(&mut self, now: u64) {
        if self.deadline <= now || self.violates_sched(now) {
            self.deadline = now + self.period as u64;
            self.budget_left = self.budget;
        }
    }

    pub fn replenish(&mut self) {
        self.deadline = self.deadline + self.period as u64;
        self.budget_left += self.budget;
    }

    pub fn consume(&mut self, dt: u64) -> Option<u64> {
        self.budget_left = self.budget_left.saturating_sub(dt as u32);

        if self.budget_left == 0 {
            return Some(self.deadline);
        }

        None
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
#[derive(Clone, Copy, TaggedLinks)]
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
    pub attrs: Option<uapi::sched::RtAttrs>,
}

/// The struct representing a thread.
#[proc_macros::fmt]
#[derive(Clone, Copy, TaggedLinks)]
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
    pub fn new(uid: UId, stack: Stack, rtattrs: Option<uapi::sched::RtAttrs>) -> Self {
        let server =
            rtattrs.map(|attrs| RtServer::new(attrs.budget, attrs.period, attrs.deadline, uid));
        Self {
            state: State {
                run_state: RunState::Ready,
                stack,
            },
            uid,
            rt_server: server,
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

#[cfg(test)]
mod tests {
    use super::RtServer;

    fn make_server(budget: u32, period: u32, deadline: u64) -> RtServer {
        let tid = super::Id::new(1, super::task::KERNEL_TASK);
        let uid = tid.get_uid(1);
        RtServer::new(budget, period, deadline, uid)
    }

    #[test]
    fn replenish_budget_overflow() {
        // 2 * budget = 4_294_967_296 > u32::MAX → overflows.
        // In release: wraps to 0, which is less than budget.
        let budget: u32 = u32::MAX / 2 + 1;

        let mut server = make_server(budget, 1, 0);

        server.replenish();
        server.budget_left();
    }
}
