pub use crate::drivers::uart::*;

pub fn open(compatible: &str, ordinal: usize, cfg: Config) -> Result<Device, Error> {
    Device::open(compatible, ordinal, cfg)
}
