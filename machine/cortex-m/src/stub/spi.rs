use hal_api::{Error, Result};

use super::device_tree;

pub struct Bus;
pub struct Device;

pub fn init(_cfg: &'static device_tree::SpiBusRegistryEntry) -> Result<Bus> {
    Err(Error::Generic)
}

pub fn deinit(_bus: &Bus) -> Result<()> {
    Err(Error::Generic)
}

pub fn init_device(
    _bus: &Bus,
    _cfg: &'static device_tree::SpiDeviceRegistryEntry,
    _freq: Option<u32>,
) -> Result<Device> {
    Err(Error::Generic)
}

pub fn deinit_device(_dev: &Device) {}

pub fn transfer_words<W>(_dev: &Device, _tx: &[W], _rx: &mut [W]) -> Result<()> {
    Err(Error::Generic)
}
