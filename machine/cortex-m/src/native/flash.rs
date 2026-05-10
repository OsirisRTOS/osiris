use super::bindings;
use super::device_tree;

pub use hal_api::flash_addr::{Error, Result};

// ---------------------------------------------------------------------------
// Chip-level queries
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Flash trait impl + type aliases
// ---------------------------------------------------------------------------

/// Zero-sized backend marker that plugs the chip queries into the shared
/// newtype constructors in `hal_api::flash`.
pub struct ArmFlash;

impl hal_api::flash_addr::Flash for ArmFlash {
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
}

pub type FlashAddress = hal_api::flash_addr::FlashAddress<ArmFlash>;
pub type FlashOffset = hal_api::flash_addr::FlashOffset<ArmFlash>;
pub type FlashPageStart = hal_api::flash_addr::FlashPageStart<ArmFlash>;

// ---------------------------------------------------------------------------
// Whole-chip operations
// ---------------------------------------------------------------------------

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

/// Erase the page that starts at `page_start`. The bank is derived
/// automatically inside the C primitive.
pub fn erase_page(page_start: FlashPageStart, timeout_ms: u32, lock_wait_ms: u32) -> Result<()> {
    with_unlock(
        || {
            let rc = unsafe { bindings::flash_erase(page_start.page_index() as u32, timeout_ms) };
            from_c_rc(rc)
        },
        lock_wait_ms,
    )
}

/// Program 64-bit doublewords starting at `start` (must be 8-byte-aligned).
/// The target range must already be erased.
///
/// Accepts any type that converts into `FlashAddress` — pass a `FlashAddress`
/// directly, a `FlashPageStart` (always page- and word-aligned), or a
/// `FlashOffset`.
pub fn program<A: Into<FlashAddress>>(
    start: A,
    data: &[u64],
    timeout_ms: u32,
    lock_wait_ms: u32,
) -> Result<()> {
    if data.is_empty() {
        return Ok(());
    }
    let start: FlashAddress = start.into();
    if start.as_usize() & 0x7 != 0 {
        return Err(Error::Misaligned);
    }
    let bytes = data.len() * core::mem::size_of::<u64>();
    let end = start
        .as_usize()
        .checked_add(bytes)
        .ok_or(Error::OutOfBounds)?;
    if end > flash_base() + total_size() {
        return Err(Error::OutOfBounds);
    }

    with_unlock(
        || {
            let rc = unsafe {
                bindings::flash_program(
                    start.as_usize() as u32,
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

/// Read `buf.len()` bytes starting at `start`. Flash is memory-mapped, so
/// this is a volatile memcpy. Accepts any `Into<FlashAddress>`.
pub fn read<A: Into<FlashAddress>>(start: A, buf: &mut [u8]) -> Result<()> {
    if buf.is_empty() {
        return Ok(());
    }
    let start: FlashAddress = start.into();
    let end = start
        .as_usize()
        .checked_add(buf.len())
        .ok_or(Error::OutOfBounds)?;
    if end > flash_base() + total_size() {
        return Err(Error::OutOfBounds);
    }
    unsafe {
        let src = start.as_usize() as *const u8;
        for i in 0..buf.len() {
            buf[i] = core::ptr::read_volatile(src.add(i));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// DT-driven partition handle
// ---------------------------------------------------------------------------

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

    /// Find the partition that contains `addr` and return it together with
    /// the byte offset of `addr` from that partition's start.
    ///
    /// The returned `usize` is **partition-relative**, not a `FlashOffset`
    /// (which is from `FLASH_BASE`) — it can be passed straight to the
    /// driver-layer `Region::read`/`erase`/`program`/`write` methods.
    ///
    /// Accepts any `Into<FlashAddress>`, so a `FlashOffset` or
    /// `FlashPageStart` works too. Returns `Err(NotFound)` if `addr` doesn't
    /// fall in any declared partition.
    pub fn get_by_address(addr: impl Into<FlashAddress>) -> Result<(Self, usize)> {
        let addr: FlashAddress = addr.into();
        device_tree::flash_partition_by_address(addr.as_usize())
            .map(|(entry, offset)| (Self(entry), offset))
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

    /// Absolute flash address at which this partition starts.
    pub fn start_address(&self) -> FlashAddress {
        // safe by DT contract: partitions live inside the parent flash node's
        // reg, so flash_base + offset is always in flash bounds.
        FlashAddress::new(device_tree::FLASH_BASE + self.0.offset)
            .expect("DT partition outside flash bounds")
    }

    /// Byte offset of this partition from `flash_base()`.
    pub fn flash_offset(&self) -> FlashOffset {
        FlashOffset::new(self.0.offset).expect("DT partition offset >= total_size")
    }

    pub fn len(&self) -> usize {
        self.0.len
    }

    pub fn is_empty(&self) -> bool {
        self.0.len == 0
    }
}
