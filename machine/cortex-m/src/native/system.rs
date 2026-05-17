//! System-level controls.

use super::bindings;

pub fn reset() -> ! {
    unsafe { bindings::system_reset() }
}
