//! This module provides pool allocator implementations.
#![allow(dead_code)]

use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    mem::MaybeUninit,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Range},
    ptr::{self, write},
};

use crate::{sync::spinlock::SpinLocked, types::bitset::BitAlloc};

pub struct FixedPoolRef<'a, T, const N: usize, const WORDS: usize> {
    idx: usize,
    pool: &'a FixedPool<T, N, WORDS>,
    _marker: PhantomData<T>,
}

impl<'a, T, const N: usize, const WORDS: usize> Deref for FixedPoolRef<'a, T, N, WORDS> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: ptr does always point to a valid block in the pool.
        // Only one Ref can exist for a block at a time, so there are no mutable references to the same block.
        unsafe { &*self.pool.access(self.idx) }
    }
}

impl<'a, T, const N: usize, const WORDS: usize> DerefMut for FixedPoolRef<'a, T, N, WORDS> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: ptr does always point to a valid block in the pool.
        // Only one Ref can exist for a block at a time, so there are no mutable references to the same block.
        unsafe { &mut *self.pool.access(self.idx) }
    }
}

impl<T, const N: usize, const WORDS: usize> Drop for FixedPoolRef<'_, T, N, WORDS> {
    fn drop(&mut self) {
        // Safety: ptr does always point to a valid block in the pool.
        unsafe { ptr::drop_in_place(self.pool.access(self.idx)) };
        self.pool.free(self.idx);
    }
}
    
pub struct FixedPool<T, const N: usize, const WORDS: usize> {
    free: SpinLocked<BitAlloc<WORDS>>,
    blocks: [UnsafeCell<MaybeUninit<T>>; N],
}

impl<T, const N: usize, const WORDS: usize> FixedPool<T, N, WORDS> {
    pub const fn new() -> Self {
        Self {
            free: SpinLocked::new(BitAlloc::from_array([!0usize; WORDS])),
            blocks: [const { UnsafeCell::new(MaybeUninit::uninit()) }; N],
        }
    }

    pub fn alloc(&self, new: T) -> Option<FixedPoolRef<'_, T, N, WORDS>> {
        // Safety: Alloc ensures that the index cannot be allocated until the next free.
        // A free can only happen when the Ref is dropped, as the function is not publicly accessible.
        // This guarantees that only one Ref can exist for a block at a time.
        let idx = self.free.lock().alloc(1);
        idx.map(|idx| {
            let ptr = self.blocks[idx].get();
            // Safety: A block can only be allocated once.
            unsafe { ptr.write(MaybeUninit::new(new)) };
            FixedPoolRef {
                idx,
                pool: self,
                _marker: PhantomData,
            }
        })
    }

    fn access(&self, idx: usize) -> *mut T {
        self.blocks[idx].get() as *mut T
    }

    fn free(&self, idx: usize) {
        self.free.lock().free(idx, 1);
    }
}

/// Meta information for a block in the pool.
struct SizedPoolMeta {
    _size: usize,
    next: Option<NonZeroUsize>,
}

/// A pool allocator that allocates fixed-size blocks.
#[deprecated(note = "Will be removed soon. Do not use!")]
pub struct SizedPool<T: Default> {
    head: Option<NonZeroUsize>,
    _marker: PhantomData<T>,
}

#[allow(deprecated)]
impl<T: Default> Default for SizedPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(deprecated)]
impl<T: Default> SizedPool<T> {
    /// Create a new empty pool.
    pub const fn new() -> Self {
        Self {
            head: None,
            _marker: PhantomData,
        }
    }

    /// Calculate the padding required to align the block to `align_of::<T>()`.
    const fn align_up() -> usize {
        let meta = size_of::<SizedPoolMeta>();
        let align = align_of::<T>();
        // Calculate the padding required to align the block.
        (align - (meta % align)) % align
    }

    /// Add a range of blocks to the pool.
    ///
    /// `range` - The range of blocks to add.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the range is valid and that the blocks are at least the size of `T` + `SizedPoolMeta` + Padding for `T`.
    pub unsafe fn add_range(&mut self, range: Range<usize>) {
        let mut ptr = range.start;

        while ptr < range.end {
            unsafe {
                self.add_block(ptr);
            }

            ptr += Self::align_up() + size_of::<SizedPoolMeta>() + size_of::<T>();
        }
    }

    /// Add a block to the pool.
    ///
    /// `ptr` - The pointer to the block to add.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and that the block is at least the size of `T` + `SizedPoolMeta` + Padding for `T`.
    unsafe fn add_block(&mut self, ptr: usize) {
        let meta = SizedPoolMeta {
            _size: size_of::<T>(),
            next: self.head,
        };

        unsafe {
            write(ptr as *mut SizedPoolMeta, meta);
        }

        self.head = Some(unsafe { NonZeroUsize::new_unchecked(ptr) });
    }

    /// Allocate a block from the pool.
    ///
    /// Returns `Some(Owned<T>)` if a block was successfully allocated, otherwise `None`.
    pub fn alloc(&mut self) -> Option<Owned<T>> {
        let head = self.head.take();

        head.map(|head| {
            let meta = unsafe { &*(head.get() as *const SizedPoolMeta) };
            self.head = meta.next;

            let ptr = head.get() + size_of::<SizedPoolMeta>() + Self::align_up();
            unsafe { write(ptr as *mut T, T::default()) };

            Owned { ptr: ptr as *mut T }
        })
    }

    /// Deallocate a block back to the pool.
    ///
    /// `block` - The block to deallocate.
    pub fn dealloc(&mut self, block: Owned<T>) {
        let ptr = block.ptr as usize - size_of::<SizedPoolMeta>() - Self::align_up();

        // Append ptr to the front of the list.
        let head = self
            .head
            .replace(unsafe { NonZeroUsize::new_unchecked(ptr) });

        // Update the next pointer to the previous head.
        let meta = unsafe { &mut *(ptr as *mut SizedPoolMeta) };
        meta.next = head;
    }
}

/// An owned block from a pool.
pub struct Owned<T> {
    ptr: *mut T,
}

impl<T: Default> Deref for Owned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T: Default> DerefMut for Owned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}
