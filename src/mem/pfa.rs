// The top level page frame allocator.

use hal::mem::PhysAddr;

use crate::sync::spinlock::SpinLocked;
use crate::types::boxed::Box;
use crate::utils::KernelError;

use core::pin::Pin;

mod bitset;

/// Page size constant (typically 4KB)
pub const PAGE_SIZE: usize = 4096;

const PAGE_CNT: usize = 100; // TODO: This should be determined by the DeviceTree.

type AllocatorType = bitset::Allocator<PAGE_CNT>;

static PFA: SpinLocked<Option<Pin<Box<AllocatorType>>>> = SpinLocked::new(None);

/// This trait abstracts over different page frame allocator implementations.
trait Allocator<const N: usize> {
    /// Returns an initializer function that can be used to create an instance of the allocator.
    /// The initializer function takes a physical address and the amount of pages needed.
    /// 
    /// Safety:
    /// 
    /// - The returned function must only be called with a useable and valid physical address.
    fn initializer() -> unsafe fn(PhysAddr, usize) -> Result<Pin<Box<Self>>, KernelError>;

    fn alloc(&mut self, page_count: usize) -> Option<PhysAddr>;
    fn free(&mut self, addr: PhysAddr, page_count: usize);
}

pub fn init_pfa(addr: PhysAddr) -> Result<(), KernelError> {
    let mut pfa = PFA.lock();
    if pfa.is_some() {
        return Err(KernelError::CustomError("Page frame allocator is already initialized"));
    }

    let initializer = AllocatorType::initializer();
    *pfa = Some(unsafe { initializer(addr, PAGE_CNT)? });

    Ok(())
}

pub fn alloc_page(page_count: usize) -> Option<PhysAddr> {
    let mut pfa = PFA.lock();
    pfa.as_mut()?.alloc(page_count)
}

pub fn free_page(addr: PhysAddr, page_count: usize) {
    let mut pfa = PFA.lock();
    if let Some(pfa) = pfa.as_mut() {
        pfa.free(addr, page_count);
    }
}