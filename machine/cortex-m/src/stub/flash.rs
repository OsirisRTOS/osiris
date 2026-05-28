pub use hal_api::flash_addr::{Error, Result};

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
pub fn write_unit_bytes() -> usize {
    core::mem::size_of::<u64>()
}

pub struct TestingFlash;

impl hal_api::flash_addr::Flash for TestingFlash {
    fn flash_base() -> usize {
        flash_base()
    }
    fn total_size() -> usize {
        total_size()
    }
    fn page_size() -> usize {
        page_size()
    }
    fn page_count() -> usize {
        page_count()
    }
    fn write_unit_bytes() -> usize {
        write_unit_bytes()
    }
}

pub type FlashAddress = hal_api::flash_addr::FlashAddress<TestingFlash>;
pub type FlashOffset = hal_api::flash_addr::FlashOffset<TestingFlash>;
pub type FlashPageStart = hal_api::flash_addr::FlashPageStart<TestingFlash>;

pub fn erase_page(
    _page_start: FlashPageStart,
    _timeout_ms: u32,
    _lock_wait_ms: u32,
) -> Result<()> {
    Err(Error::Io)
}

pub fn program<A: Into<FlashAddress>>(
    _start: A,
    _data: &[u64],
    _timeout_ms: u32,
    _lock_wait_ms: u32,
) -> Result<()> {
    Err(Error::Io)
}

pub fn read<A: Into<FlashAddress>>(_start: A, _buf: &mut [u8]) -> Result<()> {
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
    pub fn get_by_address(_addr: impl Into<FlashAddress>) -> Result<Self> {
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
    pub fn start_address(&self) -> FlashAddress {
        // host stub: total_size() is 0, so no valid FlashAddress exists.
        // Production code shouldn't hit this path; panic if it does.
        unreachable!("flash region accessed in host testing stub")
    }
    pub fn flash_offset(&self) -> FlashOffset {
        unreachable!("flash region accessed in host testing stub")
    }
    pub fn len(&self) -> usize {
        0
    }
    pub fn is_empty(&self) -> bool {
        true
    }
}
