//! This module provides the basic task and thread structures for the scheduler.
use core::borrow::Borrow;
use core::fmt::Display;
use core::num::NonZero;

use envparse::parse_env;
use hal::Stack;

use hal::stack::Stacklike;

use crate::error::Result;
use crate::sched::{ThreadMap, thread};
use crate::types::list;
use crate::{mem, sched};

use crate::mem::vmm::AddressSpacelike;
use crate::types::traits::ToIndex;

pub struct Defaults {
    pub stack_pages: usize,
}

const DEFAULTS: Defaults = Defaults {
    stack_pages: parse_env!("OSIRIS_STACKPAGES" as usize),
};

pub const KERNEL_TASK: UId = UId { uid: 0 };

/// Id of a task. This is unique across all tasks.
#[proc_macros::fmt]
#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub struct UId {
    uid: usize,
}

impl UId {
    pub fn new(uid: usize) -> Option<Self> {
        if uid == 0 { None } else { Some(Self { uid }) }
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

impl Display for UId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.uid)
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
    /// The threads belonging to this task.
    threads: list::List<thread::ThreadList, thread::UId>,
}

impl Task {
    pub fn new(id: UId, attrs: &Attributes) -> Result<Self> {
        // TODO: On MMU systems, the resrv_pgs attribute will be ignored, as memory will not be reserved.
        let resrv_pgs = attrs.resrv_pgs.ok_or(kerr!(InvalidArgument))?;
        let address_space = mem::vmm::AddressSpace::new(resrv_pgs.get())?;
        Self::from_addr_space(id, address_space)
    }

    pub fn from_addr_space(id: UId, address_space: mem::vmm::AddressSpace) -> Result<Self> {
        Ok(Self {
            id,
            address_space,
            tid_cntr: 0,
            threads: list::List::new(),
        })
    }

    fn allocate_tid(&mut self) -> sched::thread::Id {
        let tid = self.tid_cntr;
        self.tid_cntr += 1;

        sched::thread::Id::new(tid, self.id)
    }

    fn allocate_stack(&mut self, attrs: &thread::Attributes) -> Result<hal::stack::Descriptor> {
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

    pub fn create_thread<const N: usize>(
        &mut self,
        uid: usize,
        attrs: &thread::Attributes,
        storage: &mut ThreadMap<N>,
    ) -> Result<thread::UId> {
        let stack = self.allocate_stack(attrs)?;

        let stack = unsafe { Stack::new(stack) }?;
        let tid = self.allocate_tid();
        let new = sched::thread::Thread::new(tid.get_uid(uid), stack, attrs.attrs);
        storage.insert(&tid.get_uid(uid), new)?;
        self.threads.push_back(tid.get_uid(uid), storage)?;

        Ok(tid.get_uid(uid))
    }

    pub fn tid_cntr(&self) -> usize {
        self.tid_cntr
    }

    pub fn threads_mut(&mut self) -> &mut list::List<thread::ThreadList, thread::UId> {
        &mut self.threads
    }

    pub fn threads(&self) -> &list::List<thread::ThreadList, thread::UId> {
        &self.threads
    }
}
