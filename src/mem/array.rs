use core::mem::MaybeUninit;
use core::ops::Index;
use core::slice;

use super::alloc::AllocError;

pub struct IndexMap<T, const N: usize> {
    data: [Option<T>; N],
}

impl<T, const N: usize> IndexMap<T, N> {
    pub const fn new() -> Self {
        Self {
            data: [const { None }; N],
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index < N {
            self.data[index].as_ref()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index < N {
            self.data[index].as_mut()
        } else {
            None
        }
    }

    pub fn insert(&mut self, index: usize, value: T) -> Result<(), AllocError> {
        if index < N {
            self.data[index] = Some(value);
            Ok(())
        } else {
            Err(AllocError::OutOfMemory)
        }
    }

    pub fn insert_next(&mut self, value: T) -> Result<usize, AllocError> {
        for (i, slot) in self.data.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(value);
                return Ok(i);
            }
        }

        Err(AllocError::OutOfMemory)
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index < N {
            self.data[index].take()
        } else {
            None
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Option<T>> {
        self.data.iter()
    }

    pub fn iter_from_cycle(&self, index: usize) -> impl Iterator<Item = &Option<T>> {
        self.data.iter().cycle().skip(index + 1)
    }

    pub fn next(&self, index: Option<usize>) -> Option<usize> {
        let index = index.unwrap_or(0);

        for (i, elem) in self.iter_from_cycle(index).enumerate() {
            if elem.is_some() {
                return Some((index + i + 1) % N);
            }
        }

        None
    }
}

use super::{free, malloc};

pub struct Vec<'a, T, const N: usize> {
    len: usize,
    data: [MaybeUninit<T>; N],
    extra: &'a mut [MaybeUninit<T>]
}

impl<T: Clone, const N: usize> Vec<'_, T, N> {
    pub const fn new() -> Self {
        Self {
            len: 0,
            data: [const { MaybeUninit::uninit() }; N],
            extra: &mut [],
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), AllocError> {
        if self.len < N {
            self.data[self.len].write(value);
            self.len += 1;
            Ok(())
        } else {
            let extra = self.extra.len();
            
            if self.len < N + extra {
                self.extra[self.len - N].write(value);
                self.len += 1;
                Ok(())
            } else {
                let grow = extra * 2;
                let new_extra: *mut MaybeUninit<T> = malloc(grow, core::mem::align_of::<T>()).ok_or(AllocError::OutOfMemory)?.cast();

                for (i, elem) in self.extra.iter_mut().enumerate() {
                    let new = unsafe {
                        MaybeUninit::new(elem.assume_init_ref().clone())
                    };
                    unsafe { new_extra.add(i).write(new); }
                    unsafe { elem.assume_init_drop() };
                }

                unsafe {
                    free(self.extra.as_mut_ptr().cast());
                    self.extra = slice::from_raw_parts_mut(new_extra, grow);
                }

                self.extra[extra].write(value);
                self.len += 1;
                Ok(())
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn at(&self, index: usize) -> Option<&T> {
        if index < N {
            if self.len > index {
                unsafe { Some(&self.data[index].assume_init_ref()) }
            } else {
                None
            }
        } else {
            let index = index - N;
            if index < self.extra.len() {
                unsafe { Some(&self.extra[index].assume_init_ref()) }
            } else {
                None
            }
        }
    }

    pub fn swap(&mut self, a: usize, b: usize) {
        if a < N && b < N {
            self.data.swap(a, b);
        } else if a >= N && b >= N {
            self.extra.swap(a - N, b - N);
        } else if a >= N {
            self.extra.swap(a - N, b);
        } else {
            self.extra.swap(a, b - N);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<T, const N: usize> Drop for Vec<'_, T, N> {
    fn drop(&mut self) {
        for elem in &mut self.data[0..self.len] {
            unsafe { elem.assume_init_drop(); }
        }
    }
}