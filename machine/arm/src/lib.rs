#![cfg_attr(all(not(test), not(feature = "host")), no_std)]

use core::ffi::c_char;

use hal_api::{Result, Schedable};

pub mod asm;
pub mod debug;
pub mod excep;
pub mod panic;
pub mod sched;

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
#[link(name = "chip_native")]
unsafe extern "C" {}

pub struct ArmMachine;

impl hal_api::Machinelike for ArmMachine {
    fn init() {
        unsafe {
            bindings::init_hal();
            bindings::init_debug_uart();
            bindings::dwt_init();
        }
    }

    fn print(s: &str) -> Result<()> {
        use crate::asm;
        asm::disable_interrupts();

        if (unsafe { bindings::write_debug_uart(s.as_ptr() as *const c_char, s.len() as i32) } != 0) {
            asm::enable_interrupts();
            Ok(())
        } else {
            asm::enable_interrupts();
            Err(hal_api::Error::default())
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
