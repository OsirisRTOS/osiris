use macros::service;

use crate::args_from_raw;

#[service(mem_size = 0, stack_size = 4096)]
pub struct Init {

}

impl Init {
    pub extern "C" fn main(argc: usize, argv: *const *const u8) {
        let args = args_from_raw!(argc, argv);

        loop {
            let _ = hal::hprintln!("Hello, from init");

            // Loop for 20ms to simulate some work.
            for _ in 0..20_000 {
                unsafe { core::arch::asm!("nop") }
            }
        }
    }
}

#[service(mem_size = 0, stack_size = 4096)]
pub struct Dummy {

}

impl Dummy {
    pub extern "C" fn main(argc: usize, argv: *const *const u8) {
        let args = args_from_raw!(argc, argv);

        loop {
            let _ = hal::hprintln!("Hello, from dummy");

            // Loop for 20ms to simulate some work.
            for _ in 0..20_000 {
                unsafe { core::arch::asm!("nop") }
            }
        }
    }
}

#[service(mem_size = 0, stack_size = 4096)]
pub struct Dummy2 {

}

impl Dummy2 {
    pub extern "C" fn main(argc: usize, argv: *const *const u8) {
        let args = args_from_raw!(argc, argv);

        loop {
            let _ = hal::hprintln!("Hello, from dummy2");

            // Loop for 20ms to simulate some work.
            for _ in 0..20_000 {
                unsafe { core::arch::asm!("nop") }
            }
        }
    }
}

