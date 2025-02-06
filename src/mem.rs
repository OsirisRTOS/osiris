use core::ptr::NonNull;

use alloc::{Allocator, BestFitAllocator};
use hal::common::sync::SpinLocked;

use crate::BootInfo;

pub mod alloc;
pub mod pool;
pub mod boxed;
pub mod array;
pub mod heap;
pub mod queue;

#[repr(C)]
enum MemoryTypes {
    Available = 1,
    Reserved = 2,
    ACPIReclaimable = 3,
    NVS = 4,
    BadMemory = 5,
}

static GLOBAL_ALLOCATOR: SpinLocked<BestFitAllocator> = SpinLocked::new(BestFitAllocator::new());

pub fn init_memory(boot_info: &BootInfo) -> Result<(), alloc::AllocError> {
    let mut allocator = GLOBAL_ALLOCATOR.lock();

    for entry in boot_info.mmap.iter().take(boot_info.mmap_len) {
        if entry.ty == MemoryTypes::Available as u32 {
            let range = entry.addr as usize..(entry.addr + entry.length) as usize;
            unsafe {
                allocator.add_range(range)?;
            }
        }
    }

    Ok(())
}

pub fn malloc(size: usize, align: usize) -> Option<NonNull<u8>> {
    let mut allocator = GLOBAL_ALLOCATOR.lock();
    allocator.malloc(size, align).ok()
}

pub unsafe fn free(ptr: NonNull<u8>, size: usize) {
    let mut allocator = GLOBAL_ALLOCATOR.lock();
    allocator.free(ptr, size);
}

pub fn align_up(size: usize) -> usize {
    let align = align_of::<u128>();
    (size + align - 1) & !(align - 1)
}


