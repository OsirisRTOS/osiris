use core::fmt::{self, Write};

use crate::hal;
use hal::Machinelike;

pub fn print(args: fmt::Arguments) {
    let mut printer = Printer;
    printer.write_fmt(args).unwrap();
}

pub struct Printer;

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        hal::Machine::print(s).map_err(|_| fmt::Error)?;
        Ok(())
    }
}
