//! Macros for kernel development.

/// Creates a slice from the raw arguments.
#[macro_export]
macro_rules! args_from_raw {
    ($argc:expr, $argv:expr) => {{
        let argc = $argc;
        let argv = $argv;

        if argc == 0 || argv.is_null() {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(argv, argc) }
        }
    }};
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        use $crate::print::Printer;
        let mut printer = Printer;
        printer.write_fmt(format_args!($($arg)*)).unwrap();
    });
}

#[macro_export]
macro_rules! kprintln {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        use $crate::print::Printer;
        let mut printer = Printer;
        printer.write_fmt(format_args!($($arg)*)).unwrap();
        printer.write_str("\n").unwrap();
    });
}
