use core::cell::Cell;
use core::ffi::c_void;
use core::marker::PhantomData;

use hal_api::{Error, Result};

use super::bindings;
use super::device_tree;

#[derive(Clone, Copy)]
pub struct Bus {
    instance: *mut c_void,
    _no_sync: PhantomData<Cell<()>>,
}

unsafe impl Send for Bus {}
unsafe impl Sync for Bus {}

pub struct Device {
    cfg: bindings::spi_device_cfg_t,
    instance: *mut c_void,
    _no_sync: PhantomData<Cell<()>>,
}

fn cfg_from_bus(bus: &device_tree::SpiBusRegistryEntry) -> bindings::spi_bus_cfg_t {
    bindings::spi_bus_cfg_t {
        instance: bus.instance,
        sck: bindings::spi_pin_cfg_t {
            port: bus.sck.port,
            pin: bus.sck.line,
            af: bus.sck.af,
            reserved: 0,
        },
        miso: bindings::spi_pin_cfg_t {
            port: bus.miso.port,
            pin: bus.miso.line,
            af: bus.miso.af,
            reserved: 0,
        },
        mosi: bindings::spi_pin_cfg_t {
            port: bus.mosi.port,
            pin: bus.mosi.line,
            af: bus.mosi.af,
            reserved: 0,
        },
    }
}

fn cfg_from_dev(
    dev: &device_tree::SpiDeviceRegistryEntry,
    freq: Option<u32>,
) -> bindings::spi_device_cfg_t {
    let enable_pin = dev.enable.first().copied();

    bindings::spi_device_cfg_t {
        instance: dev.bus_instance,
        max_hz: freq.unwrap_or(dev.max_hz),
        cpol: dev.cpol,
        cpha: dev.cpha,
        bits_per_word: dev.bits_per_word,
        // TODO: Support LSB-first devices
        bit_order: 0,
        cs_setup_delay_us: dev.cs_setup_delay_us,
        cs_hold_delay_us: dev.cs_hold_delay_us,
        cs_inactive_delay_us: dev.cs_inactive_delay_us,
        cs: bindings::spi_cs_cfg_t {
            port: dev.cs.port,
            pin: dev.cs.line,
            active_low: dev.cs.active_low,
            reserved: 0,
        },
        enable: bindings::spi_cs_cfg_t {
            port: enable_pin.map(|pin| pin.port).unwrap_or(0),
            pin: enable_pin.map(|pin| pin.line).unwrap_or(0),
            active_low: enable_pin.map(|pin| pin.active_low).unwrap_or(0),
            reserved: 0,
        },
    }
}

pub fn init(cfg: &'static device_tree::SpiBusRegistryEntry) -> Result<Bus> {
    let cfg = cfg_from_bus(cfg);
    let instance = unsafe { bindings::spi_init(&cfg) };

    if instance.is_null() {
        Err(Error::Generic)
    } else {
        Ok(Bus {
            instance,
            _no_sync: PhantomData,
        })
    }
}

pub fn transfer_words<W>(dev: &Device, tx: &[W], rx: &mut [W]) -> Result<()> {
    if tx.len() != rx.len() || tx.is_empty() {
        return Err(Error::Generic);
    }

    let transfer = bindings::spi_transfer {
        tx_words: tx.as_ptr() as *const core::ffi::c_void,
        rx_words: rx.as_mut_ptr() as *mut core::ffi::c_void,
        word_count: tx.len() as i32,
    };

    let rc = unsafe {
        bindings::spi_transfer(
            dev.instance,
            &dev.cfg,
            &transfer as *const bindings::spi_transfer,
        )
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(Error::Generic)
    }
}

pub fn deinit(bus: &Bus) -> Result<()> {
    let rc = unsafe { bindings::spi_deinit(bus.instance) };
    if rc == 0 { Ok(()) } else { Err(Error::Generic) }
}

pub fn init_device(
    bus: &Bus,
    cfg: &'static device_tree::SpiDeviceRegistryEntry,
    freq: Option<u32>,
) -> Result<Device> {
    let cfg = cfg_from_dev(cfg, freq);
    let rc = unsafe { bindings::spi_init_device(&cfg) };
    if rc != 0 {
        return Err(Error::Generic);
    }

    Ok(Device {
        cfg,
        instance: bus.instance,
        _no_sync: PhantomData,
    })
}

pub fn deinit_device(dev: &Device) {
    unsafe { bindings::spi_deinit_device(&dev.cfg) };
}
