use core::result::Result::Ok;
pub use hal_api::*;

pub mod asm;
pub mod sched;

pub type Machine = StubMachine;
pub type Stack = sched::StubStack;

pub struct StubMachine;

impl hal_api::Machinelike for StubMachine {
    fn init() {
        // No hardware to initialize in testing.
    }

    fn print(s: &str) -> Result<()> {
        // Print to standard output in testing.
        print!("{s}");
        Ok(())
    }

    fn bench_start() {
        // No benchmarking in testing.
    }

    fn bench_end() -> (u32, f32) {
        // Return dummy values for benchmarking in testing.
        (0, 0.0)
    }

    fn monotonic_now() -> u64 {
        0
    }

    fn monotonic_freq() -> u64 {
        0
    }

    fn get_rtc_raw() -> u64 {
        0
    }

    fn set_rtc_raw(_time: u64) {
    }

    fn get_rtc_backup_register(index: u8) -> u32 {
        0
    }

    fn set_rtc_backup_register(index: u8, value: u32) {
    }

    fn systick_freq() -> u64 {
        0
    }

    type ExcepBacktrace = String;
    type ExcepStackFrame = String;

    fn backtrace(_initial_fp: *const usize, _stack_ptr: *const usize) -> Self::ExcepBacktrace {
        // Return a dummy backtrace in testing.
        "Backtrace not available in testing.".to_string()
    }

    fn stack_frame(_stack_ptr: *const usize) -> Self::ExcepStackFrame {
        // Return a dummy stack frame in testing.
        "Stack frame not available in testing.".to_string()
    }

    type FaultStatus = String;
    fn get_fault_status(_fault: hal_api::Fault) -> Self::FaultStatus {
        // Return a dummy fault status in testing.
        "Fault status not available in testing.".to_string()
    }

    fn panic_handler(info: &core::panic::PanicInfo) -> ! {
        // Print the panic information and abort in testing.
        eprintln!("Panic occurred: {info}");
        std::process::abort();
    }
}

impl Schedable for StubMachine {
    fn trigger_reschedule() {
        // No scheduling in testing.
    }
}