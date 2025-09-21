#[macro_export]
macro_rules! __macro_nop {
    () => {
        unsafe { core::arch::asm!("nop", options(nomem, nostack, preserves_flags)) };
    };
}

// This prefixing is a little cursed but necessary to avoid name conflicts, because #[macro_export] exports macros at the top level.
pub use crate::__macro_nop as nop;

/// Macro for doing a system call.
#[macro_export]
macro_rules! __macro_syscall {
    ($num:expr) => {{}};
    ($num:expr, $arg0:expr) => {{}};
    ($num:expr, $arg0:expr, $arg1:expr) => {{}};
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr) => {{}};
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {{}};
}

pub use crate::__macro_syscall as syscall;

#[inline(always)]
pub fn disable_interrupts() {}

#[inline(always)]
pub fn are_interrupts_enabled() -> bool {
    true
}

#[inline(always)]
pub fn enable_interrupts() {}

#[macro_export]
macro_rules! __macro_startup_trampoline {
    () => {{
        use core::arch::naked_asm;
        naked_asm!("")
    }};
}

pub use crate::__macro_startup_trampoline as startup_trampoline;

#[macro_export]
macro_rules! __macro_delay {
    ($cycles:expr) => {{
        for _ in 0..$cycles {
            $crate::asm::nop!();
        }
    }};
}

pub use crate::__macro_delay as delay;
