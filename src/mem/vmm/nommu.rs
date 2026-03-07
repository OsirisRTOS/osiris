use core::ptr::copy_nonoverlapping;
use std::num::NonZero;

use crate::{
    mem::{
        pfa, vmm,
    },
    utils::KernelError,
};

use interface::{PhysAddr, VirtAddr};

pub struct AddressSpace {
    begin: VirtAddr,
    size: usize,
}

impl vmm::AddressSpacelike for AddressSpace {
    fn new(size: usize) -> Result<Self, KernelError> {
        let pg_cnt = size.div_ceil(pfa::PAGE_SIZE);
        let begin = pfa::alloc_page(pg_cnt).ok_or(KernelError::OutOfMemory)?;

        Ok(Self {
            begin,
            size: pg_cnt * pfa::PAGE_SIZE,
        })
    }

    fn map(&mut self, region: vmm::Region) -> Result<PhysAddr, KernelError> {
        if region.start() + region.len() > self.size {
            return Err(KernelError::OutOfMemory);
        }

        if let Some(test) = NonZero::new(region.start()) {
            test.
        }


        match region.backing {
            vmm::Backing::Anon(phys) => {
                unsafe {
                    copy_nonoverlapping(
                        phys as *const u8,
                        (self.begin + region.start()) as *mut u8,
                        region.len(),
                    )
                };
                Ok(self.begin + region.start())
            },
            vmm::Backing::Zeroed => {
                unsafe {
                    core::ptr::write_bytes(
                        (self.begin + region.start()) as *mut u8,
                        0,
                        region.len(),
                    )
                };
                Ok(self.begin + region.start())
            },
            vmm::Backing::Uninit => Ok(self.begin + region.start()),
        }
    }

    fn unmap(&mut self, _region: &vmm::Region) -> Result<(), KernelError> {
        Ok(())
    }

    fn protect(&mut self, _region: &vmm::Region, _perms: vmm::Perms) -> Result<(), KernelError> {
        Ok(())
    }

    fn phys_to_virt(&self, addr: PhysAddr) -> Option<VirtAddr> {
        addr.checked_sub(self.begin)
    }

    fn virt_to_phys(&self, addr: VirtAddr) -> Option<PhysAddr> {
       self.begin.checked_add(addr)
    }

    fn end(&self) -> VirtAddr {
        self.size
    }

    fn activate(&self) -> Result<(), KernelError> {
        Ok(())
    }
}
