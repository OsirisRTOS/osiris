//! This is the default kernel of the osiris operating system.
//! The kernel is organized as a microkernel.

#![cfg_attr(freestanding, no_std)]

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
pub mod cmdline;

use core::ffi::c_char;

use hal::Machinelike;

use crate::cmdline::Args;

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
    /// The command line arguments.
    pub args: Args,
}

/// The kernel initialization function.
///
/// `boot_info` - The boot information.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kernel_init(boot_info: *const BootInfo) -> ! {
    // Initialize basic hardware and the logging system.
    hal::Machine::init();
    hal::Machine::bench_start();

    if !boot_info.is_null() || !boot_info.is_aligned() {
        panic!("[Kernel] Error: boot_info pointer is null or unaligned.");
    }

    // Safety: We trust the bootloader to provide a valid boot_info structure.
    let boot_info = unsafe { &*boot_info };

    print::print_header();
    print::print_boot_info(boot_info);

    // Initialize the memory allocator.
    if let Err(e) = mem::init_memory(boot_info) {
        panic!(
            "[Kernel] Error: failed to initialize memory allocator. Error: {e:?}"
        );
    }

    // Initialize the services.
    if let Err(e) = services::init_services() {
        panic!(
            "[Kernel] Error: failed to initialize services. Error: {e:?}"
        );
    }

    sched::enable_scheduler(false);

    let (cyc, ns) = hal::Machine::bench_end();
    kprintln!(
        "[Osiris] Kernel init took {} cycles taking {} ms",
        cyc,
        ns / 1e6f32
    );

    // Start the init application.
    if let Err(e) = uspace::init_app(boot_info) {
        panic!("[Kernel] Error: failed to start init application. Error: {e:?}");
    }

    loop {
        hal::asm::nop!();
    }
}
