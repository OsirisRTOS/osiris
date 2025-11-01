//! This is the default kernel of the osiris operating system.
//! The kernel is organized as a microkernel.

#![cfg_attr(freestanding, no_std)]

#[macro_use]
mod macros;
#[macro_use]
mod utils;
mod faults;
mod mem;
pub mod print;
mod sched;
mod services;
mod sync;
mod syscalls;
mod time;
mod uspace;

use hal::Machinelike;
use interface::BootInfo;

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
        panic!("[Kernel] Error: failed to initialize memory allocator. Error: {e:?}");
    }

    // Initialize the services.
    if let Err(e) = services::init_services() {
        panic!("[Kernel] Error: failed to initialize services. Error: {e:?}");
    }

    sched::enable_scheduler(false);

    let (cyc, ns) = hal::Machine::bench_end();
    kprintln!(
        "[Osiris] Kernel init took {} cycles taking {} ms",
        cyc,
        ns / 1e6f32
    );

    sched::enable_scheduler(true);

    // Start the init application.
    if let Err(e) = uspace::init_app(boot_info) {
        panic!("[Kernel] Error: failed to start init application. Error: {e:?}");
    }

    loop {
        hal::asm::nop!();
    }
}
