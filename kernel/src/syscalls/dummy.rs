use core::ffi::c_void;

use macros::syscall_handler;

#[unsafe(no_mangle)]
#[syscall_handler(args = 1, num = 1)]
extern "C" fn syscall_dummy(svc_args: *const c_void) {
    let num = unsafe { *(svc_args as *const i32) };
    if let Err(_err) = hal::hprintln!("amogus {}!", num) {
        hal::semih::write_debug(hal::cstr!("Failed to write to host."));
    }
}
