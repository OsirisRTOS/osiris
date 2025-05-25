#![cfg_attr(not(test), no_std)]

mod bindings {
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(unused)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
    include!(concat!(env!("OUT_DIR"), "/macros.rs"));
}

pub mod asm;
pub mod panic;
pub mod rcc;
pub mod sched;
pub mod uart;

use core::{arch::asm, sync::atomic::compiler_fence};

pub const HAL_OK: u32 = 0;
pub const HAL_ERROR: u32 = 1;
pub const HAL_BUSY: u32 = 2;
pub const HAL_TIMEOUT: u32 = 3;

pub fn init_hw() {
    fpu_init();

    unsafe { bindings::HAL_Init() };

    // Initialize the system clock
    rcc::init_rcc();

    // Initialize our logging UART
    uart::lpuart1_init();

    // Initialize the SysTick timer
    unsafe {
        bindings::HAL_SYSTICK_Config(bindings::HAL_RCC_GetHCLKFreq() / 10); // 100 ms tick
        bindings::HAL_SYSTICK_CLKSourceConfig(bindings::SYSTICK_CLKSOURCE_HCLK);
    }
}

fn fpu_init() {
    const SCB: *mut bindings::SCB_Type = bindings::SCB_BASE as *mut bindings::SCB_Type;
    unsafe {
        // Enable the FPU
        (*SCB).CPACR |= 0xF << 20; // Set CP10 and CP11 full access
        asm!("dsb", options(nomem, nostack, preserves_flags));
        asm!("isb", options(nomem, nostack, preserves_flags));
    }
}

#[inline(always)]
pub fn disable_interrupts() {
    unsafe { asm!("cpsid f", options(nomem, nostack, preserves_flags)) };
    compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

#[inline(always)]
pub fn are_interrupts_enabled() -> bool {
    let primask: u32;
    unsafe {
        asm!("mrs {}, primask", out(reg) primask, options(nomem, nostack, preserves_flags));
    }
    primask == 0
}

#[inline(always)]
pub fn enable_interrupts() {
    unsafe { asm!("cpsie f", options(nomem, nostack, preserves_flags)) };
    compiler_fence(core::sync::atomic::Ordering::SeqCst);
}

#[unsafe(no_mangle)]
pub extern "C" fn HAL_MspInit() {
    unsafe { bindings::HAL_NVIC_SetPriority(bindings::IRQn_Type_PendSV_IRQn, 15, 0) };
    unsafe { bindings::HAL_NVIC_SetPriority(bindings::IRQn_Type_SysTick_IRQn, 15, 0) };
    unsafe { bindings::HAL_RCC_SYSCFG_CLK_ENABLE() };
    unsafe { bindings::HAL_RCC_PWR_CLK_ENABLE() };
}
