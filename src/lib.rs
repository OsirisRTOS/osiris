//! This is the default kernel of the osiris operating system.
//! The kernel is organized as a microkernel.

#![cfg_attr(freestanding, no_std)]

#[macro_use]
mod macros;
#[macro_use]
mod utils;
mod faults;
mod mem;
mod types;
mod idle;

pub mod print;
pub mod sched;
pub mod sync;
pub mod syscalls;
pub mod time;
pub mod uspace;

use hal::Machinelike;
use interface::BootInfo;
include!(concat!(env!("OUT_DIR"), "/syscalls_export.rs"));
include!(concat!(env!("OUT_DIR"), "/device_tree.rs"));

/// The kernel initialization function.
///
/// `boot_info` - The boot information.
///
/// # Safety
///
/// This function must be called only once during the kernel startup.
/// The `boot_info` pointer must be valid and point to a properly initialized `BootInfo` structure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kernel_init(boot_info: *const BootInfo) -> ! {
    hal::asm::disable_interrupts!();
    // Initialize basic hardware and the logging system.
    hal::Machine::init();
    hal::Machine::bench_start();

    if boot_info.is_null() || !boot_info.is_aligned() {
        panic!("[Kernel] Error: boot_info pointer is null or unaligned.");
    }

    // Safety: We trust the bootloader to provide a valid boot_info structure.
    let boot_info = unsafe { &*boot_info };

    print::print_header();

    // Initialize the memory allocator.
    let kaddr_space = mem::init_memory(boot_info);

    sched::init(kaddr_space);
    idle::init();

    let (cyc, ns) = hal::Machine::bench_end();
    kprintln!(
        "[Osiris] Kernel init took {} cycles taking {} ms",
        cyc,
        ns as u32 / 1000000
    );

    // Start the init application.
    if let Err(e) = uspace::init_app(boot_info) {
        panic!("[Kernel] Error: failed to start init application. Error: {e:?}");
    }

    hal::asm::enable_interrupts!();

    loop {
        hal::asm::nop!();
    }
}
