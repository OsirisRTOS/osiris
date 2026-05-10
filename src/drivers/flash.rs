use crate::hal;

pub use hal::flash::{Error, FlashAddress, FlashOffset, FlashPageStart, Result};

pub mod raw {
    use crate::hal;
    pub use hal::flash::{
        FlashAddress, FlashOffset, FlashPageStart, erase_page, flash_base, is_dual_bank,
        page_count, page_size, program, read, total_size, write_unit_bytes,
    };
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    /// Timeout for a single page erase, in milliseconds.
    pub erase_timeout_ms: u32,
    /// Total timeout for a program operation across all doublewords, in milliseconds.
    pub program_timeout_ms: u32,
    /// Timeout for the post-op chip-busy wait if `flash_lock()` finds the chip
    /// still busy. Recovery-path only — operations don't normally hit this.
    pub lock_timeout_ms: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            erase_timeout_ms: 5000,
            program_timeout_ms: 1000,
            lock_timeout_ms: 100,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Region {
    desc: hal::flash::Region,
    config: Config,
}

impl Region {
    pub fn open(compatible: &str, ordinal: usize, config: Config) -> Result<Self> {
        Ok(Self {
            desc: hal::flash::Region::get(compatible, ordinal)?,
            config,
        })
    }

    pub fn open_by_label(label: &str, config: Config) -> Result<Self> {
        Ok(Self {
            desc: hal::flash::Region::get_by_label(label)?,
            config,
        })
    }

    /// Find the partition containing `addr` and open it. The caller already
    /// has the absolute address, so this returns only the `Region` — pass
    /// `addr` directly to `read`/`program`/`write`.
    ///
    /// Returns `Err(NotFound)` if `addr` doesn't fall in any declared partition.
    pub fn open_by_address(addr: impl Into<FlashAddress>, config: Config) -> Result<Self> {
        let (desc, _) = hal::flash::Region::get_by_address(addr)?;
        Ok(Self { desc, config })
    }

    pub fn config(&self) -> Config {
        self.config
    }

    pub fn label(&self) -> &'static str {
        self.desc.label()
    }

    pub fn compatible(&self) -> &'static str {
        self.desc.compatible()
    }

    pub fn read_only(&self) -> bool {
        self.desc.read_only()
    }

    /// Absolute flash address at which this partition starts.
    pub fn start_address(&self) -> FlashAddress {
        self.desc.start_address()
    }

    /// Byte offset of this partition from `flash_base()`.
    pub fn flash_offset(&self) -> FlashOffset {
        self.desc.flash_offset()
    }

    pub fn len(&self) -> usize {
        self.desc.len()
    }

    pub fn is_empty(&self) -> bool {
        self.desc.is_empty()
    }

    pub fn page_size(&self) -> usize {
        hal::flash::page_size()
    }

    pub fn page_count(&self) -> usize {
        let ps = self.page_size();
        if ps == 0 { 0 } else { self.len() / ps }
    }

    pub fn write_unit_bytes(&self) -> usize {
        hal::flash::write_unit_bytes()
    }

    /// Read `buf.len()` bytes starting at `addr`. Accepts any
    /// `Into<FlashAddress>` (so a `FlashOffset` or `FlashPageStart` works
    /// too). Errors with `OutOfBounds` if `[addr, addr + buf.len())` leaves
    /// this partition.
    pub fn read(&self, addr: impl Into<FlashAddress>, buf: &mut [u8]) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        let addr: FlashAddress = addr.into();
        self.check_range(addr, buf.len())?;
        hal::flash::read(addr, buf)
    }

    /// Erase `len_bytes` bytes starting at `start`. `len_bytes` must be a
    /// multiple of the chip's current page size, and `start` must be
    /// page-aligned for that same page size. Both are revalidated here
    /// against `hal::flash::page_size()` (which can change at runtime on
    /// parts that support bank reconfiguration), so a `FlashPageStart`
    /// validated at an earlier page size will be rejected.
    ///
    /// Returns `Err(ReadOnly)` if the DT marks this partition read-only.
    pub fn erase(&self, start: FlashPageStart, len_bytes: usize) -> Result<()> {
        if len_bytes == 0 {
            return Ok(());
        }
        if self.read_only() {
            return Err(Error::ReadOnly);
        }
        let ps = self.page_size();
        if ps == 0 {
            return Err(Error::Io);
        }
        if start.as_usize() % ps != 0 || len_bytes % ps != 0 {
            return Err(Error::Misaligned);
        }
        self.check_range(start.into(), len_bytes)?;
        for page in start.iter_pages(len_bytes / ps)? {
            hal::flash::erase_page(
                page,
                self.config.erase_timeout_ms,
                self.config.lock_timeout_ms,
            )?;
        }
        Ok(())
    }

    pub fn erase_all(&self) -> Result<()> {
        let start = FlashPageStart::try_from(self.start_address())?;
        self.erase(start, self.len())
    }

    /// Program bytes starting at `addr`. `addr` and `data.len()` must both be
    /// multiples of `hal::flash::write_unit_bytes()`. The target range must
    /// already be erased; flash reads as `0xFF` after erase. Returns
    /// `Err(ReadOnly)` if the DT marks this partition read-only.
    pub fn program(&self, addr: impl Into<FlashAddress>, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        if self.read_only() {
            return Err(Error::ReadOnly);
        }
        let addr: FlashAddress = addr.into();
        let unit = self.write_unit_bytes();
        if unit == 0 {
            return Err(Error::Io);
        }
        if addr.as_usize() % unit != 0 || data.len() % unit != 0 {
            return Err(Error::Misaligned);
        }
        self.check_range(addr, data.len())?;

        // Repack bytes into the HAL's native programming type. L4 wants
        // &[u64]; other STM32 HALs may want different shapes, at which point
        // this needs generalization (e.g. a Flash::WriteUnit assoc type).
        debug_assert_eq!(
            unit,
            core::mem::size_of::<u64>(),
            "kernel program(&[u8]) currently only supports u64-doubleword HALs"
        );
        const BATCH: usize = 32;
        let mut buf = [0u64; BATCH];
        let mut written = 0;
        while written < data.len() {
            let take = core::cmp::min(BATCH * unit, data.len() - written);
            let n = take / unit;
            for i in 0..n {
                let off = written + i * unit;
                buf[i] = u64::from_le_bytes([
                    data[off],
                    data[off + 1],
                    data[off + 2],
                    data[off + 3],
                    data[off + 4],
                    data[off + 5],
                    data[off + 6],
                    data[off + 7],
                ]);
            }
            hal::flash::program(
                addr.checked_add(written)?,
                &buf[..n],
                self.config.program_timeout_ms,
                self.config.lock_timeout_ms,
            )?;
            written += take;
        }
        Ok(())
    }

    /// Erase any pages overlapping `[addr, addr + data.len())`, then program
    /// `data` at `addr` (padding the trailing partial write-unit with `0xFF`
    /// if `data.len()` isn't a multiple of `hal::flash::write_unit_bytes()`).
    /// Bytes outside `data` *within the erased pages* are left as `0xFF` —
    /// this is **not** read-modify-write. Use the explicit `read` → modify →
    /// `erase` → `program` sequence if you need to preserve other content in
    /// the affected pages.
    ///
    /// Accepts any `Into<FlashAddress>`; `addr` must be write-unit-aligned.
    pub fn write(&self, addr: impl Into<FlashAddress>, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        if self.read_only() {
            return Err(Error::ReadOnly);
        }
        let addr: FlashAddress = addr.into();
        let unit = self.write_unit_bytes();
        if unit == 0 {
            return Err(Error::Io);
        }
        if addr.as_usize() % unit != 0 {
            return Err(Error::Misaligned);
        }
        self.check_range(addr, data.len())?;
        let ps = self.page_size();
        if ps == 0 {
            return Err(Error::Io);
        }

        let addr_u = addr.as_usize();
        let first_page = (addr_u / ps) * ps;
        let last_page_end = addr_u
            .checked_add(data.len())
            .and_then(|end| end.checked_add(ps - 1))
            .map(|x| (x / ps) * ps)
            .ok_or(Error::OutOfBounds)?;
        let erase_len = last_page_end - first_page;
        self.erase(FlashPageStart::new(first_page)?, erase_len)?;

        // Program whole write-units directly from `data`. If the tail is a
        // partial unit, copy into a stack buffer and pad with 0xFF — that
        // matches the post-erase state, so the padded bytes program as blank.
        let aligned = (data.len() / unit) * unit;
        if aligned > 0 {
            self.program(addr, &data[..aligned])?;
        }
        let remaining = data.len() - aligned;
        if remaining > 0 {
            let mut tail = [0xFFu8; core::mem::size_of::<u64>()];
            tail[..remaining].copy_from_slice(&data[aligned..]);
            self.program(addr.checked_add(aligned)?, &tail)?;
        }
        Ok(())
    }

    fn check_range(&self, addr: FlashAddress, len: usize) -> Result<()> {
        let region_start = self.start_address().as_usize();
        let region_end = region_start
            .checked_add(self.len())
            .ok_or(Error::OutOfBounds)?;
        let start = addr.as_usize();
        if start < region_start {
            return Err(Error::OutOfBounds);
        }
        let end = start.checked_add(len).ok_or(Error::OutOfBounds)?;
        if end > region_end {
            return Err(Error::OutOfBounds);
        }
        Ok(())
    }
}
