//! Utility functions and definitions for the kernel.
#![cfg_attr(feature = "nightly", feature(likely_unlikely))]

#[cfg(feature = "error-msg")]
use core::fmt::{self, Write};
use core::fmt::{Debug, Display};

/// These two definitions are copied from https://github.com/rust-lang/hashbrown
#[cfg(not(feature = "nightly"))]
#[allow(unused_imports)]
pub(crate) use core::convert::{identity as likely, identity as unlikely};

#[cfg(feature = "nightly")]
pub(crate) use core::hint::{likely, unlikely};

pub type Result<T> = core::result::Result<T, Error>;
pub use hal_api::PosixError;
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
        $crate::kprintln!("WARN at {}:{}", file!(), line!());
    };
    ($fmt:literal $(, $arg:expr)* $(,)?) => {{
        $crate::kprintln!(concat!("WARN at {}:{}: ", $fmt), file!(), line!() $(, $arg)*);
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
    ($posix:ident) => {
        $crate::error::Error::new($crate::error::PosixError::$posix)
    };
    ($posix:ident, $msg:expr) => {{
        #[cfg(feature = "error-msg")]
        {
            $crate::error::Error::new($crate::error::PosixError::$posix).with_msg($msg)
        }
        #[cfg(not(feature = "error-msg"))]
        {
            $crate::error::Error::new($crate::error::PosixError::$posix)
        }
    }};
}

#[derive(Clone, Eq)]
pub struct Error {
    pub kind: PosixError,
    #[cfg(feature = "error-msg")]
    msg: Option<Msg>,
}

#[cfg(feature = "error-msg")]
struct Msg {
    buf: [u8; 128],
    len: usize,
}

#[cfg(feature = "error-msg")]
impl Msg {
    fn new(args: fmt::Arguments<'_>) -> Self {
        let mut msg = Self {
            buf: [0; 128],
            len: 0,
        };
        let _ = msg.write_fmt(args);
        msg
    }

    fn as_str(&self) -> &str {
        // Safety: `Msg` is only written through `fmt::Write::write_str`, which
        // copies from valid UTF-8 string slices and only truncates at char boundaries.
        unsafe { core::str::from_utf8_unchecked(&self.buf[..self.len]) }
    }
}

#[cfg(feature = "error-msg")]
impl Write for Msg {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let remaining = self.buf.len() - self.len;
        let mut write_len = remaining.min(s.len());

        while !s.is_char_boundary(write_len) {
            write_len -= 1;
        }

        self.buf[self.len..self.len + write_len].copy_from_slice(&s.as_bytes()[..write_len]);
        self.len += write_len;
        Ok(())
    }
}

impl Error {
    pub fn new(kind: PosixError) -> Self {
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
    pub fn with_msg(mut self, msg: fmt::Arguments<'_>) -> Self {
        self.msg = Some(Msg::new(msg));
        self
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        #[cfg(feature = "error-msg")]
        {
            match &self.msg {
                Some(msg) => write!(f, "{}: {}", self.kind, msg.as_str()),
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
        match &self.msg {
            Some(msg) => write!(f, "{}: {}", self.kind, msg.as_str()),
            None => write!(f, "{}", self.kind),
        }
    }
}

impl From<PosixError> for Error {
    fn from(e: PosixError) -> Self {
        Self::new(e)
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

#[cfg(all(test, feature = "error-msg"))]
mod tests {
    use super::*;

    #[test]
    fn kerr_formats_captured_message() {
        let compatible = "sensor";
        let ordinal = 2usize;
        let err = kerr!(
            NotFound,
            "i2c device not found: compatible={compatible}, ordinal={ordinal}"
        );

        assert_eq!(
            format!("{err}"),
            "Not found: i2c device not found: compatible=sensor, ordinal=2"
        );
    }
}
