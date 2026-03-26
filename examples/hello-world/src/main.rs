#![no_std]
#![no_main]

use osiris::app_main;

#[app_main]
fn main() {
    osiris::syscall_print(0, "Hello World!".as_bytes().as_ptr(), 12);
    loop {}
}
