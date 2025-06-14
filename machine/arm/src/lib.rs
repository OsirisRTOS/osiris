#![cfg_attr(all(not(test), not(feature = "host")), no_std)]

pub mod asm;
pub mod debug;
pub mod excep;
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
    unsafe {
        bindings::dwt_init();
    }
}

#[cfg(feature = "host")]
pub fn init() { /*We do not need to init anything yet. */
}

#[cfg(not(feature = "host"))]
pub fn bench_start() {
    unsafe {
        bindings::dwt_reset();
    }
}

#[cfg(feature = "host")]
pub fn bench_start() {}

#[cfg(not(feature = "host"))]
pub fn bench_end() -> (u32, f32) {
    let cycles = unsafe { bindings::dwt_read() as u32 };
    let ns = unsafe { bindings::dwt_cycles_to_ns(cycles as i32) };

    (cycles, ns)
}

#[cfg(feature = "host")]
pub fn bench_end() -> (u32, f32) {
    (0, 0.0)
}

#[cfg(not(feature = "host"))]
pub fn print(s: &str) -> Result<(), ()> {
    use crate::asm;
    asm::disable_interrupts();

    if (unsafe { bindings::write_debug_uart(s.as_ptr() as *const u8, s.len() as i32) } != 0) {
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
