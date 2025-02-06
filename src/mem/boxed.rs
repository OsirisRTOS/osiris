use core::{
    mem::{forget, MaybeUninit},
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr::{drop_in_place, slice_from_raw_parts_mut, write, NonNull}, slice::from_raw_parts,
};

use super::{alloc::AllocError, free, malloc};

pub struct Box<T: ?Sized> {
    /// Pointer to the heap-allocated memory.
    /// This is uniquely owned, so no covariance issues.
    ptr: NonNull<T>,
}

impl<T> Box<[T]> {
    pub fn new_slice_zeroed(len: usize) -> Result<Self, AllocError> {
        if len == 0 {
            return Ok(Self::new_slice_empty());
        }

        if let Some(ptr) = malloc(size_of::<T>() * len, align_of::<T>()) {
            let ptr = slice_from_raw_parts_mut(ptr.as_ptr().cast(), len);
            Ok(Self { ptr: unsafe { NonNull::new_unchecked(ptr) }})
        } else {
            Err(AllocError::OutOfMemory)
        }
    }

    pub const fn new_slice_empty() -> Self {
        let ptr = slice_from_raw_parts_mut(NonNull::dangling().as_ptr(), 0);
        Self { ptr: unsafe { NonNull::new_unchecked(ptr) } }
    }

    pub fn new_slice_uninit(len: usize) -> Result<Box<[MaybeUninit<T>]>, AllocError> {
        if let Some(ptr) = malloc(size_of::<MaybeUninit<T>>() * len, align_of::<MaybeUninit<T>>()) {
            let ptr = slice_from_raw_parts_mut(ptr.as_ptr().cast(), len);
            Ok(Box { ptr: unsafe { NonNull::new_unchecked(ptr) }})
        } else {
            Err(AllocError::OutOfMemory)
        }
    }
}

impl<T> Box<T> {
    pub fn new(value: T) -> Option<Self> {
        if let Some(ptr) = malloc(size_of::<T>(), align_of::<T>()) {
            unsafe {
                write(ptr.as_ptr().cast(), value);
            }

            Some(Self { ptr: ptr.cast() })
        } else {
            None
        }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }

    pub fn as_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    pub fn into_raw(self) -> NonNull<T> {
        let ptr = self.ptr;
        forget(self);
        ptr
    }

    pub unsafe fn from_raw(ptr: NonNull<T>) -> Self {
        Self { ptr }
    }
}

impl<T: ?Sized> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            let size = size_of_val(self.ptr.as_ref());

            if size == 0 {
                return;
            }

            drop_in_place(self.ptr.as_ptr());
            free(self.ptr.cast(), size);
        }
    }
}

impl<T> Deref for Box<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for Box<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T> Deref for Box<[T]> {
    type Target = [T];

    fn deref(&self) -> &[T] {
        unsafe { self.ptr.as_ref() }
    }
}

impl<T> DerefMut for Box<[T]> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { self.ptr.as_mut() }
    }
}

impl<T> Index<usize> for Box<[T]> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl<T> IndexMut<usize> for Box<[T]> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.as_mut()[index]
    }
}

impl<T> AsRef<T> for Box<T> {
    fn as_ref(&self) -> &T {
        self.as_ref()
    }
}

impl<T> AsMut<T> for Box<T> {
    fn as_mut(&mut self) -> &mut T {
        self.as_mut()
    }
}
