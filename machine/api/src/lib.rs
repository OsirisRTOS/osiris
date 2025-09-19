#![cfg_attr(not(test), no_std)]

use core::{fmt::Display, ops::Range};

pub mod stack;

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub enum Error {
    #[default]
    Generic,
    OutOfMemory(usize),
    OutOfBoundsPtr(usize, Range<usize>),
}

pub enum Fault {
    Hard,
    MemManage,
    Bus,
    Usage,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::Generic => write!(f, "Generic"),
            Error::OutOfMemory(size) => write!(f, "Out of memory (requested {size} bytes)"),
            Error::OutOfBoundsPtr(ptr, range) => {
                write!(f, "Pointer {:p} out of bounds (expected in {:p}..{:p})", *ptr as *const u8, range.start as *const u8, range.end as *const u8)
            }
        }
    }
}

pub type Result<T> = core::result::Result<T, Error>;

pub trait Machinelike {
    fn init();
    fn print(s: &str) -> Result<()>;

    fn bench_start();
    fn bench_end() -> (u32, f32);

    type ExcepBacktrace: Display;
    type ExcepStackFrame: Display;
    fn backtrace(initial_fp: *const usize, stack_ptr: *const usize) -> Self::ExcepBacktrace;
    fn stack_frame(stack_ptr: *const usize) -> Self::ExcepStackFrame;

    type FaultStatus: Display;
    fn get_fault_status(fault: Fault) -> Self::FaultStatus;

    fn panic_handler(info: &core::panic::PanicInfo) -> !;
}

pub trait Schedable {
    fn trigger_reschedule();
}