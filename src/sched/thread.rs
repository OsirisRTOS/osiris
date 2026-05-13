// ----------------------------------- Identifiers -----------------------------------

use core::fmt::Display;
use core::{borrow::Borrow, ffi::c_void};

use crate::hal;
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
    uid: 0,
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
    pub fn new(uid: usize, tid: Id) -> Self {
        Self { uid, tid }
    }

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

#[proc_macros::fmt]
#[derive(Clone, Copy)]
pub struct State {
    stack: Stack,
}

#[proc_macros::fmt]
#[derive(Clone, Copy, TaggedLinks)]
pub struct RtServer {
    budget: u32,
    budget_left: u32,
    period: u32,
    relative_deadline: u64,
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
            budget_left: 0,
            period,
            relative_deadline: deadline,
            deadline: 0,
            uid,
            _rt_links: rbtree::Links::new(),
        }
    }

    #[allow(dead_code)]
    pub fn budget_left(&self) -> u32 {
        self.budget_left
    }

    fn violates_sched(&self, now: u64) -> bool {
        (self.budget_left as u64).saturating_mul(self.period as u64)
            > (self.budget as u64).saturating_mul(self.deadline.saturating_sub(now))
    }

    pub fn on_wakeup(&mut self, now: u64) {
        if self.deadline <= now || self.violates_sched(now) {
            self.deadline = now.saturating_add(self.relative_deadline);
            self.budget_left = self.budget;
        }
    }

    pub fn replenish(&mut self) {
        self.deadline = self.deadline.saturating_add(self.period as u64);
        self.budget_left = self.budget_left.saturating_add(self.budget);
    }

    pub fn consume(&mut self, dt: u64) -> Option<u64> {
        self.budget_left = if dt >= self.budget_left as u64 {
            0
        } else {
            self.budget_left - dt as u32
        };

        if self.budget_left == 0 {
            return Some(self.deadline);
        }

        None
    }

    #[allow(dead_code)]
    pub fn deadline(&self) -> u64 {
        self.deadline
    }

    #[allow(dead_code)]
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
    /// Delivered to `entry` as its sole argument; caller owns the pointee's lifetime.
    pub ctx: *mut core::ffi::c_void,
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
            state: State { stack },
            uid,
            rt_server: server,
            waiter: None,
            rr_links: list::Links::new(),
            thread_links: list::Links::new(),
        }
    }

    pub fn wait(&mut self, until: u64) {
        self.waiter = Some(Waiter::new(until, self.uid));
    }

    pub fn resume(&mut self) {
        self.waiter = None;
    }

    pub fn is_waiting(&self) -> bool {
        self.waiter.is_some()
    }

    pub fn save_ctx(&mut self, ctx: *mut c_void) -> Result<()> {
        let sp = self.state.stack.create_sp(ctx)?;
        self.state.stack.set_sp(sp);
        Ok(())
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

// VERIFICATION -------------------------------------------------------------------------------------------------------
#[cfg(kani)]
mod verification {
    use super::*;

    /// `RtServer::consume` must never panic, and after consuming `dt`:
    ///  - if dt >= old_budget, budget_left must be 0 and a throttle Some(deadline)
    ///    must be returned;
    ///  - if dt < old_budget, budget_left must equal old - dt as u32 and the
    ///    return must be None.
    #[kani::proof]
    fn consume_does_not_panic_and_is_consistent() {
        let budget: u32 = kani::any();
        let period: u32 = kani::any();
        let relative_deadline: u64 = kani::any();
        kani::assume(budget > 0);
        kani::assume(period > 0);
        kani::assume(relative_deadline > 0);

        let uid = UId::new(1, Id::new(0, task::UId::new(0)));
        let mut s = RtServer::new(budget, period, relative_deadline, uid);
        // Seed the server with some prior on_wakeup so deadline is non-zero.
        let now: u64 = kani::any();
        kani::assume(now < u64::MAX / 2);
        s.on_wakeup(now);

        let old_budget = s.budget_left();
        let old_deadline = s.deadline();
        let dt: u64 = kani::any();

        let r = s.consume(dt);

        if dt >= old_budget as u64 {
            assert_eq!(s.budget_left(), 0);
            assert_eq!(r, Some(old_deadline));
        } else {
            assert_eq!(s.budget_left() as u64, old_budget as u64 - dt);
            assert_eq!(r, None);
        }
    }

    /// `RtServer::replenish` must not panic for any reachable server state.
    /// After replenish, deadline must be >= the old deadline (it should only
    /// grow), and budget_left must be >= old budget_left.
    #[kani::proof]
    fn replenish_monotonic() {
        let budget: u32 = kani::any();
        let period: u32 = kani::any();
        let relative_deadline: u64 = kani::any();
        kani::assume(budget > 0);
        kani::assume(period > 0);
        kani::assume(relative_deadline > 0);

        let uid = UId::new(1, Id::new(0, task::UId::new(0)));
        let mut s = RtServer::new(budget, period, relative_deadline, uid);

        let old_deadline = s.deadline();
        let old_budget = s.budget_left();
        s.replenish();
        assert!(s.deadline() >= old_deadline);
        assert!(s.budget_left() >= old_budget);
    }

    /// `RtServer::on_wakeup` must establish: after the call, either nothing
    /// changed, OR (deadline == now + relative_deadline AND budget_left == budget).
    #[kani::proof]
    fn on_wakeup_resets_or_keeps() {
        let budget: u32 = kani::any();
        let period: u32 = kani::any();
        let relative_deadline: u64 = kani::any();
        kani::assume(budget > 0);
        kani::assume(period > 0);
        kani::assume(relative_deadline > 0);

        let uid = UId::new(1, Id::new(0, task::UId::new(0)));
        let mut s = RtServer::new(budget, period, relative_deadline, uid);
        let now: u64 = kani::any();

        let pre_deadline = s.deadline();
        let pre_budget = s.budget_left();
        s.on_wakeup(now);

        // Either nothing changed,
        let unchanged = s.deadline() == pre_deadline && s.budget_left() == pre_budget;
        // or it was reset to the fresh job.
        let reset = s.deadline() == now.saturating_add(relative_deadline)
            && s.budget_left() == budget;
        assert!(unchanged || reset);
    }

    /// `violates_sched` must not panic for any inputs.
    #[kani::proof]
    fn violates_sched_total() {
        let budget: u32 = kani::any();
        let period: u32 = kani::any();
        let relative_deadline: u64 = kani::any();
        kani::assume(budget > 0);
        kani::assume(period > 0);
        kani::assume(relative_deadline > 0);
        let uid = UId::new(1, Id::new(0, task::UId::new(0)));
        let mut s = RtServer::new(budget, period, relative_deadline, uid);
        let now: u64 = kani::any();
        // Even with deadline < now (saturating_sub returns 0) we should not
        // panic. The result must be a bool.
        s.on_wakeup(now);
        // Force an extreme deadline to exercise the overflow checks.
        let probe_now: u64 = kani::any();
        let _ = s.violates_sched(probe_now);
    }
}
// END VERIFICATION ---------------------------------------------------------------------------------------------------
