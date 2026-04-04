use core::ptr::copy_nonoverlapping;

use hal::mem::{PhysAddr, VirtAddr};

use crate::{
    error::Result,
    mem::{
        alloc::{Allocator, bestfit},
        pfa, vmm,
    },
};

pub struct AddressSpace {
    begin: PhysAddr,
    end: PhysAddr,
    allocator: bestfit::BestFitAllocator,
}

impl vmm::AddressSpacelike for AddressSpace {
    fn new(pgs: usize) -> Result<Self> {
        let begin = pfa::alloc_page(pgs).ok_or(kerr!(OutOfMemory))?;
        let end = begin
            .checked_add(pgs * pfa::PAGE_SIZE)
            .ok_or(kerr!(OutOfMemory))?;

        let mut allocator = bestfit::BestFitAllocator::new();
        unsafe { allocator.add_range(&(begin..end))? };

        Ok(Self {
            begin,
            end,
            allocator,
        })
    }

    fn map(&mut self, region: vmm::Region) -> Result<PhysAddr> {
        let req = region.start.and_then(|virt| self.virt_to_phys(virt));
        // TODO: per page align
        let align = core::mem::align_of::<u128>();
        let start = self.allocator.malloc::<u8>(region.len(), align, req)?;

        match region.backing {
            vmm::Backing::Anon(phys) => {
                unsafe {
                    copy_nonoverlapping(phys.as_mut_ptr::<u8>(), start.as_ptr(), region.len())
                };
            }
            vmm::Backing::Zeroed => {
                unsafe { core::ptr::write_bytes(start.as_ptr(), 0, region.len()) };
            }
            vmm::Backing::Uninit => {}
        }

        Ok(start.into())
    }

    fn unmap(&mut self, _region: &vmm::Region) -> Result<()> {
        Ok(())
    }

    fn protect(&mut self, _region: &vmm::Region, _perms: vmm::Perms) -> Result<()> {
        Ok(())
    }

    fn phys_to_virt(&self, addr: PhysAddr) -> Option<VirtAddr> {
        addr.checked_sub(self.begin.as_usize())
            .map(|phys| VirtAddr::new(phys.as_usize()))
    }

    fn virt_to_phys(&self, addr: VirtAddr) -> Option<PhysAddr> {
        self.begin.checked_add(addr.as_usize())
    }

    fn end(&self) -> VirtAddr {
        // This should always succeed.
        self.phys_to_virt(self.end).unwrap()
    }

    fn activate(&self) -> Result<()> {
        Ok(())
    }
}
