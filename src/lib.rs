#![cfg_attr(not(test), no_std)]

mod macros;
mod mem;
mod sched;
mod syscalls;
mod services;
mod uspace;

use core::ffi::{c_char, CStr};

/// The memory map entry type.
///
/// This structure shall be compatible with the multiboot_memory_map_t struct at
/// Link: https://www.gnu.org/software/grub/manual/multiboot/multiboot.html
#[repr(packed, C)]
pub struct MemMapEntry {
    size: u32,
    addr: u64,
    length: u64,
    ty: u32,
}

#[repr(C)]
pub struct BootInfo {
    pub implementer: *const c_char,
    pub variant: *const c_char,
    pub mmap: [MemMapEntry; 8],
    pub mmap_len: usize,
}

#[no_mangle]
pub unsafe extern "C" fn kernel_init(boot_info: *const BootInfo) {
    let boot_info = &*boot_info;

    let implementer = unsafe { CStr::from_ptr(boot_info.implementer) };
    let variant = unsafe { CStr::from_ptr(boot_info.variant) };

    if let (Ok(implementer), Ok(variant)) = (implementer.to_str(), variant.to_str()) {
        let _ = hal::hprintln!("[Kernel] Detected Processor:");
        let _ = hal::hprintln!("[Kernel] Manufacturer    : {}", implementer);
        let _ = hal::hprintln!("[Kernel] Name            : {}", variant);
    }

    // Initialize the memory allocator.
    if let Err(e) = mem::init_memory(boot_info) {
        panic!("[Kernel] Failed to initialize memory allocator: {:?}", e);
    }

    // Initialize the services.
    services::init_services();

    hal::hal_hw_init();

    // Start the scheduling.
    sched::reschedule();

    loop {}
}


#[cfg(all(not(test), not(doctest)))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    hal::common::panic::panic_handler(info);
}