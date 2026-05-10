use core::marker::PhantomData;

/// Forward `fmt`/`Hash` to the inner `usize`. `Debug` is left to each type so
/// the output reads as `FlashAddress(0x…)` rather than the bare integer.
macro_rules! forward_usize_traits {
    ($t:ident) => {
        impl<F: Flash> core::fmt::Display for $t<F> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Display::fmt(&self.0, f)
            }
        }
        impl<F: Flash> core::fmt::LowerHex for $t<F> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::LowerHex::fmt(&self.0, f)
            }
        }
        impl<F: Flash> core::fmt::UpperHex for $t<F> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::UpperHex::fmt(&self.0, f)
            }
        }
        impl<F: Flash> core::fmt::Octal for $t<F> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Octal::fmt(&self.0, f)
            }
        }
        impl<F: Flash> core::fmt::Binary for $t<F> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                core::fmt::Binary::fmt(&self.0, f)
            }
        }
        impl<F: Flash> core::hash::Hash for $t<F> {
            fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
                self.0.hash(state);
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidArgument,
    OutOfBounds,
    Misaligned,
    NotErased,
    Busy,
    /// Chip-level lock (`FLASH->CR.LOCK`) is set / unlock failed.
    Locked,
    DoubleUnlock,
    InvalidPage,
    TimedOut,
    /// Internal sentinel: something the generic layer expected from the HAL
    /// (e.g. `page_size() > 0`) was missing. Not for hardware-reported failures.
    Io,
    NotFound,
    /// The DT marked this partition `read-only`; mutating operations
    /// (erase/program/write) refuse to touch it.
    ReadOnly,
    /// Hardware refused the access because the cells are write- or
    /// read-protected (option-byte WRP, RDP, …).
    Protected,
    /// Hardware programming operation failed for a sequencing/alignment/size
    /// reason — typically "tried to program already-programmed cells" but
    /// also covers fast-program failures.
    ProgrammingFailed,
    /// ECC double-bit error detected during read.
    EccError,
}

pub type Result<T> = core::result::Result<T, Error>;

/// Chip-side queries that the address newtypes need to validate inputs.
/// Implementors are typically zero-sized backend marker types.
pub trait Flash {
    fn flash_base() -> usize;
    fn total_size() -> usize;
    /// Erase granularity (bytes); erase ops must be page-aligned.
    fn page_size() -> usize;
    fn page_count() -> usize;
    /// Program granularity (bytes); `program`/`write` addresses and lengths
    /// must be a multiple of this.
    fn write_unit_bytes() -> usize;
}

// ---------------------------------------------------------------------------
// FlashAddress
// ---------------------------------------------------------------------------

/// An absolute address in the primary flash bank. Construction validates the
/// address falls within `[F::flash_base(), F::flash_base() + F::total_size())`.
pub struct FlashAddress<F: Flash>(usize, PhantomData<F>);

impl<F: Flash> FlashAddress<F> {
    pub fn new(address: usize) -> Result<Self> {
        let base = F::flash_base();
        let end = base
            .checked_add(F::total_size())
            .ok_or(Error::OutOfBounds)?;
        if address < base || address >= end {
            return Err(Error::OutOfBounds);
        }
        Ok(Self(address, PhantomData))
    }

    pub fn as_usize(self) -> usize {
        self.0
    }

    pub fn flash_offset(self) -> FlashOffset<F> {
        FlashOffset(self.0 - F::flash_base(), PhantomData)
    }

    pub fn page_index(self) -> usize {
        (self.0 - F::flash_base()) / F::page_size()
    }

    /// Add `bytes`, returning `Err(OutOfBounds)` if the result lands outside
    /// the flash bank or overflows `usize`.
    pub fn checked_add(self, bytes: usize) -> Result<Self> {
        let next = self.0.checked_add(bytes).ok_or(Error::OutOfBounds)?;
        Self::new(next)
    }

    /// Subtract `bytes`, returning `Err(OutOfBounds)` if the result drops
    /// below `flash_base()` or underflows.
    pub fn checked_sub(self, bytes: usize) -> Result<Self> {
        let prev = self.0.checked_sub(bytes).ok_or(Error::OutOfBounds)?;
        Self::new(prev)
    }
}

impl<F: Flash> FlashAddress<F> {
    /// Distance in bytes from `other` to `self` (i.e. `self - other`). Returns
    /// `Err(OutOfBounds)` if `other > self`. Accepts any `Into<FlashAddress>`,
    /// so you can pass a `FlashPageStart` or `FlashOffset` directly.
    pub fn distance_from(self, other: impl Into<FlashAddress<F>>) -> Result<usize> {
        let other: FlashAddress<F> = other.into();
        self.0.checked_sub(other.0).ok_or(Error::OutOfBounds)
    }
}

impl<F: Flash> Clone for FlashAddress<F> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<F: Flash> Copy for FlashAddress<F> {}
impl<F: Flash> PartialEq for FlashAddress<F> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<F: Flash> Eq for FlashAddress<F> {}
impl<F: Flash> PartialOrd for FlashAddress<F> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<F: Flash> Ord for FlashAddress<F> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
impl<F: Flash> core::fmt::Debug for FlashAddress<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "FlashAddress({:#x})", self.0)
    }
}
forward_usize_traits!(FlashAddress);

impl<F: Flash> From<FlashPageStart<F>> for FlashAddress<F> {
    fn from(p: FlashPageStart<F>) -> Self {
        Self(p.0, PhantomData)
    }
}

impl<F: Flash> From<FlashOffset<F>> for FlashAddress<F> {
    fn from(o: FlashOffset<F>) -> Self {
        Self(F::flash_base() + o.0, PhantomData)
    }
}

impl<F: Flash> From<FlashAddress<F>> for FlashOffset<F> {
    fn from(a: FlashAddress<F>) -> Self {
        Self(a.0 - F::flash_base(), PhantomData)
    }
}

impl<F: Flash> From<FlashPageStart<F>> for FlashOffset<F> {
    fn from(p: FlashPageStart<F>) -> Self {
        Self(p.0 - F::flash_base(), PhantomData)
    }
}

impl<F: Flash> TryFrom<FlashAddress<F>> for FlashPageStart<F> {
    type Error = Error;
    fn try_from(a: FlashAddress<F>) -> Result<Self> {
        if (a.0 - F::flash_base()) % F::page_size() != 0 {
            return Err(Error::Misaligned);
        }
        Ok(Self(a.0, PhantomData))
    }
}

impl<F: Flash> TryFrom<FlashOffset<F>> for FlashPageStart<F> {
    type Error = Error;
    fn try_from(o: FlashOffset<F>) -> Result<Self> {
        if o.0 % F::page_size() != 0 {
            return Err(Error::Misaligned);
        }
        Ok(Self(F::flash_base() + o.0, PhantomData))
    }
}

impl<F: Flash> From<FlashAddress<F>> for usize {
    fn from(a: FlashAddress<F>) -> Self {
        a.0
    }
}

impl<F: Flash> From<FlashOffset<F>> for usize {
    fn from(o: FlashOffset<F>) -> Self {
        o.0
    }
}

impl<F: Flash> From<FlashPageStart<F>> for usize {
    fn from(p: FlashPageStart<F>) -> Self {
        p.0
    }
}

// ---------------------------------------------------------------------------
// FlashOffset
// ---------------------------------------------------------------------------

/// Byte offset from `F::flash_base()`. Construction validates the offset is
/// within `[0, F::total_size())`.
pub struct FlashOffset<F: Flash>(usize, PhantomData<F>);

impl<F: Flash> FlashOffset<F> {
    pub fn new(offset: usize) -> Result<Self> {
        if offset >= F::total_size() {
            return Err(Error::OutOfBounds);
        }
        Ok(Self(offset, PhantomData))
    }

    pub fn as_usize(self) -> usize {
        self.0
    }

    pub fn page_index(self) -> usize {
        self.0 / F::page_size()
    }

    /// Add `bytes`, returning `Err(OutOfBounds)` if the result is `>= total_size()`.
    pub fn checked_add(self, bytes: usize) -> Result<Self> {
        let next = self.0.checked_add(bytes).ok_or(Error::OutOfBounds)?;
        Self::new(next)
    }

    /// Subtract `bytes`, returning `Err(OutOfBounds)` if the result underflows.
    pub fn checked_sub(self, bytes: usize) -> Result<Self> {
        let prev = self.0.checked_sub(bytes).ok_or(Error::OutOfBounds)?;
        Ok(Self(prev, PhantomData))
    }
}

impl<F: Flash> FlashOffset<F> {
    /// Distance in bytes from `other` to `self` (i.e. `self - other`). Returns
    /// `Err(OutOfBounds)` if `other > self`. Accepts any `Into<FlashOffset>`,
    /// so you can pass a `FlashAddress` or `FlashPageStart` directly.
    pub fn distance_from(self, other: impl Into<FlashOffset<F>>) -> Result<usize> {
        let other: FlashOffset<F> = other.into();
        self.0.checked_sub(other.0).ok_or(Error::OutOfBounds)
    }
}

impl<F: Flash> Clone for FlashOffset<F> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<F: Flash> Copy for FlashOffset<F> {}
impl<F: Flash> PartialEq for FlashOffset<F> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<F: Flash> Eq for FlashOffset<F> {}
impl<F: Flash> PartialOrd for FlashOffset<F> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<F: Flash> Ord for FlashOffset<F> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
impl<F: Flash> core::fmt::Debug for FlashOffset<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "FlashOffset({:#x})", self.0)
    }
}
forward_usize_traits!(FlashOffset);

// ---------------------------------------------------------------------------
// FlashPageStart
// ---------------------------------------------------------------------------

/// An absolute flash address aligned to a page boundary. Construction
/// validates both that the address is in flash and that it is page-aligned,
/// so any value of this type is a valid argument to a page-erase op.
pub struct FlashPageStart<F: Flash>(usize, PhantomData<F>);

impl<F: Flash> FlashPageStart<F> {
    pub fn new(address: usize) -> Result<Self> {
        let base = F::flash_base();
        let end = base
            .checked_add(F::total_size())
            .ok_or(Error::OutOfBounds)?;
        if address < base || address >= end {
            return Err(Error::OutOfBounds);
        }
        if (address - base) % F::page_size() != 0 {
            return Err(Error::Misaligned);
        }
        Ok(Self(address, PhantomData))
    }

    pub fn from_page_index(page_index: usize) -> Result<Self> {
        if page_index >= F::page_count() {
            return Err(Error::InvalidPage);
        }
        Ok(Self(
            F::flash_base() + page_index * F::page_size(),
            PhantomData,
        ))
    }

    pub fn as_usize(self) -> usize {
        self.0
    }

    pub fn page_index(self) -> usize {
        (self.0 - F::flash_base()) / F::page_size()
    }

    pub fn next(self) -> Result<Self> {
        Self::from_page_index(self.page_index() + 1)
    }

    pub fn prev(self) -> Result<Self> {
        let idx = self.page_index().checked_sub(1).ok_or(Error::InvalidPage)?;
        Self::from_page_index(idx)
    }

    pub fn add_pages(self, n: usize) -> Result<Self> {
        let idx = self.page_index().checked_add(n).ok_or(Error::InvalidPage)?;
        Self::from_page_index(idx)
    }

    pub fn sub_pages(self, n: usize) -> Result<Self> {
        let idx = self.page_index().checked_sub(n).ok_or(Error::InvalidPage)?;
        Self::from_page_index(idx)
    }

    /// Iterator over `count` consecutive pages starting at `self`.
    ///
    /// The whole range is validated up front: if any page in `[self,
    /// self + count)` would fall outside flash, this returns
    /// `Err(InvalidPage)` and no iteration happens. Once you have the
    /// `PageIter`, iterating it is infallible.
    pub fn iter_pages(self, count: usize) -> Result<PageIter<F>> {
        if count > 0 {
            // last page of the range must be in flash
            self.add_pages(count - 1)?;
        }
        Ok(PageIter {
            next: self,
            remaining: count,
        })
    }
}

/// Iterator yielded by [`FlashPageStart::iter_pages`]. The underlying range
/// is pre-validated, so this iterator never produces invalid pages.
pub struct PageIter<F: Flash> {
    next: FlashPageStart<F>,
    remaining: usize,
}

impl<F: Flash> Iterator for PageIter<F> {
    type Item = FlashPageStart<F>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        let current = self.next;
        self.remaining -= 1;
        if self.remaining > 0 {
            // pre-validated: the next page exists.
            self.next = current.next().expect("PageIter pre-validates the range");
        }
        Some(current)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<F: Flash> ExactSizeIterator for PageIter<F> {
    fn len(&self) -> usize {
        self.remaining
    }
}

impl<F: Flash> FlashPageStart<F> {
    /// Distance in *pages* from `other` to `self` (i.e. `self - other`).
    /// Returns `Err(InvalidPage)` if `other > self`.
    pub fn pages_from(self, other: Self) -> Result<usize> {
        self.page_index()
            .checked_sub(other.page_index())
            .ok_or(Error::InvalidPage)
    }
}

impl<F: Flash> Clone for FlashPageStart<F> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<F: Flash> Copy for FlashPageStart<F> {}
impl<F: Flash> PartialEq for FlashPageStart<F> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<F: Flash> Eq for FlashPageStart<F> {}
impl<F: Flash> PartialOrd for FlashPageStart<F> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<F: Flash> Ord for FlashPageStart<F> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}
impl<F: Flash> core::fmt::Debug for FlashPageStart<F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "FlashPageStart({:#x})", self.0)
    }
}
forward_usize_traits!(FlashPageStart);
