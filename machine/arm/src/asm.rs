#[cfg(not(feature = "host"))]
#[macro_export]
macro_rules! __macro_nop {
    () => {
        unsafe { core::arch::asm!("nop", options(nomem, nostack, preserves_flags)) };
    };
}

#[cfg(feature = "host")]
#[macro_export]
macro_rules! __macro_nop {
    () => {{}};
}

// This prefixing is a little cursed but necessary to avoid name conflicts, because #[macro_export] exports macros at the top level.
pub use crate::__macro_nop as nop;

/// Macro for doing a system call.

#[cfg(not(feature = "host"))]
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

#[cfg(feature = "host")]
#[macro_export]
macro_rules! __macro_syscall {
    ($num:expr) => {{}};
    ($num:expr, $arg0:expr) => {{}};
    ($num:expr, $arg0:expr, $arg1:expr) => {{}};
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr) => {{}};
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {{}};
}

pub use crate::__macro_syscall as syscall;

use core::arch::asm;
use core::sync::atomic::compiler_fence;

#[cfg(not(feature = "host"))]
#[inline(always)]
pub fn disable_interrupts() {
    unsafe { asm!("cpsid f", options(nomem, nostack, preserves_flags)) };
    compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

#[cfg(feature = "host")]
#[inline(always)]
pub fn disable_interrupts() {}

#[cfg(not(feature = "host"))]
#[inline(always)]
pub fn are_interrupts_enabled() -> bool {
    let primask: u32;
    unsafe {
        asm!("mrs {}, primask", out(reg) primask, options(nomem, nostack, preserves_flags));
    }
    primask == 0
}

#[cfg(feature = "host")]
#[inline(always)]
pub fn are_interrupts_enabled() -> bool {
    true
}

#[cfg(not(feature = "host"))]
#[inline(always)]
pub fn enable_interrupts() {
    unsafe { asm!("cpsie f", options(nomem, nostack, preserves_flags)) };
    compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

#[cfg(feature = "host")]
#[inline(always)]
pub fn enable_interrupts() {}

#[cfg(not(feature = "host"))]
#[macro_export]
macro_rules! __macro_startup_trampoline {
    () => {{
        use core::arch::naked_asm;
        naked_asm!("ldr r1,=__stack_top", "mov sp, r1", "b _main")
    }};
}

#[cfg(feature = "host")]
#[macro_export]
macro_rules! __macro_startup_trampoline {
    () => {{
        use core::arch::naked_asm;
        naked_asm!("")
    }};
}

pub use crate::__macro_startup_trampoline as startup_trampoline;
