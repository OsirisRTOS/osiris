#![deny(warnings)]
#![allow(missing_docs)]

use core::panic::PanicInfo;

use crate::asm;

pub fn panic_handler(_info: &PanicInfo) -> ! {
    asm::disable_interrupts();

    loop {
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }
}
