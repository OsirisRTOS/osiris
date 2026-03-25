//! This module provides access to userspace structures and services.

use ::core::mem::transmute;

use crate::sched;

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

    let attrs = sched::thread::Attributes {
        entry,
        fin: None,
    };
    sched::create_thread(sched::task::KERNEL_TASK, &attrs)?;
    Ok(())
}
