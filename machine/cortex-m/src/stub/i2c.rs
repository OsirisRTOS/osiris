use hal_api::{PosixError, Result};

use super::device_tree;

pub struct Bus;
pub struct Device;

pub fn init(_cfg: &'static device_tree::I2cBusRegistryEntry) -> Result<Bus> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn deinit(_bus: &Bus) -> Result<()> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn init_device(
    _bus: &Bus,
    _cfg: &'static device_tree::I2cDeviceRegistryEntry,
) -> Result<Device> {
    Err(PosixError::EINVAL)
}

pub fn deinit_device(_dev: &Device) {}

pub fn write(_dev: &Device, _tx: &[u8]) -> Result<()> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn read(_dev: &Device, _rx: &mut [u8]) -> Result<()> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn write_read(_dev: &Device, _tx: &[u8], _rx: &mut [u8]) -> Result<()> {
    Err(PosixError::EOPNOTSUPP)
}
