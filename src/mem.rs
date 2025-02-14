//! This module provides access to the global memory allocator.

use crate::{utils, BootInfo};
use alloc::Allocator;
use core::ptr::NonNull;
use hal::common::sync::SpinLocked;

pub mod alloc;
pub mod array;
pub mod boxed;
pub mod heap;
pub mod pool;
pub mod queue;

/// The possible types of memory. Which is compatible with the multiboot2 memory map.
/// Link: https://www.gnu.org/software/grub/manual/multiboot/multiboot.html
#[repr(C)]
enum MemoryTypes {
    /// Memory that is available for use.
    Available = 1,
    /// Memory that is reserved for the system.
    Reserved = 2,
    /// Memory that is reclaimable after ACPI tables are read.
    ACPIReclaimable = 3,
    /// ACPI Non-volatile-sleeping memory.
    NVS = 4,
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
    allocator.free(ptr, size);
}

/// Aligns a size to be a multiple of the u128 alignment.
/// 
/// `size` - The size to align.
/// 
/// Returns the aligned size.
pub fn align_up(size: usize) -> usize {
    let align = align_of::<u128>();
    (size + align - 1) & !(align - 1)
}
