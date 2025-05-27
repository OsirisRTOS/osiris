//! This is the default kernel of the osiris operating system.
//! The kernel is organized as a microkernel.

#![cfg_attr(all(not(test), not(doctest), not(doc), not(kani)), no_std)]

mod macros;
#[macro_use]
mod utils;
mod mem;
mod print;
mod sched;
mod services;
mod sync;
mod syscalls;
mod time;
mod uspace;

use core::ffi::c_char;

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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kernel_init(boot_info: *const BootInfo) -> ! {
    // Initialize basic hardware and the logging system.
    hal::init();

    let boot_info = unsafe { &*boot_info };

    print::print_header();
    print::print_boot_info(boot_info);

    kprintln!("Current time: {} ms", time::time());

    // Initialize the memory allocator.
    if let Err(_e) = mem::init_memory(boot_info) {
        panic!("[Kernel] Error: failed to initialize memory allocator.");
    }

    // Initialize the services.
    if let Err(_e) = services::init_services() {
        panic!("[Kernel] Error: failed to initialize services.");
    }

    sched::enable_scheduler();

    loop {}
}

/// The panic handler.
#[cfg(all(not(test), not(doctest), not(doc), target_os = "none"))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Print the panic message.
    if let Some(location) = info.location() {
        kprintln!("[Kernel] Panic at {}:{}", location.file(), location.line());
    } else {
        kprintln!("[Kernel] Panic: {}", info);
    }
    kprintln!("[Kernel] Halting CPU...");

    hal::panic::panic_handler(info);
}
