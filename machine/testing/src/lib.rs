use hal_api::{Result, Schedable};
use core::result::Result::Ok;


pub struct TestingMachine;

impl hal_api::Machinelike for TestingMachine {
    fn init() {
        // No hardware to initialize in testing.
    }

    fn print(s: &str) -> Result<()> {
        // Print to standard output in testing.
        print!("{}", s);
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

    fn backtrace(_initial_fp: *const usize, _stack_ptr: *const usize) -> Self::ExcepBacktrace {
        // Return a dummy backtrace in testing.
        "Backtrace not available in testing.".to_string()
    }
}

impl Schedable for TestingMachine {
    fn trigger_reschedule() {
        // No scheduling in testing.
    }
}