//! Synchronization primitives.

use core::cell::UnsafeCell;
use core::ptr::NonNull;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering;

/// A mutual exclusion primitive, facilitating busy-waiting.
#[derive(Debug)]
pub struct SpinLock {
    lock: AtomicBool,
}

#[allow(dead_code)]
impl SpinLock {
    /// Creates a new SpinLock.
    pub const fn new() -> Self {
        SpinLock {
            lock: AtomicBool::new(false),
        }
    }

    /// Waits until the SpinLock can be acquired and lock it.
    pub fn lock(&self) {
        let lock = &self.lock;

        if lock.load(Ordering::Relaxed) {
            hal::asm::nop!();
        }

        loop {
            if lock
                .compare_exchange_weak(false, true, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }
    }

    /// Tries to lock the SpinLock.
    /// Returns `true` if the lock was acquired.
    pub fn try_lock(&self) -> bool {
        !self.lock.swap(true, Ordering::Acquire)
    }

    /// Unlocks the SpinLock.
    /// Returns `true` if the lock was released.
    ///
    /// # Safety
    /// Precondition: The SpinLock must be locked by the current thread.
    /// Postcondition: The SpinLock is unlocked.
    pub unsafe fn unlock(&self) {
        self.lock.store(false, Ordering::Release)
    }
}

/// A guard that releases the SpinLock when dropped.
#[derive(Debug)]
pub struct SpinLockGuard<'a, T: ?Sized> {
    lock: &'a SpinLock,
    value: NonNull<T>,
    marker: core::marker::PhantomData<&'a mut T>,
}

impl<T: ?Sized> core::ops::Deref for SpinLockGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { self.value.as_ref() }
    }
}

impl<T: ?Sized> core::ops::DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.value.as_mut() }
    }
}

impl<T: ?Sized> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            self.lock.unlock();
        }
    }
}

/// A mutual exclusion primitive that allows at most one thread to access a resource at a time.
pub struct SpinLocked<T> {
    lock: SpinLock,
    value: UnsafeCell<T>,
}

unsafe impl<T> Sync for SpinLocked<T> {}

/// Test
#[allow(dead_code)]
impl<T> SpinLocked<T> {
    /// Creates a new SpinLocked.
    pub const fn new(value: T) -> Self {
        SpinLocked {
            lock: SpinLock::new(),
            value: UnsafeCell::new(value),
        }
    }

    /// Locks the SpinLocked and returns a guard that releases the lock when dropped.
    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        self.lock.lock();
        SpinLockGuard {
            lock: &self.lock,
            value: unsafe { NonNull::new_unchecked(self.value.get()) },
            marker: core::marker::PhantomData,
        }
    }

    /// Tries to lock the SpinLocked and returns a guard that releases the lock when dropped.
    pub fn try_lock(&self) -> Option<SpinLockGuard<'_, T>> {
        if self.lock.try_lock() {
            Some(SpinLockGuard {
                lock: &self.lock,
                value: unsafe { NonNull::new_unchecked(self.value.get()) },
                marker: core::marker::PhantomData,
            })
        } else {
            None
        }
    }
}
