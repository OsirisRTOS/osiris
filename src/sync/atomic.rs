//! Atomic abstractions for single and multi-core systems.

#[cfg(all(feature = "multi-core", feature = "no-atomic-cas"))]
compile_error!(
    "The `multi-core` feature requires atomic-cas operations to be available on the target. Enable the `atomic-cas` feature."
);

#[cfg(all(feature = "no-atomic-cas", not(target_has_atomic = "8")))]
compile_error!(
    "The `atomic-cas` feature requires the target to have atomic operations on at least 8-bit integers."
);

#[allow(unused_imports)]
pub use core::sync::atomic::Ordering;

#[inline(always)]
pub fn irq_free<T>(f: impl FnOnce() -> T) -> T {
    let enabled = hal::asm::are_interrupts_enabled();
    if enabled {
        hal::asm::disable_interrupts();
    }

    let result = f();

    if enabled {
        hal::asm::enable_interrupts();
    }

    result
}

// ----------------------------AtomicU8----------------------------
#[cfg(any(feature = "no-atomic-cas", not(target_has_atomic = "64")))]
use core::cell::UnsafeCell;

#[cfg(all(feature = "no-atomic-cas"))]
/// An atomic `u8`.
pub struct AtomicU8 {
    value: UnsafeCell<u8>,
}

#[allow(unused_imports)]
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

#[allow(unused_imports)]
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

// ----------------------------AtomicU64----------------------------
#[allow(unused_imports)]
#[cfg(target_has_atomic = "64")]
pub use core::sync::atomic::AtomicU64;

#[cfg(not(target_has_atomic = "64"))]
/// An atomic `u64` implemented by disabling interrupts around each operation.
pub struct AtomicU64 {
    value: UnsafeCell<u64>,
}

#[cfg(not(target_has_atomic = "64"))]
unsafe impl Sync for AtomicU64 {}

#[cfg(not(target_has_atomic = "64"))]
impl AtomicU64 {
    /// Creates a new atomic u64.
    pub const fn new(value: u64) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    /// Loads the value.
    pub fn load(&self, _: Ordering) -> u64 {
        irq_free(|| {
            // SAFETY: Interrupts are disabled, so this read is exclusive with writes.
            unsafe { *self.value.get() }
        })
    }

    /// Stores a value.
    pub fn store(&self, value: u64, _: Ordering) {
        irq_free(|| {
            // SAFETY: Interrupts are disabled, so this write is exclusive with other access.
            unsafe {
                *self.value.get() = value;
            }
        });
    }

    /// Compares the value and exchanges it.
    pub fn compare_exchange(
        &self,
        current: u64,
        new: u64,
        _: Ordering,
        _: Ordering,
    ) -> Result<u64, u64> {
        irq_free(|| {
            // SAFETY: Interrupts are disabled, so this read-modify-write is exclusive.
            unsafe {
                let value = self.value.get();
                if *value == current {
                    *value = new;
                    Ok(current)
                } else {
                    Err(*value)
                }
            }
        })
    }

    /// Fetches and adds, returning the previous value.
    pub fn fetch_add(&self, value: u64, _: Ordering) -> u64 {
        irq_free(|| {
            // SAFETY: Interrupts are disabled, so this read-modify-write is exclusive.
            unsafe {
                let ptr = self.value.get();
                let old = *ptr;
                *ptr = old.wrapping_add(value);
                old
            }
        })
    }

    /// Fetches a value, applies the function and writes it back atomically.
    pub fn fetch_update<F>(&self, _: Ordering, _: Ordering, mut f: F) -> Result<u64, u64>
    where
        F: FnMut(u64) -> Option<u64>,
    {
        irq_free(|| {
            // SAFETY: Interrupts are disabled, so this read-modify-write is exclusive.
            unsafe {
                let ptr = self.value.get();
                let old = *ptr;
                if let Some(new) = f(old) {
                    *ptr = new;
                    Ok(old)
                } else {
                    Err(old)
                }
            }
        })
    }
}
