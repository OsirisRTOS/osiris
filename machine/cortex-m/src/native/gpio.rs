//! GPIO HAL plus per-line edge-callback demuxer. Kernel drivers use
//! `register_edge_handler` and the IRQ slot from [`irq_slot_for_line`].

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

// Keep Rust and C pull constants in sync.
const _: () = {
    assert!(Pull::None as u32 == bindings::GPIO_PULL_NONE);
    assert!(Pull::Up as u32 == bindings::GPIO_PULL_UP);
    assert!(Pull::Down as u32 == bindings::GPIO_PULL_DOWN);
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Low = 0,
    High = 1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Edges(u8);

impl Edges {
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

fn port_ptr(pin: Pin) -> *mut c_void {
    pin.port as *mut c_void
}

fn pin_mask(pin: Pin) -> Result<u16> {
    if pin.line >= 16 {
        return Err(PosixError::EINVAL);
    }
    Ok(1u16 << pin.line)
}

pub fn configure_input(pin: Pin, pull: Pull) -> Result<()> {
    let mask = pin_mask(pin)?;
    let rc = unsafe { bindings::gpio_configure_input(port_ptr(pin), mask, pull as u8) };
    ok_or_err(rc, ())
}

pub fn configure_output(pin: Pin, initial: Level) -> Result<()> {
    let mask = pin_mask(pin)?;
    let rc = unsafe { bindings::gpio_configure_output_pp(port_ptr(pin), mask, initial as u8) };
    ok_or_err(rc, ())
}

pub fn write(pin: Pin, level: Level) -> Result<()> {
    let mask = pin_mask(pin)?;
    let rc = unsafe { bindings::gpio_write(port_ptr(pin), mask, level as u8) };
    ok_or_err(rc, ())
}

pub fn enable_port_clock(pin: Pin) -> Result<()> {
    let rc = unsafe { bindings::gpio_clock_enable(port_ptr(pin)) };
    ok_or_err(rc, ())
}

pub fn read(pin: Pin) -> Result<Level> {
    let mask = pin_mask(pin)?;
    let rc = unsafe { bindings::gpio_read(port_ptr(pin), mask) };
    match rc {
        0 => Ok(Level::Low),
        1 => Ok(Level::High),
        other if other < 0 => Err(PosixError::from_errno(-other)),
        _ => Err(PosixError::EIO),
    }
}

pub fn toggle(pin: Pin) -> Result<()> {
    let mask = pin_mask(pin)?;
    let rc = unsafe { bindings::gpio_toggle(port_ptr(pin), mask) };
    ok_or_err(rc, ())
}

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

// One slot per EXTI GPIO line; lock-free reads from `dispatch`,
// CAS-claimed in registration.
static LINES: [LineCb; 16] = [const { LineCb::new() }; 16];

/// Install an edge handler for `pin` and unmask the line. `EBUSY` if
/// the line is already claimed — call [`unregister_edge_handler`]
/// first. The caller registers `dispatch` at the matching
/// [`irq_slot_for_line`] (once per slot).
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

    // Publish `ctx` and `handler` together so `dispatch` never sees
    // one without the other. CAS rejects re-registration.
    let state = super::asm::disable_irq_save();
    let claimed = slot
        .handler
        .compare_exchange(
            core::ptr::null_mut(),
            handler as *mut (),
            Ordering::AcqRel,
            Ordering::Relaxed,
        )
        .is_ok();
    if claimed {
        slot.ctx.store(ctx as *mut (), Ordering::Release);
    }
    super::asm::enable_irq_restr(state);

    if !claimed {
        return Err(PosixError::EBUSY);
    }

    let rc = unsafe {
        bindings::exti_configure(port_ptr(pin), pin.line, edges.bits(), nvic_priority)
    };
    if rc != 0 {
        // `exti_configure` errors before any peripheral writes, so the
        // line is still masked and clearing the claim can't race `dispatch`.
        slot.ctx.store(core::ptr::null_mut(), Ordering::Release);
        slot.handler.store(core::ptr::null_mut(), Ordering::Release);
        return Err(PosixError::from_errno(-rc));
    }
    Ok(())
}

pub fn unregister_edge_handler(pin: Pin) -> Result<()> {
    if pin.line >= 16 {
        return Err(PosixError::EINVAL);
    }
    let slot = &LINES[pin.line as usize];

    // IRQs masked during teardown. The handler is cleared before the
    // line so an in-flight `dispatch` sees a null handler and skips
    // after acking.
    let state = super::asm::disable_irq_save();
    slot.handler.store(core::ptr::null_mut(), Ordering::Release);
    slot.ctx.store(core::ptr::null_mut(), Ordering::Release);
    let rc = unsafe { bindings::exti_release(pin.line) };
    super::asm::enable_irq_restr(state);

    ok_or_err(rc, ())
}

/// IRQ-context demuxer; register once per used IRQ slot. Only
/// services lines belonging to `vector`.
pub fn dispatch(_ctx: *mut u8, vector: usize, _userdata: Option<usize>) {
    let pending = unsafe { bindings::exti_pending() };
    let owned = lines_for_slot(vector) as u32;
    let serviced = pending & owned;
    if serviced == 0 {
        return;
    }
    // Ack before servicing — a new edge re-pends the bit.
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
        let ctx = slot.ctx.load(Ordering::Acquire) as *mut ();
        // A higher-priority IRQ may have called `unregister_edge_handler`
        // between the two loads above; re-check before the call so a
        // teardown that clears the slot is honoured.
        if slot.handler.load(Ordering::Acquire).is_null() {
            continue;
        }
        // SAFETY: `register_edge_handler` only stores values of type `EdgeHandler`.
        let handler: EdgeHandler = unsafe { core::mem::transmute(h) };
        handler(line, ctx);
    }
}

/// IRQ slot that fires for `line`, or None for lines outside 0..15.
/// `const` so callers can validate DT-derived lines at compile time.
pub const fn irq_slot_for_line(line: u8) -> Option<usize> {
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

/// Bitmask of GPIO lines whose IRQ slot is `vector`.
const fn lines_for_slot(vector: usize) -> u16 {
    let mut mask: u16 = 0;
    let mut line: u8 = 0;
    while line < 16 {
        if let Some(v) = irq_slot_for_line(line) {
            if v == vector {
                mask |= 1u16 << line;
            }
        }
        line += 1;
    }
    mask
}
