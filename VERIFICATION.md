# Scheduler verification

This document summarizes a verification pass over the Osiris scheduler:
property-based testing with proptest, bounded model checking with Kani, and
concurrency model checking with Loom.

## Bugs found and fixed

### B1 — `kill` removed from only one of {RT, RR, wakeup} (data corruption)

`kill!` short-circuited with `or_else` across RT / RR / wakeup tree. If a
thread ended up in more than one structure (most easily reachable via
`sleep_until` followed by `enqueue`), only the first hit was removed and the
others retained pointers into the freed slot. After the slot was reused by a
later insert, those stale pointers silently referenced a different thread.

- Reproducer: `regression_b1_enqueue_sleeping_thread_then_kill_leaves_wakeup_dangling`
- Discovery: proptest, minimal failing case 4 ops long
- Fix: `kill!` now removes from all three structures unconditionally and
  clears the waiter; the slot can be safely freed afterwards.

### B2 — RT throttle no-op at exact deadline (real-time correctness)

When an RT thread's budget runs out at its deadline, `sync_to_sched` calls
`sleep_until(t, deadline, now)` with `until == now`. The old
`if until <= now { return Ok(()); }` early-return made this a silent no-op,
so the thread stayed in the EDF tree with `budget_left = 0` and `select_next`
re-picked it with a zero-tick budget on the next pass.

- Reproducer: `regression_b2_rt_throttle_at_exact_deadline_returns_zero_budget`
- Discovery: proptest, surfaced via the `step_always_returns_live_thread`
  property requiring `budget > 0`
- Fix: `sleep_until` parks the thread in the wakeup tree even when
  `until <= now`; `do_wakeups` then resumes it on the same pass and
  `enqueue`'s replenish branch refills the budget.

### B3 — `kill_by_thread(Some(uid))` fails when no current thread

The body started with
`let uid = uid.unwrap_or(self.current.ok_or(kerr!(EINVAL))?);`. Because
`Option::unwrap_or` eagerly evaluates its argument, the `?` fires whenever
`self.current` is `None` — even when the caller already supplied a UID. The
practical effect: a thread cannot be killed by uid during early boot or
right after a `kill_by_task` that cleared `current`.

- Reproducer: `regression_b3_kill_by_thread_fails_when_no_current_even_with_explicit_uid`
- Discovery: proptest harness failure on a 0-op-prefix case
- Fix: lazy `match` so `self.current` is only consulted when `uid` is `None`.

## What was verified

### Proptest (`src/sched/tests.rs`)

Drives the scheduler through arbitrary op sequences using test-only
constructors that bypass the memory subsystem (`insert_*_for_test`). The
generator emits NewThread / NewTask / Enqueue / Sleep / Kick / KickByUid /
Dequeue / KillThread / KillTask / Step.

Properties asserted:

- `random_ops_preserve_invariants` — structural invariants after every op.
- `step_always_returns_live_thread` — `step(now)` never returns a dead UID
  or zero budget.
- `sleeping_thread_not_picked_before_deadline` — temporal correctness.
- `enqueued_rr_thread_runs_eventually` — non-RT progress.
- `round_robin_visits_all_threads` — RR fairness across N runnable threads.
- `edf_picks_earliest_deadline` — EDF correctness.
- `kick_wakes_a_sleeper` — kick semantics.
- `kill_by_thread_immediately_removes_thread` — kill liveness.
- `killed_task_drops_all_its_threads` — task-level kill.

Configuration: default 1024 cases per harness; `PROPTEST_CASES` env var
overrides. `just proptest` runs with 32 768 cases in release. The harness is
part of `just test` so default CI exercises it.

Failing seeds are checked in under `proptest-regressions/sched/tests.txt`
and replayed automatically.

### Kani (`#[cfg(kani)]` modules, run via `just verify`)

- `src/sched/thread.rs::verification` — four proofs covering `RtServer`:
  `consume`, `replenish`, `on_wakeup`, `violates_sched`. Each verifies
  totality (no panics on any input) and the algebraic spec. Verification
  time per harness: ~0.3s.
- `src/types/list.rs::verification` — three proofs for `List` (push/remove
  round-trips, two-push head/tail, idempotent re-push) at N=2, `unwind(4)`.

Not attempted in Kani:

- The full scheduler `do_sched` loop — Kani would have to model the
  intrusive RB-tree, the bit reclaim map, AND the global statics. The state
  space is too large at any meaningful bound, and proptest already gives
  exhaustive small-bound coverage for these scenarios.
- The RB-tree rebalance proofs — the unit-test suite already exercises
  many random insertion / removal sequences. Adding Kani harnesses here
  would have high cost (large state, deep loops) and low marginal value.

### Loom (`tests/loom_isr_mainline.rs`, run via `just loom`)

Approach: option 2 from the verification brief — a test-only mirror module.
The production code uses `core::sync::atomic` directly. Per the explicit
constraint that the no_std build must not break, we did NOT swap to
`loom::sync::atomic` via cfg in production code.

Instead, `tests/loom_isr_mainline.rs` is gated behind `cfg(loom)` and
re-implements just the primitives the scheduler relies on (`SpinLocked<T>`,
an `irq_free` shim that masks an atomic bool) using loom atomics. The
interaction shape — mainline holding the spinlock under irq_free; an
ISR-callable function re-entering — is then driven explicitly.

What this catches: spinlock CAS bugs, atomic ordering issues, deadlock
from ISR re-entry. What this does NOT catch: scheduler algorithmic bugs.
Those are covered by proptest and Kani.

Three models pass:

- `no_data_race_when_isr_is_masked` — ISR refuses to fire while masked,
  the critical section is race-free.
- `spinlock_protects_critical_section` — mutual exclusion under two
  concurrent writers.
- `next_tick_monotonic_visibility` — Release/Acquire visibility on the
  shared `NEXT_TICK` analogue.

## Known limitations

- Scheduler tests bypass `create_thread`/`create_task`'s memory allocation
  via `insert_*_for_test` helpers. The dispatch / context-switch path is
  not host-testable (it requires real Cortex-M stacks).
- The Loom model verifies the locking primitives in isolation; bugs that
  require the full scheduler state under interleaving (e.g. a hypothetical
  RB-tree race) would not be caught.
- Kani proofs intentionally do not cover the full `do_sched` loop; see the
  reasoning above.

## How to run

```
just test       # includes proptest at 1024 cases per harness
just proptest   # release-mode 32k cases per harness
just verify     # all Kani proofs (existing recipe)
just loom       # cfg-gated Loom model
```
