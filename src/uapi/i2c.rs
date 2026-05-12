use crate::drivers::i2c::*;
use crate::error::Result;

pub fn open(compatible: &str, ordinal: usize) -> Result<Device> {
    Device::open(compatible, ordinal)
}
