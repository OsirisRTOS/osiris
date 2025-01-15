use core::{fmt::Debug, num::NonZeroUsize, ops::Range};

pub trait Allocator {
    fn malloc(&mut self, size: usize, align: usize) -> Result<*mut u8, AllocError>;
    unsafe fn free(&mut self, ptr: *mut u8);
}

struct BestFitMeta {
    size: usize,
    next: Option<NonZeroUsize>,
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
    head: Option<NonZeroUsize>,
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

        let meta = BestFitMeta {
            size: range.end - ptr,
            next: self.head,
        };

        core::ptr::write(ptr as *mut BestFitMeta, meta);
        self.head = Some(unsafe { NonZeroUsize::new_unchecked(ptr) });
        Ok(())
    }

    fn align_up() -> usize {
        let meta = size_of::<BestFitMeta>();
        let align = align_of::<u128>();
        // Calculate the padding required to align the block.
        (align - (meta % align)) % align
    }

    /// Selects the best fit block for the given size.
    fn select_block(&mut self, size: usize) -> Result<(NonZeroUsize, Option<NonZeroUsize>), AllocError> {
        let mut best_fit = Err(AllocError::OutOfMemory);
        let mut best_fit_size = usize::MAX;

        let mut current = self.head;
        let mut prev = None;

        while let Some(ptr) = current {
            let meta = unsafe { &*(ptr.get() as *const BestFitMeta) };

            if meta.size >= size && meta.size < best_fit_size {
                best_fit = Ok((ptr, prev));
                prev = current;
                best_fit_size = meta.size;
            }

            current = meta.next;
        }

        best_fit
    }
}

impl Allocator for BestFitAllocator {
    fn malloc(&mut self, size: usize, align: usize) -> Result<*mut u8, AllocError> {
        if align > align_of::<u128>() {
            return Err(AllocError::InvalidAlign);
        }

        let size = super::align_up(size);
        let (block, prev) = self.select_block(size)?;

        let meta = unsafe { &*(block.get() as *const BestFitMeta) };

        let min = size + size_of::<BestFitMeta>() + Self::align_up();

        if meta.size > min {
            let remaining_meta = BestFitMeta {
                size: meta.size - min,
                next: meta.next,
            };

            let ptr = block.get() + min;

            unsafe {
                core::ptr::write(ptr as *mut BestFitMeta, remaining_meta);
            }

            if let Some(prev) = prev {
                let prev_meta = unsafe { &mut *(prev.get() as *mut BestFitMeta) };
                prev_meta.next = Some(unsafe { NonZeroUsize::new_unchecked(ptr) });
            }
        } else if let Some(prev) = prev {
            let prev_meta = unsafe { &mut *(prev.get() as *mut BestFitMeta) };
            prev_meta.next = None;
        }

        self.head = meta.next;

        Ok(block.get() as *mut u8)
    }

    unsafe fn free(&mut self, ptr: *mut u8) {
        let block = ptr as usize - size_of::<BestFitMeta>();

        let head = self
            .head
            .replace(unsafe { NonZeroUsize::new_unchecked(block) });

        let meta = unsafe { &mut *(block as *mut BestFitMeta) };
        meta.next = head;
    }
}
