//! GPIO HAL over `interface/gpio.c` plus the EXTI line→callback demuxer.
//! Hides SYSCFG routing and shared `EXTI9_5`/`EXTI15_10` vectors from
//! kernel drivers, which only need `register_edge_handler(pin, edges, fn, ctx)`
//! and the matching NVIC slot from [`nvic_vector_for_line`].

use core::ffi::c_void;
use core::sync::atomic::{AtomicPtr, Ordering};

use hal_api::{PosixError, Result, ok_or_err};

use super::bindings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pin {
    pub port: usize,
    pub line: u8,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pull {
    None = 0,
    Up = 1,
    Down = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Low = 0,
    High = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edges(u8);

impl Edges {
    pub const NONE: Edges = Edges(0);
    pub const RISING: Edges = Edges(0x1);
    pub const FALLING: Edges = Edges(0x2);
    pub const BOTH: Edges = Edges(0x3);

    pub const fn bits(self) -> u8 {
        self.0
    }
}

impl core::ops::BitOr for Edges {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Edges(self.0 | rhs.0)
    }
}

/// IRQ-context callback. No allocation, no blocking.
pub type EdgeHandler = extern "C" fn(line: u8, ctx: *mut ());

// ----------------------------------------------------------------------
// I/O
// ----------------------------------------------------------------------

fn port_ptr(pin: Pin) -> *mut c_void {
    pin.port as *mut c_void
}

fn pin_mask(pin: Pin) -> u16 {
    1u16 << (pin.line & 0xF)
}

pub fn configure_input(pin: Pin, pull: Pull) -> Result<()> {
    let rc = unsafe { bindings::gpio_configure_input(port_ptr(pin), pin_mask(pin), pull as u8) };
    ok_or_err(rc, ())
}

pub fn configure_output(pin: Pin, initial: Level) -> Result<()> {
    let rc =
        unsafe { bindings::gpio_configure_output_pp(port_ptr(pin), pin_mask(pin), initial as u8) };
    ok_or_err(rc, ())
}

pub fn write(pin: Pin, level: Level) -> Result<()> {
    let rc = unsafe { bindings::gpio_write(port_ptr(pin), pin_mask(pin), level as u8) };
    ok_or_err(rc, ())
}

pub fn read(pin: Pin) -> Result<Level> {
    let rc = unsafe { bindings::gpio_read(port_ptr(pin), pin_mask(pin)) };
    match rc {
        0 => Ok(Level::Low),
        1 => Ok(Level::High),
        other if other < 0 => Err(PosixError::from_errno(-other)),
        _ => Err(PosixError::EIO),
    }
}

pub fn toggle(pin: Pin) -> Result<()> {
    let rc = unsafe { bindings::gpio_toggle(port_ptr(pin), pin_mask(pin)) };
    ok_or_err(rc, ())
}

// ----------------------------------------------------------------------
// EXTI line → callback table + per-vector demuxer
// ----------------------------------------------------------------------

struct LineCb {
    handler: AtomicPtr<()>,
    ctx: AtomicPtr<()>,
}

impl LineCb {
    const fn new() -> Self {
        Self {
            handler: AtomicPtr::new(core::ptr::null_mut()),
            ctx: AtomicPtr::new(core::ptr::null_mut()),
        }
    }
}

static LINES: [LineCb; 16] = [const { LineCb::new() }; 16];

/// Install an edge handler for `pin` and unmask the EXTI line. The caller
/// is responsible for registering [`dispatch`] at the NVIC slot returned
/// by [`nvic_vector_for_line`] (once per slot).
pub fn register_edge_handler(
    pin: Pin,
    edges: Edges,
    handler: EdgeHandler,
    ctx: *mut (),
    nvic_priority: u8,
) -> Result<()> {
    if pin.line >= 16 || edges.bits() == 0 {
        return Err(PosixError::EINVAL);
    }
    let slot = &LINES[pin.line as usize];

    // Release-store before unmasking so the first IRQ sees a complete LineCb.
    slot.ctx.store(ctx as *mut (), Ordering::Release);
    slot.handler.store(handler as *mut (), Ordering::Release);

    let rc = unsafe {
        bindings::exti_configure(port_ptr(pin), pin.line, edges.bits(), nvic_priority)
    };
    if rc != 0 {
        // Roll back so a retry can't see a stale handler.
        slot.handler.store(core::ptr::null_mut(), Ordering::Release);
        slot.ctx.store(core::ptr::null_mut(), Ordering::Release);
        return Err(PosixError::from_errno(-rc));
    }
    Ok(())
}

pub fn unregister_edge_handler(pin: Pin) -> Result<()> {
    if pin.line >= 16 {
        return Err(PosixError::EINVAL);
    }
    let rc = unsafe { bindings::exti_release(pin.line) };
    let slot = &LINES[pin.line as usize];
    slot.handler.store(core::ptr::null_mut(), Ordering::Release);
    slot.ctx.store(core::ptr::null_mut(), Ordering::Release);
    ok_or_err(rc, ())
}

/// IRQ-context demuxer; register once per used EXTI NVIC slot. Signature
/// matches `crate::irq::IrqHandler` in the kernel.
pub fn dispatch(_ctx: *mut u8, _vector: usize, _userdata: Option<usize>) {
    let pending = unsafe { bindings::exti_pending() };
    let serviced = pending & 0xFFFFu32; // GPIO lines are bits 0..15.
    if serviced == 0 {
        return;
    }
    // Ack first: a new edge after ack repends the bit and we'll service
    // it on the next IRQ rather than dropping it.
    unsafe { bindings::exti_ack(serviced) };

    let mut bits = serviced;
    while bits != 0 {
        let line = bits.trailing_zeros() as u8;
        bits &= bits - 1;
        let slot = &LINES[line as usize];
        let h = slot.handler.load(Ordering::Acquire);
        if h.is_null() {
            continue;
        }
        // SAFETY: `register_edge_handler` only stores values of type `EdgeHandler`.
        let handler: EdgeHandler = unsafe { core::mem::transmute(h) };
        let ctx = slot.ctx.load(Ordering::Acquire) as *mut ();
        handler(line, ctx);
    }
}

/// Cortex-M vector slot (`IRQn + 16`) that fires for `line`. Returns
/// None for lines outside 0..15.
pub fn nvic_vector_for_line(line: u8) -> Option<usize> {
    let irqn: usize = match line {
        0 => 6,
        1 => 7,
        2 => 8,
        3 => 9,
        4 => 10,
        5..=9 => 23,
        10..=15 => 40,
        _ => return None,
    };
    Some(irqn + 16)
}
