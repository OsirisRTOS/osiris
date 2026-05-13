//! Stub GPIO HAL for host/test builds. Mirrors `native::gpio` shape so
//! kernel-side drivers compile unchanged; everything is a no-op or returns
//! `EOPNOTSUPP`.

use hal_api::{PosixError, Result};

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

pub type EdgeHandler = extern "C" fn(line: u8, ctx: *mut ());

pub fn configure_input(_pin: Pin, _pull: Pull) -> Result<()> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn configure_output(_pin: Pin, _initial: Level) -> Result<()> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn write(_pin: Pin, _level: Level) -> Result<()> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn read(_pin: Pin) -> Result<Level> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn toggle(_pin: Pin) -> Result<()> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn register_edge_handler(
    _pin: Pin,
    _edges: Edges,
    _handler: EdgeHandler,
    _ctx: *mut (),
    _nvic_priority: u8,
) -> Result<()> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn unregister_edge_handler(_pin: Pin) -> Result<()> {
    Err(PosixError::EOPNOTSUPP)
}

pub fn dispatch(_ctx: *mut u8, _vector: usize, _userdata: Option<usize>) {}

pub const fn irq_slot_for_line(_line: u8) -> Option<usize> {
    None
}
