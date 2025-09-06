#![cfg_attr(not(test), no_std)]

pub mod asm;

pub use hal_api::*;

#[cfg(all(not(test), target_arch = "arm"))]
pub type Machine = hal_arm::ArmMachine;
#[cfg(all(not(test), target_arch = "arm"))]
pub type Stack = hal_arm::sched::ArmStack;

#[cfg(test)]
pub type Machine = hal_testing::TestingMachine;