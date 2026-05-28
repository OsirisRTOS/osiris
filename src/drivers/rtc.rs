use hal_api::PosixError;

use crate::hal;
use crate::hal::Machinelike;

/// The monotonic clock is brought up by [hal::Machine::init()]
pub fn init() {
    if let Err(e) = hal::Machine::init_rtc() {
        kprintln!("failed to activate RTC: {}", e);
    }
}

pub fn rtc_backup_register(index: u8) -> u32 {
    hal::Machine::rtc_backup_register(index)
}

pub fn set_rtc_backup_register(index: u8, value: u32) {
    hal::Machine::set_rtc_backup_register(index, value)
}

pub fn walltime() -> Result<u64, PosixError> {
    hal::Machine::rtc()
}

pub fn set_walltime(time: u64) -> Result<(), PosixError> {
    hal::Machine::set_rtc(time)
}
