//! This module provides the core userspace services of the microkernel.

use macros::service;

/// The init service.
#[service(mem_size = 8192)]
pub struct Init {}

impl Init {
    /// The entry point of the init service. TODO: Currently, this is a dummy implementation.
    pub extern "C" fn main() {
        loop {
            hal::asm::syscall!(0, 0, "Hello from Init!".as_bytes().as_ptr(), 16);

            for _ in 0..1_000 {
                unsafe { core::arch::asm!("nop") }
            }
        }
    }
}

/// A dummy service. TODO: Currently, this is a dummy implementation.
#[service(mem_size = 8192)]
pub struct Dummy {}

impl Dummy {
    /// The entry point of the dummy service. TODO: Currently, this is a dummy implementation.
    pub extern "C" fn main() {
        loop {
            // The first argument is a pointer to a string.
            hal::asm::syscall!(0, 0, "Hello from Dummy!".as_bytes().as_ptr(), 17);

            for _ in 0..1_000 {
                unsafe { core::arch::asm!("nop") }
            }
        }
    }
}

/// A second dummy service. TODO: Currently, this is a dummy implementation.
#[service(mem_size = 8192)]
pub struct Dummy2 {}

impl Dummy2 {
    /// The entry point of the second dummy service. TODO: Currently, this is a dummy implementation.
    pub extern "C" fn main() {
        loop {
            hal::asm::syscall!(0, 0, "Hello from Dummy2!".as_bytes().as_ptr(), 18);

            for _ in 0..1_000 {
                unsafe { core::arch::asm!("nop") }
            }
        }
    }
}
