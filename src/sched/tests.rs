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
    // INV-2: wakeup_min must point at a live thread that is_waiting.
    if let Some(min) = s.wakeup_min() {
        let t = s.threads.get(min).expect("wakeup_min points at dead thread");
        assert!(t.is_waiting(), "wakeup_min thread {} is not waiting", min);
    }
    // INV-3: Step shouldn't return a UID whose backing slot is dead.
    // (We can't drive `step` here without mutating, so this is a weak check.)
}

/// A stronger invariant check: drive a `step(now)` and assert it returns a
/// live thread. The harness uses this after each operation that should leave
/// the scheduler in a consistent runnable state.
#[track_caller]
fn check_step_consistency(s: &mut TestSched, now: u64) {
    use crate::types::traits::Get;
    let (picked, budget) = s.step(now);
    let thread = s.threads.get(picked);
    assert!(
        thread.is_some(),
        "step({}) picked dead thread {}",
        now,
        picked
    );
    // RT scheduler is supposed to only return positive budgets for runnable
    // threads. The RR fallback returns the remaining quantum or `quantum`.
    // Idle returns 1000 unconditionally. Either way budget should be >0.
    assert!(budget > 0, "step({}) returned zero budget for {}", now, picked);
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

/// Bug B2: when an RT thread's budget runs out exactly at its deadline,
/// `sync_to_sched` calls `sleep_until(t, deadline, now)` with `deadline == now`.
/// The early `if until <= now { return Ok(()); }` in sleep_until makes this a
/// silent no-op, so the throttle never happens. On the next pick the RT
/// scheduler returns the same thread with `budget_left == 0`, which is then
/// treated as a 0-tick reschedule budget downstream.
#[test]
fn regression_b2_rt_throttle_at_exact_deadline_returns_zero_budget() {
    let mut s = make_sched();
    let (_task, _idle) = ensure_idle(&mut s);
    let rt_attrs = RtAttrs {
        deadline: 100,
        period: 200,
        budget: 50,
    };
    let t_rt = s
        .insert_thread_for_test(task::UId::new(0), Some(rt_attrs))
        .unwrap();
    s.enqueue(0, t_rt).unwrap();
    s.set_current_for_test(Some(t_rt));

    // Advance to t=100 (== deadline). consume(100) zeroes the budget; the
    // throttle path tries sleep_until(t_rt, 100, 100), which currently no-ops.
    let (picked, budget) = s.step(100);
    assert_eq!(picked, t_rt, "RT thread should still be the natural pick");
    assert!(
        budget > 0,
        "B2 reproduced: RT thread picked with budget_left = 0"
    );
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

    /// After any op sequence, `step(now)` must return a live thread.
    #[test]
    fn step_always_returns_live_thread(ops in ops_strategy()) {
        let mut h = Harness::new();
        for op in ops {
            h.apply(op);
        }
        check_step_consistency(&mut h.sched, h.now);
    }

    /// A thread that has been sleeping until T must not be picked at any
    /// step whose clock is strictly less than T.
    #[test]
    fn sleeping_thread_not_picked_before_deadline(
        deadline in 1u64..1000,
        wait_for in 0u64..500,
    ) {
        let mut h = Harness::new();
        let t1 = h.sched.insert_thread_for_test(h.task, None).unwrap();
        h.sched.set_current_for_test(Some(t1));
        h.sched.sleep_until(Some(t1), deadline, 0).unwrap();
        let probe_at = (deadline.saturating_sub(1)).min(wait_for);
        for now in 0..=probe_at {
            let (picked, _) = h.sched.step(now);
            prop_assert_ne!(picked, t1, "thread picked at {} before deadline {}", now, deadline);
        }
    }

    /// After enqueueing a thread without RT attrs and stepping, eventually that
    /// thread is picked (assuming no other RR thread monopolizes the queue).
    #[test]
    fn enqueued_rr_thread_runs_eventually(
        steps in 1u64..20,
    ) {
        let mut h = Harness::new();
        let t1 = h.sched.insert_thread_for_test(h.task, None).unwrap();
        h.sched.enqueue(0, t1).unwrap();
        let mut seen = false;
        for now in 0..=steps {
            let (picked, _) = h.sched.step(now * 100);
            if picked == t1 {
                seen = true;
                break;
            }
        }
        prop_assert!(seen, "T1 never picked within {} steps", steps);
    }
}
