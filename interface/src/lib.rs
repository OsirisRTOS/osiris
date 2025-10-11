#![cfg_attr(freestanding, no_std)]

use core::ffi::c_char;

/// The memory map entry type.
///
/// This structure shall be compatible with the multiboot_memory_map_t struct at
/// Link: [https://www.gnu.org/software/grub/manual/multiboot/multiboot.html]()
#[repr(packed, C)]
#[derive(Debug, Clone, Copy)]
pub struct MemMapEntry {
    /// The size of the entry.
    pub size: u32,
    /// The base address of the memory region.
    pub addr: u64,
    /// The length of the memory region.
    pub length: u64,
    /// The type of the memory region.
    pub ty: u32,
}

#[repr(C)]
pub struct InitDescriptor {
    /// Pointer to the start of the binary of the init program.
    pub begin: *const usize,
    /// Length of the binary of the init program.
    pub len: usize,
    pub entry_offset: usize,
}

#[repr(C)]
pub struct Args {
    pub init: InitDescriptor,
}

pub const BOOT_INFO_MAGIC: u32 = 0xD34D60D;

/// The boot information structure.
#[repr(C)]
pub struct BootInfo {
    /// The magic number that indicates valid boot information.
    pub magic: u32,
    /// The version of the boot information structure.
    pub version: u32,
    /// The implementer of the processor.
    pub implementer: *const c_char,
    /// The variant of the processor.
    pub variant: *const c_char,
    /// The memory map.
    pub mmap: [MemMapEntry; 8],
    /// The length of the memory map.
    pub mmap_len: usize,
    /// The command line arguments.
    pub args: Args,
}

unsafe extern "C" {
    pub fn kernel_init(boot_info: *const BootInfo) -> !;
}
