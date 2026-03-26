//! This module provides access to the global memory allocator.

use crate::sync::spinlock::SpinLocked;
use crate::{BootInfo, utils};
use alloc::Allocator;
use core::ptr::NonNull;

pub mod alloc;
pub mod array;
pub mod boxed;
pub mod heap;
pub mod pool;
pub mod queue;

/// The possible types of memory. Which is compatible with the multiboot2 memory map.
/// Link: https://www.gnu.org/software/grub/manual/multiboot/multiboot.html
#[repr(C)]
#[allow(unused)]
enum MemoryTypes {
    /// Memory that is available for use.
    Available = 1,
    /// Memory that is reserved for the system.
    Reserved = 2,
    /// Memory that is reclaimable after ACPI tables are read.
    ACPIReclaimable = 3,
    /// ACPI Non-volatile-sleeping memory.
    Nvs = 4,
    /// Memory that is bad.
    BadMemory = 5,
}

/// The global memory allocator.
static GLOBAL_ALLOCATOR: SpinLocked<alloc::BestFitAllocator> =
    SpinLocked::new(alloc::BestFitAllocator::new());

/// Initialize the memory allocator.
///
/// `regions` - The memory node module of device tree codegen file.
///
/// Returns an error if the memory allocator could not be initialized.
pub fn init_memory(regions: &[(&str, usize, usize)]) -> Result<(), utils::KernelError> {
    let mut allocator = GLOBAL_ALLOCATOR.lock();

    for &(_, base, size) in regions {
        let range = base..base + size;
        unsafe {
            allocator.add_range(range)?;
        }
    }

    Ok(())
}

/// Allocate a memory block. Normally Box<T> or SizedPool<T> should be used instead of this function.
///
/// `size` - The size of the memory block to allocate.
/// `align` - The alignment of the memory block.
///
/// Returns a pointer to the allocated memory block if the allocation was successful, or `None` if the allocation failed.
pub fn malloc(size: usize, align: usize) -> Option<NonNull<u8>> {
    let mut allocator = GLOBAL_ALLOCATOR.lock();
    allocator.malloc(size, align).ok()
}

/// Free a memory block.
///
/// `ptr` - The pointer to the memory block.
/// `size` - The size of the memory block.
///
/// # Safety
///
/// The caller must ensure that the pointer is from a previous call to `malloc` and that the size is still the same.
pub unsafe fn free(ptr: NonNull<u8>, size: usize) {
    let mut allocator = GLOBAL_ALLOCATOR.lock();
    unsafe { allocator.free(ptr, size) };
}

/// Aligns a size to be a multiple of the u128 alignment.
///
/// `size` - The size to align.
///
/// Returns the aligned size.
pub fn align_up(size: usize) -> usize {
    if size >= (usize::MAX - align_of::<u128>()) {
        return usize::MAX;
    }

    let align = align_of::<u128>();
    (size + align - 1) & !(align - 1)
}

// VERIFICATION -----------------------------------------------------------------------------------
#[cfg(kani)]
mod verification {
    use super::*;
    use crate::mem::alloc::MAX_ADDR;

    fn mock_ptr_write<T>(dst: *mut T, src: T) {
        // Just a noop
    }

    #[kani::proof]
    #[kani::stub(core::ptr::write, mock_ptr_write)]
    fn proof_init_allocator_good() {
        const MAX_REGIONS: usize = 8;
        let regions: [(&str, usize, usize); MAX_REGIONS] =
            core::array::from_fn(|i| ("dummy", kani::any(), kani::any()));

        // contrain all regions
        for &(_, base, size) in regions.iter() {
            kani::assume(base % align_of::<u128>() as u64 == 0);
            kani::assume(base > 0);
            kani::assume(size > 0);
            kani::assume(
                size < alloc::MAX_ADDR as u64
                    && size > alloc::BestFitAllocator::MIN_RANGE_SIZE as u64,
            );
            kani::assume(base < alloc::MAX_ADDR as u64 - size);
        }

        // for any i, j, i != j as indices into the memory regions the following should hold
        let i: usize = kani::any();
        let j: usize = kani::any();
        kani::assume(i < MAX_REGIONS);
        kani::assume(j < MAX_REGIONS);
        kani::assume(i != j);

        /// non-overlapping regions
        let (base_i, size_i) = regions[i];
        let (base_j, size_j) = regions[j];
        kani::assert(
            base_i + size_i <= base_j || base_j + size_j <= base_i,
            "memory regions should not overlap",
        );

        // verify memory init
        assert!(init_memory(&regions).is_ok());
    }

    #[kani::proof]
    fn check_align_up() {
        let size = kani::any();
        kani::assume(size > 0);

        let align = align_up(size);
        assert_ne!(align, 0);

        if align != usize::MAX {
            assert_eq!(align % align_of::<u128>(), 0);
            assert!(align >= size);
        }
    }
}
// END VERIFICATION
