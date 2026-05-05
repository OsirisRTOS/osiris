use core::{ops::Range, ptr::NonNull};

use crate::hal::mem::PhysAddr;

use crate::error::Result;

/// The metadata that is before any block in the BestFitAllocator.
struct BestFitMeta {
    /// The size of the block in bytes.
    size: usize,
    /// The pointer to the next free block. This is `None` if the block is allocated.
    next: Option<NonNull<u8>>,
}

/// This is an allocator implementation that uses the best fit strategy.
/// That does mean, when we allocate a block, we try to find the smallest block that fits the requested size.
/// Blocks are stored in a singly linked list. The important part is that the linked list is stored in-line with the memory blocks.
/// This means that every block has a header that contains the size of the block and a pointer to the next block.
#[proc_macros::fmt]
pub struct BestFitAllocator {
    /// Head of the free block list.
    head: Option<NonNull<u8>>,
}

// Safety: BestFitAllocator is not Copy or Clone.
// BestFitAllocator owns all its data exclusively.
// The user must ensure that the returned pointers by malloc do not outlive the allocator.
unsafe impl Send for BestFitAllocator {}
// Safety: BestFitAllocator does only allow access to its data through &mut self.
unsafe impl Sync for BestFitAllocator {}

/// Implementation of the BestFitAllocator.
impl BestFitAllocator {
    pub const MIN_RANGE_SIZE: usize = size_of::<BestFitMeta>() + Self::align_up() + 1;

    /// Creates a new BestFitAllocator.
    ///
    /// Returns the new BestFitAllocator.
    pub const fn new() -> Self {
        Self { head: None }
    }

    /// Adds a range of memory to the allocator.
    ///
    /// `range` - The range of memory to add.
    ///
    /// Returns `Ok(())` if the range was added successfully, otherwise an error.
    ///
    /// # Safety
    ///
    /// The range must be valid, 128bit aligned and must not overlapping with any other current or future range.
    /// The range must also be at least as large as `MIN_RANGE_SIZE`.
    /// Also the range must stay valid, for the whole lifetime of the allocator. Also the lifetime of any allocation is only valid as long as the allocator is valid.
    pub unsafe fn add_range(&mut self, range: &Range<PhysAddr>) -> Result<()> {
        let ptr = range.start;

        // Check if the pointer is 128bit aligned.
        if !ptr.is_multiple_of(align_of::<u128>()) {
            return Err(kerr!(InvalidArgument));
        }

        if range.end.diff(range.start) < Self::MIN_RANGE_SIZE {
            return Err(kerr!(InvalidArgument));
        }

        debug_assert!(range.end > range.start);
        debug_assert!(range.end.diff(range.start) > size_of::<BestFitMeta>() + Self::align_up());
        debug_assert!(range.end.as_usize() <= isize::MAX as usize);

        // The user pointer is the pointer to the user memory. So we need to add the size of the meta data and possibly add padding.
        let user_pointer = ptr + size_of::<BestFitMeta>() + Self::align_up();

        // Set the current head as the next block, so we can add the new block to the head.
        let meta = BestFitMeta {
            size: range.end.diff(user_pointer),
            next: self.head,
        };

        // Write the header to the memory.
        unsafe { core::ptr::write(ptr.as_mut_ptr::<BestFitMeta>(), meta) };

        // Set the head to the new block.
        self.head = Some(unsafe { NonNull::new_unchecked(ptr.as_mut_ptr::<u8>()) });
        Ok(())
    }

    /// Calculates the padding required to align the block. Note: We only align to 128bit.
    ///
    /// Returns the padding in bytes.
    const fn align_up() -> usize {
        let meta = size_of::<BestFitMeta>();
        let align = align_of::<u128>();
        // Calculate the padding required to align the block.
        (align - (meta % align)) % align
    }

    /// Selects the best fit block for the given size.
    ///
    /// `size` - The size of the block.
    ///
    /// Returns the control pointer to the block and the control pointer to the previous block.
    fn select_block(
        &mut self,
        size: usize,
        requested: Option<PhysAddr>,
    ) -> Result<(NonNull<u8>, Option<NonNull<u8>>)> {
        let mut best_fit = Err(kerr!(OutOfMemory));
        let mut best_fit_size = usize::MAX;

        let mut current = self.head;
        let mut prev = None;

        if let Some(requested) = requested {
            while let Some(ptr) = current {
                // Get the metadata of the block.
                let meta = unsafe { ptr.cast::<BestFitMeta>().as_ref() };

                if unsafe { Self::contains(meta, requested, size) } {
                    return Ok((ptr, prev));
                }

                // Move to the next block.
                prev = current;
                current = meta.next;
            }
        }

        // Iterate over all blocks and find the best fit.
        while let Some(ptr) = current {
            // Get the metadata of the block.
            let meta = unsafe { ptr.cast::<BestFitMeta>().as_ref() };

            // Check if the block is big enough and smaller than the current best fit.
            if meta.size >= size && meta.size <= best_fit_size {
                best_fit = Ok((ptr, prev));
                best_fit_size = meta.size;
            }

            // Move to the next block.
            prev = current;
            current = meta.next;
        }

        best_fit
    }

    /// Calculates the user pointer from the control pointer.
    ///
    /// `ptr` - The control pointer.
    ///
    /// Returns the user pointer.
    ///
    /// # Safety
    ///
    /// The ptr must be a valid control pointer. Note: After the allocator which allocated the pointer is dropped, the control pointer is always considered invalid.
    unsafe fn user_ptr(ptr: NonNull<u8>) -> NonNull<u8> {
        debug_assert!(
            (ptr.as_ptr() as usize)
                <= isize::MAX as usize - size_of::<BestFitMeta>() - Self::align_up()
        );
        unsafe { ptr.byte_add(size_of::<BestFitMeta>() + Self::align_up()) }
    }

    /// Calculates the control pointer from the user pointer.
    ///
    /// `ptr` - The user pointer.
    ///
    /// Returns the control pointer.
    ///
    /// # Safety
    ///
    /// The ptr must be a valid user pointer. Note: After the allocator which allocated the pointer is dropped, the user pointer is always considered invalid.
    unsafe fn control_ptr(ptr: NonNull<u8>) -> NonNull<u8> {
        debug_assert!((ptr.as_ptr() as usize) > size_of::<BestFitMeta>() + Self::align_up());
        unsafe { ptr.byte_sub(size_of::<BestFitMeta>() + Self::align_up()) }
    }

    unsafe fn contains(meta: &BestFitMeta, target: PhysAddr, size: usize) -> bool {
        let begin = unsafe {
            Self::user_ptr(NonNull::new_unchecked(
                meta as *const BestFitMeta as *mut u8,
            ))
        };
        debug_assert!(size > 0);

        if target >= begin.into() {
            if let Some(target) = target.checked_add(size) {
                if target > (unsafe { begin.add(meta.size) }).into() {
                    return false;
                }
            } else {
                return false;
            }
            return true;
        }
        false
    }
}

/// Implementation of the Allocator trait for BestFitAllocator.
impl super::Allocator for BestFitAllocator {
    /// Allocates a block of memory with the given size and alignment. Note: This function will always yield an invalid align for align > 128bit.
    ///
    /// `size` - The size of the block.
    /// `align` - The alignment of the block.
    ///
    /// Returns the user pointer to the block if successful, otherwise an error.
    /// 
    /// # Safety
    ///
    /// The caller must ensure that the returned pointer is not used after the allocator is dropped.
    unsafe fn malloc<T>(
        &mut self,
        size: usize,
        align: usize,
        request: Option<PhysAddr>,
    ) -> Result<NonNull<T>> {
        // Check if the alignment is valid.
        if align == 0 || align > align_of::<u128>() {
            return Err(kerr!(InvalidAlign));
        }

        if let Some(request) = request {
            if !request.is_multiple_of(align) {
                return Err(kerr!(InvalidAlign));
            }
        }

        // Check if the size is valid.
        if size == 0 {
            return Err(kerr!(InvalidArgument));
        }

        // For some cfg this warning is correct. But for others its not.
        #[allow(clippy::absurd_extreme_comparisons)]
        if size >= super::MAX_ADDR {
            return Err(kerr!(InvalidArgument));
        }

        // Align the size.
        let aligned_size = super::super::align_up(size);
        debug_assert!(aligned_size >= size);
        debug_assert!(aligned_size <= isize::MAX as usize);

        // Find the best fit block.
        let (split, block, prev) = match self.select_block(aligned_size, request) {
            Ok((block, prev)) => {
                // Get the metadata of the block.
                let meta = unsafe { block.cast::<BestFitMeta>().as_mut() };

                // If we requested a specific address. The size must be extended by the offset from block start to the requested address.
                let aligned_size = if let Some(request) = request {
                    aligned_size + request.diff(unsafe { Self::user_ptr(block) }.into())
                } else {
                    aligned_size
                };

                // Calculate the amount of bytes until the beginning of the possibly next metadata.
                let min = aligned_size.saturating_add(size_of::<BestFitMeta>() + Self::align_up());

                debug_assert!(
                    (block.as_ptr() as usize)
                        <= isize::MAX as usize
                            - meta.size
                            - size_of::<BestFitMeta>()
                            - Self::align_up()
                );

                debug_assert!(
                    meta.size < isize::MAX as usize - size_of::<BestFitMeta>() - Self::align_up()
                );

                // If the block is big enough to split. Then it also needs to be big enough to store the metadata + align of the next block.
                if meta.size > min {
                    // Calculate the remaining size of the block and thus the next metadata.
                    let remaining_meta = BestFitMeta {
                        size: meta.size - min,
                        next: meta.next,
                    };

                    // Shrink the current block to the requested aligned_size + padding (which is not available to the user).
                    meta.size = aligned_size;

                    // Calculate the pointer to the next metadata.
                    let ptr = unsafe { Self::user_ptr(block).byte_add(aligned_size) };

                    unsafe {
                        // Write the new metadata to the memory.
                        ptr.cast::<BestFitMeta>().write(remaining_meta);
                    }

                    // If there is a previous block, we insert the new block after it. Otherwise we set it as the new head.
                    if let Some(prev) = prev {
                        let prev_meta = unsafe { prev.cast::<BestFitMeta>().as_mut() };
                        prev_meta.next = Some(ptr);
                    } else {
                        self.head = Some(ptr);
                    }

                    // The next block of an allocated block is always None.
                    meta.next = None;

                    (true, block, prev)
                } else {
                    (false, block, prev)
                }
            }
            Err(_) => {
                let (block, prev) = self.select_block(size, request)?;
                (false, block, prev)
            }
        };

        if !split {
            // Get the metadata of the block.
            let meta = unsafe { block.cast::<BestFitMeta>().as_mut() };

            if let Some(prev) = prev {
                let prev_meta = unsafe { prev.cast::<BestFitMeta>().as_mut() };
                // If there is a previous block, we remove the current block from the list. Ie. we set the next block of the previous block to the next block of the current block.
                prev_meta.next = meta.next;
            } else {
                // If there is no previous block, we set the next block as the new head.
                self.head = meta.next;
            }

            // The next block of an allocated block is always None.
            meta.next = None;
        }

        if let Some(request) = request {
            debug_assert!(unsafe {
                Self::contains(block.cast::<BestFitMeta>().as_ref(), request, size)
            });
        }

        // Return the user pointer.
        Ok(unsafe { Self::user_ptr(block).cast() })
    }

    /// Frees a block of memory.
    ///
    /// `ptr` - The pointer to the block.
    /// `size` - The size of the block. (This is used to check if the size of the block is correct.)
    unsafe fn free<T>(&mut self, ptr: NonNull<T>, size: usize) {
        let block = unsafe { Self::control_ptr(ptr.cast()) };
        let meta = unsafe { block.cast::<BestFitMeta>().as_mut() };

        // The next block of a free block is always the current head. We essentially insert the block at the beginning of the list.
        meta.next = self.head;

        // Check if the size of the block is correct.
        bug_on!(
            size > meta.size,
            "allocation size {} is larger than block size {}",
            size,
            meta.size
        );

        // Set the block as the new head.
        self.head = Some(block);
    }
}

// TESTING ------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::error::Kind;
    use crate::mem::align_up;

    use super::super::*;
    use super::*;

    fn verify_block(user_ptr: NonNull<u8>, size: usize, next: Option<NonNull<u8>>) {
        let control_ptr = unsafe { BestFitAllocator::control_ptr(user_ptr) };
        let meta = unsafe { control_ptr.cast::<BestFitMeta>().as_ref() };

        assert!(meta.size >= size);
        assert_eq!(meta.next, next);
    }

    fn verify_ptrs_not_overlaping(ptrs: &[(NonNull<u8>, usize)]) {
        for (i, (ptr1, size1)) in ptrs.iter().enumerate() {
            for (j, (ptr2, size2)) in ptrs.iter().enumerate() {
                if i == j {
                    continue;
                }

                let begin1 = ptr1.as_ptr() as usize;
                let end1 = begin1 + size1;
                let begin2 = ptr2.as_ptr() as usize;
                let end2 = begin2 + size2;

                assert!(end1 <= begin2 || end2 <= begin1);
                assert!(begin1 != begin2);
                assert!(end1 != end2);
                assert!(*size1 > 0);
                assert!(*size2 > 0);
                assert!(end1 > begin1);
                assert!(end2 > begin2);
            }
        }
    }

    fn alloc_range(length: usize) -> Range<PhysAddr> {
        let alloc_range = std::alloc::Layout::from_size_align(length, align_of::<u128>()).unwrap();
        let ptr = unsafe { std::alloc::alloc(alloc_range) };
        PhysAddr::new(ptr as usize)..PhysAddr::new(ptr as usize + length)
    }

    #[test]
    fn allocate_one() {
        let mut allocator = BestFitAllocator::new();

        let range = alloc_range(4096);
        unsafe {
            allocator.add_range(&range).unwrap();
        }

        let ptr = unsafe { allocator.malloc(128, 1, None).unwrap() };

        verify_block(ptr, 128, None);
    }

    #[test]
    fn alloc_request() {
        let mut allocator = BestFitAllocator::new();

        let range = alloc_range(4096);
        unsafe {
            allocator.add_range(&range).unwrap();
        }

        let request = range.start + 128;
        let ptr = unsafe { allocator.malloc::<u8>(128, 1, Some(request)).unwrap() };

        // Check that the returned pointer contains the requested address.
        let meta = unsafe {
            BestFitAllocator::control_ptr(ptr)
                .cast::<BestFitMeta>()
                .as_ref()
        };
        assert!(unsafe { BestFitAllocator::contains(meta, request, 128) });
    }

    #[test]
    fn alloc_request_to_big() {
        let mut allocator = BestFitAllocator::new();

        let range = alloc_range(4096);
        unsafe {
            allocator.add_range(&range).unwrap();
        }

        let request = range.start + 4096;
        let ptr = unsafe { allocator.malloc::<u8>(128, 1, Some(request)) };

        assert!(ptr.is_err_and(|e| e.kind == Kind::OutOfMemory));
    }

    #[test]
    fn alloc_request_not_aligned() {
        let mut allocator = BestFitAllocator::new();

        let range = alloc_range(4096);
        unsafe {
            allocator.add_range(&range).unwrap();
        }

        let request = range.start + 127;
        let ptr = unsafe { allocator.malloc::<u8>(128, 8, Some(request)) };

        assert!(ptr.is_err_and(|e| e.kind == Kind::InvalidAlign));
    }

    #[test]
    fn alloc_request_not_available() {
        let mut allocator = BestFitAllocator::new();

        let range = alloc_range(4096);
        unsafe {
            allocator.add_range(&range).unwrap();
        }

        let request = range.start + 128;
        let ptr = unsafe { allocator.malloc::<u8>(128, 1, Some(request)).unwrap() };
        verify_block(ptr, 128, None);

        let ptr = unsafe { allocator.malloc::<u8>(128, 1, Some(request)) };
        assert!(ptr.is_err_and(|e| e.kind == Kind::OutOfMemory));
    }

    #[test]
    fn alloc_request_out_of_range() {
        let mut allocator = BestFitAllocator::new();

        let range = alloc_range(4096);
        unsafe {
            allocator.add_range(&range).unwrap();
        }

        let request = range.end + 128;
        let ptr = unsafe { allocator.malloc::<u8>(128, 1, Some(request)) };

        assert!(ptr.is_err_and(|e| e.kind == Kind::OutOfMemory));
    }

    #[test]
    fn alloc_alot() {
        let mut allocator = BestFitAllocator::new();
        const CNT: usize = 100;
        const SIZE: usize = 128;

        let range = alloc_range(SIZE * CNT * 2);
        unsafe {
            allocator.add_range(&range).unwrap();
        }

        let mut ptrs = Vec::new();
        for _ in 0..CNT {
            let ptr = unsafe { allocator.malloc(SIZE, 1, None).unwrap() };
            verify_block(ptr, SIZE, None);
            ptrs.push((ptr, SIZE));
        }

        verify_ptrs_not_overlaping(ptrs.as_slice());
    }

    #[test]
    fn alloc_exact() {
        let mut allocator = BestFitAllocator::new();
        const CNT: usize = 10;
        const SIZE: usize = 128;

        let range =
            alloc_range((SIZE + size_of::<BestFitMeta>() + BestFitAllocator::align_up()) * CNT);
        unsafe {
            allocator.add_range(&range).unwrap();
        }

        let mut ptrs = Vec::new();
        for _ in 0..CNT {
            let ptr = unsafe { allocator.malloc(SIZE, 1, None).unwrap() };
            verify_block(ptr, SIZE, None);
            ptrs.push((ptr, SIZE));
        }

        verify_ptrs_not_overlaping(ptrs.as_slice());
    }

    #[test]
    fn alloc_oom() {
        let mut allocator = BestFitAllocator::new();
        const CNT: usize = 10;
        const SIZE: usize = 128;

        let range =
            alloc_range((SIZE + size_of::<BestFitMeta>() + BestFitAllocator::align_up()) * CNT - 1);
        unsafe {
            allocator.add_range(&range).unwrap();
        }

        let mut ptrs = Vec::new();
        for _ in 0..CNT - 1 {
            let ptr = unsafe { allocator.malloc(SIZE, 1, None).unwrap() };
            verify_block(ptr, SIZE, None);
            ptrs.push(ptr);
        }

        let ptr = unsafe { allocator.malloc::<u8>(SIZE, 1, None) };
        assert!(ptr.is_err_and(|e| e.kind == Kind::OutOfMemory));
    }

    #[test]
    fn alloc_no_oom_through_free() {
        let mut allocator = BestFitAllocator::new();
        const SIZE: usize = 128;

        let range = alloc_range(SIZE + size_of::<BestFitMeta>() + BestFitAllocator::align_up());
        unsafe {
            allocator.add_range(&range).unwrap();
        }

        let ptr = unsafe { allocator.malloc(SIZE, 1, None).unwrap() };
        verify_block(ptr, SIZE, None);

        unsafe {
            allocator.free(ptr, SIZE);
        }

        let ptr = unsafe { allocator.malloc(SIZE, 1, None).unwrap() };
        verify_block(ptr, SIZE, None);
    }

    #[test]
    fn multi_range_alloc() {
        let mut allocator = BestFitAllocator::new();
        const CNT: usize = 10;
        const SIZE: usize = 128;

        let mut ranges = Vec::new();
        for _ in 0..CNT {
            let range = alloc_range(SIZE + size_of::<BestFitMeta>() + BestFitAllocator::align_up());
            unsafe {
                allocator.add_range(&range).unwrap();
            }
            ranges.push(range);
        }

        let mut ptrs = Vec::new();
        for _ in 0..CNT {
            let ptr = unsafe { allocator.malloc(SIZE, 1, None).unwrap() };
            verify_block(ptr, SIZE, None);
            ptrs.push((ptr, SIZE));
        }

        verify_ptrs_not_overlaping(ptrs.as_slice());
    }

    #[test]
    fn multi_range_no_oom_through_free() {
        // This function allocates multiple ranges and then frees one of them randomly. And only then there is no oom.
        let mut allocator = BestFitAllocator::new();

        const CNT: usize = 10;
        const SIZE: usize = 128;

        let mut ranges = Vec::new();
        for _ in 0..CNT {
            let range = alloc_range(SIZE + size_of::<BestFitMeta>() + BestFitAllocator::align_up());
            unsafe {
                allocator.add_range(&range).unwrap();
            }
            ranges.push(range);
        }

        let mut ptrs = Vec::new();

        let ptr = unsafe { allocator.malloc::<u8>(SIZE, 1, None).unwrap() };

        for _ in 0..CNT - 1 {
            let ptr = unsafe { allocator.malloc(SIZE, 1, None).unwrap() };
            verify_block(ptr, SIZE, None);
            ptrs.push((ptr, SIZE));
        }

        unsafe {
            allocator.free(ptr, SIZE);
        }

        let ptr = unsafe { allocator.malloc(SIZE, 1, None).unwrap() };
        ptrs.push((ptr, SIZE));

        verify_ptrs_not_overlaping(ptrs.as_slice());
    }

    #[test]
    fn free_corrupts_metadata() {
        let mut allocator = BestFitAllocator::new();
        const SIZE: usize = 17;
        const ALIGNED: usize = 32;
        assert!(align_up(SIZE) == ALIGNED);

        let range = alloc_range(ALIGNED + size_of::<BestFitMeta>() + BestFitAllocator::align_up());
        unsafe {
            allocator.add_range(&range).unwrap();
        }

        let ptr1: core::ptr::NonNull<u8> = unsafe { allocator.malloc(SIZE, 1, None).unwrap() };

        unsafe {
            allocator.free(ptr1, SIZE);
        }

        let ptr2: core::ptr::NonNull<u8> = unsafe { allocator.malloc(SIZE, 1, None).unwrap() };

        unsafe {
            allocator.free(ptr2, SIZE);
        }
    }

    #[test]
    fn multi_range_oom() {
        // This function allocates multiple ranges and then frees one of them randomly. And only then there is no oom.
        let mut allocator = BestFitAllocator::new();

        const CNT: usize = 10;
        const SIZE: usize = 128;

        let mut ranges = Vec::new();
        for _ in 0..CNT {
            let range = alloc_range(SIZE + size_of::<BestFitMeta>() + BestFitAllocator::align_up());
            unsafe {
                allocator.add_range(&range).unwrap();
            }
            ranges.push(range);
        }

        let mut ptrs = Vec::new();

        for _ in 0..CNT {
            let ptr = unsafe { allocator.malloc(SIZE, 1, None).unwrap() };
            verify_block(ptr, SIZE, None);
            ptrs.push((ptr, SIZE));
        }

        let ptr = unsafe { allocator.malloc::<u8>(SIZE, 1, None) };
        assert!(ptr.is_err_and(|e| e.kind == Kind::OutOfMemory));

        verify_ptrs_not_overlaping(ptrs.as_slice());
    }
}

// END TESTING --------------------------------------------------------------------------------------------------------

// VERIFICATION -------------------------------------------------------------------------------------------------------
#[cfg(kani)]
mod verification {
    use super::*;
    use crate::mem::alloc::Allocator;
    use crate::mem::alloc::MAX_ADDR;

    fn verify_block(user_ptr: NonNull<u8>, size: usize, next: Option<NonNull<u8>>) {
        let control_ptr = unsafe { BestFitAllocator::control_ptr(user_ptr) };
        let meta = unsafe { control_ptr.cast::<BestFitMeta>().as_ref() };

        assert!(meta.size >= size);
        assert_eq!(meta.next, next);
    }

    fn alloc_range(length: usize) -> Option<Range<PhysAddr>> {
        let alloc_range = std::alloc::Layout::from_size_align(length, align_of::<u128>()).unwrap();
        let ptr = unsafe { std::alloc::alloc(alloc_range) };

        if ptr.is_null() || ((ptr as usize) >= isize::MAX as usize - length) {
            None
        } else {
            Some(PhysAddr::new(ptr as usize)..PhysAddr::new(ptr as usize + length))
        }
    }

    #[kani::proof]
    #[kani::unwind(2)]
    fn allocate_one() {
        let mut allocator = BestFitAllocator::new();

        let size: usize = kani::any();
        kani::assume(size < MAX_ADDR - size_of::<BestFitMeta>() - BestFitAllocator::align_up());
        kani::assume(size > 0);
        let larger_size: usize = kani::any_where(|&x| {
            x > size + size_of::<BestFitMeta>() + BestFitAllocator::align_up() && x < MAX_ADDR
        });

        if let Some(range) = alloc_range(larger_size) {
            unsafe {
                assert_eq!(allocator.add_range(&range), Ok(()));
            }

            let ptr = allocator.malloc(size, 1, None).unwrap();

            verify_block(ptr, size, None);
        }
    }
}
// END VERIFICATION ---------------------------------------------------------------------------------------------------
