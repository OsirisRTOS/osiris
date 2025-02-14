//! This module provides the core userspace services of the microkernel.

use crate::args_from_raw;
use macros::service;

/// The init service.
#[service(mem_size = 0, stack_size = 4096)]
pub struct Init {}

impl Init {

    /// The entry point of the init service. TODO: Currently, this is a dummy implementation.
    pub extern "C" fn main(argc: usize, argv: *const *const u8) {
        let args = args_from_raw!(argc, argv);

        loop {
            let _ = hal::hprintln!("Hello, from init");

            for _ in 0..1_000 {
                unsafe { core::arch::asm!("nop") }
            }
        }
    }
}

/// A dummy service. TODO: Currently, this is a dummy implementation.
#[service(mem_size = 0, stack_size = 4096)]
pub struct Dummy {}

impl Dummy {

    /// The entry point of the dummy service. TODO: Currently, this is a dummy implementation.
    pub extern "C" fn main(argc: usize, argv: *const *const u8) {
        let args = args_from_raw!(argc, argv);

        loop {
            let _ = hal::hprintln!("Hello, from dummy");

            for _ in 0..1_000 {
                unsafe { core::arch::asm!("nop") }
            }
        }
    }
}

/// A second dummy service. TODO: Currently, this is a dummy implementation.
#[service(mem_size = 0, stack_size = 4096)]
pub struct Dummy2 {}

impl Dummy2 {

    /// The entry point of the second dummy service. TODO: Currently, this is a dummy implementation.
    pub extern "C" fn main(argc: usize, argv: *const *const u8) {
        let args = args_from_raw!(argc, argv);

        loop {
            let _ = hal::hprintln!("Hello, from dummy2");

            for _ in 0..1_000 {
                unsafe { core::arch::asm!("nop") }
            }
        }
    }
}
