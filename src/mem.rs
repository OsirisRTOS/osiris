//! This module provides access to the global memory allocator.

use crate::mem::pfa::PAGE_SIZE;
use crate::mem::vmm::{AddressSpacelike, Backing, Perms, Region};
use crate::sync::spinlock::SpinLocked;
use crate::{BootInfo, sched, utils};
use alloc::Allocator;
use hal::mem::{PhysAddr, VirtAddr};
use core::ptr::NonNull;

pub mod alloc;
pub mod vmm;
pub mod pfa;

pub const BITS_PER_PTR: usize = core::mem::size_of::<usize>() * 8;

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
pub fn init_memory(boot_info: &BootInfo) -> vmm::AddressSpace {
    if let Err(e) = pfa::init_pfa(PhysAddr::new(0x20000000)) { // TODO: Get this from the DeviceTree.
        panic!("[Kernel] Error: failed to initialize PFA. Error: {e:?}");
    }

    // TODO: Configure.
    let pgs = 4;

    let mut kaddr_space = vmm::AddressSpace::new(pgs).unwrap_or_else(|e| {
        panic!("[Kernel] Error: failed to create kernel address space.");  
    });

    let begin = kaddr_space.map(Region::new(VirtAddr::new(0), pgs * PAGE_SIZE, Backing::Zeroed, Perms::all())).unwrap_or_else(|e| {
        panic!("[Kernel] Error: failed to map kernel address space.");
    });

    let mut allocator = GLOBAL_ALLOCATOR.lock();

    let range = begin.as_usize()..(begin.as_usize() + pgs * PAGE_SIZE);

    if let Err(e) = unsafe { allocator.add_range(range) } {
        panic!("[Kernel] Error: failed to add range to allocator.");
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