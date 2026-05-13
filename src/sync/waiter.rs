//! Single-consumer parked-waiter primitive. Stores a thread uid in an
//! `AtomicU32` so `wake()` is callable from IRQ context without a lock.
//! A second `arm()` overwrites the first — these drivers expose a
//! single read endpoint per device, so the newest waiter wins.

use core::sync::atomic::{AtomicU32, Ordering};

pub struct ParkedWaiter {
    uid: AtomicU32,
}

impl ParkedWaiter {
    pub const fn new() -> Self {
        Self {
            uid: AtomicU32::new(0),
        }
    }

    /// Park `uid` as the waiter. A second call overwrites the first.
    pub fn arm(&self, uid: u32) {
        self.uid.store(uid, Ordering::Release);
    }

    /// Clear the parked waiter, if any. Idempotent.
    pub fn disarm(&self) {
        self.uid.store(0, Ordering::Release);
    }

    /// Wake the parked thread, if any. Safe from IRQ context.
    pub fn wake(&self) {
        let uid = self.uid.load(Ordering::Acquire);
        if uid != 0 {
            crate::sched::kick_thread(uid);
        }
    }
}
