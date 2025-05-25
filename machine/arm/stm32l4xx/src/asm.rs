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
    ($num:expr) => {
        use core::arch::asm;
        unsafe {
            asm!("svc {0}", const $num);
        }
    };
    ($num:expr, $arg0:expr) => {
        use core::arch::asm;
        unsafe {
            asm!("mov r0, {0}", "svc {1}", in(reg)$arg0, const $num);
        }
    };
    ($num:expr, $arg0:expr, $arg1:expr) => {
        use core::arch::asm;
        unsafe {
            asm!("mov r0, {0}", "mov r1, {1}", "svc {2}", in(reg)$arg0, in(reg)$arg1, const $num);
        }
    };
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr) => {
        use core::arch::asm;
        unsafe {
            asm!("mov r0, {0}", "mov r1, {1}", "mov r2, {2}", "svc {3}", in(reg)$arg0, in(reg)$arg1, in(reg)$arg2, const $num);
        }
    };
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        use core::arch::asm;
        unsafe {
            asm!("mov r0, {0}", "mov r1, {1}", "mov r2, {2}", "mov r3, {3}", "svc {4}", in(reg)$arg0, in(reg)$arg1, in(reg)$arg2, in(reg)$arg3, const $num);
        }
    };
}

pub use crate::__macro_syscall as syscall;
