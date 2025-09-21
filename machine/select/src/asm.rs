#[cfg(all(not(test), target_arch = "arm"))]
pub use hal_arm::asm::*;

#[cfg(target_arch = "x86_64")]
pub use hal_testing::asm::*;
