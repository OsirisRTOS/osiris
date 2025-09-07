//! This is the default kernel of the osiris operating system.
//! The kernel is organized as a microkernel.

#![cfg_attr(all(not(test), not(doctest), not(doc), not(kani)), no_std)]

#[macro_use]
pub mod macros;
#[macro_use]
pub mod utils;
pub mod faults;
pub mod mem;
pub mod print;
pub mod sched;
pub mod services;
pub mod sync;
pub mod syscalls;
pub mod time;
pub mod uspace;

use core::ffi::c_char;

use hal::Machinelike;

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
    hal::Machine::init();

    //hal::asm::disable_interrupts();

    hal::Machine::bench_start();

    let boot_info = unsafe { &*boot_info };

    print::print_header();
    print::print_boot_info(boot_info);

    // Initialize the memory allocator.
    if let Err(e) = mem::init_memory(boot_info) {
        panic!("[Kernel] Error: failed to initialize memory allocator. Error: {:?}", e);
    }

    // Initialize the services.
    if let Err(e) = services::init_services() {
        panic!("[Kernel] Error: failed to initialize services. Error: {:?}", e);
    }

    sched::enable_scheduler(false);

    let (cyc, ns) = hal::Machine::bench_end();
    kprintln!(
        "[Osiris] Init took {} cycles taking {} ms",
        cyc,
        ns / 1e6f32
    );

    sched::enable_scheduler(true);

    loop {
        hal::asm::nop!();
    }
}
