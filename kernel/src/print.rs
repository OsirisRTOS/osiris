use core::ffi::CStr;
use core::fmt::{self, Write};

use crate::kprintln;
use crate::hal;

pub struct Printer;

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        hal::print(s).map_err(|_| fmt::Error)?;
        Ok(())
    }
}

pub fn print_header() {
    kprintln!("****************************************************************");
    kprintln!("  ___      _      _       ____ _____ ___  ____   ");
    kprintln!(" / _ \\ ___(_)_ __(_)___  |  _ \\_   _/ _ \\/ ___|  ");
    kprintln!("| | | / __| | '__| / __| | |_) || || | | \\___ \\  ");
    kprintln!("| |_| \\__ \\ | |  | \\__ \\ |  _ < | || |_| |___) | ");
    kprintln!(" \\___/|___/_|_|  |_|___/ |_| \\_\\|_| \\___/|____/  ");
    kprintln!("");
    kprintln!("****************************************************************");
    kprintln!("");
}

pub fn print_boot_info(boot_info: &crate::BootInfo) {
    kprintln!("[Osiris] Booting kernel...");

    let implementer = unsafe { CStr::from_ptr(boot_info.implementer) };
    let variant = unsafe { CStr::from_ptr(boot_info.variant) };

    if let (Ok(implementer), Ok(variant)) = (implementer.to_str(), variant.to_str()) {
        kprintln!("[Osiris] Detected Processor:");
        kprintln!("[Osiris] Implementer     : {}", implementer);
        kprintln!("[Osiris] Name            : {}", variant);
        kprintln!("");
    } else {
        kprintln!("[Osiris] Error: failed to read processor information.");
        kprintln!("");
    }
}
