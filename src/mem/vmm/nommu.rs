use core::ptr::copy_nonoverlapping;

use hal::mem::{PhysAddr, VirtAddr};

use crate::{
    mem::{
        pfa, vmm,
    },
    utils::KernelError,
};

pub struct AddressSpace {
    begin: PhysAddr,
    end: PhysAddr,
}

impl vmm::AddressSpacelike for AddressSpace {
    fn new(size: usize) -> Result<Self, KernelError> {
        let pg_cnt = size.div_ceil(pfa::PAGE_SIZE);
        let begin = pfa::alloc_page(pg_cnt).ok_or(KernelError::OutOfMemory)?;
        let end = begin.checked_add(pg_cnt * pfa::PAGE_SIZE).ok_or(KernelError::OutOfMemory)?;

        Ok(Self {
            begin,
            end,
        })
    }

    fn map(&mut self, region: vmm::Region) -> Result<PhysAddr, KernelError> {
        // Do both checks in one statement.
        let phys = self.virt_to_phys(region.start()).and_then(|phys| {
            if phys > self.end {
                None
            } else {
                Some(phys)
            }
        }).ok_or(KernelError::InvalidArgument)?;

        match region.backing {
            vmm::Backing::Anon(phys) => {
                unsafe {
                    copy_nonoverlapping(
                        phys.as_mut_ptr::<u8>(),
                        phys.as_mut_ptr::<u8>(),
                        region.len(),
                    )
                };
            },
            vmm::Backing::Zeroed => {
                unsafe {
                    core::ptr::write_bytes(
                        phys.as_mut_ptr::<u8>(),
                        0,
                        region.len(),
                    )
                };
            },
            vmm::Backing::Uninit => {},
        }

         Ok(phys)
    }

    fn unmap(&mut self, _region: &vmm::Region) -> Result<(), KernelError> {
        Ok(())
    }

    fn protect(&mut self, _region: &vmm::Region, _perms: vmm::Perms) -> Result<(), KernelError> {
        Ok(())
    }

    fn phys_to_virt(&self, addr: PhysAddr) -> Option<VirtAddr> {
        addr.checked_sub(self.begin.as_usize()).map(|phys| VirtAddr::new(phys.as_usize()))
    }

    fn virt_to_phys(&self, addr: VirtAddr) -> Option<PhysAddr> {
       self.begin.checked_add(addr.as_usize())
    }

    fn end(&self) -> VirtAddr {
        // This should always succeed.
        self.phys_to_virt(self.end).unwrap()
    }

    fn activate(&self) -> Result<(), KernelError> {
        Ok(())
    }
}
