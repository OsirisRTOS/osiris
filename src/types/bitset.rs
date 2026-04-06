//! A simple bitset allocator that can be used to allocate contiguous runs of bits.

use crate::types::array::Vec;

pub struct BitAlloc<const N: usize> {
    l1: Vec<usize, N>,
}

impl<const N: usize> BitAlloc<N> {
    pub const BITS_PER_WORD: usize = usize::BITS as usize;

    pub fn new(free_count: usize) -> Option<Self> {
        let mut l1 = Vec::new();
        let words = free_count.div_ceil(Self::BITS_PER_WORD);

        for i in 0..words {
            let rem = free_count.saturating_sub(i * Self::BITS_PER_WORD);
            if rem >= Self::BITS_PER_WORD {
                l1.push(!0usize).ok()?;
            } else {
                l1.push((!0usize).unbounded_shl((Self::BITS_PER_WORD - rem) as u32))
                    .ok()?;
            }
        }

        Some(Self { l1 })
    }

    pub const fn from_array(arr: [usize; N]) -> Self {
        Self {
            l1: Vec::from_array(arr),
        }
    }

    pub fn alloc(&mut self, bit_count: usize) -> Option<usize> {
        // If a bit is 1 the bit is free. If a bit is 0 the bit is allocated.
        let mut start = 0;
        let mut len = 0usize;

        let rem = bit_count.saturating_sub(Self::BITS_PER_WORD);
        let mask = (!0usize).unbounded_shl((Self::BITS_PER_WORD.saturating_sub(bit_count)) as u32);

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

            if len >= bit_count {
                // Mark the allocated pages as used.
                let mut idx = start / Self::BITS_PER_WORD;

                // Mark all bits in the first word as used.
                {
                    let skip = start % Self::BITS_PER_WORD;
                    let rem = (Self::BITS_PER_WORD - skip).min(len);

                    self.l1[idx] &=
                        !((!0usize).unbounded_shl((Self::BITS_PER_WORD - rem) as u32) >> skip);

                    if len <= rem {
                        return Some(start);
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
                self.l1[idx] &= !((!0usize)
                    .unbounded_shl((Self::BITS_PER_WORD - (len % Self::BITS_PER_WORD)) as u32));
                return Some(start);
            }
        }

        None
    }

    pub fn free(&mut self, bit: usize, bit_count: usize) {
        let mut idx = bit / Self::BITS_PER_WORD;
        let mut bit_idx = bit % Self::BITS_PER_WORD;

        // TODO: slow
        for _ in 0..bit_count {
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
    fn lsb_no_underflow_works() {
        let mut alloc = BitAlloc::<1>::new(1).unwrap();
        // Only the LSB in word 0 is free
        alloc.l1[0] = 1;
        let result = alloc.alloc(1);

        assert!(result.is_some());
    }

    #[test]
    fn msb_no_underflow_works() {
        let mut alloc = BitAlloc::<1>::new(1).unwrap();
        // Only the MSB in word 0 is free
        alloc.l1[0] = 1 << (BitAlloc::<1>::BITS_PER_WORD - 1);
        let result = alloc.alloc(1);

        assert!(result.is_some());
    }

    #[test]
    fn test_random_pattern() {
        const ITARATIONS: usize = 10000;

        for _ in 0..ITARATIONS {
            const N: usize = 1024;
            const BITS: usize = BitAlloc::<N>::BITS_PER_WORD;

            let alloc_size = rand::random::<usize>() % (N / 2) + 1;

            let mut alloc = BitAlloc::<N>::new(N).unwrap();

            // Generate a random bit pattern.
            for i in 0..N {
                let is_zero = rand::random::<bool>();

                if is_zero {
                    alloc.l1[i / BITS] &= !(1 << ((BITS - 1) - (i % BITS)));
                }
            }

            // Place a run of alloc_size contiguous bits set to 1 at a random position.
            let start = rand::random::<usize>() % (N - alloc_size);
            for i in start..(start + alloc_size) {
                alloc.l1[i / BITS] |= 1 << ((BITS - 1) - (i % BITS));
            }

            let pre = alloc.l1.clone();
            let idx = alloc.alloc(alloc_size).expect("Failed to allocate bits");

            // Check that the bits in returned indices is all ones in pre.
            for i in 0..alloc_size {
                let bit = (pre[(idx + i) / BITS] >> ((BITS - 1) - ((idx + i) % BITS))) & 1;
                assert_eq!(bit, 1, "Bit at index {} is not set", idx + i);
            }

            // Check that the bits in returned indices is all zeros in allocator.l1.
            for i in 0..alloc_size {
                let bit = (alloc.l1[(idx + i) / BITS] >> ((BITS - 1) - ((idx + i) % BITS))) & 1;
                assert_eq!(bit, 0, "Bit at index {} is not cleared", idx + i);
            }

            // Check that the bits in other indices are unchanged.
            for i in 0..N {
                if i >= idx && i < idx + alloc_size {
                    continue;
                }
                let pre_bit = (pre[i / BITS] >> ((BITS - 1) - (i % BITS))) & 1;
                let post_bit = (alloc.l1[i / BITS] >> ((BITS - 1) - (i % BITS))) & 1;
                assert_eq!(pre_bit, post_bit, "Bit at index {} was modified", i);
            }
        }
    }
}
