//! Host-side property tests for the scheduler. Tests use the
//! `insert_*_for_test` constructors which bypass the memory subsystem.

#![allow(clippy::needless_range_loop)]

use super::*;
use crate::sched::thread::UId as ThreadUId;
use crate::uapi::sched::RtAttrs;

const TEST_N: usize = 8;
type TestSched = Scheduler<TEST_N>;

fn make_sched() -> TestSched {
    Scheduler::new()
}

// `select_next` falls back to `IDLE_THREAD` (uid 0) when nothing else is
// runnable; without uid 0 in the thread map `do_sched` would panic. We do not
// enqueue idle — that would steal RR quanta from real threads.
fn ensure_idle(sched: &mut TestSched) -> (task::UId, ThreadUId) {
    let task = sched.insert_task_for_test().expect("task slot");
    let idle = sched.insert_thread_for_test(task, None).expect("thread slot");
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

#[track_caller]
fn check_step_consistency(s: &mut TestSched, now: u64) {
    use crate::types::traits::Get;
    let (picked, budget) = s.step(now);
    assert!(
        s.threads.get(picked).is_some(),
        "step({}) picked dead thread {}",
        now,
        picked
    );
    // RT pick returns a thread's budget_left; with budget=0 the thread should
    // be throttled, not picked. RR and idle fallback always return >0.
    assert!(budget > 0, "step({}) returned zero budget for {}", now, picked);
}

// B3: kill_by_thread used `unwrap_or(self.current.ok_or(EINVAL)?)`, which is
// eagerly evaluated. With `current = None`, the `?` fires unconditionally, so
// killing by an explicit uid failed during early boot or right after a
// kill_by_task that cleared `current`.
#[test]
fn regression_b3_kill_by_thread_fails_when_no_current_even_with_explicit_uid() {
    let mut s = make_sched();
    let (task_uid, _idle) = ensure_idle(&mut s);
    let victim = s.insert_thread_for_test(task_uid, None).unwrap();
    s.enqueue(0, victim).unwrap();
    assert!(s.current().is_none());
    let res = s.kill_by_thread(Some(victim));
    assert!(
        res.is_ok(),
        "B3 reproduced: kill_by_thread(Some({})) returned {:?} with current=None",
        victim,
        res
    );
}

// B1: enqueue does not remove a sleeping thread from the wakeup tree, so after
// `sleep_until` + `enqueue` the thread lives in both rr/edf AND wakeup. The
// old kill! macro used `or_else` to remove from rt -> rr -> wakeup, so it only
// removed from the first hit and left a dangling pointer into the freed slot.
#[test]
fn regression_b1_enqueue_sleeping_thread_then_kill_leaves_wakeup_dangling() {
    let mut s = make_sched();
    let (_task, _idle) = ensure_idle(&mut s);
    let t1 = s.insert_thread_for_test(task::UId::new(0), None).unwrap();

    s.set_current_for_test(Some(t1));
    s.sleep_until(Some(t1), 1, 0).unwrap();
    assert!(s.is_waiting(t1));

    s.enqueue(0, t1).unwrap();
    assert!(s.is_waiting(t1));
    assert_eq!(s.wakeup_min(), Some(t1));

    s.kill_by_thread(Some(t1)).unwrap();

    if let Some(min) = s.wakeup_min() {
        use crate::types::traits::Get;
        assert!(
            s.threads.get(min).is_some(),
            "wakeup_min points at dead thread {} (B1 reproduced)",
            min
        );
    }
}

// B2: with budget exhausted exactly at deadline, the throttle path called
// sleep_until(t, deadline, deadline). The old `if until <= now { return Ok }`
// made this a no-op, leaving the thread in EDF with budget_left = 0; select_next
// then re-picked it with zero budget.
#[test]
fn regression_b2_rt_throttle_at_exact_deadline_returns_zero_budget() {
    let mut s = make_sched();
    let (_task, _idle) = ensure_idle(&mut s);
    let rt_attrs = RtAttrs { deadline: 100, period: 200, budget: 50 };
    let t_rt = s
        .insert_thread_for_test(task::UId::new(0), Some(rt_attrs))
        .unwrap();
    s.enqueue(0, t_rt).unwrap();
    s.set_current_for_test(Some(t_rt));

    let (picked, budget) = s.step(100);
    assert_eq!(picked, t_rt);
    assert!(budget > 0, "B2 reproduced: RT thread picked with budget_left = 0");
}

// ---------------- Proptest harness ----------------

use proptest::prelude::*;

#[derive(Debug, Clone)]
enum Op {
    NewThread { rt: Option<(u32, u32, u32)> }, // (budget, period, relative_deadline)
    NewTask,
    Enqueue { idx: u8 },
    Sleep { idx: u8, until: u64 },
    Kick { idx: u8 },
    KickByUid { uid: u8 },
    Dequeue { idx: u8 },
    KillThread { idx: u8 },
    KillTask { task_idx: u8 },
    Step { advance: u64 },
}

fn op_strategy() -> impl Strategy<Value = Op> {
    // Step is weighted higher so we exercise the put/wakeup paths often.
    prop_oneof![
        1 => Just(Op::NewThread { rt: None }),
        1 => (1u32..200, 1u32..400, 1u32..200)
                .prop_map(|(b, p, _rd)| {
                    // budget <= period (basic RT sanity)
                    let p = p.max(b);
                    Op::NewThread { rt: Some((b, p, b.max(1))) }
                }),
        1 => Just(Op::NewTask),
        3 => any::<u8>().prop_map(|idx| Op::Enqueue { idx }),
        3 => (any::<u8>(), 0u64..1_000_000).prop_map(|(idx, until)| Op::Sleep { idx, until }),
        2 => any::<u8>().prop_map(|idx| Op::Kick { idx }),
        1 => any::<u8>().prop_map(|uid| Op::KickByUid { uid }),
        2 => any::<u8>().prop_map(|idx| Op::Dequeue { idx }),
        1 => any::<u8>().prop_map(|idx| Op::KillThread { idx }),
        1 => any::<u8>().prop_map(|task_idx| Op::KillTask { task_idx }),
        4 => (0u64..50_000).prop_map(|advance| Op::Step { advance }),
    ]
}

fn ops_strategy() -> impl Strategy<Value = Vec<Op>> {
    prop::collection::vec(op_strategy(), 0..80)
}

struct Harness {
    sched: TestSched,
    task: task::UId,
    tasks: Vec<task::UId>,
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
            tasks: vec![task],
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

    fn pick_task(&self, idx: u8) -> Option<task::UId> {
        if self.tasks.is_empty() {
            None
        } else {
            Some(self.tasks[(idx as usize) % self.tasks.len()])
        }
    }

    fn apply(&mut self, op: Op) {
        match op {
            Op::NewThread { rt } => {
                let rtattrs = rt.map(|(b, p, d)| RtAttrs {
                    deadline: d as u64,
                    period: p,
                    budget: b,
                });
                let task = self.tasks.last().copied().unwrap_or(self.task);
                if let Ok(uid) = self.sched.insert_thread_for_test(task, rtattrs) {
                    self.threads.push(uid);
                }
            }
            Op::NewTask => {
                if let Ok(t) = self.sched.insert_task_for_test() {
                    self.tasks.push(t);
                }
            }
            Op::Enqueue { idx } => {
                if let Some(uid) = self.pick(idx) {
                    let _ = self.sched.enqueue(self.now, uid);
                }
            }
            Op::Sleep { idx, until } => {
                if let Some(uid) = self.pick(idx) {
                    // Setting current makes the reschedule branch of sleep_until reachable.
                    self.sched.set_current_for_test(Some(uid));
                    let _ = self.sched.sleep_until(
                        Some(uid),
                        self.now.saturating_add(until).saturating_add(1),
                        self.now,
                    );
                }
            }
            Op::Kick { idx } => {
                if let Some(uid) = self.pick(idx) {
                    let _ = self.sched.kick(uid);
                }
            }
            Op::KickByUid { uid } => {
                let _ = self.sched.kick_by_uid(uid as usize);
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
            Op::KillTask { task_idx } => {
                if let Some(tid) = self.pick_task(task_idx) {
                    // The kernel task owns the idle thread; killing it removes
                    // the IDLE_THREAD fallback target and trips select_next.
                    if tid != self.task {
                        let _ = self.sched.kill_by_task(tid);
                        self.tasks.retain(|&t| t != tid);
                        let live = self.sched.live_threads();
                        self.threads.retain(|u| live.contains(u));
                    }
                }
            }
            Op::Step { advance } => {
                self.now = self.now.saturating_add(advance);
                let (picked, _) = self.sched.step(self.now);
                let _ = picked;
            }
        }
    }
}

proptest! {
    // PROPTEST_CASES env var overrides cases at runtime. 1024 keeps `just test`
    // fast; `just proptest` bumps to ~32k for overnight stress.
    #![proptest_config(ProptestConfig {
        cases: 1024,
        max_shrink_iters: 8192,
        ..ProptestConfig::default()
    })]

    #[test]
    fn random_ops_preserve_invariants(ops in ops_strategy()) {
        let mut h = Harness::new();
        check_invariants(&h.sched);
        for op in ops {
            h.apply(op);
            check_invariants(&h.sched);
        }
    }

    #[test]
    fn step_always_returns_live_thread(ops in ops_strategy()) {
        let mut h = Harness::new();
        for op in ops {
            h.apply(op);
        }
        check_step_consistency(&mut h.sched, h.now);
    }

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

    /// RR fairness: every runnable thread is picked within n+1 quanta.
    #[test]
    fn round_robin_visits_all_threads(
        n in 2usize..6,
    ) {
        let mut h = Harness::new();
        let mut tids = Vec::new();
        for _ in 0..n {
            let t = h.sched.insert_thread_for_test(h.task, None).unwrap();
            h.sched.enqueue(0, t).unwrap();
            tids.push(t);
        }
        let mut seen: std::collections::HashSet<u64> =
            std::collections::HashSet::new();
        // Quantum is 1000 ticks; we step in increments of 100 -> ~10 steps per
        // quantum and need n+1 quanta to cover every thread.
        for i in 0..((n as u64 + 1) * 1010 / 100) {
            let (picked, _) = h.sched.step(i * 100);
            seen.insert(picked.as_usize() as u64);
        }
        for t in &tids {
            prop_assert!(
                seen.contains(&(t.as_usize() as u64)),
                "thread {} never picked in {} steps; seen={:?}", t, n + 1, seen
            );
        }
    }

    #[test]
    fn edf_picks_earliest_deadline(
        d1 in 50u64..200,
        d2 in 50u64..200,
        d3 in 50u64..200,
    ) {
        let mut h = Harness::new();
        let mk = |d: u64| RtAttrs {
            deadline: d,
            period: (d as u32) * 2,
            budget: (d / 2) as u32,
        };
        let t1 = h.sched.insert_thread_for_test(h.task, Some(mk(d1))).unwrap();
        let t2 = h.sched.insert_thread_for_test(h.task, Some(mk(d2))).unwrap();
        let t3 = h.sched.insert_thread_for_test(h.task, Some(mk(d3))).unwrap();
        h.sched.enqueue(0, t1).unwrap();
        h.sched.enqueue(0, t2).unwrap();
        h.sched.enqueue(0, t3).unwrap();
        let (picked, _) = h.sched.step(0);
        let pairs = [(t1, d1), (t2, d2), (t3, d3)];
        let min_pair = pairs.iter().min_by_key(|(_, d)| *d).unwrap();
        // Ties broken by UID; the picked thread must have the min deadline.
        let picked_d = pairs.iter().find(|(t, _)| *t == picked).map(|(_, d)| *d);
        prop_assert_eq!(
            picked_d,
            Some(min_pair.1),
            "EDF picked {} (deadline {:?}); expected min deadline {}",
            picked, picked_d, min_pair.1
        );
    }

    #[test]
    fn kick_wakes_a_sleeper(
        until in 100u64..10_000,
    ) {
        let mut h = Harness::new();
        let t1 = h.sched.insert_thread_for_test(h.task, None).unwrap();
        h.sched.enqueue(0, t1).unwrap();
        h.sched.set_current_for_test(Some(t1));
        h.sched.sleep_until(Some(t1), until, 0).unwrap();
        prop_assert!(h.sched.is_waiting(t1));
        h.sched.kick(t1).unwrap();
        prop_assert!(!h.sched.is_waiting(t1));
        let (picked, _) = h.sched.step(0);
        prop_assert_eq!(picked, t1, "kicked thread {} should be runnable", t1);
    }

    // Slot indices are reused after kill, so a later-inserted thread may share
    // the victim's `uid` field; we only check the state immediately after kill.
    #[test]
    fn kill_by_thread_immediately_removes_thread(
        ops_before in prop::collection::vec(op_strategy(), 0..20),
    ) {
        let mut h = Harness::new();
        let victim = h.sched.insert_thread_for_test(h.task, None).unwrap();
        h.sched.enqueue(0, victim).unwrap();
        for op in ops_before { h.apply(op); }
        let kill_res = h.sched.kill_by_thread(Some(victim));
        prop_assert!(kill_res.is_ok(), "kill_by_thread({}) failed: {:?}", victim, kill_res);
        let live = h.sched.live_threads();
        prop_assert!(
            !live.contains(&victim),
            "killed thread {} remained in live set", victim
        );
        if let Some(min) = h.sched.wakeup_min() {
            prop_assert_ne!(
                min, victim,
                "wakeup tree still references killed thread {}", victim
            );
        }
    }

    #[test]
    fn killed_task_drops_all_its_threads(
        n in 1usize..5,
    ) {
        let mut h = Harness::new();
        let t = h.sched.insert_task_for_test().unwrap();
        let mut tids = Vec::new();
        for _ in 0..n {
            let tid = h.sched.insert_thread_for_test(t, None).unwrap();
            h.sched.enqueue(0, tid).unwrap();
            tids.push(tid);
        }
        h.sched.kill_by_task(t).unwrap();
        let live = h.sched.live_threads();
        for tid in &tids {
            prop_assert!(
                !live.contains(tid),
                "after kill_by_task({}), thread {} still live", t, tid
            );
        }
    }
}
