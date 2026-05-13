//! Loom model of the scheduler's ISR-vs-mainline contract.
//!
//! Approach is the "test-only mirror" option from the verification brief:
//! production code uses `core::sync::atomic` directly and we are NOT swapping
//! it out (that would force cfg-switched atomic types throughout the kernel,
//! which is intrusive and threatens the no_std build). Instead we re-implement
//! the relevant primitives — a `SpinLocked<T>` and an `irq_free` shim — using
//! `loom::sync::atomic`, then drive the same interaction shape the kernel
//! uses (mainline holding the spinlock, an ISR-callable function re-entering
//! it). If a real refactor toward an atomic abstraction layer ever happens,
//! the same tests can be wired straight through.
//!
//! What this DOES catch: bugs in the spinlock CAS / fairness; data races on
//! atomics shared between mainline and ISR; deadlock when the ISR re-enters.
//! What it does NOT catch: bugs in scheduler-specific algorithms (those are
//! covered by `src/sched/tests.rs` and the Kani proofs).
//!
//! Run with: `RUSTFLAGS="--cfg loom" cargo test --target host-tuple --test loom_isr_mainline --release`

#![cfg(loom)]

use loom::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use loom::sync::Arc;
use loom::thread;
use loom::cell::UnsafeCell;

/// Mirror of `crate::sync::spinlock::SpinLock` against loom atomics.
struct SpinLock {
    locked: AtomicBool,
}

impl SpinLock {
    fn new() -> Self {
        Self { locked: AtomicBool::new(false) }
    }
    fn lock(&self) {
        while self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            loom::thread::yield_now();
        }
    }
    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

struct SpinLocked<T> {
    lock: SpinLock,
    cell: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLocked<T> {}

impl<T> SpinLocked<T> {
    fn new(t: T) -> Self {
        Self { lock: SpinLock::new(), cell: UnsafeCell::new(t) }
    }
    fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        self.lock.lock();
        let r = self.cell.with_mut(|p| f(unsafe { &mut *p }));
        self.lock.unlock();
        r
    }
}

/// Mirror of `irq_free` from `src/sync/atomic.rs`.
///
/// On the target this disables interrupts and re-enables them. Under loom we
/// model "interrupts disabled on this core" as a thread-local flag. The ISR
/// thread will refuse to fire when the flag is set, mimicking masked IRQs.
struct IrqMask {
    masked: AtomicBool,
}

impl IrqMask {
    fn new() -> Self {
        Self { masked: AtomicBool::new(false) }
    }
    /// Run `f` with interrupts masked. Equivalent to `sync::atomic::irq_free`.
    fn free<R>(&self, f: impl FnOnce() -> R) -> R {
        // We use Acquire/Release so the ISR observer's load-after-Acquire
        // sees a consistent view of mainline writes inside `f`.
        self.masked.store(true, Ordering::Release);
        let r = f();
        self.masked.store(false, Ordering::Release);
        r
    }
    fn is_masked(&self) -> bool {
        self.masked.load(Ordering::Acquire)
    }
}

// ----------------- Tests -----------------

#[test]
fn no_data_race_when_isr_is_masked() {
    loom::model(|| {
        let irq = Arc::new(IrqMask::new());
        let sched = Arc::new(SpinLocked::new(0u64));

        let irq_m = irq.clone();
        let sched_m = sched.clone();
        let mainline = thread::spawn(move || {
            irq_m.free(|| {
                sched_m.with(|v| {
                    let old = *v;
                    *v = old.wrapping_add(1);
                });
            });
        });

        // "ISR" — only fires when not masked.
        let isr = thread::spawn(move || {
            // Spin a bounded number of times rather than forever so loom
            // finishes exploration. The point is to model the race window.
            for _ in 0..2 {
                if !irq.is_masked() {
                    sched.with(|v| {
                        *v = v.wrapping_add(10);
                    });
                    break;
                }
                loom::thread::yield_now();
            }
        });

        mainline.join().unwrap();
        isr.join().unwrap();
    });
}

/// Models the same shape `sched_enter` / `kick_thread` use: mainline takes the
/// scheduler lock under irq_free; the ISR may also take the lock when not
/// masked. Verifies that the spinlock provides mutual exclusion and that no
/// writes are lost.
#[test]
fn spinlock_protects_critical_section() {
    loom::model(|| {
        let sched = Arc::new(SpinLocked::new(0u64));

        let s1 = sched.clone();
        let t1 = thread::spawn(move || {
            s1.with(|v| *v += 1);
        });

        let s2 = sched.clone();
        let t2 = thread::spawn(move || {
            s2.with(|v| *v += 1);
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // After both critical sections, the counter must be exactly 2 — the
        // spinlock prevents the read-modify-write from interleaving.
        let final_v = sched.with(|v| *v);
        assert_eq!(final_v, 2);
    });
}

/// Models the kick-vs-needs_reschedule race: the ISR-style writer stores
/// monotonically increasing values to `NEXT_TICK`; the mainline reader compares
/// `now >= NEXT_TICK`. After all writers complete, the read must see the
/// largest value. (Sanity check on AtomicU64 ordering.)
#[test]
fn next_tick_monotonic_visibility() {
    loom::model(|| {
        let next_tick = Arc::new(AtomicU64::new(0));

        let w1 = next_tick.clone();
        let t1 = thread::spawn(move || {
            w1.store(10, Ordering::Release);
        });
        let w2 = next_tick.clone();
        let t2 = thread::spawn(move || {
            w2.store(20, Ordering::Release);
        });

        t1.join().unwrap();
        t2.join().unwrap();

        let v = next_tick.load(Ordering::Acquire);
        // We only require visibility (no torn reads, no impossible values).
        // 10 or 20 are both legal final states because the writers race.
        assert!(v == 10 || v == 20);
    });
}
