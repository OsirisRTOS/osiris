//! Utility functions and definitions for the kernel.
#![cfg_attr(feature = "nightly", feature(likely_unlikely))]

use core::fmt::Debug;
use core::fmt::Display;
use hal::mem::PhysAddr;

/// These two definitions are copied from https://github.com/rust-lang/hashbrown
#[cfg(not(feature = "nightly"))]
#[allow(unused_imports)]
pub(crate) use core::convert::{identity as likely, identity as unlikely};

#[cfg(feature = "nightly")]
pub(crate) use core::hint::{likely, unlikely};

pub type Result<T> = core::result::Result<T, Error>;

/// This is a macro that is used to panic when a bug is detected.
/// It is similar to the BUG() macro in the Linux kernel. Link: [https://www.kernel.org/]()
#[macro_export]
macro_rules! bug {
    () => {
        panic!("BUG at {}:{}", file!(), line!());
    };
    ($fmt:literal $(, $arg:expr)* $(,)?) => {{
        panic!(concat!("BUG at {}:{}: ", $fmt), file!(), line!() $(, $arg)*);
    }};
}

#[macro_export]
macro_rules! warn {
    () => {
        kprintln!("WARN at {}:{}", file!(), line!());
    };
    ($fmt:literal $(, $arg:expr)* $(,)?) => {{
        kprintln!(concat!("WARN at {}:{}: ", $fmt), file!(), line!() $(, $arg)*);
    }};
}

/// This is a macro that is used to panic when a condition is true.
/// It is similar to the BUG_ON() macro in the Linux kernel.  Link: [https://www.kernel.org/]()
macro_rules! bug_on {
    ($cond:expr) => {{
        let cond = $cond;
        #[allow(unused_unsafe)]
        if unsafe { $crate::error::unlikely(cond) } {
            panic!("BUG({}) at {}:{}", stringify!($cond), file!(), line!());
        }
    }};
    ($cond:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {{
        let cond = $cond;
        #[allow(unused_unsafe)]
        if unsafe { $crate::error::unlikely(cond) } {
            panic!(concat!("BUG({}) at {}:{}: ", $fmt), stringify!($cond), file!(), line!() $(, $arg)*);
        }
    }};
}

#[allow(unused_macros)]
macro_rules! warn_on {
    ($cond:expr) => {{
        let cond = $cond;
        #[allow(unused_unsafe)]
        if unsafe { $crate::error::unlikely(cond) } {
            kprintln!("WARN({}) at {}:{}", stringify!($cond), file!(), line!());
        }
    }};
    ($cond:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {{
        let cond = $cond;
        #[allow(unused_unsafe)]
        if unsafe { $crate::error::unlikely(cond) } {
            kprintln!(concat!("WARN({}) at {}:{}: ", $fmt), stringify!($cond), file!(), line!() $(, $arg)*);
        }
    }};
}

macro_rules! kerr {
    ($kind:ident) => {
        $crate::error::Error::new($crate::error::Kind::$kind)
    };
    ($kind:expr, $msg:expr) => {
        use $crate::error::Error;
        #[cfg(feature = "error-msg")]
        {
            Error::new($crate::error::Kind::$kind).with_msg($msg)
        }
        #[cfg(not(feature = "error-msg"))]
        {
            Error::new($crate::error::Kind::$kind)
        }
    };
}

#[proc_macros::fmt]
#[allow(dead_code)]
#[derive(Clone, PartialEq, Eq)]
pub enum Kind {
    InvalidAlign,
    OutOfMemory,
    InvalidSize,
    InvalidAddress(PhysAddr),
    InvalidArgument,
    NotFound,
    Hal(hal::Error),
}

impl Display for Kind {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Kind::InvalidAlign => write!(f, "Invalid alignment"),
            Kind::OutOfMemory => write!(f, "Out of memory"),
            Kind::InvalidSize => write!(f, "Invalid size"),
            Kind::InvalidAddress(addr) => write!(f, "Invalid address: {addr:#x}"),
            Kind::InvalidArgument => write!(f, "Invalid argument"),
            Kind::NotFound => write!(f, "Not found"),
            Kind::Hal(e) => write!(f, "HAL error: {e:?}"),
        }
    }
}

pub struct Error {
    pub kind: Kind,
    #[cfg(feature = "error-msg")]
    msg: Option<&'static str>,
}

impl Error {
    pub fn new(kind: Kind) -> Self {
        #[cfg(feature = "error-msg")]
        {
            Self { kind, msg: None }
        }
        #[cfg(not(feature = "error-msg"))]
        {
            Self { kind }
        }
    }

    #[cfg(feature = "error-msg")]
    pub fn with_msg(mut self, msg: &'static str) -> Self {
        self.msg = Some(msg);
        self
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(feature = "error-msg")]
        {
            match self.msg {
                Some(msg) => write!(f, "{}: {}", self.kind, msg),
                None => write!(f, "{}", self.kind),
            }
        }
        #[cfg(not(feature = "error-msg"))]
        {
            write!(f, "{}", self.kind)
        }
    }
}

impl Display for Error {
    #[cfg(not(feature = "error-msg"))]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.kind)
    }

    #[cfg(feature = "error-msg")]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.msg {
            Some(msg) => write!(f, "{}: {}", self.kind, msg),
            None => write!(f, "{}", self.kind),
        }
    }
}

impl From<hal::Error> for Error {
    fn from(e: hal::Error) -> Self {
        Self::new(Kind::Hal(e))
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}
