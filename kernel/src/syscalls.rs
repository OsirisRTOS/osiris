//! This module provides access to all the syscalls.

use core::ffi::{c_int, c_uint};

mod file;
mod tasks;

// We need to import everything so that the macro is able to find the entry functions.
use file::*;
use tasks::*;

#[unsafe(no_mangle)]
pub extern "C" fn handle_syscall(number: usize, args: *const c_uint) -> c_int
{
    // All functions that are annotated with the #[syscall_handler(num = X)] macro are syscalls.
    // build.rs will generate a match statement that matches the syscall number to the function which is then included here.
    include!(concat!(env!("OUT_DIR"), "/syscall_dispatcher.in"))
}
