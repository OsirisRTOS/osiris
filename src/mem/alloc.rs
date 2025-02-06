use core::{fmt::Debug, ops::Range, ptr::NonNull};

use crate::BUG_ON;

pub trait Allocator {
    fn malloc(&mut self, size: usize, align: usize) -> Result<NonNull<u8>, AllocError>;
    unsafe fn free(&mut self, ptr: NonNull<u8>, size: usize);
}

struct BestFitMeta {
    size: usize,
    next: Option<NonNull<u8>>,
}

pub enum AllocError {
    InvalidAlign,
    InvalidPtr,
    OutOfMemory,
}

impl Debug for AllocError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AllocError::InvalidAlign => write!(f, "Invalid alignment"),
            AllocError::InvalidPtr => write!(f, "Invalid pointer"),
            AllocError::OutOfMemory => write!(f, "Out of memory"),
        }
    }
}

pub struct BestFitAllocator {
    head: Option<NonNull<u8>>,
}

impl BestFitAllocator {
    pub const fn new() -> Self {
        Self { head: None }
    }

    pub unsafe fn add_range(&mut self, range: Range<usize>) -> Result<(), AllocError> {
        let ptr = range.start;

        if ptr % align_of::<u128>() != 0 {
            return Err(AllocError::InvalidAlign);
        }

        let user_pointer = ptr + size_of::<BestFitMeta>() + Self::align_up();

        let meta = BestFitMeta {
            size: range.end - user_pointer,
            next: self.head,
        };

        core::ptr::write(ptr as *mut BestFitMeta, meta);
        self.head = Some(unsafe { NonNull::new_unchecked(ptr as *mut u8) });
        Ok(())
    }

    fn align_up() -> usize {
        let meta = size_of::<BestFitMeta>();
        let align = align_of::<u128>();
        // Calculate the padding required to align the block.
        (align - (meta % align)) % align
    }

    /// Selects the best fit block for the given size.
    fn select_block(&mut self, size: usize) -> Result<(NonNull<u8>, Option<NonNull<u8>>), AllocError> {
        let mut best_fit = Err(AllocError::OutOfMemory);
        let mut best_fit_size = usize::MAX;

        let mut current = self.head;
        let mut prev = None;

        while let Some(ptr) = current {
            let meta = unsafe { ptr.cast::<BestFitMeta>().as_ref() };

            if meta.size >= size && meta.size < best_fit_size {
                best_fit = Ok((ptr, prev));
                best_fit_size = meta.size;
            }
            prev = current;
            current = meta.next;
        }

        best_fit
    }

    unsafe fn user_ptr(ptr: NonNull<u8>) -> NonNull<u8> {
        ptr.byte_add(size_of::<BestFitMeta>() + Self::align_up())
    }

    unsafe fn control_ptr(ptr: NonNull<u8>) -> NonNull<u8> {
        ptr.byte_sub(size_of::<BestFitMeta>() + Self::align_up())
    }
}

impl Allocator for BestFitAllocator {
    fn malloc(&mut self, size: usize, align: usize) -> Result<NonNull<u8>, AllocError> {
        BUG_ON!(size > 500000, "That's a lot of memory!");

        if align > align_of::<u128>() {
            return Err(AllocError::InvalidAlign);
        }

        let size = super::align_up(size);
        let (block, prev) = self.select_block(size)?;

        let meta = unsafe { block.cast::<BestFitMeta>().as_mut() };

        let min = size_of::<BestFitMeta>() + Self::align_up() + size;

        if meta.size > min + size_of::<BestFitMeta>() + Self::align_up() {
            let remaining_meta = BestFitMeta {
                size: meta.size - min,
                next: meta.next,
            };

            meta.size = size;
            meta.next = None;

            let ptr = unsafe { block.byte_add(min) };

            unsafe {
                ptr.cast::<BestFitMeta>().write(remaining_meta);
            }

            if let Some(prev) = prev {
                let prev_meta = unsafe { prev.cast::<BestFitMeta>().as_mut() };
                prev_meta.next = Some(ptr);
            } else {
                self.head = Some(ptr);
            }
        } else if let Some(prev) = prev {
            let prev_meta = unsafe { prev.cast::<BestFitMeta>().as_mut() };
            prev_meta.next = None;
        } else {
            self.head = None;
        }

        Ok(unsafe { Self::user_ptr(block) })
    }

    unsafe fn free(&mut self, ptr: NonNull<u8>, size: usize) {
        let block = Self::control_ptr(ptr);
        let meta = block.cast::<BestFitMeta>().as_mut();

        meta.next = self.head;
        BUG_ON!(meta.size != size, "Invalid size in free()");
        meta.size = size;

        self.head = Some(block);
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_one() {
        let mut allocator = BestFitAllocator::new();

        let alloc_range = std::alloc::Layout::from_size_align(4096, 1).unwrap();
        let ptr = unsafe { std::alloc::alloc(alloc_range) };

        let begin = ptr as usize;
        let range = begin..ptr as usize + 4096;
        unsafe {
            allocator.add_range(range).unwrap();
        }

        let ptr = allocator.malloc(128, 1).unwrap();
        //assert_eq!(ptr., begin + size_of::<BestFitMeta>());
    }

    #[test]
    fn allocate_two() {
        let mut allocator = BestFitAllocator::new();
        let alloc_range = std::alloc::Layout::from_size_align(4096, 1).unwrap();
        let ptr = unsafe { std::alloc::alloc(alloc_range) };

        let begin = ptr as usize;
        let range = ptr as usize..ptr as usize + 4096;
        unsafe {
            allocator.add_range(range).unwrap();
        }

        let ptr1 = allocator.malloc(128, 1).unwrap();
        let ptr2 = allocator.malloc(128, 1).unwrap();
        //assert_eq!(ptr1 as usize, begin + size_of::<BestFitMeta>());
        //assert_eq!(ptr2 as usize, begin + size_of::<BestFitMeta>() + 128 + size_of::<BestFitMeta>());
    }

    #[test]
    fn allocate_check_no_overwrite() {
        let mut allocator = BestFitAllocator::new();
        let alloc_range = std::alloc::Layout::from_size_align(4096, 1).unwrap();
        let ptr = unsafe { std::alloc::alloc(alloc_range) };

        let begin = ptr as usize;
        let range = ptr as usize..ptr as usize + 4096;
        unsafe {
            allocator.add_range(range).unwrap();
        }

        let ptr1 = allocator.malloc(128, 1).unwrap();
        let ptr2 = allocator.malloc(128, 1).unwrap();
        //assert_eq!(ptr1 as usize, begin + size_of::<BestFitMeta>());
        //assert_eq!(ptr2 as usize, begin + size_of::<BestFitMeta>() + 128 + size_of::<BestFitMeta>());

        // Overwrite the whole allocation and check that the metadata of the second block is still intact.
        for i in 0..128 {
            unsafe {
                std::ptr::write((begin + i) as *mut u8, 0);
            }
        }

        //let meta = unsafe { &*((ptr2 as usize - size_of::<BestFitMeta>()) as *mut BestFitMeta) };
        //assert_eq!(meta.size, 128);
        //assert_eq!(meta.next, None);
    }
}


