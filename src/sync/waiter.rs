//! Single-consumer park/wake primitive.
//!
//! [`ParkedWaiter`] holds at most one parked uid in an `AtomicU32`.
//! Producers call [`wake`](ParkedWaiter::wake) from IRQ context — it is
//! lock-free. Consumers either:
//!
//! - drive their own park loop with the low-level [`arm`] / [`disarm`]
//!   pair, or
//! - call [`park`] / [`park_current`], which couples arming and the
//!   scheduler-side park so a wake that fires in between cannot be lost.
//!
//! Either entry rejects a concurrent second consumer with `EBUSY`
//! instead of silently overwriting the first uid (which would strand
//! that thread).
//!
//! [`arm`]: ParkedWaiter::arm
//! [`disarm`]: ParkedWaiter::disarm
//! [`park`]: ParkedWaiter::park
//! [`park_current`]: ParkedWaiter::park_current

use core::sync::atomic::{AtomicU32, Ordering};

use crate::error::Result;

/// uid 0 is the idle thread, which cannot park; safe sentinel.
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

    pub fn arm(&self, uid: u32) -> Result<()> {
        if uid == UNARMED {
            return Err(kerr!(EINVAL, "ParkedWaiter::arm requires non-zero uid"));
        }
        self.uid
            .compare_exchange(UNARMED, uid, Ordering::Release, Ordering::Relaxed)
            .map(|_| ())
            .map_err(|_| kerr!(EBUSY, "ParkedWaiter already armed"))
    }

    pub fn disarm(&self) {
        self.uid.store(UNARMED, Ordering::Release);
    }

    /// Look up the current thread and park it. Returns `EINVAL` if
    /// invoked outside of a thread context (e.g. from the idle thread or
    /// pre-`sched::enable`), `EBUSY` if another consumer is already
    /// parked here.
    pub fn park_current(&self) -> Result<()> {
        let uid = crate::sched::with(|s| s.current_uid())
            .ok_or_else(|| kerr!(EINVAL, "park_current with no current thread"))?
            as u32;
        if uid == UNARMED {
            return Err(kerr!(EINVAL, "idle thread cannot park"));
        }
        self.park(uid)
    }

    /// Atomically (vs IRQs) arm `uid` and park it on the scheduler's
    /// wakeup tree. The intervening edge — wake fires after `arm` returns
    /// but before `sched::with` runs — would otherwise kick a uid that
    /// the scheduler does not yet see as sleeping, dropping the wakeup.
    pub fn park(&self, uid: u32) -> Result<()> {
        crate::sync::atomic::irq_free(|| -> Result<()> {
            self.arm(uid)?;
            crate::sched::with(|s| {
                if s.sleep_until(u64::MAX, crate::time::tick()).is_err() {
                    bug!("park with no current thread despite armed uid");
                }
            });
            Ok(())
        })?;
        // The scheduler switches us out once IRQs re-enable; control
        // resumes here after `wake` has run.
        self.disarm();
        Ok(())
    }

    /// Wake the parked thread, if any. IRQ-safe and lock-free.
    pub fn wake(&self) {
        let uid = self.uid.load(Ordering::Acquire);
        if uid != UNARMED {
            crate::sched::kick_thread(uid);
        }
    }
}
