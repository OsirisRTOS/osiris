//! Utility functions and definitions for the kernel.
#![cfg_attr(feature = "nightly", feature(likely_unlikely))]

use core::fmt::Debug;

/// These two definitions are copied from https://github.com/rust-lang/hashbrown
#[cfg(not(feature = "nightly"))]
#[allow(unused_imports)]
pub(crate) use core::convert::{identity as likely, identity as unlikely};

#[cfg(feature = "nightly")]
pub(crate) use core::hint::{likely, unlikely};


/// This is a macro that is used to panic when a bug is detected.
/// It is similar to the BUG() macro in the Linux kernel. Link: [https://www.kernel.org/]()
#[macro_export]
macro_rules! BUG {
    () => {
        panic!("BUG triggered at {}:{}", file!(), line!());
    };
    ($msg:expr) => {
        panic!("BUG triggered: {} at {}:{}", $msg, file!(), line!());
    };
}

/// This is a macro that is used to panic when a condition is true.
/// It is similar to the BUG_ON() macro in the Linux kernel.  Link: [https://www.kernel.org/]()
#[macro_export]
macro_rules! BUG_ON {
    ($cond:expr) => {
        {
            let cond = $cond;
            #[allow(unused_unsafe)]
            if unsafe { $crate::utils::unlikely(cond) } {
                BUG!();
            }
        }
    };
    ($cond:expr, $msg:expr) => {
        {
            let cond = $cond;
            #[allow(unused_unsafe)]
            if unsafe { $crate::utils::unlikely(cond) } {
                BUG!($msg);
            }
        }
    };
}

/// The error type that is returned when an error in the kernel occurs.
#[derive(PartialEq, Eq, Clone)]
pub enum KernelError {
    /// The alignment is invalid.
    InvalidAlign,
    /// The kernel is out of memory.
    OutOfMemory,
    InvalidSize,
    InvalidAddress,
    InvalidArgument,
    HalError(hal::Error),
}

/// Debug msg implementation for KernelError.
impl Debug for KernelError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            KernelError::InvalidAlign => write!(f, "Invalid alignment"),
            KernelError::OutOfMemory => write!(f, "Out of memory"),
            KernelError::InvalidSize => write!(f, "Invalid size"),
            KernelError::InvalidAddress => write!(f, "Invalid address"),
            KernelError::InvalidArgument => write!(f, "Invalid argument"),
            KernelError::HalError(e) => write!(f, "{e} (in HAL)"),
        }
    }
}

impl From<hal::Error> for KernelError {
    fn from(err: hal::Error) -> Self {
        KernelError::HalError(err)
    }
}
