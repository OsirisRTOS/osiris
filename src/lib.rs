//! This is the default kernel of the osiris operating system.
//! The kernel is organized as a microkernel.

#![cfg_attr(freestanding, no_std)]

#[macro_use]
mod macros;
#[macro_use]
mod error;
mod faults;
mod mem;
mod types;
mod idle;
mod uspace;
mod print;

mod sched;
mod sync;
mod syscalls;
mod time;

pub mod uapi;

use hal::Machinelike;
include!(concat!(env!("OUT_DIR"), "/device_tree.rs"));

pub use hal;
pub use proc_macros::app_main;

/// The kernel initialization function.
///
/// # Safety
///
/// This function must be called only once during the kernel startup.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kernel_init() -> ! {
    // Initialize basic hardware and the logging system.
    hal::Machine::init();
    hal::Machine::bench_start();

    print::print_header();

    error!("Hello World!");

    // Initialize the memory allocator.
    let kaddr_space = mem::init_memory();
    kprintln!("Memory initialized.");

    sched::init(kaddr_space);
    kprintln!("Scheduler initialized.");

    idle::init();
    kprintln!("Idle thread initialized.");

    let (cyc, ns) = hal::Machine::bench_end();
    kprintln!(
        "Kernel init took {} cycles.", cyc
    );

    // Start the init application.
    uspace::init_app();
    
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
