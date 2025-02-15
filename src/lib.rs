//! This is the default kernel of the osiris operating system.
//! The kernel is organized as a microkernel.

#![cfg_attr(all(not(test), not(doctest), not(doc)), no_std)]

mod macros;
#[macro_use]
mod utils;
mod mem;
mod sched;
mod services;
mod syscalls;
mod uspace;

use core::ffi::{c_char, CStr};

/// The memory map entry type.
///
/// This structure shall be compatible with the multiboot_memory_map_t struct at
/// Link: [https://www.gnu.org/software/grub/manual/multiboot/multiboot.html]()
#[repr(packed, C)]
pub struct MemMapEntry {
    /// The size of the entry.
    size: u32,
    /// The base address of the memory region.
    addr: u64,
    /// The length of the memory region.
    length: u64,
    /// The type of the memory region.
    ty: u32,
}

/// The boot information structure.
#[repr(C)]
pub struct BootInfo {
    /// The implementer of the processor.
    pub implementer: *const c_char,
    /// The variant of the processor.
    pub variant: *const c_char,
    /// The memory map.
    pub mmap: [MemMapEntry; 8],
    /// The length of the memory map.
    pub mmap_len: usize,
}

/// The kernel initialization function.
///
/// `boot_info` - The boot information.
#[no_mangle]
pub unsafe extern "C" fn kernel_init(boot_info: *const BootInfo) {
    let boot_info = &*boot_info;

    let implementer = unsafe { CStr::from_ptr(boot_info.implementer) };
    let variant = unsafe { CStr::from_ptr(boot_info.variant) };

    if let (Ok(implementer), Ok(variant)) = (implementer.to_str(), variant.to_str()) {
        //let _ = hal::hprintln!("[Kernel] Detected Processor:");
        //let _ = hal::hprintln!("[Kernel] Manufacturer    : {}", implementer);
        //let _ = hal::hprintln!("[Kernel] Name            : {}", variant);
    }

    // Initialize the memory allocator.
    if let Err(e) = mem::init_memory(boot_info) {
        panic!("[Kernel] Failed to initialize memory allocator.");
    }

    // Initialize the services.
    services::init_services();

    hal::hal_hw_init();

    // Start the scheduling.
    sched::reschedule();

    loop {}
}

/// The panic handler.
#[cfg(all(not(test), not(doctest), not(doc)))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    hal::common::panic::panic_handler(info);
}
