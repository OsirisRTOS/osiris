use core::ops::{Add, Sub, Div, Rem};

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct PhysAddr(usize);

impl PhysAddr {
    pub const MAX: Self = Self(usize::MAX);

    #[inline]
    pub fn new(addr: usize) -> Self {
        Self(addr)
    }

    #[inline]
    pub fn as_usize(&self) -> usize {
        self.0
    }

    pub fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    pub fn checked_add(&self, other: usize) -> Option<Self> {
        self.0.checked_add(other).map(Self)
    }

    pub fn checked_sub(&self, other: usize) -> Option<Self> {
        self.0.checked_sub(other).map(Self)
    }

    pub fn is_multiple_of(&self, align: usize) -> bool {
       self.0.is_multiple_of(align)
    }
}

impl Add<usize> for PhysAddr {
    type Output = Self;

    #[inline]
    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<usize> for PhysAddr {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Div<usize> for PhysAddr {
    type Output = Self;

    #[inline]
    fn div(self, rhs: usize) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl Rem<usize> for PhysAddr {
    type Output = Self;

    #[inline]
    fn rem(self, rhs: usize) -> Self::Output {
        Self(self.0 % rhs)
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

    #[inline]
    pub fn saturating_add(&self, other: usize) -> Self {
        Self(self.0.saturating_add(other))
    }

    #[inline]
    pub fn saturating_sub(&self, other: usize) -> Self {
        Self(self.0.saturating_sub(other))
    }
}

impl From<VirtAddr> for usize {
    #[inline]
    fn from(addr: VirtAddr) -> Self {
        addr.0
    }
}