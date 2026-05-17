use hal_api::PosixError;

use crate::{drivers::clock, time};

pub fn mono_now() -> u64 {
    time::mono_now()
}

pub fn mono_freq() -> u64 {
    time::mono_freq()
}

pub fn tick() -> u64 {
    time::tick()
}

pub fn walltime() -> Result<u64, PosixError> {
    clock::walltime()
}

pub fn set_walltime(time: u64) -> Result<(), PosixError> {
    clock::set_walltime(time)
}
