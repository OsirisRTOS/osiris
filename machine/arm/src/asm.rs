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
    {
        let result: isize;
        unsafe {
            core::arch::asm!(
                "svc #{0}",  // const $num is operand 0
                const $num,
                lateout("r0") result,
                clobber_abi("C"),
            );
        }
        result
    }
    };
    ($num:expr, $arg0:expr) => {
    {
        let result: isize;
        unsafe {
            core::arch::asm!(
                "svc #{0}",  // const $num is operand 1 (after r0)
                const $num,
                inout("r0") $arg0 => result,
                clobber_abi("C"),
            );
        }
        result
    }
    };
    ($num:expr, $arg0:expr, $arg1:expr) => {
    {
        let result: isize;
        unsafe {
            core::arch::asm!(
                "svc #{0}",  // const $num is operand 2 (after r0, r1)
                const $num,
                inout("r0") $arg0 => result,
                in("r1") $arg1,
                clobber_abi("C"),
            );
        }
        result
    }
    };
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr) => {
    {
        let result: isize;
        unsafe {
            core::arch::asm!(
                "svc #{0}",  // const $num is operand 3
                const $num,
                inout("r0") $arg0 => result,
                in("r1") $arg1,
                in("r2") $arg2,
                clobber_abi("C"),
            );
        }
        result
    }
    };
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
    {
        let result: isize;
        unsafe {
            core::arch::asm!(
                "svc #{0}",  // const $num is operand 4
                const $num,
                inout("r0") $arg0 => result,
                in("r1") $arg1,
                in("r2") $arg2,
                in("r3") $arg3,
                clobber_abi("C"),
            );
        }
        result
    }
    };
}

#[cfg(feature = "host")]
#[macro_export]
macro_rules! __macro_syscall {
    ($num:expr) => {
        0isize
    };
    ($num:expr, $arg0:expr) => {{ 0isize }};
    ($num:expr, $arg0:expr, $arg1:expr) => {{ 0isize }};
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr) => {{ 0isize }};
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {{ 0isize }};
}

pub use crate::__macro_syscall as syscall;

#[cfg(not(feature = "host"))]
#[inline(always)]
pub fn disable_interrupts() {
    use core::arch::asm;
    use core::sync::atomic::compiler_fence;

    unsafe { asm!("cpsid i", options(nomem, nostack, preserves_flags)) };
    compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

#[cfg(feature = "host")]
#[inline(always)]
pub fn disable_interrupts() {}

#[cfg(not(feature = "host"))]
#[inline(always)]
pub fn are_interrupts_enabled() -> bool {
    use core::arch::asm;

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
    use core::arch::asm;
    use core::sync::atomic::compiler_fence;

    unsafe { asm!("cpsie i", options(nomem, nostack, preserves_flags)) };
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
        naked_asm!("ldr r1,=__stack_top", "mov sp, r1", "b bootstrap")
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

#[cfg(not(feature = "host"))]
#[macro_export]
macro_rules! __macro_delay {
    ($cycles:expr) => {{
        for _ in 0..$cycles {
            $crate::asm::nop!();
        }
    }};
}

#[cfg(feature = "host")]
#[macro_export]
macro_rules! __macro_delay {
    ($cycles:expr) => {{}};
}

pub use crate::__macro_delay as delay;

#[cfg(not(feature = "host"))]
#[macro_export]
macro_rules! __macro_fault_do_not_use_under_any_circumstances {
    () => {{
        use core::arch::asm;
        use core::sync::atomic::compiler_fence;
        compiler_fence(core::sync::atomic::Ordering::SeqCst);
        asm!("udf #0", options(nomem, nostack, preserves_flags));
        compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }};
}

#[cfg(feature = "host")]
#[macro_export]
macro_rules! __macro_fault_do_not_use_under_any_circumstances {
    () => {{}};
}

pub use crate::__macro_fault_do_not_use_under_any_circumstances as fault_do_not_use_under_any_circumstances;
