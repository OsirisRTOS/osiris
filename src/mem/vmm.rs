use crate::error::Result;
use crate::hal::mem::{PhysAddr, VirtAddr};

mod nommu;

pub type AddressSpace = nommu::AddressSpace;

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    pub struct Perms: u8 {
        const Read = 0b0001;
        const Write = 0b0010;
        const Exec = 0b0100;
    }
}

#[derive(Clone)]
#[allow(dead_code)]
pub enum Backing {
    Zeroed,
    Uninit,
    Anon(PhysAddr),
}

#[derive(Clone)]
#[allow(dead_code)]
pub struct Region {
    start: Option<VirtAddr>,
    len: usize,
    backing: Backing,
    perms: Perms,
}

impl Region {
    /// Creates a new region.
    ///
    /// - `start` is the starting virtual address of the region. If `None`, the system will choose a suitable address.
    /// - `len` is the length of the region in bytes.
    /// - `backing` is the backing type of the region, which determines how the region is initialized and where its contents come from.
    /// - `perms` is the permissions of the region, which determines how the region can be accessed.
    ///
    pub fn new(start: Option<VirtAddr>, len: usize, backing: Backing, perms: Perms) -> Self {
        Self {
            start,
            len,
            backing,
            perms,
        }
    }

    #[allow(dead_code)]
    pub fn start(&self) -> VirtAddr {
        self.start.unwrap_or_else(|| VirtAddr::new(0))
    }

    pub fn len(&self) -> usize {
        self.len
    }

    #[allow(dead_code)]
    pub fn contains(&self, addr: VirtAddr) -> bool {
        let Some(start) = self.start else {
            return false;
        };
        start.saturating_add(self.len()) > addr && addr >= start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unplaced_region_contains_nothing() {
        let r = Region::new(None, 100, Backing::Uninit, Perms::Read);
        assert!(!r.contains(VirtAddr::new(0)));
        assert!(!r.contains(VirtAddr::new(50)));
        assert!(!r.contains(VirtAddr::new(100)));
    }

    #[test]
    fn placed_region_contains_within_bounds() {
        let r = Region::new(Some(VirtAddr::new(100)), 50, Backing::Uninit, Perms::Read);
        assert!(!r.contains(VirtAddr::new(99)));
        assert!(r.contains(VirtAddr::new(100)));
        assert!(r.contains(VirtAddr::new(149)));
        assert!(!r.contains(VirtAddr::new(150)));
    }

    #[test]
    fn placed_region_saturates_at_usize_max() {
        let r = Region::new(
            Some(VirtAddr::new(usize::MAX - 10)),
            100,
            Backing::Uninit,
            Perms::Read,
        );
        assert!(r.contains(VirtAddr::new(usize::MAX - 10)));
        assert!(r.contains(VirtAddr::new(usize::MAX - 1)));
        assert!(!r.contains(VirtAddr::new(usize::MAX)));
    }
}

#[allow(dead_code)]
pub trait AddressSpacelike {
    // Size is the amount of pages in the address space. On nommu systems this will be reserved.
    fn new(pages: usize) -> Result<Self>
    where
        Self: Sized;
    fn map(&mut self, region: Region) -> Result<PhysAddr>;
    fn unmap(&mut self, region: &Region) -> Result<()>;
    fn protect(&mut self, region: &Region, perms: Perms) -> Result<()>;
    fn virt_to_phys(&self, addr: VirtAddr) -> Option<PhysAddr>;
    fn phys_to_virt(&self, addr: PhysAddr) -> Option<VirtAddr>;
    fn end(&self) -> VirtAddr;
    fn activate(&self) -> Result<()>;
}
