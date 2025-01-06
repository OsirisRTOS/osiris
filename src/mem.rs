use alloc::{Allocator, BestFitAllocator};
use hal::common::sync::SpinLocked;

use crate::BootInfo;

pub mod alloc;
pub mod pool;

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
