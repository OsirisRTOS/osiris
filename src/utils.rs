
/// These two definitions are copied from https://github.com/rust-lang/hashbrown 
#[cfg(not(feature = "unstable"))]
pub(crate) use core::convert::{identity as likely, identity as unlikely};
#[cfg(feature = "unstable")]
pub(crate) use core::intrinsics::{likely, unlikely};

#[macro_export]
macro_rules! BUG {
    () => {
        panic!("BUG triggered at {}:{}", file!(), line!());
    };
    ($msg:expr) => {
        panic!("BUG triggered: {} at {}:{}", $msg, file!(), line!());  
    };
}

#[macro_export]
macro_rules! BUG_ON {
    ($cond:expr) => {
        if unsafe { $crate::utils::unlikely($cond) } {
            BUG!();
        }
    };
    ($cond:expr, $msg:expr) => {
        if unsafe { $crate::utils::unlikely($cond) } {
            BUG!($msg);
        }
    };
}
