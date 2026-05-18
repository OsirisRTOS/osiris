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

use crate::error::Result;
use crate::sched::thread::{self, Id};
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

// 0 is the idle thread's uid, which is never allowed to park.
const UNARMED: usize = 0;

pub struct ParkedWaiter {
    uid: AtomicUsize,
}

impl ParkedWaiter {
    pub const fn new() -> Self {
        Self {
            uid: AtomicUsize::new(UNARMED),
        }
    }

    pub fn arm(&self, uid: usize) -> Result<()> {
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
            .ok_or_else(|| kerr!(EINVAL, "park_current with no current thread"))?;
        if uid == UNARMED {
            return Err(kerr!(EINVAL, "idle thread cannot park"));
        }
        self.park(uid)
    }

    /// Park `uid`. Prefer [`park_current`](Self::park_current) unless
    /// you already have the uid in hand.
    pub fn park(&self, uid: usize) -> Result<()> {
        // IRQs masked across arm + scheduler park so a wake firing
        // in between can't kick a uid the scheduler hasn't yet
        // recorded as sleeping.
        crate::sync::atomic::irq_free(|| -> Result<()> {
            self.arm(uid)?;
            let tid = thread::UId::new(uid, thread::Id::new(0, crate::sched::task::UId::new(0)));
            crate::sched::with(|s| {
                if s.sleep_until(Some(tid), u64::MAX, crate::time::tick())
                    .is_err()
                {
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
            crate::sched::kick_thread(uid as u32);
        }
    }

    /// Userspace wait: re-`probe`s, parking via the pending-wake
    /// syscall between tries, until `Some` or `timeout_ticks` elapse.
    /// `u64::MAX` waits forever. Single consumer: returns `probe()`
    /// immediately if another waiter holds the slot (`EBUSY`).
    /// Userspace-only; kernel code uses [`park`](Self::park).
    pub fn wait_while<T>(
        &self,
        uid: usize,
        timeout_ticks: u64,
        mut probe: impl FnMut() -> Option<T>,
    ) -> Option<T> {
        if let Some(v) = probe() {
            return Some(v);
        }
        if self.arm(uid).is_err() {
            return probe();
        }
        let forever = timeout_ticks == u64::MAX;
        let deadline = crate::uapi::time::tick().saturating_add(timeout_ticks);
        let result = loop {
            if let Some(v) = probe() {
                break Some(v);
            }
            let now = crate::uapi::time::tick();
            if !forever && now >= deadline {
                break None;
            }
            let remaining = if forever { u64::MAX } else { deadline - now };
            let _ = crate::uapi::sched::park_pending(remaining);
        };
        self.disarm();
        result
    }
}

#[cfg(test)]
impl ParkedWaiter {
    pub(crate) fn armed_uid(&self) -> usize {
        self.uid.load(Ordering::Acquire)
    }
}

// Scheduler-free contract tests. The arm↔kick / park_pending
// interleaving is verified on QEMU/HW.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arm_is_single_consumer_without_overwrite() {
        let w = ParkedWaiter::new();
        assert!(w.arm(1).is_ok());
        assert!(w.arm(2).is_err());
        assert_eq!(w.armed_uid(), 1);
        w.disarm();
        assert!(w.arm(2).is_ok());
    }

    #[test]
    fn arm_rejects_idle_uid() {
        let w = ParkedWaiter::new();
        assert!(matches!(
            w.arm(UNARMED).unwrap_err().kind,
            crate::error::PosixError::EINVAL
        ));
    }

    #[test]
    fn wake_unarmed_is_noop() {
        ParkedWaiter::new().wake();
    }

    #[test]
    fn wait_while_ready_returns_without_arming() {
        let w = ParkedWaiter::new();
        assert_eq!(w.wait_while(5, 0, || Some(42)), Some(42));
        assert_eq!(w.armed_uid(), UNARMED);
    }

    #[test]
    fn wait_while_degrades_when_slot_busy() {
        let w = ParkedWaiter::new();
        w.arm(1).unwrap();
        assert_eq!(w.wait_while(2, 0, || -> Option<()> { None }), None);
        // First consumer's uid is left intact.
        assert_eq!(w.armed_uid(), 1);
    }
}
