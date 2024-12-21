#![no_std]

extern crate hal;
extern crate macros;

use core::{arch::asm, ffi::c_void};

pub mod task;

#[no_mangle]
pub extern "C" fn kernel_init() {
    hal::hal_hw_init();

    hal::semih::write_debug(hal::cstr!("Hello, world!\n"));

    if let Err(err) = hal::hprintln!("The magic number is {}!", 42) {
        hal::semih::write_debug(hal::cstr!("Failed to write to host."));
    }

    unsafe {
        asm!("mov r0, 75", "svc 1");
    }

    panic!("End of kernel_init");

    loop {}
}

use hal::common::types::SchedCtx;
use macros::syscall_handler;

/// cbindgen:ignore
/// cbindgen:no-export
#[no_mangle]
extern "C" fn sched_call(ctx_in: SchedCtx) -> SchedCtx {
    // For now the scheduler does not switch tasks, so we just return the context as is.
    ctx_in
}

/// cbindgen:ignore
/// cbindgen:no-export
#[no_mangle]
#[syscall_handler(args = 1, num = 1)]
extern "C" fn among(svc_args: *const c_void) {
    let num = unsafe { *(svc_args as *const i32) };
    if let Err(err) = hal::hprintln!("amogus {}!", num) {
        hal::semih::write_debug(hal::cstr!("Failed to write to host."));
    }
}
