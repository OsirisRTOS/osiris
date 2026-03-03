#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU32, Ordering};

// Global variable to verify RWPI r9 setup is working correctly.
// If r9 is not set to the user's static base before entry, this will read garbage or fault.
static COUNTER: AtomicU32 = AtomicU32::new(42);

#[unsafe(no_mangle)]
extern "C" fn main() {
    osiris::syscall_print(0, "Loading counter...".as_bytes().as_ptr(), 18);
    let val = COUNTER.load(Ordering::Relaxed);
    if val == 42 {
        osiris::syscall_print(0, b"[hello-world] COUNTER=42 (globals OK)\n".as_ptr(), 38);
    } else {
        osiris::syscall_print(
            0,
            b"[hello-world] COUNTER=BAD (globals BROKEN)\n".as_ptr(),
            43,
        );
    }
    osiris::syscall_print(0, "Hello World!".as_bytes().as_ptr(), 12);
}

#[cfg(freestanding)]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
