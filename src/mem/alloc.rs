//! This module provides a simple allocator.
//! One implementation is the BestFitAllocator, which uses the best fit strategy.

use core::ptr::NonNull;

use hal::mem::PhysAddr;

use crate::error::Result;

pub mod bestfit;

#[cfg(target_pointer_width = "64")]
pub const MAX_ADDR: usize = 2_usize.pow(48);

#[cfg(target_pointer_width = "32")]
pub const MAX_ADDR: usize = usize::MAX;

/// Allocator trait that provides a way to allocate and free memory.
/// Normally you don't need to use this directly, rather use the `boxed::Box` type.
///
/// # Safety
///
/// Every block returned by `malloc` must be freed by `free` exactly once.
/// A pointer allocated by one allocator must not be freed by another allocator.
/// Each range added to the allocator must be valid for the whole lifetime of the allocator and must not overlap with any other range.
/// The lifetime of any allocation is only valid as long as the allocator is valid. (A pointer must not be used after the allocator is dropped.)
pub trait Allocator {
    fn malloc<T>(
        &mut self,
        size: usize,
        align: usize,
        request: Option<PhysAddr>,
    ) -> Result<NonNull<T>>;
    unsafe fn free<T>(&mut self, ptr: NonNull<T>, size: usize);
}
