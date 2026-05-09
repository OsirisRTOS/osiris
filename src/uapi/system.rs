//! Userspace re-export of `hal::system`.

use crate::hal;

pub fn reset() -> ! {
    hal::system::reset()
}
