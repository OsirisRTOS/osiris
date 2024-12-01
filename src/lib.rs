#![no_std]

use core::ffi::CStr;

#[no_mangle]
pub extern "C" fn kernel_init() {
    hal::hal_hw_init();
    hal::semih::write(hal::cstr!("Hello, world!\n"));
    loop {}
}
