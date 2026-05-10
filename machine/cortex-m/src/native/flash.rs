use super::bindings;
use super::device_tree;

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

fn from_c_rc(rc: u32) -> Result<()> {
    if rc == bindings::FLASH_OK {
        return Ok(());
    }

    if rc & bindings::ERR_FLASH_TIMEOUT != 0 {
        return Err(Error::TimedOut);
    }
    if rc & bindings::ERR_FLASH_BUSY != 0 {
        return Err(Error::Busy);
    }
    if rc & bindings::ERR_FLASH_DOUBLE_UNLOCK != 0 {
        return Err(Error::DoubleUnlock);
    }
    if rc & bindings::ERR_FLASH_NOT_UNLOCKED != 0 {
        return Err(Error::Locked);
    }
    if rc & bindings::ERR_FLASH_INVALID_PAGE != 0 {
        return Err(Error::InvalidPage);
    }
    if rc & bindings::ERR_FLASH_INVALID_BANK != 0 {
        return Err(Error::InvalidPage);
    }
    if rc & bindings::ERR_FLASH_ILLEGAL != 0 {
        return Err(Error::InvalidArgument);
    }

    if rc & bindings::HAL_FLASH_ERROR_PROG != 0 {
        return Err(Error::NotErased);
    }

    Err(Error::Io)
}

pub fn flash_base() -> usize {
    device_tree::FLASH_BASE
}

pub fn total_size() -> usize {
    unsafe { bindings::flash_size() as usize }
}

pub fn is_dual_bank() -> bool {
    unsafe { bindings::flash_is_dual_bank() }
}

pub fn page_size() -> usize {
    unsafe { bindings::flash_page_size() as usize }
}

pub fn page_count() -> usize {
    unsafe { bindings::flash_page_count() as usize }
}

/// Absolute start address of the flash page identified by `page_index`.
pub fn page_address(page_index: usize) -> Option<usize> {
    if page_index >= page_count() {
        return None;
    }
    Some(flash_base() + page_index * page_size())
}

/// Page index containing the byte at absolute `address`.
pub fn page_index_for_address(address: usize) -> Option<usize> {
    let base = flash_base();
    let size = total_size();
    if address < base || address >= base + size {
        return None;
    }
    Some((address - base) / page_size())
}

fn with_unlock<F: FnOnce() -> Result<()>>(f: F, lock_wait_ms: u32) -> Result<()> {
    let unlock_rc = unsafe { bindings::flash_unlock() };
    from_c_rc(unlock_rc)?;
    let result = f();
    let lock_rc = unsafe { bindings::flash_lock() };
    if lock_rc != bindings::FLASH_OK {
        // chip may still be busy from the op — wait briefly and retry once.
        let _ = unsafe { bindings::flash_wait_for_last_operation(lock_wait_ms) };
        let _ = unsafe { bindings::flash_lock() };
    }
    result
}

/// Erase the flash page identified by `page_index` (i.e., a page number, not
/// an address). The bank is derived automatically inside the C primitive.
pub fn erase_page(page_index: usize, timeout_ms: u32, lock_wait_ms: u32) -> Result<()> {
    if page_index >= page_count() {
        return Err(Error::InvalidPage);
    }
    with_unlock(
        || {
            let rc = unsafe { bindings::flash_erase(page_index as u32, timeout_ms) };
            from_c_rc(rc)
        },
        lock_wait_ms,
    )
}

pub fn program(address: usize, data: &[u64], timeout_ms: u32, lock_wait_ms: u32) -> Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    if address & 0x7 != 0 {
        return Err(Error::Misaligned);
    }
    let base = flash_base();
    let size = total_size();
    let bytes = data.len() * core::mem::size_of::<u64>();
    let end = address.checked_add(bytes).ok_or(Error::OutOfBounds)?;
    if address < base || end > base + size {
        return Err(Error::OutOfBounds);
    }

    with_unlock(
        || {
            let rc = unsafe {
                bindings::flash_program(
                    address as u32,
                    data.as_ptr(),
                    data.len() as u32,
                    timeout_ms,
                )
            };
            from_c_rc(rc)
        },
        lock_wait_ms,
    )
}

pub fn read(address: usize, buf: &mut [u8]) -> Result<()> {
    if buf.is_empty() {
        return Ok(());
    }
    let base = flash_base();
    let size = total_size();
    let end = address.checked_add(buf.len()).ok_or(Error::OutOfBounds)?;
    if address < base || end > base + size {
        return Err(Error::OutOfBounds);
    }
    // Flash is memory-mapped; copy via volatile reads to keep the compiler honest.
    unsafe {
        let src = address as *const u8;
        for i in 0..buf.len() {
            buf[i] = core::ptr::read_volatile(src.add(i));
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
pub struct Region(&'static device_tree::FlashPartitionRegistryEntry);

impl Region {
    pub fn get(compatible: &str, ordinal: usize) -> Result<Self> {
        device_tree::flash_partition_by_compatible(compatible, ordinal)
            .map(Self)
            .ok_or(Error::NotFound)
    }

    pub fn get_by_label(label: &str) -> Result<Self> {
        device_tree::flash_partition_by_label(label)
            .map(Self)
            .ok_or(Error::NotFound)
    }

    pub fn label(&self) -> &'static str {
        self.0.label
    }

    pub fn compatible(&self) -> &'static str {
        self.0.compatible
    }

    pub fn read_only(&self) -> bool {
        self.0.read_only
    }

    pub fn start(&self) -> usize {
        device_tree::FLASH_BASE + self.0.offset
    }

    pub fn offset_in_flash(&self) -> usize {
        self.0.offset
    }

    pub fn len(&self) -> usize {
        self.0.len
    }

    pub fn is_empty(&self) -> bool {
        self.0.len == 0
    }
}
