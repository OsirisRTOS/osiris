use hal_api::{PosixError, Result};

use super::device_tree;

pub struct Bus;
pub struct Device;

pub fn init(_cfg: &'static device_tree::SpiBusRegistryEntry) -> Result<Bus> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn deinit(_bus: &Bus) -> Result<()> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn init_device(
    _bus: &Bus,
    _cfg: &'static device_tree::SpiDeviceRegistryEntry,
    _freq: Option<u32>,
) -> Result<Device> {
    Err(PosixError::EINVAL)
}

pub fn deinit_device(_dev: &Device) {}

pub fn transfer_words<W>(_dev: &Device, _tx: &[W], _rx: &mut [W]) -> Result<()> {
    Err(PosixError::EOPNOTSUPP)
}
