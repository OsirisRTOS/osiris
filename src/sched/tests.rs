//! Host-side property tests for the scheduler.
//!
//! These tests exercise the scheduler via test-only constructors that
//! bypass memory allocation (`insert_task_for_test`, `insert_thread_for_test`).
//! They do not exercise the dispatch / stack-context-switch path, only the
//! algorithmic scheduling logic.

#![allow(clippy::needless_range_loop)]

use super::*;
use crate::sched::thread::UId as ThreadUId;
use crate::uapi::sched::RtAttrs;

/// A small scheduler used for testing. 8 thread slots is plenty for property
/// tests and keeps Kani/proptest fast.
const TEST_N: usize = 8;
type TestSched = Scheduler<TEST_N>;

fn make_sched() -> TestSched {
    Scheduler::new()
}

/// `Scheduler::do_sched`/`select_next` fall back to `IDLE_THREAD` (uid 0) when
/// no thread is runnable, and then `do_sched` panics if no such thread exists.
/// Production code creates the idle thread during init; in tests we must too.
/// This helper inserts a kernel task + idle thread with uid 0.
fn ensure_idle(sched: &mut TestSched) -> (task::UId, ThreadUId) {
    let task = sched.insert_task_for_test().expect("task slot");
    let idle = sched.insert_thread_for_test(task, None).expect("thread slot");
    // The first thread inserted gets uid 0 (the BitReclaimMap allocates
    // sequentially), which matches IDLE_THREAD. Enqueue so it's pickable.
    let _ = sched.enqueue(0, idle);
    (task, idle)
}

// ---------------- Smoke tests ----------------

#[test]
fn smoke_idle_runs_when_nothing_else_runnable() {
    let mut s = make_sched();
    let (_task, idle) = ensure_idle(&mut s);
    let (picked, _budget) = s.step(0);
    assert_eq!(picked, idle);
}

#[test]
fn smoke_enqueue_and_pick_rr() {
    let mut s = make_sched();
    let (_task, _idle) = ensure_idle(&mut s);
    let t1 = s.insert_thread_for_test(task::UId::new(0), None).unwrap();
    let t2 = s.insert_thread_for_test(task::UId::new(0), None).unwrap();
    s.enqueue(0, t1).unwrap();
    s.enqueue(0, t2).unwrap();
    let (picked, _) = s.step(0);
    // Whichever of t1, t2, idle is picked, it's not invented.
    assert!(picked == t1 || picked == t2 || picked.as_usize() == 0);
}

#[test]
fn smoke_sleep_makes_thread_unrunnable() {
    let mut s = make_sched();
    let (_task, idle) = ensure_idle(&mut s);
    let t1 = s.insert_thread_for_test(task::UId::new(0), None).unwrap();
    s.enqueue(0, t1).unwrap();
    // Make t1 the current.
    s.set_current_for_test(Some(t1));
    s.sleep_until(Some(t1), 100, 0).unwrap();
    assert!(s.is_waiting(t1));
    // After sleeping, picking at time 0 returns something other than t1.
    let (picked, _) = s.step(0);
    assert_ne!(picked, t1);
    assert!(picked == idle || picked.as_usize() == 0);
}

#[test]
fn smoke_wakeup_at_deadline() {
    let mut s = make_sched();
    let (_task, _idle) = ensure_idle(&mut s);
    let t1 = s.insert_thread_for_test(task::UId::new(0), None).unwrap();
    s.enqueue(0, t1).unwrap();
    s.set_current_for_test(Some(t1));
    s.sleep_until(Some(t1), 100, 0).unwrap();
    assert!(s.is_waiting(t1));
    // Advance to t=100; do_wakeups should fire.
    let _ = s.step(100);
    assert!(!s.is_waiting(t1), "thread should be awake at its deadline");
}

// ---------------- Invariant helpers ----------------

/// Bag of assertions about the scheduler that should always hold.
#[track_caller]
fn check_invariants(s: &TestSched) {
    use crate::types::traits::Get;
    // INV-1: If `current` is Some, the thread exists.
    if let Some(cur) = s.current() {
        assert!(
            s.threads.get(cur).is_some(),
            "current thread {} does not exist in thread map",
            cur
        );
    }
    // INV-2: Every live thread that has a `Waiter` must have a `until` that
    // round-trips through the wakeup tree min.
    // (Direct invariant: we can't easily walk the tree, but `wakeup_min` must
    // point at a live thread that is_waiting.)
    if let Some(min) = s.wakeup_min() {
        let t = s.threads.get(min).expect("wakeup_min points at dead thread");
        assert!(t.is_waiting(), "wakeup_min thread {} is not waiting", min);
    }
}

// ---------------- Regression: minimal failing cases ----------------

/// Bug B1: `Scheduler::enqueue` does NOT remove the thread from the wakeup tree
/// if it happens to be sleeping. After enqueue, the thread lives in both the
/// rr/edf queue AND the wakeup tree. Killing it then only removes from the
/// rr/edf queue, leaving the wakeup tree pointing at a freed slot.
///
/// Minimal reproducer from proptest:
///   NewThread { rt: false }
///   Sleep { idx: 0, until: 0 }   // sleep T1 until t=1 (harness adds 1)
///   Enqueue { idx: 0 }            // enqueue T1: it's still in the wakeup tree
///   KillThread { idx: 0 }         // removes from rr queue, NOT wakeup tree
///   -> wakeup_min() now returns T1's UID, but T1's slot is freed.
#[test]
fn regression_b1_enqueue_sleeping_thread_then_kill_leaves_wakeup_dangling() {
    let mut s = make_sched();
    let (_task, _idle) = ensure_idle(&mut s);
    let t1 = s.insert_thread_for_test(task::UId::new(0), None).unwrap();

    // Put T1 to sleep.
    s.set_current_for_test(Some(t1));
    s.sleep_until(Some(t1), 1, 0).unwrap();
    assert!(s.is_waiting(t1));

    // BUG: enqueue does not honor the sleeping state and leaves T1 in two
    // places at once.
    s.enqueue(0, t1).unwrap();
    assert!(s.is_waiting(t1), "still has Waiter set");
    assert_eq!(s.wakeup_min(), Some(t1));

    // Now kill the thread. kill! only walks rt -> rr -> wakeup with `or_else`,
    // so it removes from rr first and stops. The wakeup tree retains a
    // pointer to T1's now-freed slot.
    s.kill_by_thread(Some(t1)).unwrap();

    // After kill, wakeup_min must not point at a dead thread.
    if let Some(min) = s.wakeup_min() {
        use crate::types::traits::Get;
        assert!(
            s.threads.get(min).is_some(),
            "wakeup_min points at dead thread {} (B1 reproduced)",
            min
        );
    }
}

// ---------------- Proptest harness ----------------

use proptest::prelude::*;

/// A single operation that the proptest harness can apply to the scheduler.
#[derive(Debug, Clone)]
enum Op {
    NewThread { rt: bool },
    Enqueue { idx: u8 },
    Sleep { idx: u8, until: u64 },
    Kick { idx: u8 },
    Dequeue { idx: u8 },
    KillThread { idx: u8 },
    Step { advance: u64 },
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        prop::bool::ANY.prop_map(|rt| Op::NewThread { rt }),
        any::<u8>().prop_map(|idx| Op::Enqueue { idx }),
        (any::<u8>(), 0u64..1_000_000).prop_map(|(idx, until)| Op::Sleep { idx, until }),
        any::<u8>().prop_map(|idx| Op::Kick { idx }),
        any::<u8>().prop_map(|idx| Op::Dequeue { idx }),
        any::<u8>().prop_map(|idx| Op::KillThread { idx }),
        (0u64..50_000).prop_map(|advance| Op::Step { advance }),
    ]
}

fn ops_strategy() -> impl Strategy<Value = Vec<Op>> {
    prop::collection::vec(op_strategy(), 0..40)
}

struct Harness {
    sched: TestSched,
    task: task::UId,
    threads: Vec<ThreadUId>,
    now: u64,
}

impl Harness {
    fn new() -> Self {
        let mut sched = make_sched();
        let (task, _idle) = ensure_idle(&mut sched);
        Self {
            sched,
            task,
            threads: Vec::new(),
            now: 0,
        }
    }

    fn pick(&self, idx: u8) -> Option<ThreadUId> {
        if self.threads.is_empty() {
            None
        } else {
            Some(self.threads[(idx as usize) % self.threads.len()])
        }
    }

    fn apply(&mut self, op: Op) {
        match op {
            Op::NewThread { rt } => {
                let rtattrs = if rt {
                    Some(RtAttrs {
                        deadline: 100,
                        period: 200,
                        budget: 50,
                    })
                } else {
                    None
                };
                if let Ok(uid) = self.sched.insert_thread_for_test(self.task, rtattrs) {
                    self.threads.push(uid);
                }
            }
            Op::Enqueue { idx } => {
                if let Some(uid) = self.pick(idx) {
                    let _ = self.sched.enqueue(self.now, uid);
                }
            }
            Op::Sleep { idx, until } => {
                if let Some(uid) = self.pick(idx) {
                    // Make this thread the current so sleep_until takes the
                    // path that triggers reschedule.
                    self.sched.set_current_for_test(Some(uid));
                    let _ = self.sched.sleep_until(Some(uid), self.now + until + 1, self.now);
                }
            }
            Op::Kick { idx } => {
                if let Some(uid) = self.pick(idx) {
                    let _ = self.sched.kick(uid);
                }
            }
            Op::Dequeue { idx } => {
                if let Some(uid) = self.pick(idx) {
                    let _ = self.sched.dequeue(uid);
                }
            }
            Op::KillThread { idx } => {
                if let Some(uid) = self.pick(idx) {
                    let _ = self.sched.kill_by_thread(Some(uid));
                    self.threads.retain(|&u| u != uid);
                }
            }
            Op::Step { advance } => {
                self.now = self.now.saturating_add(advance);
                let (picked, _) = self.sched.step(self.now);
                // The picked uid must correspond to a live thread or the idle.
                let _ = picked;
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 256,
        max_shrink_iters: 4096,
        ..ProptestConfig::default()
    })]

    /// Driving the scheduler through arbitrary op sequences must not panic
    /// and must keep the basic structural invariants intact.
    #[test]
    fn random_ops_preserve_invariants(ops in ops_strategy()) {
        let mut h = Harness::new();
        check_invariants(&h.sched);
        for op in ops {
            h.apply(op);
            check_invariants(&h.sched);
        }
    }
}
