#![cfg_attr(freestanding, no_std)]

/// The memory map entry type.
///
/// This structure shall be compatible with the multiboot_memory_map_t struct at
/// Link: [https://www.gnu.org/software/grub/manual/multiboot/multiboot.html]()
#[repr(packed, C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
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
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InitDescriptor {
    /// Pointer to the start of the binary of the init program.
    pub begin: u64,
    /// Length of the binary of the init program.
    pub len: u64,
    pub entry_offset: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Args {
    pub init: InitDescriptor,
}

pub const BOOT_INFO_MAGIC: u32 = 0xD34D60D;

/// The boot information structure.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BootInfo {
    /// The magic number that indicates valid boot information.
    pub magic: u32,
    /// The version of the boot information structure.
    pub version: u32,
    /// The implementer of the processor.
    //pub implementer: u64,
    /// The variant of the processor.
    //pub variant: u64,
    /// The memory map.
    pub mmap: [MemMapEntry; 8],
    /// The length of the memory map.
    pub mmap_len: u64,
    /// The command line arguments.
    pub args: Args,
}

unsafe extern "C" {
    pub fn kernel_init(boot_info: *const BootInfo) -> !;
}
