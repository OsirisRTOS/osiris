#[macro_export]
macro_rules! __macro_nop {
    () => {
        unsafe { core::arch::asm!("nop", options(nomem, nostack, preserves_flags)) };
    };
}

// This prefixing is a little cursed but necessary to avoid name conflicts, because #[macro_export] exports macros at the top level.
pub use __macro_nop as nop;

/// Macro for doing a system call.
#[macro_export]
macro_rules! __macro_syscall {
    ($num:expr) => {
        {
            use core::arch::asm;
            let ret: isize;
            unsafe {
                asm!(
                    "svc {num}",
                    lateout("r0") ret,
                    num = const $num,
                    clobber_abi("C")
                );
            }
            ret
        }
    };
    ($num:expr, $arg0:expr) => {
        {
            use core::arch::asm;
            let ret: isize;
            unsafe {
                asm!(
                    "svc {num}",
                    inlateout("r0") $arg0 => ret,
                    num = const $num,
                    clobber_abi("C")
                );
            }
            ret
        }
    };
    ($num:expr, $arg0:expr, $arg1:expr) => {
        {
            use core::arch::asm;
            let ret: isize;
            unsafe {
                asm!(
                    "svc {num}",
                    inlateout("r0") $arg0 => ret,
                    in("r1") $arg1,
                    num = const $num,
                    clobber_abi("C")
                );
            }
            ret
        }
    };
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr) => {
        {
            use core::arch::asm;
            let ret: isize;
            unsafe {
                asm!(
                    "svc {num}",
                    inlateout("r0") $arg0 => ret,
                    in("r1") $arg1,
                    in("r2") $arg2,
                    num = const $num,
                    clobber_abi("C")
                );
            }
            ret
        }
    };
    ($num:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        {
            use core::arch::asm;
            let ret: isize;
            unsafe {
                asm!(
                    "svc {num}",
                    inlateout("r0") $arg0 => ret,
                    in("r1") $arg1,
                    in("r2") $arg2,
                    in("r3") $arg3,
                    num = const $num,
                    clobber_abi("C")
                );
            }
            ret
        }
    };
}

pub use __macro_syscall as syscall;

#[inline(always)]
pub fn disable_irq_save() -> usize {
    use core::arch::asm;

    let old: usize;

    unsafe {
        asm!(
            "mrs {old}, primask",
            "cpsid i",
            "isb",
            old = out(reg) old,
            options(nostack, preserves_flags)
        );
    }
    old
}

#[inline(always)]
pub fn are_interrupts_enabled() -> bool {
    use core::arch::asm;

    let primask: u32;
    unsafe {
        asm!("mrs {}, primask", out(reg) primask, options(nomem, nostack, preserves_flags));
    }
    primask == 0
}

#[inline(always)]
pub fn enable_irq_restr(state: usize) {
    use core::arch::asm;

    unsafe {
        asm!(
            "dsb",
            "msr primask, {state}",
            "isb",
            state = in(reg) state,
            options(nostack, preserves_flags)
        );
    }
}

#[macro_export]
macro_rules! __macro_startup_trampoline {
    () => {{
        use core::arch::naked_asm;
        naked_asm!("ldr r1,=__stack_top", "mov sp, r1", "b bootstrap")
    }};
}

pub use __macro_startup_trampoline as startup_trampoline;

#[macro_export]
macro_rules! __macro_delay {
    ($cycles:expr) => {{
        for _ in 0..$cycles {
            $crate::asm::nop!();
        }
    }};
}

pub use __macro_delay as delay;

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

pub use __macro_fault_do_not_use_under_any_circumstances as fault_do_not_use_under_any_circumstances;
