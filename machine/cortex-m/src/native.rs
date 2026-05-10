pub use hal_api::*;

pub mod asm;
pub mod debug;
pub mod excep;
pub mod i2c;
pub mod panic;
pub mod sched;
pub mod spi;
pub mod uart;

mod crit;
mod print;

mod bindings {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(unused)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

#[link(name = "common", kind = "static", modifiers = "+whole-archive")]
#[link(name = "device_native")]
#[link(name = "hal_native")]
#[link(name = "interface_native")]
unsafe extern "C" {}

include!(concat!(env!("OUT_DIR"), "/vector_table.rs"));
include!(concat!(env!("OUT_DIR"), "/device_tree.rs"));

pub type Machine = ArmMachine;
pub type Stack = sched::ArmStack;

pub struct ArmMachine;

impl hal_api::Machinelike for ArmMachine {
    fn init() {
        unsafe {
            bindings::init_hal();
        }
        let _ = uart::init_console_from_dt();
        unsafe {
            bindings::dwt_init();
        }
    }

    fn print(s: &str) -> Result<()> {
        let state = asm::disable_irq_save();
        let ok = uart::console_write(s.as_bytes()).is_ok();
        asm::enable_irq_restr(state);
        if ok {
            Ok(())
        } else {
            Err(hal_api::PosixError::EIO)
        }
    }

    fn bench_start() {
        unsafe {
            bindings::dwt_reset();
        }
    }

    fn bench_end() -> (u32, f32) {
        let cycles = unsafe { bindings::dwt_read() };
        let ns = unsafe { bindings::dwt_cycles_to_ns(cycles) };

        (cycles as u32, ns)
    }

    fn monotonic_now() -> u64 {
        unsafe { bindings::monotonic_now() }
    }

    fn monotonic_freq() -> u64 {
        unsafe { bindings::monotonic_freq() }
    }

    fn systick_freq() -> u64 {
        unsafe { bindings::systick_freq() }
    }

    fn do_tick() {
        unsafe {
            bindings::do_tick();
        }
    }

    type ExcepBacktrace = excep::ExcepBacktrace;
    type ExcepStackFrame = excep::ExcepStackFrame;

    fn backtrace(initial_fp: *const usize, stack_ptr: *const usize) -> Self::ExcepBacktrace {
        let frame = excep::ExcepStackFrame::new(stack_ptr);
        excep::ExcepBacktrace::new(frame, initial_fp)
    }

    fn stack_frame(stack_ptr: *const usize) -> Self::ExcepStackFrame {
        excep::ExcepStackFrame::new(stack_ptr)
    }

    fn panic_handler(info: &core::panic::PanicInfo) -> ! {
        panic::panic_handler(info)
    }

    type FaultStatus = excep::FaultStatus;
    fn get_fault_status(fault: hal_api::Fault) -> Self::FaultStatus {
        excep::FaultStatus { fault }
    }
}

impl Schedable for ArmMachine {
    fn trigger_reschedule() {
        unsafe {
            bindings::reschedule();
        }
    }
}
