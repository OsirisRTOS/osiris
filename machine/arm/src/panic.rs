#![deny(warnings)]
#![allow(missing_docs)]

use core::panic::PanicInfo;

pub fn panic_handler(_info: &PanicInfo) -> ! {
    loop {
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}
