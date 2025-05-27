#![cfg_attr(not(test), no_std)]

mod bindings {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(unused)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub mod asm;
pub mod panic;
pub mod sched;

pub fn init() {
    unsafe { bindings::init_hal() };
    unsafe { bindings::init_debug_uart() };
}

pub fn print(s: &str) -> Result<(), ()> {
    asm::disable_interrupts();

    if (unsafe { bindings::write_debug_uart(s.as_ptr(), s.len() as i32) } != 0) {
        asm::enable_interrupts();
        Ok(())
    } else {
        asm::enable_interrupts();
        Err(())
    }
}
