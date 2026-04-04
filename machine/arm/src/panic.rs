#![deny(warnings)]
#![allow(missing_docs)]

use core::panic::PanicInfo;

use crate::asm;

pub fn panic_handler(_info: &PanicInfo) -> ! {
    asm::disable_irq_save();
    #[allow(clippy::empty_loop)]
    loop {}
}
