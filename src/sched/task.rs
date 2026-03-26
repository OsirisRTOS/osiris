//! This module provides the basic task and thread structures for the scheduler.
use core::num::NonZero;
use core::borrow::Borrow;

use envparse::parse_env;
use hal::{Stack};

use hal::stack::{Stacklike};

use crate::sched::thread;
use crate::{mem, sched};

use crate::mem::vmm::{AddressSpacelike};
use crate::types::traits::ToIndex;
use crate::utils::KernelError;

pub struct Defaults {
    pub stack_pages: usize,
}

const DEFAULTS: Defaults = Defaults {
    stack_pages: parse_env!("OSIRIS_STACKPAGES" as usize),
};

pub const KERNEL_TASK: UId = UId { uid: 0 };

/// Id of a task. This is unique across all tasks.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub struct UId {
    uid: usize,
}

impl UId {
    pub fn new(uid: usize) -> Option<Self> {
        if uid == 0 {
            None
        } else {
            Some(Self { uid })
        }
    }

    pub fn is_kernel(&self) -> bool {
        self.uid == 0
    }
}

impl ToIndex for UId {
    fn to_index<Q: Borrow<Self>>(idx: Option<Q>) -> usize {
        idx.as_ref().map_or(0, |uid| uid.borrow().uid)
    }
}

pub struct Attributes {
    pub resrv_pgs: Option<NonZero<usize>>,
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
    pub fn new(id: UId, attrs: &Attributes) -> Result<Self, KernelError> {
        // TODO: On MMU systems, the resrv_pgs attribute will be ignored, as memory will not be reserved.
        let resrv_pgs = attrs.resrv_pgs.ok_or(KernelError::OutOfMemory)?;
        let address_space = mem::vmm::AddressSpace::new(resrv_pgs.get())?;
        Self::from_addr_space(id, address_space)
    }

    pub fn from_addr_space(id: UId, address_space: mem::vmm::AddressSpace) -> Result<Self, KernelError> {
        Ok(Self {
            id,
            address_space,
            tid_cntr: 0,
        })
    }

    fn allocate_tid(&mut self) -> sched::thread::Id {
        let tid = self.tid_cntr;
        self.tid_cntr += 1;

        sched::thread::Id::new(tid, self.id)
    }

    fn allocate_stack(
        &mut self,
        attrs: &thread::Attributes,
    ) -> Result<hal::stack::Descriptor, KernelError> {
        let size = DEFAULTS.stack_pages * mem::pfa::PAGE_SIZE;
        let region = mem::vmm::Region::new(
            None,
            size,
            mem::vmm::Backing::Uninit,
            mem::vmm::Perms::Read | mem::vmm::Perms::Write,
        );
        let pa = self.address_space.map(region)?;

        Ok(hal::stack::Descriptor {
            top: pa + size,
            size: NonZero::new(size).unwrap(),
            entry: attrs.entry,
            fin: attrs.fin,
        })
    }

    pub fn create_thread(
        &mut self,
        uid: usize,
        attrs: &thread::Attributes,
    ) -> Result<sched::thread::Thread, KernelError> {
        let stack = self.allocate_stack(attrs)?;

        let stack = unsafe { Stack::new(stack) }?;
        let tid = self.allocate_tid();

        Ok(sched::thread::Thread::new(tid.get_uid(uid), stack))
    }
}
