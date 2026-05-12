use core::cell::Cell;
use core::ffi::c_void;
use core::marker::PhantomData;

use hal_api::{PosixError, Result, ok_or_err};

use super::bindings;
use super::device_tree;

pub struct Bus {
    handle: *mut c_void,
    _no_sync: PhantomData<Cell<()>>,
}

unsafe impl Send for Bus {}
unsafe impl Sync for Bus {}

pub struct Device {
    cfg: bindings::i2c_device_cfg_t,
    handle: *mut c_void,
    _no_sync: PhantomData<Cell<()>>,
}

fn cfg_from_bus(bus: &device_tree::I2cBusRegistryEntry) -> bindings::i2c_bus_cfg_t {
    bindings::i2c_bus_cfg_t {
        instance: bus.instance,
        hz: bus.hz,
        timingr: bus.timingr,
        scl: bindings::i2c_pin_cfg_t {
            port: bus.scl.port,
            pin: bus.scl.line,
            af: bus.scl.af,
            reserved: 0,
        },
        sda: bindings::i2c_pin_cfg_t {
            port: bus.sda.port,
            pin: bus.sda.line,
            af: bus.sda.af,
            reserved: 0,
        },
    }
}

fn cfg_from_dev(
    dev: &device_tree::I2cDeviceRegistryEntry,
) -> Result<bindings::i2c_device_cfg_t> {
    let enable_pin = dev.enable.first().copied();

    Ok(bindings::i2c_device_cfg_t {
        instance: dev.bus_instance,
        address: dev.address,
        reserved: 0,
        enable: bindings::i2c_gpio_cfg_t {
            port: enable_pin.map(|pin| pin.port).unwrap_or(0),
            pin: enable_pin.map(|pin| pin.line).unwrap_or(0),
            active_low: enable_pin.map(|pin| pin.active_low).unwrap_or(0),
            reserved: 0,
        },
    })
}

pub fn init(cfg: &'static device_tree::I2cBusRegistryEntry) -> Result<Bus> {
    let cfg = cfg_from_bus(cfg);
    let handle = unsafe { bindings::i2c_init(&cfg as *const bindings::i2c_bus_cfg_t) };

    if handle.is_null() {
        Err(PosixError::EINVAL)
    } else {
        Ok(Bus {
            handle,
            _no_sync: PhantomData,
        })
    }
}

pub fn recover_bus(dev: &Device) -> Result<()> {
    let rc = unsafe { bindings::i2c_recover_bus(dev.handle) };
    ok_or_err(rc, ())
}

pub fn bus_recovery_needed(dev: &Device) -> Result<bool> {
    let rc = unsafe { bindings::i2c_bus_recovery_needed(dev.handle) };
    match rc {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(PosixError::from_errno(-other)),
    }
}

pub fn write(dev: &Device, tx: &[u8], timeout: u16) -> Result<()> {
    if tx.is_empty() {
        return Err(PosixError::EINVAL);
    }

    let mut transfer = bindings::i2c_transfer {
        tx: tx.as_ptr(),
        rx: core::ptr::null_mut(),
        tx_len: tx.len() as i32,
        rx_len: 0,
        timeout,
    };

    let rc = unsafe {
        bindings::i2c_write(
            dev.handle,
            &dev.cfg as *const bindings::i2c_device_cfg_t,
            &mut transfer as *mut bindings::i2c_transfer,
        )
    };
    ok_or_err(rc, ())
}

pub fn read(dev: &Device, rx: &mut [u8], timeout: u16) -> Result<()> {
    if rx.is_empty() {
        return Err(PosixError::EINVAL);
    }

    let mut transfer = bindings::i2c_transfer {
        tx: core::ptr::null(),
        rx: rx.as_mut_ptr(),
        tx_len: 0,
        rx_len: rx.len() as i32,
        timeout,
    };

    let rc = unsafe {
        bindings::i2c_read(
            dev.handle,
            &dev.cfg as *const bindings::i2c_device_cfg_t,
            &mut transfer as *mut bindings::i2c_transfer,
        )
    };
    ok_or_err(rc, ())
}

pub fn write_read(dev: &Device, tx: &[u8], rx: &mut [u8], timeout: u16) -> Result<()> {
    if tx.is_empty() || rx.is_empty() {
        return Err(PosixError::EINVAL);
    }

    let transfer = bindings::i2c_transfer {
        tx: tx.as_ptr(),
        rx: rx.as_mut_ptr(),
        tx_len: tx.len() as i32,
        rx_len: rx.len() as i32,
        timeout,
    };

    let rc = unsafe {
        bindings::i2c_write_read(
            dev.handle,
            &dev.cfg as *const bindings::i2c_device_cfg_t,
            &transfer as *const bindings::i2c_transfer,
        )
    };
    ok_or_err(rc, ())
}

pub fn deinit(bus: &Bus) -> Result<()> {
    let rc = unsafe { bindings::i2c_deinit(bus.handle) };
    ok_or_err(rc, ())
}

pub fn init_device(
    bus: &Bus,
    cfg: &'static device_tree::I2cDeviceRegistryEntry,
) -> Result<Device> {
    let cfg = cfg_from_dev(cfg)?;
    let rc = unsafe { bindings::i2c_init_device(&cfg) };

    ok_or_err(rc, Device {
        cfg,
        handle: bus.handle,
        _no_sync: PhantomData,
    })
}

pub fn deinit_device(dev: &Device) {
    unsafe { bindings::i2c_deinit_device(&dev.cfg) };
}
