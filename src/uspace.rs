//! This module provides access to userspace structures and services.

pub fn init_app(boot_info: &crate::BootInfo) -> Result<(), crate::utils::KernelError> {
    let len = boot_info.args.init.len;

    if len == 0 {
        return Err(crate::utils::KernelError::InvalidArgument);
    }

    let entry_addr = boot_info.args.init.begin as usize
        + boot_info.args.init.entry_offset as usize;
    let static_base = boot_info.args.init.begin as usize
        + boot_info.args.init.static_base_offset as usize;

    // On ARM targets, set r9 to the user program's static base (required by the RWPI relocation
    // model) then branch to the entry point without a return address (bx, not blx).
    #[cfg(target_arch = "arm")]
    unsafe {
        core::arch::asm!(
            "mov r9, {sb}",
            "bx {entry}",
            sb = in(reg) static_base,
            entry = in(reg) entry_addr,
            options(noreturn),
        );
    }

    // On non-ARM (host) builds the RWPI model is not active; call the entry normally.
    #[cfg(not(target_arch = "arm"))]
    {
        let _ = static_base;
        let entry_fn =
            unsafe { core::mem::transmute::<usize, extern "C" fn()>(entry_addr) };
        entry_fn();
        Ok(())
    }
}
