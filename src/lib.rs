#![no_std]

mod mem;
mod sched;
mod syscalls;

use core::arch::asm;
use core::ffi::{c_char, CStr};

use hal::hprintln;

/// The memory map entry type.
///
/// This structure shall be compatible with the multiboot_memory_map_t struct at
/// Link: https://www.gnu.org/software/grub/manual/multiboot/multiboot.html
#[repr(packed, C)]
pub struct MemMapEntry {
    size: u32,
    addr: u64,
    length: u64,
    ty: u32,
}

#[repr(C)]
pub struct BootInfo {
    pub implementer: *const c_char,
    pub variant: *const c_char,
    pub mem_map: *const MemMapEntry,
    pub mem_map_len: usize,
}

#[no_mangle]
pub unsafe extern "C" fn kernel_init(boot_info: BootInfo) {
    //let _ = hprintln!("[Kernel] Initializing Osiris...");

    let implementer = unsafe { CStr::from_ptr(boot_info.implementer) };
    let variant = unsafe { CStr::from_ptr(boot_info.variant) };

    /*if let (Ok(implementer), Ok(variant)) = (implementer.to_str(), variant.to_str()) {
        let _ = hprintln!("[Kernel] Detected Processor:");
        let _ = hprintln!("[Kernel] Manufacturer    : {}", implementer);
        let _ = hprintln!("[Kernel] Name            : {}", variant);
    }*/

    hal::hal_hw_init();

    // Initialize the memory allocator.
    if let Err(e) = mem::init_memory(&boot_info) {
        panic!("[Kernel] Failed to initialize memory allocator: {:?}", e);
    }

    syscall!(SYSCALL_DUMMY_NUM, 75);

    panic!("[Kernel] Abort.");
}

use hal::common::{syscall, types::SchedCtx};
use syscalls::dummy::SYSCALL_DUMMY_NUM;

/// cbindgen:ignore
/// cbindgen:no-export
#[no_mangle]
extern "C" fn sched_call(ctx_in: SchedCtx) -> SchedCtx {
    // For now the scheduler does not switch tasks, so we just return the context as is.
    ctx_in
}
