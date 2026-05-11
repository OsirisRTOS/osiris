use core::ptr::{NonNull, copy_nonoverlapping};

use crate::hal::mem::{PhysAddr, VirtAddr};

use crate::{
    error::Result,
    mem::{
        alloc::{Allocator, bestfit},
        pfa, vmm,
    },
};

pub struct AddressSpace {
    begin: PhysAddr,
    #[allow(dead_code)]
    end: PhysAddr,
    allocator: bestfit::BestFitAllocator,
}

impl vmm::AddressSpacelike for AddressSpace {
    fn new(pgs: usize) -> Result<Self> {
        let begin = pfa::alloc_page(pgs).ok_or(kerr!(ENOMEM))?;
        let end = begin
            .checked_add(pgs * pfa::PAGE_SIZE)
            .ok_or(kerr!(ENOMEM))?;

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
        let start = unsafe { self.allocator.malloc::<u8>(region.len(), align, req)? };

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

    fn unmap(&mut self, region: &vmm::Region) -> Result<()> {
        let virt = region.start.ok_or(kerr!(EINVAL))?;
        let phys = self.virt_to_phys(virt).ok_or(kerr!(EINVAL))?;
        let ptr = NonNull::new(phys.as_mut_ptr::<u8>()).ok_or(kerr!(EINVAL))?;
        unsafe { self.allocator.free(ptr, region.len()) };
        Ok(())
    }

    fn protect(&mut self, _region: &vmm::Region, _perms: vmm::Perms) -> Result<()> {
        Ok(())
    }

    fn phys_to_virt(&self, addr: PhysAddr) -> Option<VirtAddr> {
        if addr < self.begin || addr >= self.end {
            return None;
        }
        addr.checked_sub(self.begin.as_usize())
            .map(|phys| VirtAddr::new(phys.as_usize()))
    }

    fn virt_to_phys(&self, addr: VirtAddr) -> Option<PhysAddr> {
        let phys = self.begin.checked_add(addr.as_usize())?;
        if phys >= self.end {
            return None;
        }
        Some(phys)
    }

    fn end(&self) -> VirtAddr {
        VirtAddr::new(self.end.diff(self.begin))
    }

    fn activate(&self) -> Result<()> {
        Ok(())
    }
}

impl Drop for AddressSpace {
    fn drop(&mut self) {
        // Without this the per-task page reservation returns to the PFA only on
        // process death, which means PFA exhaustion under task churn.
        let pgs = self.end.diff(self.begin) / pfa::PAGE_SIZE;
        if pgs > 0 {
            pfa::free_page(self.begin, pgs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mem::vmm::{AddressSpacelike, Backing, Perms, Region};

    fn make_addr_space(size: usize) -> AddressSpace {
        let layout = std::alloc::Layout::from_size_align(size, core::mem::align_of::<u128>()).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) };
        let begin = PhysAddr::new(ptr as usize);
        let end = begin + size;
        let mut allocator = bestfit::BestFitAllocator::new();
        unsafe { allocator.add_range(&(begin..end)).unwrap() };
        AddressSpace {
            begin,
            end,
            allocator,
        }
    }

    #[test]
    fn unmap_returns_space_to_allocator() {
        let mut as_ = make_addr_space(4096);

        let region = Region::new(None, 2048, Backing::Uninit, Perms::Read);
        let phys = as_.map(region).unwrap();

        let virt = as_.phys_to_virt(phys).unwrap();
        let placed = Region::new(Some(virt), 2048, Backing::Uninit, Perms::Read);
        as_.unmap(&placed).unwrap();

        let region2 = Region::new(None, 2048, Backing::Uninit, Perms::Read);
        as_.map(region2)
            .expect("re-map after unmap should not OOM");
    }

    #[test]
    fn unmap_unplaced_region_rejected() {
        let mut as_ = make_addr_space(4096);
        let region = Region::new(None, 128, Backing::Uninit, Perms::Read);
        assert!(as_.unmap(&region).is_err());
    }

    #[test]
    fn virt_to_phys_rejects_out_of_range() {
        let as_ = make_addr_space(4096);
        let size = as_.end.diff(as_.begin);
        assert!(as_.virt_to_phys(VirtAddr::new(size)).is_none());
        assert!(as_.virt_to_phys(VirtAddr::new(size + 1)).is_none());
        assert!(as_.virt_to_phys(VirtAddr::new(usize::MAX)).is_none());
    }

    #[test]
    fn phys_to_virt_rejects_out_of_range() {
        let as_ = make_addr_space(4096);
        assert!(as_.phys_to_virt(as_.end).is_none());
        assert!(as_.phys_to_virt(as_.begin - 1).is_none());
        assert!(as_.phys_to_virt(as_.end + 1).is_none());
    }

    #[test]
    fn virt_phys_roundtrip() {
        let as_ = make_addr_space(4096);
        let v = VirtAddr::new(128);
        let p = as_.virt_to_phys(v).unwrap();
        assert_eq!(as_.phys_to_virt(p), Some(v));
    }
}
