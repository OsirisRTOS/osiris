#![cfg_attr(not(test), no_std)]

pub mod asm;

pub use hal_api::*;

#[cfg(all(freestanding, target_arch = "arm"))]
pub mod arm;
#[cfg(all(freestanding, target_arch = "arm"))]
pub use arm::*;

#[cfg(not(freestanding))]
pub mod testing;
#[cfg(not(freestanding))]
pub use testing::*;
