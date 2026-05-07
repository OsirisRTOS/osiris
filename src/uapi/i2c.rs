use crate::drivers::i2c::*;
use crate::error::Result;
use crate::hal;

pub fn open(compatible: &str, ordinal: usize) -> Result<Device> {
    Device::open(compatible, ordinal)
}
