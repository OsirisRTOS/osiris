use core::result::Result::Ok;
use hal_api::{Result, Schedable};

pub mod asm;
pub mod sched;

pub struct TestingMachine;

impl hal_api::Machinelike for TestingMachine {
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

impl Schedable for TestingMachine {
    fn trigger_reschedule() {
        // No scheduling in testing.
    }
}
