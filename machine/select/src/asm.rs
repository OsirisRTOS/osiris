#[cfg(all(not(test), target_arch = "arm"))]
pub use hal_arm::asm::*;