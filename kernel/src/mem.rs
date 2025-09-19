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
/// `boot_info` - The boot information. This contains the memory map.
///
/// Returns an error if the memory allocator could not be initialized.
pub fn init_memory(boot_info: &BootInfo) -> Result<(), utils::KernelError> {
    let mut allocator = GLOBAL_ALLOCATOR.lock();

    for entry in boot_info.mmap.iter().take(boot_info.mmap_len) {
        // We only add available memory to the allocator.
        if entry.ty == MemoryTypes::Available as u32 {
            let range = entry.addr as usize..(entry.addr + entry.length) as usize;
            unsafe {
                allocator.add_range(range)?;
            }
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

// VERIFICATION -------------------------------------------------------------------------------------------------------
#[cfg(kani)]
mod verification {
    use crate::MemMapEntry;

    use super::*;
    use kani::Arbitrary;

    impl Arbitrary for MemMapEntry {
        fn any() -> Self {
            let size = size_of::<MemMapEntry>() as u32;
            let length = kani::any();
            let addr = kani::any();

            kani::assume(length < alloc::MAX_ADDR as u64 && length > alloc::BestFitAllocator::MIN_RANGE_SIZE as u64);
            kani::assume(addr < alloc::MAX_ADDR as u64 - length && addr > 0);
            
            MemMapEntry {
                size,
                addr,
                length,
                ty: kani::any(),
            }
        }

        fn any_array<const MAX_ARRAY_LENGTH: usize>() -> [Self; MAX_ARRAY_LENGTH] {
            [(); MAX_ARRAY_LENGTH].map(|_| Self::any())
        }
    }

    fn mock_ptr_write<T>(dst: *mut T, src: T) {
       // Just a noop
    }

    #[kani::proof]
    #[kani::stub(core::ptr::write, mock_ptr_write)]
    fn proof_init_allocator_good() {
        let mmap: [MemMapEntry; _] = kani::any();

        kani::assume(mmap.len() > 0 && mmap.len() <= 8);
        for entry in mmap.iter() {
            // Ensure aligned.
            kani::assume(entry.addr % align_of::<u128>() as u64 == 0);
            // Ensure valid range.
            kani::assume(entry.addr > 0);
            kani::assume(entry.length > 0);

            // Ensure non overlapping entries
            for other in mmap.iter() {
                if entry.addr != other.addr {
                    kani::assume(entry.addr + entry.length <= other.addr || other.addr + other.length <= entry.addr);
                }
            }
        }

        let mmap_len = mmap.len();

        let boot_info = BootInfo {
            implementer: core::ptr::null(),
            variant: core::ptr::null(),
            mmap,
            mmap_len,
        };

        assert!(init_memory(&boot_info).is_ok());
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