//! Synchronization primitives.
#![allow(dead_code)]


use crate::hal;

use core::cell::UnsafeCell;
use core::ptr::NonNull;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::AtomicIsize;
use core::sync::atomic::Ordering;

/// A busy-waiting reader-writer lock.
pub struct RwSpinLock {
    lock: AtomicIsize,
}

impl RwSpinLock {
    /// Creates a new unlocked RwSpinLock.
    pub const fn new() -> Self {
        RwSpinLock {
            lock: AtomicIsize::new(0),
        }
    }

    /// Waits until a read lock can be acquired.
    pub fn read_lock(&self) {
        loop {
            let count = self.lock.load(Ordering::Acquire);
            if count >= 0 && count < isize::MAX {
                if self
                    .lock
                    .compare_exchange_weak(count, count + 1, Ordering::SeqCst, Ordering::Relaxed)
                    .is_ok()
                {
                    break;
                }
            }
        }
    }

    /// Releases one read lock.
    ///
    /// # Safety
    ///
    /// The caller must hold a read lock acquired from this RwSpinLock.
    pub unsafe fn read_unlock(&self) {
        self.lock.fetch_sub(1, Ordering::Release);
    }

    /// Waits until a write lock can be acquired.
    pub fn write_lock(&self) {
        while self
            .lock
            .compare_exchange_weak(0, -1, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {}
    }

    /// Releases the write lock.
    ///
    /// # Safety
    ///
    /// The caller must hold the write lock acquired from this RwSpinLock.
    pub unsafe fn write_unlock(&self) {
        self.lock.store(0, Ordering::Release);
    }
}

/// A mutual exclusion primitive, facilitating busy-waiting.
#[proc_macros::fmt]
pub struct SpinLock {
    lock: AtomicBool,
}

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

/// A guard that releases a read lock when dropped.
pub struct RwSpinLockReadGuard<'a, T: ?Sized> {
    lock: &'a RwSpinLock,
    value: NonNull<T>,
    marker: core::marker::PhantomData<&'a T>,
}

impl<T: ?Sized> core::ops::Deref for RwSpinLockReadGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { self.value.as_ref() }
    }
}

impl<T: ?Sized> Drop for RwSpinLockReadGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            self.lock.read_unlock();
        }
    }
}

/// A guard that releases a write lock when dropped.
pub struct RwSpinLockWriteGuard<'a, T: ?Sized> {
    lock: &'a RwSpinLock,
    value: NonNull<T>,
    marker: core::marker::PhantomData<&'a mut T>,
}

impl<T: ?Sized> core::ops::Deref for RwSpinLockWriteGuard<'_, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        unsafe { self.value.as_ref() }
    }
}

impl<T: ?Sized> core::ops::DerefMut for RwSpinLockWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.value.as_mut() }
    }
}

impl<T: ?Sized> Drop for RwSpinLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            self.lock.write_unlock();
        }
    }
}

/// A guard that releases the SpinLock when dropped.
#[proc_macros::fmt]
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

/// Protects a value with a busy-waiting reader-writer lock.
///
/// Multiple readers may access the value concurrently, while a writer has
/// exclusive access. This lock is not reentrant and does not provide writer
/// priority.
pub struct RwSpinLocked<T> {
    lock: RwSpinLock,
    value: UnsafeCell<T>,
}

// Safety: access to `value` is synchronized by `lock`. `T` must be `Sync`
// because read guards expose `&T`, and `Send` because write guards expose
// exclusive access from a shared lock reference.
unsafe impl<T: Send + Sync> Sync for RwSpinLocked<T> {}

impl<T> RwSpinLocked<T> {
    /// Creates a new RwSpinLocked.
    pub const fn new(value: T) -> Self {
        RwSpinLocked {
            lock: RwSpinLock::new(),
            value: UnsafeCell::new(value),
        }
    }

    /// Locks the value for shared read access.
    pub fn read_lock(&self) -> RwSpinLockReadGuard<'_, T> {
        self.lock.read_lock();
        RwSpinLockReadGuard {
            lock: &self.lock,
            value: unsafe { NonNull::new_unchecked(self.value.get()) },
            marker: core::marker::PhantomData,
        }
    }

    /// Locks the value for exclusive write access.
    pub fn write_lock(&self) -> RwSpinLockWriteGuard<'_, T> {
        self.lock.write_lock();
        RwSpinLockWriteGuard {
            lock: &self.lock,
            value: unsafe { NonNull::new_unchecked(self.value.get()) },
            marker: core::marker::PhantomData,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rw_spin_lock_tracks_multiple_readers() {
        let lock = RwSpinLock::new();

        lock.read_lock();
        lock.read_lock();

        assert_eq!(lock.lock.load(Ordering::Acquire), 2);

        unsafe {
            lock.read_unlock();
        }

        assert_eq!(lock.lock.load(Ordering::Acquire), 1);

        unsafe {
            lock.read_unlock();
        }

        assert_eq!(lock.lock.load(Ordering::Acquire), 0);
    }

    #[test]
    fn rw_spin_lock_tracks_writer() {
        let lock = RwSpinLock::new();

        lock.write_lock();

        assert_eq!(lock.lock.load(Ordering::Acquire), -1);

        unsafe {
            lock.write_unlock();
        }

        assert_eq!(lock.lock.load(Ordering::Acquire), 0);
    }

    #[test]
    fn rw_spin_locked_read_guards_allow_shared_access() {
        let value = RwSpinLocked::new(7usize);
        let first = value.read_lock();
        let second = value.read_lock();

        assert_eq!(*first, 7);
        assert_eq!(*second, 7);
        assert_eq!(value.lock.lock.load(Ordering::Acquire), 2);
    }

    #[test]
    fn rw_spin_locked_read_guard_releases_on_drop() {
        let value = RwSpinLocked::new(7usize);

        {
            let _guard = value.read_lock();

            assert_eq!(value.lock.lock.load(Ordering::Acquire), 1);
        }

        assert_eq!(value.lock.lock.load(Ordering::Acquire), 0);
    }

    #[test]
    fn rw_spin_locked_write_guard_updates_value() {
        let value = RwSpinLocked::new(7usize);

        {
            let mut guard = value.write_lock();
            *guard = 11;

            assert_eq!(*guard, 11);
            assert_eq!(value.lock.lock.load(Ordering::Acquire), -1);
        }

        assert_eq!(value.lock.lock.load(Ordering::Acquire), 0);
        assert_eq!(*value.read_lock(), 11);
    }

    #[test]
    fn spin_lock_try_lock_reports_state() {
        let lock = SpinLock::new();

        assert!(lock.try_lock());
        assert!(!lock.try_lock());

        unsafe {
            lock.unlock();
        }

        assert!(lock.try_lock());

        unsafe {
            lock.unlock();
        }
    }

    #[test]
    fn spin_lock_lock_and_unlock_update_state() {
        let lock = SpinLock::new();

        lock.lock();

        assert!(lock.lock.load(Ordering::Acquire));

        unsafe {
            lock.unlock();
        }

        assert!(!lock.lock.load(Ordering::Acquire));
    }

    #[test]
    fn spin_locked_guard_updates_value() {
        let value = SpinLocked::new(7usize);

        {
            let mut guard = value.lock();
            *guard = 11;

            assert_eq!(*guard, 11);
        }

        assert_eq!(*value.lock(), 11);
    }

    #[test]
    fn spin_locked_try_lock_returns_guard_when_unlocked() {
        let value = SpinLocked::new(7usize);

        {
            let mut guard = value.try_lock().expect("lock should be available");
            *guard = 11;

            assert!(value.try_lock().is_none());
        }

        assert_eq!(*value.try_lock().expect("lock should be available"), 11);
    }

    #[test]
    fn locked_types_are_sync_for_shareable_values() {
        fn assert_sync<T: Sync>() {}

        assert_sync::<RwSpinLocked<usize>>();
        assert_sync::<SpinLocked<usize>>();
    }
}
