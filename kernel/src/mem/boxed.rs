//! This module provides a simple heap-allocated memory block for in-kernel use.

use super::{free, malloc};
use crate::utils::KernelError;
use core::{
    mem::{MaybeUninit, forget},
    ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeTo},
    ptr::{NonNull, drop_in_place, slice_from_raw_parts_mut, write},
};

/// A heap-allocated memory block.
pub struct Box<T: ?Sized> {
    /// Pointer to the heap-allocated memory.
    /// This is uniquely owned, so no covariance issues.
    ptr: NonNull<T>,
}

impl<T> Box<[T]> {
    /// Create a new zeroed heap-allocated slice with the given length.
    ///
    /// `len` - The length of the slice.
    ///
    /// Returns a new heap-allocated slice with the given length or an error if the allocation failed.
    pub fn new_slice_zeroed(len: usize) -> Result<Self, KernelError> {
        if len == 0 {
            return Ok(Self::new_slice_empty());
        }

        if let Some(ptr) = malloc(size_of::<T>() * len, align_of::<T>()) {
            let ptr = slice_from_raw_parts_mut(ptr.as_ptr().cast(), len);
            Ok(Self {
                ptr: unsafe { NonNull::new_unchecked(ptr) },
            })
        } else {
            Err(KernelError::OutOfMemory)
        }
    }

    /// Create a new empty slice.
    ///
    /// Returns a new empty slice.
    pub const fn new_slice_empty() -> Self {
        let ptr = slice_from_raw_parts_mut(NonNull::dangling().as_ptr(), 0);
        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
        }
    }

    /// Create a new uninit heap-allocated slice with the given length.
    ///
    /// `len` - The length of the slice.
    ///
    /// Returns a new heap-allocated slice with the given length or an error if the allocation failed.
    pub fn new_slice_uninit(len: usize) -> Result<Box<[MaybeUninit<T>]>, KernelError> {
        if let Some(ptr) = malloc(
            size_of::<MaybeUninit<T>>() * len,
            align_of::<MaybeUninit<T>>(),
        ) {
            let ptr = slice_from_raw_parts_mut(ptr.as_ptr().cast(), len);
            Ok(Box {
                ptr: unsafe { NonNull::new_unchecked(ptr) },
            })
        } else {
            Err(KernelError::OutOfMemory)
        }
    }
}

impl<T> Box<T> {
    /// Create a new heap-allocated value.
    ///
    /// `value` - The value to store on the heap.
    ///
    /// Returns a new heap-allocated value or `None` if the allocation failed.
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

    /// Returns a mutable reference to the heap-allocated value.
    pub fn as_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }

    /// Returns an immutable reference to the heap-allocated value.
    pub fn as_ref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }

    /// Consumes the `Box`, returning a pointer to the heap-allocated value.
    ///
    /// The caller is responsible for freeing the memory with the global `free` function.
    /// A pointer created with this function can be converted back into a `Box` with the `from_raw` function.
    pub fn into_raw(self) -> NonNull<T> {
        let ptr = self.ptr;
        forget(self);
        ptr
    }

    /// Moves a pointer to a heap-allocated value into a `Box`.
    ///
    /// `ptr` - The pointer to the heap-allocated value.
    ///
    /// Returns a new `Box` managing the given pointer.
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and that the memory is not freed while the `Box` is alive.
    ///
    /// The caller must ensure that the following conditions are met:
    ///
    /// * The pointer must be allocated with the global `malloc` function.
    /// * The pointer must be unique and not aliased.
    /// * The pointer must be properly aligned.
    /// * The pointer must point to a valid `T`.
    ///
    /// The `Box` takes ownership of the memory and will free it with the global allocator when dropped.
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

impl<T> Index<Range<usize>> for Box<[T]> {
    type Output = [T];

    fn index(&self, index: Range<usize>) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl<T> Index<RangeTo<usize>> for Box<[T]> {
    type Output = [T];

    fn index(&self, index: RangeTo<usize>) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl<T> Index<RangeFrom<usize>> for Box<[T]> {
    type Output = [T];

    fn index(&self, index: RangeFrom<usize>) -> &Self::Output {
        &self.as_ref()[index]
    }
}

impl<T> IndexMut<usize> for Box<[T]> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.as_mut()[index]
    }
}

impl<T> IndexMut<Range<usize>> for Box<[T]> {
    fn index_mut(&mut self, index: Range<usize>) -> &mut Self::Output {
        &mut self.as_mut()[index]
    }
}

impl<T> IndexMut<RangeTo<usize>> for Box<[T]> {
    fn index_mut(&mut self, index: RangeTo<usize>) -> &mut Self::Output {
        &mut self.as_mut()[index]
    }
}

impl<T> IndexMut<RangeFrom<usize>> for Box<[T]> {
    fn index_mut(&mut self, index: RangeFrom<usize>) -> &mut Self::Output {
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
