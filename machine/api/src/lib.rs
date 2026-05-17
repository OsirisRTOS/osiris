#![cfg_attr(not(test), no_std)]

use core::fmt::Display;
pub mod error;
pub mod mem;
pub mod stack;

pub use error::*;

pub enum Fault {
    Hard,
    MemManage,
    Bus,
    Usage,
}

/// IRQ handler signature: `(ctx, vector, userdata)`.
pub type IrqHandler = fn(*mut u8, usize, Option<usize>);

/// Registration callback the kernel hands to the HAL during init.
pub type IrqRegister = fn(usize, IrqHandler, Option<usize>) -> Result<()>;

pub trait Machinelike {
    fn init();
    /// Register HAL-owned IRQs through the kernel-supplied callback.
    fn init_irqs(register: IrqRegister);

    fn print(s: &str) -> Result<()>;

    fn bench_start();
    fn bench_end() -> (u32, f32);

    fn monotonic_now() -> u64;
    fn monotonic_freq() -> u64;
    fn rtc_raw() -> u64;
    fn set_rtc_raw(time: u64) -> i32;
    fn init_rtc() -> i32;
    // index 0..32, 31 is used by the RTC
    fn rtc_backup_register(index: u8) -> u32;
    // index 0..32, 31 is used by the RTC
    fn set_rtc_backup_register(index: u8, value: u32);
    // Returns the frequency of the machine's systick timer in Hz.
    fn systick_freq() -> u64;

    type ExcepBacktrace: Display;
    type ExcepStackFrame: Display;
    fn backtrace(initial_fp: *const usize, stack_ptr: *const usize) -> Self::ExcepBacktrace;
    fn stack_frame(stack_ptr: *const usize) -> Self::ExcepStackFrame;

    type FaultStatus: Display;
    fn get_fault_status(fault: Fault) -> Self::FaultStatus;

    fn panic_handler(info: &core::panic::PanicInfo) -> !;
    fn do_tick();
}

pub trait Schedable {
    fn trigger_reschedule();
}
