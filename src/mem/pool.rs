use core::{
    marker::PhantomData,
    num::NonZeroUsize,
    ops::{Deref, DerefMut, Range},
    ptr::write,
};

struct SizedPoolMeta {
    size: usize,
    next: Option<NonZeroUsize>,
}

pub struct SizedPool<T: Default> {
    head: Option<NonZeroUsize>,
    _marker: PhantomData<T>,
}

impl<T: Default> SizedPool<T> {
    pub const fn new() -> Self {
        Self {
            head: None,
            _marker: PhantomData,
        }
    }

    const fn align_up() -> usize {
        let meta = size_of::<SizedPoolMeta>();
        let align = align_of::<T>();
        // Calculate the padding required to align the block.
        (align - (meta % align)) % align
    }

    /// Add a range of blocks to the pool.
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
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid and that the block is at least the size of `T` + `SizedPoolMeta` + Padding for `T`.
    unsafe fn add_block(&mut self, ptr: usize) {
        let meta = SizedPoolMeta {
            size: size_of::<T>(),
            next: self.head,
        };

        unsafe {
            write(ptr as *mut SizedPoolMeta, meta);
        }

        self.head = Some(NonZeroUsize::new_unchecked(ptr));
    }

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
