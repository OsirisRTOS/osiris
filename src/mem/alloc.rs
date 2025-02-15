//! This module provides a simple allocator.
//! One implementation is the BestFitAllocator, which uses the best fit strategy.

use core::{ops::Range, ptr::NonNull};

use crate::{utils, BUG_ON};

/// Allocator trait that provides a way to allocate and free memory.
/// Normally you don't need to use this directly, rather use the `boxed::Box` type.
///
/// # Safety
///
/// Every block returned by `malloc` must be freed by `free` exactly once.
/// A pointer allocated by one allocator must not be freed by another allocator.
/// Each range added to the allocator must be valid for the whole lifetime of the allocator and must not overlap with any other range.
/// The lifetime of any allocation is only valid as long as the allocator is valid. (A pointer must not be used after the allocator is dropped.)
pub trait Allocator {
    fn malloc(&mut self, size: usize, align: usize) -> Result<NonNull<u8>, utils::KernelError>;
    unsafe fn free(&mut self, ptr: NonNull<u8>, size: usize);
}

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
pub struct BestFitAllocator {
    /// Head of the free block list.
    head: Option<NonNull<u8>>,
}

/// Implementation of the BestFitAllocator.
impl BestFitAllocator {
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
    /// Also the range must stay valid, for the whole lifetime of the allocator. Also the lifetime of any allocation is only valid as long as the allocator is valid.
    pub unsafe fn add_range(&mut self, range: Range<usize>) -> Result<(), utils::KernelError> {
        let ptr = range.start;

        // Check if the pointer is 128bit aligned.
        if ptr % align_of::<u128>() != 0 {
            return Err(utils::KernelError::InvalidAlign);
        }

        // The user pointer is the pointer to the user memory. So we need to add the size of the meta data and possibly add padding.
        let user_pointer = ptr + size_of::<BestFitMeta>() + Self::align_up();

        // Set the current head as the next block, so we can add the new block to the head.
        let meta = BestFitMeta {
            size: range.end - user_pointer,
            next: self.head,
        };

        // Write the header to the memory.
        core::ptr::write(ptr as *mut BestFitMeta, meta);

        // Set the head to the new block.
        self.head = Some(unsafe { NonNull::new_unchecked(ptr as *mut u8) });
        Ok(())
    }

    /// Calculates the padding required to align the block. Note: We only align to 128bit.
    ///
    /// Returns the padding in bytes.
    fn align_up() -> usize {
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
    ) -> Result<(NonNull<u8>, Option<NonNull<u8>>), utils::KernelError> {
        let mut best_fit = Err(utils::KernelError::OutOfMemory);
        let mut best_fit_size = usize::MAX;

        let mut current = self.head;
        let mut prev = None;

        // Iterate over all blocks and find the best fit.
        while let Some(ptr) = current {
            // Get the metadata of the block.
            let meta = unsafe { ptr.cast::<BestFitMeta>().as_ref() };

            // Check if the block is big enough and smaller than the current best fit.
            if meta.size >= size && meta.size < best_fit_size {
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
        ptr.byte_add(size_of::<BestFitMeta>() + Self::align_up())
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
        ptr.byte_sub(size_of::<BestFitMeta>() + Self::align_up())
    }
}

/// Implementation of the Allocator trait for BestFitAllocator.
impl Allocator for BestFitAllocator {
    /// Allocates a block of memory with the given size and alignment. Note: This function will always yield an invalid align for align > 128bit.
    ///
    /// `size` - The size of the block.
    /// `align` - The alignment of the block.
    ///
    /// Returns the user pointer to the block if successful, otherwise an error.
    fn malloc(&mut self, size: usize, align: usize) -> Result<NonNull<u8>, utils::KernelError> {
        // Check if the alignment is valid.
        if align > align_of::<u128>() {
            return Err(utils::KernelError::InvalidAlign);
        }

        // Align the size.
        let size = super::align_up(size);

        // Find the best fit block.
        let (block, prev) = self.select_block(size)?;

        // Get the metadata of the block.
        let meta = unsafe { block.cast::<BestFitMeta>().as_mut() };

        // Calculate the amount of bytes until the beginning of the possibly next metadata.
        let min = size_of::<BestFitMeta>() + Self::align_up() + size;

        // If the block is big enough to split. Then it also needs to be big enough to store the metadata + align of the next block.
        if meta.size > min + size_of::<BestFitMeta>() + Self::align_up() {
            // Calculate the remaining size of the block and thus the next metadata.
            let remaining_meta = BestFitMeta {
                size: meta.size - min,
                next: meta.next,
            };

            // Shrink the current block to the requested size + padding (which is not available to the user).
            meta.size = size;

            // Calculate the pointer to the next metadata.
            let ptr = unsafe { block.byte_add(min) };

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
        } else if let Some(prev) = prev {
            let prev_meta = unsafe { prev.cast::<BestFitMeta>().as_mut() };

            // If there is a previous block, we remove the current block from the list. Ie. we set the next block of the previous block to the next block of the current block.
            prev_meta.next = meta.next;
        } else {
            // If there is no previous block, we set the next block as the new head.
            self.head = meta.next;
        }

        // The next block of an allocated block is always None.
        meta.next = None;

        // Return the user pointer.
        Ok(unsafe { Self::user_ptr(block) })
    }

    /// Frees a block of memory.
    ///
    /// `ptr` - The pointer to the block.
    /// `size` - The size of the block. (This is used to check if the size of the block is correct.)
    unsafe fn free(&mut self, ptr: NonNull<u8>, size: usize) {
        let block = Self::control_ptr(ptr);
        let meta = block.cast::<BestFitMeta>().as_mut();

        // The next block of a free block is always the current head. We essentially insert the block at the beginning of the list.
        meta.next = self.head;

        // Check if the size of the block is correct.
        BUG_ON!(meta.size != size, "Invalid size in free()");

        // Set the size of the block.
        meta.size = size;

        // Set the block as the new head.
        self.head = Some(block);
    }
}

// TESTING ------------------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn verify_block(user_ptr: NonNull<u8>, size: usize, next: Option<NonNull<u8>>) {
        let control_ptr = unsafe { BestFitAllocator::control_ptr(user_ptr) };
        let meta = unsafe { control_ptr.cast::<BestFitMeta>().as_ref() };

        assert_eq!(meta.size, size);
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

    fn alloc_range(length: usize) -> Range<usize> {
        let alloc_range = std::alloc::Layout::from_size_align(length, 1).unwrap();
        let ptr = unsafe { std::alloc::alloc(alloc_range) };
        ptr as usize..ptr as usize + length
    }

    #[test]
    fn allocate_one() {
        let mut allocator = BestFitAllocator::new();

        let range = alloc_range(4096);
        unsafe {
            allocator.add_range(range).unwrap();
        }

        let ptr = allocator.malloc(128, 1).unwrap();

        verify_block(ptr, 128, None);
    }

    #[test]
    fn alloc_alot() {
        let mut allocator = BestFitAllocator::new();
        const CNT: usize = 100;
        const SIZE: usize = 128;

        let range = alloc_range(SIZE * CNT * 2);
        unsafe {
            allocator.add_range(range).unwrap();
        }

        let mut ptrs = Vec::new();
        for _ in 0..CNT {
            let ptr = allocator.malloc(SIZE, 1).unwrap();
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
            allocator.add_range(range).unwrap();
        }

        let mut ptrs = Vec::new();
        for _ in 0..CNT {
            let ptr = allocator.malloc(SIZE, 1).unwrap();
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
            allocator.add_range(range).unwrap();
        }

        let mut ptrs = Vec::new();
        for _ in 0..CNT - 1 {
            let ptr = allocator.malloc(SIZE, 1).unwrap();
            verify_block(ptr, SIZE, None);
            ptrs.push(ptr);
        }

        let ptr = allocator.malloc(SIZE, 1);
        assert!(ptr.is_err_and(|e| e == utils::KernelError::OutOfMemory));
    }

    #[test]
    fn alloc_no_oom_through_free() {
        let mut allocator = BestFitAllocator::new();
        const SIZE: usize = 128;

        let range = alloc_range(SIZE + size_of::<BestFitMeta>() + BestFitAllocator::align_up());
        unsafe {
            allocator.add_range(range).unwrap();
        }

        let ptr = allocator.malloc(SIZE, 1).unwrap();
        verify_block(ptr, SIZE, None);

        unsafe {
            allocator.free(ptr, SIZE);
        }

        let ptr = allocator.malloc(SIZE, 1).unwrap();
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
                allocator.add_range(range.clone()).unwrap();
            }
            ranges.push(range);
        }

        let mut ptrs = Vec::new();
        for _ in 0..CNT {
            let ptr = allocator.malloc(SIZE, 1).unwrap();
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
                allocator.add_range(range.clone()).unwrap();
            }
            ranges.push(range);
        }

        let mut ptrs = Vec::new();

        let ptr = allocator.malloc(SIZE, 1).unwrap();

        for _ in 0..CNT - 1 {
            let ptr = allocator.malloc(SIZE, 1).unwrap();
            verify_block(ptr, SIZE, None);
            ptrs.push((ptr, SIZE));
        }

        unsafe {
            allocator.free(ptr, SIZE);
        }

        let ptr = allocator.malloc(SIZE, 1).unwrap();
        ptrs.push((ptr, SIZE));

        verify_ptrs_not_overlaping(ptrs.as_slice());
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
                allocator.add_range(range.clone()).unwrap();
            }
            ranges.push(range);
        }

        let mut ptrs = Vec::new();

        for _ in 0..CNT {
            let ptr = allocator.malloc(SIZE, 1).unwrap();
            verify_block(ptr, SIZE, None);
            ptrs.push((ptr, SIZE));
        }

        let ptr = allocator.malloc(SIZE, 1);
        assert!(ptr.is_err_and(|e| e == utils::KernelError::OutOfMemory));

        verify_ptrs_not_overlaping(ptrs.as_slice());
    }
}

// END TESTING --------------------------------------------------------------------------------------------------------

// VERIFICATION -------------------------------------------------------------------------------------------------------
#[cfg(kani)]
mod verification {
    use super::*;
    use core::alloc::Layout;

    fn verify_block(user_ptr: NonNull<u8>, size: usize, next: Option<NonNull<u8>>) {
        let control_ptr = unsafe { BestFitAllocator::control_ptr(user_ptr) };
        let meta = unsafe { control_ptr.cast::<BestFitMeta>().as_ref() };

        assert_eq!(meta.size, size);
        assert_eq!(meta.next, next);
    }

    fn alloc_range(length: usize) -> Range<usize> {
        let alloc_range = std::alloc::Layout::from_size_align(length, 1).unwrap();
        let ptr = unsafe { std::alloc::alloc(alloc_range) };
        ptr as usize..ptr as usize + length
    }

    #[kani::proof]
    #[kani::unwind(2)]
    fn allocate_one() {
        let mut allocator = BestFitAllocator::new();

        let size: usize = kani::any();
        kani::assume(size < usize::MAX - size_of::<BestFitMeta>() - BestFitAllocator::align_up());
        let larger_size: usize = kani::any_where(|&x| x > size + size_of::<BestFitMeta>() + BestFitAllocator::align_up());

        let range = alloc_range(larger_size);
        unsafe {
            allocator.add_range(range).unwrap();
        }

        let ptr = allocator.malloc(size, 1).unwrap();

        verify_block(ptr, size, None);
    }
}
// END VERIFICATION ---------------------------------------------------------------------------------------------------
