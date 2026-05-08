//! System-level controls (reset, shutdown future). Top-level userspace
//! re-export of `hal::system`.

use crate::hal;

/// Hard-reset the MCU. Does not return.
pub fn reset() -> ! {
    hal::system::reset()
}
