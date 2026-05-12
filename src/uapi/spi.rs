use crate::drivers::spi::*;
use crate::error::Result;

pub fn open(compatible: &str, ordinal: usize, config: Config) -> Result<Device> {
    Device::open(compatible, ordinal, config)
}
