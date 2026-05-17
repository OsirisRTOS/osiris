//! This module provides a simple allocator.
//! One implementation is the BestFitAllocator, which uses the best fit strategy.

use core::ptr::NonNull;

use crate::hal::mem::PhysAddr;

use crate::error::Result;

pub mod bestfit;

/// Snapshot of allocator resource usage. Available when the `metrics` feature is enabled.
#[cfg(any(feature = "metrics", metrics))]
#[derive(Debug, Clone, Copy, Default)]
pub struct Metrics {
    pub total_bytes: usize,
    pub free_bytes: usize,
    pub free_blocks: usize,
    pub alloc_count: u64,
    pub free_count: u64,
}

#[cfg(any(feature = "metrics", metrics))]
impl Metrics {
    pub const fn new() -> Self {
        Self {
            total_bytes: 0,
            free_bytes: 0,
            free_blocks: 0,
            alloc_count: 0,
            free_count: 0,
        }
    }

    pub fn allocated_bytes(&self) -> usize {
        self.total_bytes.saturating_sub(self.free_bytes)
    }

    pub(crate) fn record_add_range(&mut self, total: usize, free: usize) {
        self.total_bytes = self.total_bytes.saturating_add(total);
        self.free_bytes = self.free_bytes.saturating_add(free);
        self.free_blocks += 1;
    }

    pub(crate) fn record_alloc(&mut self, consumed_bytes: usize, blocks_removed: usize) {
        self.free_bytes = self.free_bytes.saturating_sub(consumed_bytes);
        self.free_blocks = self.free_blocks.saturating_sub(blocks_removed);
        self.alloc_count += 1;
    }

    pub(crate) fn record_free(&mut self, added_bytes: usize) {
        self.free_bytes = self.free_bytes.saturating_add(added_bytes);
        self.free_blocks += 1;
        self.free_count += 1;
    }
}

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
    unsafe fn malloc<T>(
        &mut self,
        size: usize,
        align: usize,
        request: Option<PhysAddr>,
    ) -> Result<NonNull<T>>;
    unsafe fn free<T>(&mut self, ptr: NonNull<T>, size: usize);
}
