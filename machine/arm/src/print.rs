use core::fmt::{self, Write};

use hal_api::Machinelike;

#[allow(unused_macros)]
macro_rules! print {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        use $crate::print::Printer;
        let mut printer = Printer;
        printer.write_fmt(format_args!($($arg)*)).unwrap();
    });
}

#[allow(unused_imports)]
pub(crate) use print;

#[allow(unused_macros)]
macro_rules! println {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        use $crate::print::Printer;
        let mut printer = Printer;
        printer.write_fmt(format_args!($($arg)*)).unwrap();
        printer.write_str("\n").unwrap();
    });
}

#[allow(unused_imports)]
pub(crate) use println;

pub struct Printer;

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        crate::ArmMachine::print(s).map_err(|_| fmt::Error)?;
        Ok(())
    }
}