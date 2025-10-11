//! This module provides access to userspace structures and services.

use ::core::{mem::transmute, ptr::NonNull};

use crate::mem;

pub mod core;
pub mod util;

pub fn init_app(boot_info: &crate::BootInfo) -> Result<(), crate::utils::KernelError> {
    let src = NonNull::new(boot_info.args.init.begin as *mut u8)
        .ok_or(crate::utils::KernelError::InvalidAddress)?;
    let len = boot_info.args.init.len;

    if len == 0 {
        return Err(crate::utils::KernelError::InvalidArgument);
    }

    // TODO: For now we let the app run in kernel mode. In the future we also want to dispatch the app in user mode.
    // This should not be a problem in regards to the syscall interface, since we can also syscall from kernel mode.
    let mem = mem::malloc(len, size_of::<u128>()).ok_or(crate::utils::KernelError::OutOfMemory)?;
    unsafe {
        mem.copy_from_nonoverlapping(src, len);
    };

    let entry = unsafe {
        transmute::<*const u8, extern "C" fn()>(mem.as_ptr().add(boot_info.args.init.entry_offset))
    };

    // We don't expect coming back from the init program.
    // But for future user mode support the init program will be run by the scheduler, thus we leave a result as a return value here.
    entry();
    Ok(())
}
