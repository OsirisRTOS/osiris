use core::{fmt::Display, ops::{Add, Div, Rem, Sub}, ptr::NonNull};

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
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

    pub fn diff(&self, other: Self) -> usize {
        if self.0 >= other.0 {
            // Cannot underflow because of the check above.
            self.0.checked_sub(other.0).unwrap()
        } else {
            // Cannot underflow because of the check above.
            other.0.checked_sub(self.0).unwrap()
        }
    }
}

impl<T> From<NonNull<T>> for PhysAddr {
    #[inline]
    fn from(ptr: NonNull<T>) -> Self {
        Self(ptr.as_ptr() as usize)
    }
}

impl Display for PhysAddr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:x}", self.0)
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