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
include!(concat!(env!("OUT_DIR"), "/syscalls_export.rs"));
include!(concat!(env!("OUT_DIR"), "/device_tree.rs"));

pub use hal;
pub use proc_macros::app_main;

/// The kernel initialization function.
///
/// `boot_info` - The boot information.
///
/// # Safety
///
/// This function must be called only once during the kernel startup.
/// The `boot_info` pointer must be valid and point to a properly initialized `BootInfo` structure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kernel_init() -> ! {
    // Initialize basic hardware and the logging system.
    hal::Machine::init();
    hal::Machine::bench_start();

    print::print_header();

    // Initialize the memory allocator.
    let kaddr_space = mem::init_memory();

    kprintln!("Memory initialized.");

    if let Err(e) = sched::init(kaddr_space) {
        panic!("failed to initialize scheduler. Error: {e:?}");
    }

    kprintln!("Scheduler initialized.");

    idle::init();

    kprintln!("Idle thread initialized.");

    let (cyc, ns) = hal::Machine::bench_end();
    kprintln!(
        "Kernel init took {} cycles.", cyc
    );

    // Start the init application.
    if let Err(e) = uspace::init_app() {
        panic!("failed to start init application. Error: {e:?}");
    }
    
    sched::enable();

    loop {}
}

pub fn panic(info: &core::panic::PanicInfo) -> ! {
    kprintln!("**************************** PANIC ****************************");
    kprintln!("");
    kprintln!("Message: {}", info.message());

    if let Some(location) = info.location() {
        kprintln!("Location: {}:{}", location.file(), location.line());
    }

    kprintln!("**************************** PANIC ****************************");

    hal::Machine::panic_handler(info);
}
