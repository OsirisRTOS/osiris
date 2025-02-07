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

            for _ in 0..1_000 {
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

            for _ in 0..1_000 {
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

            for _ in 0..1_000 {
                unsafe { core::arch::asm!("nop") }
            }
        }
    }
}

