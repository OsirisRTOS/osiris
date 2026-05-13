//! Single-consumer park/wake primitive: at most one thread parks per
//! [`ParkedWaiter`] at a time. Producers call [`wake`] from IRQ context
//! (lock-free); consumers call [`park`] / [`park_current`], or drive
//! arming and parking themselves with [`arm`] / [`disarm`]. A second
//! concurrent consumer is rejected with `EBUSY`.
//!
//! [`arm`]: ParkedWaiter::arm
//! [`disarm`]: ParkedWaiter::disarm
//! [`park`]: ParkedWaiter::park
//! [`park_current`]: ParkedWaiter::park_current
//! [`wake`]: ParkedWaiter::wake

use core::sync::atomic::{AtomicU32, Ordering};

use crate::error::Result;

// 0 is the idle thread's uid, which is never allowed to park.
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

    /// Park the calling thread; returns `EBUSY` if another consumer is
    /// already parked here.
    pub fn park_current(&self) -> Result<()> {
        let uid = crate::sched::with(|s| s.current_uid())
            .ok_or_else(|| kerr!(EINVAL, "park_current with no current thread"))?
            as u32;
        if uid == UNARMED {
            return Err(kerr!(EINVAL, "idle thread cannot park"));
        }
        self.park(uid)
    }

    /// Park `uid`. Prefer [`park_current`](Self::park_current) unless
    /// you already have the uid in hand.
    pub fn park(&self, uid: u32) -> Result<()> {
        // IRQs masked across arm + scheduler park so a wake firing
        // in between can't kick a uid the scheduler hasn't yet
        // recorded as sleeping.
        crate::sync::atomic::irq_free(|| -> Result<()> {
            self.arm(uid)?;
            crate::sched::with(|s| {
                if s.sleep_until(u64::MAX, crate::time::tick()).is_err() {
                    bug!("park with no current thread despite armed uid");
                }
            });
            Ok(())
        })?;
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
