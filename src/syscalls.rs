//! This module provides access to all the syscalls.

use core::ffi::{c_int, c_uint};

mod file;
mod sched;

// We need to import everything so that the macro is able to find the entry functions.
use file::*;
use sched::*;

#[unsafe(no_mangle)]
pub extern "C" fn handle_syscall(number: usize, args: *const c_uint) -> c_int {
    let number = number as u16;
    // All functions that are annotated with the #[syscall_handler(num = X)] macro are syscalls.
    // build.rs will generate a match statement that matches the syscall number to the function which is then included here.
    include!(concat!(env!("OUT_DIR"), "/syscall_match.in"))
}
