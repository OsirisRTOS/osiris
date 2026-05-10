#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidArgument,
    OutOfBounds,
    Misaligned,
    NotErased,
    Busy,
    Locked,
    DoubleUnlock,
    InvalidPage,
    TimedOut,
    Io,
    NotFound,
}

pub type Result<T> = core::result::Result<T, Error>;

pub fn flash_base() -> usize {
    0x0800_0000
}
pub fn total_size() -> usize {
    0
}
pub fn is_dual_bank() -> bool {
    false
}
pub fn page_size() -> usize {
    8 * 1024
}
pub fn page_count() -> usize {
    0
}
pub fn page_address(_page_index: usize) -> Option<usize> {
    None
}
pub fn page_index_for_address(_address: usize) -> Option<usize> {
    None
}
pub fn erase_page(_page_index: usize, _timeout_ms: u32, _lock_wait_ms: u32) -> Result<()> {
    Err(Error::Io)
}
pub fn program(_address: usize, _data: &[u64], _timeout_ms: u32, _lock_wait_ms: u32) -> Result<()> {
    Err(Error::Io)
}
pub fn read(_address: usize, _buf: &mut [u8]) -> Result<()> {
    Err(Error::Io)
}

#[derive(Clone, Copy)]
pub struct Region;

impl Region {
    pub fn get(_compatible: &str, _ordinal: usize) -> Result<Self> {
        Err(Error::NotFound)
    }
    pub fn get_by_label(_label: &str) -> Result<Self> {
        Err(Error::NotFound)
    }
    pub fn label(&self) -> &'static str {
        ""
    }
    pub fn compatible(&self) -> &'static str {
        ""
    }
    pub fn read_only(&self) -> bool {
        false
    }
    pub fn start(&self) -> usize {
        0
    }
    pub fn offset_in_flash(&self) -> usize {
        0
    }
    pub fn len(&self) -> usize {
        0
    }
    pub fn is_empty(&self) -> bool {
        true
    }
}
