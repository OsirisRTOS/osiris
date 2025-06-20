//! Atomic abstractions for single and multi-core systems.

#[cfg(all(feature = "multi-core", feature = "no-atomic-cas"))]
compile_error!(
    "The `multi-core` feature requires atomic-cas operations to be available on the target. Enable the `atomic-cas` feature."
);

#[cfg(all(feature = "no-atomic-cas", not(target_has_atomic = "8")))]
compile_error!(
    "The `atomic-cas` feature requires the target to have atomic operations on at least 8-bit integers."
);

// ----------------------------AtomicU8----------------------------
#[cfg(all(feature = "no-atomic-cas"))]
pub use core::sync::atomic::Ordering;

#[cfg(all(feature = "no-atomic-cas"))]
use core::cell::UnsafeCell;

#[cfg(all(feature = "no-atomic-cas"))]
/// An atomic `u8`.
pub struct AtomicU8 {
    value: UnsafeCell<u8>,
}

#[cfg(not(all(feature = "no-atomic-cas")))]
pub use core::sync::atomic::AtomicU8;

#[cfg(all(feature = "no-atomic-cas"))]
impl AtomicU8 {
    /// Creates a new atomic u8.
    pub const fn new(value: u8) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    /// Loads the value.
    pub fn load(&self, _: Ordering) -> u8 {
        todo!("Implement atomic load for u8");
    }

    /// Stores a value.
    pub fn store(&self, value: u8, _: Ordering) {
        todo!("Implement atomic store for u8");
    }

    /// Compares the value and exchanges it.
    pub fn compare_exchange(
        &self,
        current: u8,
        new: u8,
        _: Ordering,
        _: Ordering,
    ) -> Result<u8, u8> {
        todo!("Implement atomic compare_exchange for u8");
    }

    ///fetch a value, apply the function and write back the modified value atomically
    pub fn fetch_update<F>(&self, _: Ordering, _: Ordering, f: F) -> Result<u8, u8>
    where
        F: FnMut(u8) -> Option<u8>,
    {
        todo!("Implement atomic fetch_update for u8");
    }
}

#[cfg(not(all(feature = "no-atomic-cas")))]
pub use core::sync::atomic::AtomicBool;

#[cfg(all(feature = "no-atomic-cas"))]
/// An atomic `bool`.
pub struct AtomicBool {
    value: UnsafeCell<bool>,
}

#[cfg(all(feature = "no-atomic-cas"))]
impl AtomicBool {
    /// Creates a new atomic bool.
    pub const fn new(value: bool) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    /// Loads the value.
    pub fn load(&self, _: Ordering) -> bool {
        todo!("Implement atomic load for bool");
    }

    /// Stores a value.
    pub fn store(&self, value: bool, _: Ordering) {
        todo!("Implement atomic store for bool");
    }

    /// Compares the value and exchanges it.
    pub fn compare_exchange(
        &self,
        current: bool,
        new: bool,
        _: Ordering,
        _: Ordering,
    ) -> Result<bool, bool> {
        todo!("Implement atomic compare_exchange for bool");
    }
}
