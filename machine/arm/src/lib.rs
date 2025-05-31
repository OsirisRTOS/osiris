#![cfg_attr(all(not(test), not(feature = "host")), no_std)]

pub mod asm;
pub mod panic;
pub mod sched;

mod bindings {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(unused)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[cfg(not(feature = "host"))]
pub fn init() {
    unsafe { bindings::init_hal() };
    unsafe { bindings::init_debug_uart() };
}

#[cfg(feature = "host")]
pub fn init() { /*We do not need to init anything yet. */ }

#[cfg(not(feature = "host"))]
pub fn print(s: &str) -> Result<(), ()> {
    use crate::asm;
    asm::disable_interrupts();

    if (unsafe { bindings::write_debug_uart(s.as_ptr(), s.len() as i32) } != 0) {
        asm::enable_interrupts();
        Ok(())
    } else {
        asm::enable_interrupts();
        Err(())
    }
}

#[cfg(feature = "host")]
pub fn print(s: &str) -> Result<(), ()> {
    // just use the normal std print implementation.
    println!("{}", s);
    return Ok(());
}
