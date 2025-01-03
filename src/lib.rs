#![no_std]

extern crate hal;
extern crate macros;

pub mod syscalls;
pub mod sched;

use core::arch::asm;
use syscalls::dummy::*;

#[no_mangle]
pub extern "C" fn kernel_init() {
    hal::hal_hw_init();

    hal::semih::write_debug(hal::cstr!("Hello, world!\n"));

    if let Err(_err) = hal::hprintln!("The magic number is {}!", 42) {
        hal::semih::write_debug(hal::cstr!("Failed to write to host."));
    }

    syscall!(SYSCALL_DUMMY_NUM, 75);

    panic!("End of kernel_init");
}

use hal::common::{syscall, types::SchedCtx};

/// cbindgen:ignore
/// cbindgen:no-export
#[no_mangle]
extern "C" fn sched_call(ctx_in: SchedCtx) -> SchedCtx {
    // For now the scheduler does not switch tasks, so we just return the context as is.
    ctx_in
}
