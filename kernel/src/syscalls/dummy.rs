use core::ffi::c_void;

use macros::syscall_handler;

use crate::kprintln;

#[unsafe(no_mangle)]
#[syscall_handler(args = 1, num = 1)]
extern "C" fn syscall_dummy(svc_args: *const c_void) {
    let num = unsafe { *(svc_args as *const i32) };
    kprintln!("amogus {}!", num);
}
