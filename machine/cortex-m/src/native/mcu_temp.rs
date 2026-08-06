//! MCU internal junction-temperature sensor.

use hal_api::{PosixError, Result};

use super::bindings;

/// Read the MCU die temperature in degrees Celsius.
///
/// Returns `EIO` if ADC init or conversion failed.
pub fn read() -> Result<f32> {
    let millidegc = unsafe { bindings::hal_mcu_temp_millidegc() };
    if millidegc == i32::MIN {
        return Err(PosixError::EIO);
    }
    Ok(millidegc as f32 / 1000.0)
}
