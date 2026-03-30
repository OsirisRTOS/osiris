//! Macros for kernel development.
use defmt_rtt as _;


#[macro_export]
macro_rules! debug {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        #[cfg(feature = "defmt")]
        defmt::debug!($fmt $(, $arg)*);
    };
}

#[macro_export]
macro_rules! trace {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        #[cfg(feature = "defmt")]
        defmt::trace!($fmt $(, $arg)*);
    };
}

#[macro_export]
macro_rules! info {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        #[cfg(feature = "defmt")]
        defmt::info!($fmt $(, $arg)*);
    };
}

#[macro_export]
macro_rules! error {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        #[cfg(feature = "defmt")]
        defmt::error!($fmt $(, $arg)*);
    };
}

#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => ({
    
    });
}

#[macro_export]
macro_rules! kprintln {
    ($($arg:tt)*) => ({
        use core::fmt::Write;
        use $crate::print::Printer;

        let mut printer = Printer;
        const MICROS_PER_SEC: u64 = 1000000;
        let hz = $crate::time::mono_freq();
        let secs = $crate::time::mono_now() / hz;
        let rem = $crate::time::mono_now() % hz;
        let frac = (rem * MICROS_PER_SEC) / hz;

        write!(&mut printer, "[{}.{:06}] ", secs, frac).unwrap();
        printer.write_fmt(format_args!($($arg)*)).unwrap();
        printer.write_str("\n").unwrap();
    });
}
