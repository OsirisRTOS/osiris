use core::fmt::{self, Write};

use hal::Machinelike;

pub struct Printer;

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        hal::Machine::print(s).map_err(|_| fmt::Error)?;
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
