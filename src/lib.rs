//! This is the default kernel of the osiris operating system.
//! The kernel is organized as a microkernel.

#![cfg_attr(freestanding, no_std)]

#[macro_use]
mod error;
mod faults;
mod idle;
mod irq;
mod mem;

#[macro_use]
mod print;
mod types;
mod uspace;

mod sched;
mod sync;
mod syscalls;
mod time;

// Public, for now.
pub mod drivers;
pub mod uapi;

pub use hal_cortex_m::*;
// Add new hals here. No cfg needed.

pub use hal::Machinelike;
pub use hal_api::error::*;
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

    // Initialize the memory allocator.
    let kaddr_space = mem::init_memory();
    kprint!("Memory initialized.\n");

    drivers::init();
    kprint!("Drivers initialized.\n");

    sched::init(kaddr_space);
    kprint!("Scheduler initialized.\n");

    idle::init();
    kprint!("Idle thread initialized.\n");

    let (cyc, _ns) = hal::Machine::bench_end();
    kprint!("Kernel init took {} cycles.\n", cyc);

    // Start the init application.
    uspace::init_app();

    sched::enable();

    loop {}
}

pub fn panic(info: &core::panic::PanicInfo) -> ! {
    kprint!("**************************** PANIC ****************************\n");
    kprint!("\n");
    kprint!("Message: {}\n", info.message());

    if let Some(location) = info.location() {
        kprint!("Location: {}:{}\n", location.file(), location.line());
    }

    kprint!("**************************** PANIC ****************************\n");

    hal::Machine::panic_handler(info);
}
