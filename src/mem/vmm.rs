use hal::mem::{PhysAddr, VirtAddr};

use crate::error::Result;

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
pub enum Backing {
    Zeroed,
    Uninit,
    Anon(PhysAddr),
}

#[derive(Clone)]
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

    pub fn start(&self) -> VirtAddr {
        self.start.unwrap_or_else(|| VirtAddr::new(0))
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn contains(&self, addr: VirtAddr) -> bool {
        self.start().saturating_add(self.len()) > addr && addr >= self.start()
    }
}

pub trait AddressSpacelike {
    // Size is the amount of pages in the address space. On nommu systems this will be reserved.
    fn new(pages: usize) -> Result<Self> where Self: Sized;
    fn map(&mut self, region: Region) -> Result<PhysAddr>;
    fn unmap(&mut self, region: &Region) -> Result<()>;
    fn protect(&mut self, region: &Region, perms: Perms) -> Result<()>;
    fn virt_to_phys(&self, addr: VirtAddr) -> Option<PhysAddr>;
    fn phys_to_virt(&self, addr: PhysAddr) -> Option<VirtAddr>;
    fn end(&self) -> VirtAddr;
    fn activate(&self) -> Result<()>;
}