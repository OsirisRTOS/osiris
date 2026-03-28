use core::pin::Pin;
use core::ptr::NonNull;

use hal::mem::PhysAddr;

use crate::{
    error::Result, types::boxed::{self, Box}
};

pub struct Allocator<const N: usize> {
    begin: PhysAddr,
    l1: [usize; N],
}

impl<const N: usize> Allocator<N> {
    const BITS_PER_WORD: usize = usize::BITS as usize;

    pub fn new(begin: PhysAddr) -> Option<Self> {
        if !begin.is_multiple_of(super::PAGE_SIZE) {
            return None;
        }

        if begin > PhysAddr::MAX - (N * super::PAGE_SIZE * usize::BITS as usize) {
            return None;
        }

        Some(Self {
            begin,
            l1: [!0; N], // All bits are set to 1, meaning all pages are free.
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
            unsafe { core::ptr::write(ptr.as_ptr(), Self::new(begin).ok_or(kerr!(InvalidArgument))?) };

            // Safety: Ptr is properly aligned and non-null. The validity of the memory at that address is valid by the call contract.
            Ok(Pin::new(unsafe { boxed::Box::from_raw(ptr) }))
        }
    }

    fn alloc(&mut self, page_count: usize) -> Option<PhysAddr> {
        // If a bit is 1 the page is free. If a bit is 0 the page is allocated.
        let mut start = 0;
        let mut len = 0usize;

        let rem = page_count.saturating_sub(Self::BITS_PER_WORD);
        let mask = (!0usize).unbounded_shl((Self::BITS_PER_WORD.saturating_sub(page_count)) as u32);

        for idx in 0..N {
            if self.l1[idx] == 0 {
                len = 0;
                continue;
            }

            let mut byte = self.l1[idx];

            let mut shift = if len > 0 {
                0usize
            } else {
                byte.leading_zeros() as usize
            };

            byte <<= shift;

            while shift < Self::BITS_PER_WORD {
                // Make the mask smaller if we already have some contiguous bits.
                let mask = if rem.saturating_sub(len) == 0 {
                    mask << (len - rem)
                } else {
                    mask
                };

                // We shifted byte to MSB, mask is already aligned to the left.
                // We compare them via and and shift to the right to shift out extra bits from the mask that would overflow into the next word.
                let mut found = (byte & mask) >> shift;

                // We also need to shift the mask to the right so that we can compare mask and found.
                if found == (mask >> shift) {
                    if len == 0 {
                        start = idx * Self::BITS_PER_WORD + shift;
                    }

                    // Shift completely to the right.
                    found >>= found.trailing_zeros();

                    // As all found bits are now on the right we can just count them to get the amount we found.
                    len += found.trailing_ones() as usize;
                    // Continue to the next word if we haven't found enough bits yet.
                    break;
                } else {
                    len = 0;
                }

                shift += 1;
                byte <<= 1;
            }

            if len >= page_count {
                // Mark the allocated pages as used.
                let mut idx = start / Self::BITS_PER_WORD;

                // Mark all bits in the first word as used.
                {
                    let skip = start % Self::BITS_PER_WORD;
                    let rem = len.min(Self::BITS_PER_WORD) - skip;

                    self.l1[idx] &= !((!0usize).unbounded_shl((Self::BITS_PER_WORD - rem) as u32) >> skip);

                    if len <= rem {
                        return Some(self.begin + (start * super::PAGE_SIZE));
                    }

                    len -= rem;
                    idx += 1;
                }

                // Mark all bits in the middle words as used.
                {
                    let mid_cnt = len / Self::BITS_PER_WORD;
                
                    for i in 0..mid_cnt {
                        self.l1[idx + i] = 0;
                    }

                    idx += mid_cnt;
                }
                
                // Mark the remaining bits in the last word as used.
                self.l1[idx] &= !((!0usize).unbounded_shl((Self::BITS_PER_WORD - (len % Self::BITS_PER_WORD)) as u32));
                return Some(self.begin + (start * super::PAGE_SIZE));
            }
        }

        None
    }

    fn free(&mut self, addr: PhysAddr, page_count: usize) {
        if !addr.is_multiple_of(super::PAGE_SIZE) {
            panic!("Address must be page aligned");
        }

        let mut idx = (addr.as_usize() - self.begin.as_usize()) / super::PAGE_SIZE / Self::BITS_PER_WORD;
        let mut bit_idx = ((addr.as_usize() - self.begin.as_usize()) / super::PAGE_SIZE) % Self::BITS_PER_WORD;

        // TODO: slow
        for _ in 0..page_count {
            self.l1[idx] |= 1 << (Self::BITS_PER_WORD - 1 - bit_idx);

            bit_idx += 1;

            if bit_idx == Self::BITS_PER_WORD {
                bit_idx = 0;
                idx += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_pattern() {
        const ITARATIONS: usize = 1000;

        for i in 0..ITARATIONS {
            const N: usize = 1024;
            const BITS: usize = Allocator::<N>::BITS_PER_WORD;
            const ALLOC_SIZE: usize = 100;

            let mut allocator = Allocator::<N>::new(PhysAddr::new(0x0)).unwrap();

            // Generate a random bit pattern.
            for i in 0..N {
                let is_zero = rand::random::<bool>();

                if is_zero {
                    allocator.l1[i / BITS] &= !(1 << ((BITS - 1) - (i % BITS)));
                }
            }

            // Place a run of ALLOC_SIZE contiguous bits set to 1 at a random position.
            let start = rand::random::<usize>() % (N - ALLOC_SIZE);
            for i in start..(start + ALLOC_SIZE) {
                allocator.l1[i / BITS] |= 1 << ((BITS - 1) - (i % BITS));
            }

            let pre = allocator.l1.clone();

            let addr = super::super::Allocator::alloc(&mut allocator, ALLOC_SIZE).unwrap();
            let idx = addr.as_usize() / super::super::PAGE_SIZE;

            // Check that the bits in returned addresses is all ones in pre.
            for i in 0..ALLOC_SIZE {
                let bit = (pre[(idx + i) / BITS] >> ((BITS - 1) - ((idx + i) % BITS))) & 1;
                assert_eq!(bit, 1, "Bit at index {} is not set", idx + i);
            }

            // Check that the bits in returned addresses is all zeros in allocator.l1.
            for i in 0..ALLOC_SIZE {
                let bit = (allocator.l1[(idx + i) / BITS] >> ((BITS - 1) - ((idx + i) % BITS))) & 1;
                assert_eq!(bit, 0, "Bit at index {} is not cleared", idx + i);
            }
        }
    }
}