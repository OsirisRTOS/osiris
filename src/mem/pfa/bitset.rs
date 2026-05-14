use core::pin::Pin;
use core::ptr::NonNull;

use crate::hal::mem::PhysAddr;

use crate::{
    error::Result,
    types::{
        bitset::BitAlloc,
        boxed::{self, Box},
    },
};

pub struct Allocator<const N: usize> {
    begin: PhysAddr,
    bitalloc: BitAlloc<N>,
}

impl<const WORDS: usize> Allocator<WORDS> {
    pub fn new(begin: PhysAddr) -> Option<Self> {
        if !begin.is_multiple_of(super::PAGE_SIZE) {
            return None;
        }

        if begin > PhysAddr::MAX - (WORDS * super::PAGE_SIZE * usize::BITS as usize) {
            return None;
        }

        Some(Self {
            begin,
            bitalloc: BitAlloc::new(WORDS * BitAlloc::<WORDS>::BITS_PER_WORD)?,
        })
    }
}

impl<const WORDS: usize> super::Allocator<WORDS> for Allocator<WORDS> {
    fn initializer() -> unsafe fn(PhysAddr, usize) -> Result<Pin<Box<Self>>> {
        |addr: PhysAddr, pcnt: usize| -> Result<Pin<Box<Self>>> {
            if pcnt > WORDS {
                todo!("Runtime page frame allocator for more than {} pages", WORDS)
            }

            if !addr.is_multiple_of(core::mem::align_of::<Self>()) {
                return Err(kerr!(EINVAL));
            }

            let ptr = NonNull::new(addr.as_mut_ptr::<Self>()).ok_or(kerr!(EINVAL))?;
            // Align this up to PAGE_SIZE
            let begin = addr + size_of::<Self>();
            let begin = if begin.is_multiple_of(super::PAGE_SIZE) {
                begin
            } else {
                PhysAddr::new((begin.as_usize() + super::PAGE_SIZE - 1) & !(super::PAGE_SIZE - 1))
            };
            // TODO: Subtract the needed pages from the available
            unsafe { core::ptr::write(ptr.as_ptr(), Self::new(begin).ok_or(kerr!(EINVAL))?) };

            // Safety: Ptr is properly aligned and non-null. The validity of the memory at that address is valid by the call contract.
            Ok(Pin::new(unsafe { boxed::Box::from_raw(ptr) }))
        }
    }

    fn alloc(&mut self, page_count: usize) -> Option<PhysAddr> {
        let idx = self.bitalloc.alloc(page_count)?;
        Some(self.begin + (idx * super::PAGE_SIZE))
    }

    fn free(&mut self, addr: PhysAddr, page_count: usize) {
        bug_on!(
            !addr.is_multiple_of(super::PAGE_SIZE),
            "free address {} is not page-aligned",
            addr
        );
        // diff() is absolute, so a sub-begin address would silently map to a
        // bit elsewhere in the bitmap.
        bug_on!(
            addr < self.begin,
            "free address {} below allocator begin {}",
            addr,
            self.begin
        );
        let idx = addr.diff(self.begin) / super::PAGE_SIZE;
        self.bitalloc.free(idx, page_count);
    }
}

#[cfg(test)]
mod tests {
    use super::super::Allocator as _;
    use super::*;

    fn test_begin() -> PhysAddr {
        let layout = std::alloc::Layout::from_size_align(
            2 * 64 * super::super::PAGE_SIZE,
            super::super::PAGE_SIZE,
        )
        .unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        PhysAddr::new(ptr as usize)
    }

    #[test]
    fn alloc_free_roundtrip() {
        let begin = test_begin();
        let mut alloc = Allocator::<2>::new(begin).unwrap();

        let a = alloc.alloc(1).unwrap();
        let b = alloc.alloc(1).unwrap();
        assert_ne!(a, b);

        alloc.free(a, 1);
        let c = alloc.alloc(1).unwrap();
        assert_eq!(a, c, "freed page is returned by next alloc");
    }

    #[test]
    fn alloc_returns_addresses_in_range() {
        let begin = test_begin();
        let mut alloc = Allocator::<1>::new(begin).unwrap();
        let end = begin + 64 * super::super::PAGE_SIZE;

        while let Some(addr) = alloc.alloc(1) {
            assert!(
                addr >= begin && addr < end,
                "addr {addr} outside [{begin}, {end})"
            );
            assert!(
                addr.is_multiple_of(super::super::PAGE_SIZE),
                "addr {addr} not page-aligned"
            );
        }
    }

    #[test]
    #[should_panic(expected = "below allocator begin")]
    fn free_below_begin_panics() {
        let begin = test_begin() + super::super::PAGE_SIZE;
        let mut alloc = Allocator::<2>::new(begin).unwrap();
        // diff() is absolute, so without the bound check a sub-begin address
        // would silently clear a bit elsewhere in the bitmap.
        alloc.free(begin - super::super::PAGE_SIZE, 1);
    }
}
