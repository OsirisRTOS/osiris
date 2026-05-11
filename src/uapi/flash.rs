use crate::error::Result;

pub use crate::drivers::flash::raw;
pub use crate::drivers::flash::{Config, FlashAddress, FlashOffset, FlashPageStart, Region};

pub fn open(compatible: &str, ordinal: usize, config: Config) -> Result<Region> {
    Region::open(compatible, ordinal, config)
}

pub fn open_by_label(label: &str, config: Config) -> Result<Region> {
    Region::open_by_label(label, config)
}

pub fn open_by_address(addr: impl Into<FlashAddress>, config: Config) -> Result<Region> {
    Region::open_by_address(addr, config)
}
