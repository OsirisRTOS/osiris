#![allow(dead_code)]

use crate::hal;

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::ops::Deref;
use core::sync::atomic::AtomicU8;
use core::sync::atomic::Ordering;

/// A synchronization primitive that can be used to block a thread until a value is ready.
/// The procedure is as follows:
/// 1. The Caller calls step(NOT_READY) to indicate that it is about to start the initialization process.
/// 2. The Caller initializes the value.
/// 3. The Caller calls step(IN_TRANSIT) to indicate that the value is ready.
///    If step 1 fails, the value is already being initialized and the Caller must wait until is() returns true.
pub struct Ready {
    ready: AtomicU8,
}

impl Ready {
    const READY: u8 = 2;
    const IN_TRANSIT: u8 = 1;
    const NOT_READY: u8 = 0;

    /// Initializes a new Ready.
    pub const fn new() -> Self {
        Self {
            ready: AtomicU8::new(0),
        }
    }

    /// Move the Ready to the next state, if it is in state `from`.
    pub fn step(&self, from: u8) -> bool {
        self.forward(from, from + 1)
    }

    /// Move the Ready to state `to` if it is in state `from`.
    fn forward(&self, _from: u8, _to: u8) -> bool {
        self.ready
            .compare_exchange(_from, _to, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// Returns true if the value is ready.
    pub fn is(&self) -> bool {
        self.ready.load(Ordering::Acquire) == Self::READY
    }
}

/// A synchronization primitive that represents a value that is initialized at most once.
pub struct OnceCell<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    init: Ready,
}

/// Safety:
/// 1. A `value` is only written to atomically and once.
/// 2. A `value` is only readable from after the initialization process is finished.
/// 3. A `init` is only written and read from atomically.
unsafe impl<T> Sync for OnceCell<T> {}

impl<T> OnceCell<T> {
    /// Initializes a new OnceCell.
    pub const fn new() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            init: Ready::new(),
        }
    }

    /// Returns a reference to the value if it is initialized.
    pub fn get(&self) -> Option<&T> {
        if self.init.is() {
            // Safety:
            // 1. By contract, is the value initialized if init.is() returns true.
            // 2. No writes are allowed to the value after the initialization process is finished.
            Some(unsafe { self.get_unchecked() })
        } else {
            None
        }
    }

    /// Sets the value if it is not already initialized, and returns a reference to the value.
    pub fn set_or_get(&self, value: T) -> &T {
        if let Some(value) = self.set(value) {
            value
        } else {
            // If we reach this point, initialization is already in progress.
            while !self.init.is() {
                hal::asm::nop!();
            }
            // Safety:
            // 1. By contract, is the value initialized if init.is() returns true.
            // 2. No writes are allowed to the value after the initialization process is finished.
            unsafe { self.get_unchecked() }
        }
    }

    /// Sets the value if it is not already initialized, and returns a reference to the value.
    pub fn do_or_get<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        self.get_or_init(f)
    }

    /// Returns the value, initializing it with `f` if needed.
    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        if let Some(value) = self.get() {
            return value;
        }

        if self.init.step(Ready::NOT_READY) {
            // Safety: We are now in the IN_TRANSIT state, so we are the only ones that can write to the value.
            unsafe {
                self.value.get().write(MaybeUninit::new(f()));
            }

            if self.init.step(Ready::IN_TRANSIT) {
                // Safety: We are now in the READY state, so no writes can happen to the value.
                return unsafe { self.get_unchecked() };
            }

            // By contract, only the thread that started the initialization process can finish it.
            unreachable!();
        }

        // If we reach this point, initialization is already in progress.
        while !self.init.is() {
            hal::asm::nop!();
        }

        // Safety:
        // 1. By contract, is the value initialized if init.is() returns true.
        // 2. No writes are allowed to the value after the initialization process is finished.
        unsafe { self.get_unchecked() }
    }

    /// Sets the value if it is not already initialized, returns a reference to the value if it was not set previously.
    pub fn set(&self, value: T) -> Option<&T> {
        if self.init.is() {
            return None;
        }

        if self.init.step(Ready::NOT_READY) {
            // Safety: We are now in the IN_TRANSIT state, so we are the only ones that can write to the value.
            // We are also the only ones that can read from the value.
            unsafe {
                self.value.get().write(MaybeUninit::new(value));
            }

            if self.init.step(Ready::IN_TRANSIT) {
                // Safety: We are now in the READY state, so no writes can happen to the value.
                // 1. It is safe to create a immutable reference to the value.
                // 2. We initialized the value, so it is safe to return a reference to it.
                return Some(unsafe { self.get_unchecked() });
            }

            // By contract, only the thread that started the initialization process can finish it.
            unreachable!();
        }

        None
    }

    /// Returns a reference to the value, unchecked.
    ///
    /// # Safety
    /// Preconditions: The value must be initialized.
    /// Postconditions: The value is returned.
    unsafe fn get_unchecked(&self) -> &T {
        unsafe { (*self.value.get()).assume_init_ref() }
    }
}

/// A lazily initialized value.
pub struct LazyLock<T, F = fn() -> T> {
    cell: OnceCell<T>,
    init: UnsafeCell<Option<F>>,
}

// Safety:
// 1. `cell` synchronizes initialization and only exposes shared references once ready.
// 2. `init` is taken only by the thread that transitions `cell` into initialization.
unsafe impl<T: Sync, F: Send> Sync for LazyLock<T, F> {}
unsafe impl<T: Send, F: Send> Send for LazyLock<T, F> {}

impl<T, F> LazyLock<T, F> {
    /// Creates a new LazyLock.
    pub const fn new(f: F) -> Self {
        Self {
            cell: OnceCell::new(),
            init: UnsafeCell::new(Some(f)),
        }
    }
}

impl<T, F> LazyLock<T, F>
where
    F: FnOnce() -> T,
{
    /// Returns the lazily initialized value.
    pub fn force(this: &Self) -> &T {
        this.cell.get_or_init(|| {
            // Safety:
            // 1. `get_or_init` calls this closure only for the thread that won initialization.
            // 2. No other thread can access `init` after initialization starts.
            let init = unsafe {
                (*this.init.get())
                    .take()
                    .expect("LazyLock initializer missing")
            };
            init()
        })
    }
}

impl<T, F> Deref for LazyLock<T, F>
where
    F: FnOnce() -> T,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        Self::force(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn once_cell_get_or_init_initializes_once() {
        let cell = OnceCell::new();
        let mut calls = 0usize;

        assert_eq!(
            *cell.get_or_init(|| {
                calls += 1;
                7
            }),
            7
        );

        assert_eq!(*cell.get_or_init(|| 11), 7);
        assert_eq!(calls, 1);
    }

    #[test]
    fn once_cell_do_or_get_does_not_call_initializer_when_ready() {
        let cell = OnceCell::new();

        assert_eq!(*cell.set_or_get(7), 7);
        assert_eq!(*cell.do_or_get(|| panic!("initializer should not run")), 7);
    }

    #[test]
    fn lazy_lock_initializes_on_first_access() {
        let value = LazyLock::new(|| 7usize);

        assert_eq!(*value, 7);
        assert_eq!(*LazyLock::force(&value), 7);
    }
}
