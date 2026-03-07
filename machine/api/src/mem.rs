
pub mod stack;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct PhysAddr(usize);

impl PhysAddr {
    #[inline]
    pub fn new(addr: usize) -> Self {
        Self(addr)
    }

    #[inline]
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl From<PhysAddr> for usize {
    #[inline]
    fn from(addr: PhysAddr) -> Self {
        addr.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct VirtAddr(usize); 

impl VirtAddr {
    #[inline]
    pub fn new(addr: usize) -> Self {
        Self(addr)
    }

    #[inline]
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl From<VirtAddr> for usize {
    #[inline]
    fn from(addr: VirtAddr) -> Self {
        addr.0
    }
}