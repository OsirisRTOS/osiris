#![no_std]

mod task;

extern crate hal;

#[no_mangle]
pub extern "C" fn kernel_init() {
    hal::hal_hw_init();

    hal::semih::write_debug(hal::cstr!("Hello, world!\n"));

    if let Err(err) = hal::hprintln!("The magic number is {}!", 42) {
        hal::semih::write_debug(hal::cstr!("Failed to write to host."));
    }
    loop {}
}
