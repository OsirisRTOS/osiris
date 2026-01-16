#![no_std]
#![no_main]

#[unsafe(no_mangle)]
extern "C" fn main() {
    osiris::syscall_print(0, "Hello World!".as_bytes().as_ptr(), 12);
}

#[cfg(freestanding)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
