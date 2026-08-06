//! MCU internal junction-temperature sensor (host stub).

use hal_api::Result;

/// Return a fixed dummy temperature on the host build.
pub fn read() -> Result<f32> {
    Ok(25.0)
}
