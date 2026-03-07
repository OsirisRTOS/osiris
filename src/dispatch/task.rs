//! This module provides the basic task and thread structures for the scheduler.
use core::num::NonZero;
use core::ops::Range;
use core::ptr::NonNull;
use std::borrow::Borrow;

use hal::{Stack, stack};

use hal::stack::Stacklike;

use crate::{mem, sched};

use crate::mem::alloc::{Allocator, BestFitAllocator};
use crate::mem::vmm::{AddressSpace, AddressSpacelike, Region};
use crate::types::traits::ToIndex;
use crate::utils::KernelError;

/// Id of a task. This is unique across all tasks.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct UId {
    uid: usize,
}

impl ToIndex for UId {
    fn to_index<Q: Borrow<Self>>(idx: Option<Q>) -> usize {
        idx.as_ref().map_or(0, |uid| uid.borrow().uid)
    }
}

pub struct Attributes {
    reserved: Option<NonZero<usize>>,
}

/// The struct representing a task.
pub struct Task {
    /// The unique identifier of the task.
    pub id: UId,
    /// The counter for the thread ids.
    tid_cntr: usize,
    /// Sets up the memory for the task.
    address_space: mem::vmm::AddressSpace,
}

impl Task {
    /// Create a new task.
    ///
    /// `memory_size` - The size of the memory that the task requires.
    ///
    /// Returns a new task if the task was created successfully, or an error if the task could not be created.
    pub fn new(id: UId, attrs: &Attributes) -> Result<Self, KernelError> {
        Ok(Self {
            id,
            address_space: AddressSpace::new(),
            tid_cntr: 0,
        })
    }

    fn allocate_tid(&mut self) -> sched::thread::Id {
        let tid = self.tid_cntr;
        self.tid_cntr += 1;

        sched::thread::Id::new(tid, self.id)
    }

    pub fn allocate(&mut self, size: usize, align: usize) -> Result<mem::vmm::Region, KernelError> {
        self.address_space.map(size, align)
    }

    pub fn create_thread(
        &mut self,
        entry: extern "C" fn(),
        fin: Option<extern "C" fn() -> !>,
    ) -> Result<ThreadDescriptor, KernelError> {

        // Create the stack for the thread.
        let size = 1 * mem::pfa::PAGE_SIZE; // TODO: Make this configurable
        let start = self.address_space.end() - size;
        let region = mem::vmm::Region::new(
            start,
            size,
            mem::vmm::Backing::Uninit,
            mem::vmm::Perms::Read | mem::vmm::Perms::Write,
        );
        let stack_pa = self.address_space.map(region)?;

        let stack = hal::stack::StackDescriptor {
            top: stack_pa,
            // Safe unwrap because stack size is non zero.
            size: NonZero::new(size).unwrap(),
            entry,
            fin,
        };

        let stack = unsafe { Stack::new(stack) }?;
        let tid = self.allocate_tid();

        // TODO: Revert if error occurs
        self.register_thread(tid)?;

        Ok(ThreadDescriptor { tid, stack, timing })
    }

    /// Register a thread with the task.
    ///
    /// `thread_id` - The id of the thread to register.
    ///
    /// Returns `Ok(())` if the thread was registered successfully, or an error if the thread could not be registered. TODO: Check if the thread is using the same memory as the task.
    fn register_thread(&mut self, thread_id: ThreadId) -> Result<(), KernelError> {
        self.threads.push(thread_id)
    }
}

/// The memory of a task.
#[derive(Debug)]
pub struct TaskMemory {
    /// The beginning of the memory.
    begin: NonNull<u8>,
    /// The size of the memory.
    size: usize,

    /// The allocator for the task's memory.
    alloc: BestFitAllocator,
}

#[allow(dead_code)]
impl TaskMemory {
    /// Create a new task memory.
    ///
    /// `size` - The size of the memory.
    ///
    /// Returns a new task memory if the memory was created successfully, or an error if the memory could not be created.
    pub fn new(size: usize) -> Result<Self, KernelError> {
        let begin = mem::malloc(size, align_of::<u128>()).ok_or(KernelError::OutOfMemory)?;

        let mut alloc = BestFitAllocator::new();
        let range = Range {
            start: begin.as_ptr() as usize,
            end: begin.as_ptr() as usize + size,
        };

        if let Err(e) = unsafe { alloc.add_range(range) } {
            unsafe { mem::free(begin, size) };
            return Err(e);
        }

        Ok(Self { begin, size, alloc })
    }

    pub fn malloc<T>(&mut self, size: usize, align: usize) -> Result<NonNull<T>, KernelError> {
        self.alloc.malloc(size, align)
    }

    pub fn free<T>(&mut self, ptr: NonNull<T>, size: usize) {
        unsafe { self.alloc.free(ptr, size) }
    }
}

impl Drop for TaskMemory {
    fn drop(&mut self) {
        unsafe { mem::free(self.begin, self.size) };
    }
}
