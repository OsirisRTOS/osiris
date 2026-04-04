//! This module provides access to the global memory allocator.

use crate::mem::pfa::PAGE_SIZE;
use crate::mem::vmm::{AddressSpacelike, Backing, Perms, Region};
use crate::sync::spinlock::SpinLocked;
use alloc::Allocator;
use hal::mem::{PhysAddr};
use core::ptr::NonNull;

pub mod alloc;
pub mod vmm;
pub mod pfa;

pub const BITS_PER_PTR: usize = core::mem::size_of::<usize>() * 8;

unsafe extern "C" {
   unsafe static __stack_top: u8;
}

/// The global memory allocator.
static GLOBAL_ALLOCATOR: SpinLocked<alloc::bestfit::BestFitAllocator> =
    SpinLocked::new(alloc::bestfit::BestFitAllocator::new());

/// Initialize the memory allocator.
///
/// `regions` - The memory node module of device tree codegen file.
///
/// Returns an error if the memory allocator could not be initialized.
pub fn init_memory() -> vmm::AddressSpace {
    let stack_top = &raw const __stack_top as usize;
    if let Err(e) = pfa::init_pfa(PhysAddr::new(stack_top)) { // TODO: Get this from the DeviceTree.
        panic!("failed to initialize PFA. Error: {e}");
    }

    // TODO: Configure.
    let pgs = 10;

    let mut kaddr_space = vmm::AddressSpace::new(pgs).unwrap_or_else(|e| {
        panic!("failed to create kernel address space. Error: {e}");  
    });

    let begin = kaddr_space.map(Region::new(None, 2 * PAGE_SIZE, Backing::Zeroed, Perms::all())).unwrap_or_else(|e| {
        panic!("failed to map kernel address space. Error: {e}");
    });

    {
        let mut allocator = GLOBAL_ALLOCATOR.lock();

        let range = begin..(begin + pgs * PAGE_SIZE);
        if let Err(e) = unsafe { allocator.add_range(&range) } {
            panic!("failed to add range to allocator. Error: {e}");
        }
    }

    kaddr_space
}

/// Allocate a memory block. Normally Box<T> or SizedPool<T> should be used instead of this function.
///
/// `size` - The size of the memory block to allocate.
/// `align` - The alignment of the memory block.
///
/// Returns a pointer to the allocated memory block if the allocation was successful, or `None` if the allocation failed.
pub fn malloc(size: usize, align: usize) -> Option<NonNull<u8>> {
    let mut allocator = GLOBAL_ALLOCATOR.lock();
    allocator.malloc(size, align, None).ok()
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
