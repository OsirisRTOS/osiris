//! This module provides access to userspace structures and services.

use ::core::mem::transmute;

pub fn init_app(boot_info: &crate::BootInfo) -> Result<(), crate::utils::KernelError> {
    let len = boot_info.args.init.len;

    if len == 0 {
        return Err(crate::utils::KernelError::InvalidArgument);
    }

    let entry = unsafe {
        transmute::<usize, extern "C" fn()>(
            boot_info.args.init.begin as usize + boot_info.args.init.entry_offset as usize,
        )
    };

    // We don't expect coming back from the init program.
    // But for future user mode support the init program will be run by the scheduler, thus we leave a result as a return value here.
    entry();
    Ok(())
}
