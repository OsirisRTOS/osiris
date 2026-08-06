use core::ffi::c_char;

pub use hal_api::*;

pub mod asm;
pub mod can;
pub mod debug;
pub mod excep;
pub mod gpio;
pub mod i2c;
pub mod panic;
pub mod rtc;
pub mod sched;
pub mod spi;
pub mod system;

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

fn monotonic_overflow_irq(_ctx: *mut u8, _vector: usize, _userdata: Option<usize>) {
    unsafe { bindings::tim2_hndlr() };
}

fn nmi_irq(_ctx: *mut u8, _vector: usize, _userdata: Option<usize>) {
    if unsafe { bindings::irq_is_css() } {
        unsafe { bindings::css_hndlr() }
    }
}

fn rcc_irq(_ctx: *mut u8, _vector: usize, _userdata: Option<usize>) {
    if unsafe { bindings::irq_is_lse_css() } {
        unsafe { bindings::css_lse_hndlr() }
    }
}

impl hal_api::Machinelike for ArmMachine {
    fn init() {
        unsafe {
            let ret = bindings::init_hal();
            if ret != 0 {
                panic!("init_hal failed: {}", ret);
            }
            bindings::init_debug_uart();
            bindings::dwt_init();
        }
    }

    fn init_irqs(register: hal_api::IrqRegister) {
        // Monotonic timer - usually TIM2 - picked by the board via `osiris,monotonic-timer`
        // in /chosen; vector = irqn + 16 (Cortex-M IPSR offset).
        let Some(&(_, device_tree::PropValue::Str(path))) = device_tree::chosen::EXTRAS
            .iter()
            .find(|(k, _)| *k == "osiris,monotonic-timer")
        else {
            panic!("device tree: missing `osiris,monotonic-timer` in /chosen");
        };
        let Some(timer) = device_tree::peripheral_by_path(path) else {
            panic!("device tree: `osiris,monotonic-timer` path {path} did not resolve");
        };
        let Some(&irqn) = timer.interrupts.first() else {
            panic!("device tree: monotonic timer at {path} has no `interrupts` entry");
        };
        let vector = irqn as usize + 16;
        if let Err(e) = register(vector, monotonic_overflow_irq, None) {
            panic!("failed to register monotonic timer IRQ at vector {vector}: {e}");
        }

        let vector = 2;
        if let Err(e) = register(vector, nmi_irq, None) {
            panic!("failed to register CSS IRQ at vector {vector}: {e}");
        }

        let vector = unsafe { bindings::CONST_RCC_IRQn as usize } + 16;
        if let Err(e) = register(vector, rcc_irq, None) {
            panic!("failed to register CSS LSE IRQ at vector {vector}: {e}");
        }
    }

    fn print(s: &str) -> Result<()> {
        // Mask PendSV only — a full cpsid_i across a polled-UART line at
        // 115200 baud (~13 ms) overruns the bxCAN FIFO at 1 Mbit/s.
        let state = asm::disable_pendsv_save();

        let ok =
            unsafe { bindings::write_debug_uart(s.as_ptr() as *const c_char, s.len() as i32) } != 0;

        asm::enable_pendsv_restr(state);

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

    fn init_rtc() -> Result<()> {
        rtc::init_rtc()
    }

    fn rtc() -> Result<u64> {
        rtc::rtc()
    }

    fn set_rtc(time: u64) -> Result<()> {
        rtc::set_rtc(time)
    }

    fn rtc_backup_register(index: u8) -> u32 {
        assert!(index < 32, "RTC backup register index out of bounds");
        assert!(index != 31, "RTC uses this register for restart continuity");
        unsafe { bindings::rtc_backup_register(index) }
    }

    fn set_rtc_backup_register(index: u8, value: u32) {
        assert!(index < 32, "RTC backup register index out of bounds");
        assert!(index != 31, "RTC uses this register for restart continuity");
        unsafe { bindings::set_rtc_backup_register(index, value) }
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
