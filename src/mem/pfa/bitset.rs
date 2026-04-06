use core::pin::Pin;
use core::ptr::NonNull;

use hal::mem::PhysAddr;

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

impl<const N: usize> Allocator<N> {
    pub fn new(begin: PhysAddr) -> Option<Self> {
        if !begin.is_multiple_of(super::PAGE_SIZE) {
            return None;
        }

        if begin > PhysAddr::MAX - (N * super::PAGE_SIZE * usize::BITS as usize) {
            return None;
        }

        Some(Self {
            begin,
            bitalloc: BitAlloc::new(N * BitAlloc::<N>::BITS_PER_WORD)?,
        })
    }
}

impl<const N: usize> super::Allocator<N> for Allocator<N> {
    fn initializer() -> unsafe fn(PhysAddr, usize) -> Result<Pin<Box<Self>>> {
        |addr: PhysAddr, pcnt: usize| -> Result<Pin<Box<Self>>> {
            if pcnt > N {
                todo!("Runtime page frame allocator for more than {} pages", N)
            }

            if !addr.is_multiple_of(core::mem::align_of::<Self>()) {
                return Err(kerr!(InvalidArgument));
            }

            let ptr = NonNull::new(addr.as_mut_ptr::<Self>()).ok_or(kerr!(InvalidArgument))?;
            // Align this up to PAGE_SIZE
            let begin = addr + size_of::<Self>();
            let begin = if begin.is_multiple_of(super::PAGE_SIZE) {
                begin
            } else {
                PhysAddr::new((begin.as_usize() + super::PAGE_SIZE - 1) & !(super::PAGE_SIZE - 1))
            };
            // TODO: Subtract the needed pages from the available
            unsafe {
                core::ptr::write(
                    ptr.as_ptr(),
                    Self::new(begin).ok_or(kerr!(InvalidArgument))?,
                )
            };

            // Safety: Ptr is properly aligned and non-null. The validity of the memory at that address is valid by the call contract.
            Ok(Pin::new(unsafe { boxed::Box::from_raw(ptr) }))
        }
    }

    fn alloc(&mut self, page_count: usize) -> Option<PhysAddr> {
        let idx = self.bitalloc.alloc(page_count)?;
        Some(self.begin + (idx * super::PAGE_SIZE))
    }

    fn free(&mut self, addr: PhysAddr, page_count: usize) {
        if !addr.is_multiple_of(super::PAGE_SIZE) {
            panic!("Address must be page aligned");
        }
        let idx = addr.diff(self.begin) / super::PAGE_SIZE;
        self.bitalloc.free(idx, page_count);
    }
}
