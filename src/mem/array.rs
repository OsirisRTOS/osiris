use core::mem::MaybeUninit;

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
        self.data.iter().cycle().skip(index)
    }

    pub fn next(&self, index: usize) -> Option<usize> {
        for (i, elem) in self.iter_from_cycle(index).enumerate() {
            if elem.is_some() {
                return Some(i);
            }
        }

        None
    }
}


struct Vec<T, const N: usize> {
    len: usize,
    data: [MaybeUninit<T>; N],
}

impl<T, const N: usize> Vec<T, N> {
    pub fn new() -> Self {
        Self {
            data: [const { MaybeUninit::uninit() }; N],
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), AllocError> {
        if self.len < N {
            self.data[self.len].write(value);
            self.len += 1;
            Ok(())
        } else {
            Err(AllocError::OutOfMemory)
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }
}

impl<T, const N: usize> Drop for Vec<T, N> {
    fn drop(&mut self) {
        for elem in &mut self.data[0..self.len] {
            unsafe { elem.assume_init_drop(); }
        }
    }
}