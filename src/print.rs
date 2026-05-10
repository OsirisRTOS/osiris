use crate::hal::{self, Machinelike};
use core::fmt::{self, Write};

pub struct Printer;

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        hal::Machine::print(s).map_err(|_| fmt::Error)?;
        Ok(())
    }
}

pub fn print(args: fmt::Arguments) {
    use core::fmt::Write;
    let mut printer = Printer;
    printer.write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {{
        use $crate::time;
        use $crate::print::print;
        // Print seconds and microseconds since boot.
        let (secs, frac) = time::to_secs(time::mono_now(), time::mono_freq() as u32, 6);
        print(format_args!("[{}.{:06}] ", secs, frac));
        print(format_args!($($arg)*));
    }};
}

#[macro_export]
macro_rules! kpcont {
    ($($arg:tt)*) => {{
        use $crate::print::print;
        print(format_args!($($arg)*));
    }};
}

pub fn print_header() {
    kprint!("****************************************************************\n");
    kprint!("  ___      _      _       ____ _____ ___  ____   \n");
    kprint!(" / _ \\ ___(_)_ __(_)___  |  _ \\_   _/ _ \\/ ___|  \n");
    kprint!("| | | / __| | '__| / __| | |_) || || | | \\___ \\  \n");
    kprint!("| |_| \\__ \\ | |  | \\__ \\ |  _ < | || |_| |___) | \n");
    kprint!(" \\___/|___/_|_|  |_|___/ |_| \\_\\|_| \\___/|____/  \n");
    kprint!("\n");
    kprint!("****************************************************************\n");
    kprint!("\n");
}
