use core::fmt::{self, Write};

use hal::Machinelike;

#[macro_export]
macro_rules! uprintln {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        use osiris::uapi::print::Printer;

        let mut printer = Printer;
        printer.write_fmt(format_args!($($arg)*)).unwrap();
        printer.write_str("\n").unwrap();
    });
}

pub struct Printer;

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        hal::Machine::print(s).map_err(|_| fmt::Error)?;
        Ok(())
    }
}