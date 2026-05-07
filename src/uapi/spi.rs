use crate::drivers::spi::*;
use crate::error::Result;
use crate::hal;

pub fn open(compatible: &str, ordinal: usize, config: Config) -> Result<Device> {
    Device::open(compatible, ordinal, config)
}
