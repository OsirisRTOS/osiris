use core::{mem::forget, ops::{Deref, DerefMut}, ptr::{drop_in_place, write}};

use super::{free, malloc};

pub struct Box<T> {
    ptr: *mut T,
}

impl<T> Box<T> {
    pub fn new(value: T) -> Option<Self> {
        if let Some(ptr) = malloc(size_of::<T>(), align_of::<T>()) {
            unsafe {
                write(ptr as *mut T, value);
            }

            Some(Self { ptr: ptr as *mut T })
        } else {
            None
        }
    }

    pub fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.ptr }
    }

    pub fn as_ref(&self) -> &T {
        unsafe { &*self.ptr }
    }

    pub fn into_raw(self) -> *mut T {
        let ptr = self.ptr;
        forget(self);
        ptr
    }

    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        Self { ptr }
    }
}

impl<T> Drop for Box<T> {
    fn drop(&mut self) {
        unsafe {
            drop_in_place(self.ptr);
            free(self.ptr as *mut u8);
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