pub use hal::flash::Error;
pub use hal::flash::Result;

pub mod raw {
    pub use hal::flash::{
        erase_page, flash_base, is_dual_bank, page_address, page_count, page_index_for_address,
        page_size, program, read, total_size,
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

    pub fn start(&self) -> usize {
        self.desc.start()
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

    pub fn read(&self, offset: usize, buf: &mut [u8]) -> Result<()> {
        if buf.is_empty() {
            return Ok(());
        }
        self.check_range(offset, buf.len())?;
        hal::flash::read(self.start() + offset, buf)
    }

    /// Erase `len` bytes starting at `offset` within the partition.
    /// Both `offset` and `len` must be multiples of `page_size()`.
    pub fn erase(&self, offset: usize, len: usize) -> Result<()> {
        if len == 0 {
            return Ok(());
        }
        self.check_range(offset, len)?;
        let ps = self.page_size();
        if ps == 0 {
            return Err(Error::Io);
        }
        if offset % ps != 0 || len % ps != 0 {
            return Err(Error::Misaligned);
        }
        let first_page_index = hal::flash::page_index_for_address(self.start() + offset)
            .ok_or(Error::OutOfBounds)?;
        let page_count = len / ps;
        for i in 0..page_count {
            hal::flash::erase_page(
                first_page_index + i,
                self.config.erase_timeout_ms,
                self.config.lock_timeout_ms,
            )?;
        }
        Ok(())
    }

    pub fn erase_all(&self) -> Result<()> {
        self.erase(0, self.len())
    }

    /// Program 64-bit doublewords at `offset` (which must be 8-byte-aligned).
    /// The target range must already be erased; flash reads as `0xFF` after erase.
    pub fn program(&self, offset: usize, data: &[u64]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        if offset % core::mem::size_of::<u64>() != 0 {
            return Err(Error::Misaligned);
        }
        let bytes = data.len() * core::mem::size_of::<u64>();
        self.check_range(offset, bytes)?;
        hal::flash::program(
            self.start() + offset,
            data,
            self.config.program_timeout_ms,
            self.config.lock_timeout_ms,
        )
    }

    /// Erase any pages overlapping `[offset, offset + data.len())`, then program
    /// `data` at `offset` (padding the trailing 8-byte chunk with `0xFF` if
    /// `data.len()` isn't a multiple of 8). Bytes outside `data` *within the
    /// erased pages* are left as `0xFF` — this is **not** read-modify-write.
    /// Use the explicit `read` → modify → `erase` → `program` sequence if you
    /// need to preserve other content in the affected pages.
    pub fn write(&self, offset: usize, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        self.check_range(offset, data.len())?;
        if offset % core::mem::size_of::<u64>() != 0 {
            return Err(Error::Misaligned);
        }
        let ps = self.page_size();
        if ps == 0 {
            return Err(Error::Io);
        }

        let first = (offset / ps) * ps;
        let last = offset
            .checked_add(data.len())
            .and_then(|end| end.checked_add(ps - 1))
            .map(|x| (x / ps) * ps)
            .ok_or(Error::OutOfBounds)?;
        let erase_len = last - first;
        if first + erase_len > self.len() {
            return Err(Error::OutOfBounds);
        }
        self.erase(first, erase_len)?;

        // Program one doubleword at a time. `&[u8]` has no alignment guarantee,
        // so build each u64 from explicit bytes; the trailing partial chunk is
        // padded with 0xFF so the cell programs as "blank" past the data end.
        let mut written = 0usize;
        while written < data.len() {
            let mut buf = [0xFFu8; 8];
            let take = core::cmp::min(8, data.len() - written);
            buf[..take].copy_from_slice(&data[written..written + take]);
            let word = u64::from_le_bytes(buf);
            self.program(offset + written, &[word])?;
            written += 8;
        }

        Ok(())
    }

    fn check_range(&self, offset: usize, len: usize) -> Result<()> {
        let end = offset.checked_add(len).ok_or(Error::OutOfBounds)?;
        if end > self.len() {
            return Err(Error::OutOfBounds);
        }
        Ok(())
    }
}
