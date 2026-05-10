use crate::hal;

pub use hal::flash::{Error, FlashAddress, FlashOffset, FlashPageStart, Result};

pub mod raw {
    use crate::hal;
    pub use hal::flash::{
        FlashAddress, FlashOffset, FlashPageStart, erase_page, flash_base, is_dual_bank,
        page_count, page_size, program, read, total_size,
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

    /// Find the partition that contains `addr`, open it with `config`, and
    /// return both the `Region` and the **partition-relative** byte offset
    /// of `addr` within it. The returned `usize` is what
    /// `Region::read`/`erase`/`program`/`write` expect — i.e. distance from
    /// `region.start_address()`, not a `FlashOffset` (which is from
    /// `FLASH_BASE`).
    ///
    /// ```ignore
    /// let (slot, off) = flash::open_by_address(addr, Config::default())?;
    /// slot.read(off, &mut buf)?;
    /// ```
    ///
    /// Accepts any `Into<FlashAddress>` (so a `FlashOffset` or
    /// `FlashPageStart` works too). Returns `Err(NotFound)` if `addr`
    /// doesn't fall in any declared partition.
    pub fn open_by_address(
        addr: impl Into<FlashAddress>,
        config: Config,
    ) -> Result<(Self, usize)> {
        let (desc, partition_offset) = hal::flash::Region::get_by_address(addr)?;
        Ok((Self { desc, config }, partition_offset))
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

    /// Read `buf.len()` bytes starting at `partition_offset` (a byte offset
    /// from the partition's start, not an absolute flash address).
    pub fn read(&self, partition_offset: usize, buf: &mut [u8]) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        self.check_range(partition_offset, buf.len())?;
        hal::flash::read(self.start_address().checked_add(partition_offset)?, buf)
    }

    /// Erase `len` bytes starting at `partition_offset`.
    /// Both `partition_offset` and `len` must be multiples of `page_size()`.
    /// Returns `Err(ReadOnly)` if the DT marks this partition `read-only`.
    pub fn erase(&self, partition_offset: usize, len: usize) -> Result<()> {
        if len == 0 {
            return Ok(());
        }
        if self.read_only() {
            return Err(Error::ReadOnly);
        }
        self.check_range(partition_offset, len)?;
        let ps = self.page_size();
        if ps == 0 {
            return Err(Error::Io);
        }
        if partition_offset % ps != 0 || len % ps != 0 {
            return Err(Error::Misaligned);
        }
        let first_page = FlashPageStart::new(
            self.start_address().checked_add(partition_offset)?.as_usize(),
        )?;
        for page in first_page.iter_pages(len / ps)? {
            hal::flash::erase_page(
                page,
                self.config.erase_timeout_ms,
                self.config.lock_timeout_ms,
            )?;
        }
        Ok(())
    }

    pub fn erase_all(&self) -> Result<()> {
        self.erase(0, self.len())
    }

    /// Program 64-bit doublewords at `partition_offset` (a byte offset from
    /// the partition's start; must be 8-byte-aligned). The target range must
    /// already be erased; flash reads as `0xFF` after erase. Returns
    /// `Err(ReadOnly)` if the DT marks this partition `read-only`.
    pub fn program(&self, partition_offset: usize, data: &[u64]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        if self.read_only() {
            return Err(Error::ReadOnly);
        }
        if partition_offset % core::mem::size_of::<u64>() != 0 {
            return Err(Error::Misaligned);
        }
        let bytes = data.len() * core::mem::size_of::<u64>();
        self.check_range(partition_offset, bytes)?;
        hal::flash::program(
            self.start_address().checked_add(partition_offset)?,
            data,
            self.config.program_timeout_ms,
            self.config.lock_timeout_ms,
        )
    }

    /// Erase any pages overlapping `[partition_offset, partition_offset + data.len())`,
    /// then program `data` at `partition_offset` (padding the trailing 8-byte
    /// chunk with `0xFF` if `data.len()` isn't a multiple of 8). Bytes outside
    /// `data` *within the erased pages* are left as `0xFF` — this is **not**
    /// read-modify-write. Use the explicit `read` → modify → `erase` →
    /// `program` sequence if you need to preserve other content in the
    /// affected pages.
    pub fn write(&self, partition_offset: usize, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        if self.read_only() {
            return Err(Error::ReadOnly);
        }
        self.check_range(partition_offset, data.len())?;
        if partition_offset % core::mem::size_of::<u64>() != 0 {
            return Err(Error::Misaligned);
        }
        let ps = self.page_size();
        if ps == 0 {
            return Err(Error::Io);
        }

        let first_page_offset = (partition_offset / ps) * ps;
        let last_page_end = partition_offset
            .checked_add(data.len())
            .and_then(|end| end.checked_add(ps - 1))
            .map(|x| (x / ps) * ps)
            .ok_or(Error::OutOfBounds)?;
        let erase_len = last_page_end - first_page_offset;
        if first_page_offset + erase_len > self.len() {
            return Err(Error::OutOfBounds);
        }
        self.erase(first_page_offset, erase_len)?;

        // Program one doubleword at a time. `&[u8]` has no alignment guarantee,
        // so build each u64 from explicit bytes; the trailing partial chunk is
        // padded with 0xFF so the cell programs as "blank" past the data end.
        let mut written = 0usize;
        while written < data.len() {
            let mut buf = [0xFFu8; 8];
            let take = core::cmp::min(8, data.len() - written);
            buf[..take].copy_from_slice(&data[written..written + take]);
            let word = u64::from_le_bytes(buf);
            self.program(partition_offset + written, &[word])?;
            written += 8;
        }

        Ok(())
    }

    fn check_range(&self, partition_offset: usize, len: usize) -> Result<()> {
        let end = partition_offset
            .checked_add(len)
            .ok_or(Error::OutOfBounds)?;
        if end > self.len() {
            return Err(Error::OutOfBounds);
        }
        Ok(())
    }
}
