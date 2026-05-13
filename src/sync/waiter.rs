//! Single-consumer park/wake primitive.
//!
//! [`ParkedWaiter`] stores one thread uid in an `AtomicU32`. Producers
//! call [`wake`](ParkedWaiter::wake) from IRQ context (lock-free). The
//! consumer side has two layers:
//!
//! - [`arm`](ParkedWaiter::arm) / [`disarm`](ParkedWaiter::disarm) — low
//!   level. The caller is responsible for the actual park (e.g. via the
//!   `sleep`/`sleep_for` syscalls in CAN's loop pattern).
//! - [`park`](ParkedWaiter::park) — combined arm + scheduler park as a
//!   single operation. Keeps the "mask IRQs across arm and park" rule in
//!   one place so a wake that fires between the two cannot be lost.
//!
//! Either entry point rejects a concurrent second consumer with
//! [`PosixError::EBUSY`] rather than silently overwriting it — sharing a
//! single waiter across threads loses the first thread's wakeup.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::error::Result;

/// Sentinel meaning "no waiter parked". 0 is a fine sentinel because
/// uid 0 is the idle thread, which is not allowed to park.
const UNARMED: u32 = 0;

pub struct ParkedWaiter {
    uid: AtomicU32,
}

impl ParkedWaiter {
    pub const fn new() -> Self {
        Self {
            uid: AtomicU32::new(UNARMED),
        }
    }

    /// Park `uid` as this waiter's single consumer. Returns
    /// [`PosixError::EBUSY`] if another uid is already armed.
    pub fn arm(&self, uid: u32) -> Result<()> {
        if uid == UNARMED {
            return Err(kerr!(EINVAL, "ParkedWaiter::arm requires non-zero uid"));
        }
        self.uid
            .compare_exchange(UNARMED, uid, Ordering::Release, Ordering::Relaxed)
            .map(|_| ())
            .map_err(|_| kerr!(EBUSY, "ParkedWaiter already armed"))
    }

    /// Clear the armed uid. Idempotent.
    pub fn disarm(&self) {
        self.uid.store(UNARMED, Ordering::Release);
    }

    /// Combined arm + scheduler park. Atomically (vs. IRQs) registers the
    /// current thread and parks it on the scheduler's wakeup tree; the
    /// thread wakes when [`wake`](Self::wake) kicks it.
    ///
    /// Returns [`PosixError::EBUSY`] if another consumer is already
    /// parked, leaving that other thread's park intact.
    pub fn park(&self, uid: u32) -> Result<()> {
        // Mask IRQs across arm + sched-park: an edge that fires between
        // the two would otherwise kick a uid that's not yet in the
        // wakeup tree, and the kick would be lost.
        crate::sync::atomic::irq_free(|| {
            self.arm(uid)?;
            crate::sched::with(|s| {
                // `sleep_until` only errors when no current thread is
                // set, which cannot happen from a thread that just
                // proved its own uid via the caller's `current_uid`.
                if s.sleep_until(u64::MAX, crate::time::tick()).is_err() {
                    bug!("ParkedWaiter::park with no current thread");
                }
            });
            Ok::<(), crate::error::Error>(())
        })?;

        // PendSV fires once IRQs re-enable, switching this thread out.
        // We resume here after `wake` -> kick_thread -> PendSV.
        self.disarm();
        Ok(())
    }

    /// Wake the parked thread, if any. Safe from IRQ context, lock-free.
    /// A spurious call with nothing armed is a no-op.
    pub fn wake(&self) {
        let uid = self.uid.load(Ordering::Acquire);
        if uid != UNARMED {
            crate::sched::kick_thread(uid);
        }
    }
}
