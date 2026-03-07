use core::ops::Range;

use crate::{utils::KernelError};

use interface::{PhysAddr, VirtAddr};

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
    range: Range<VirtAddr>,
    backing: Backing,
    perms: Perms,
}

impl Region {
    pub fn new(start: VirtAddr, len: usize, backing: Backing, perms: Perms) -> Self {
        Self {
            range: start..start.saturating_add(len),
            backing,
            perms,
        }
    }

    pub fn start(&self) -> VirtAddr {
        self.range.start
    }

    pub fn len(&self) -> usize {
        self.range.end.saturating_sub(self.range.start)
    }

    pub fn contains(&self, addr: VirtAddr) -> bool {
        self.range.contains(&addr)
    }
}

pub trait AddressSpacelike {
    // Size is the amount of pages in the address space. On nommu systems this will be reserved.
    fn new(pages: usize) -> Result<Self, KernelError> where Self: Sized;
    fn map(&mut self, region: Region) -> Result<PhysAddr, KernelError>;
    fn unmap(&mut self, region: &Region) -> Result<(), KernelError>;
    fn protect(&mut self, region: &Region, perms: Perms) -> Result<(), KernelError>;
    fn virt_to_phys(&self, addr: VirtAddr) -> Option<PhysAddr>;
    fn phys_to_virt(&self, addr: PhysAddr) -> Option<VirtAddr>;
    fn end(&self) -> VirtAddr;
    fn activate(&self) -> Result<(), KernelError>;
}