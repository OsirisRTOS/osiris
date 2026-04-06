//! This module provides the basic task and thread structures for the scheduler.
use core::borrow::Borrow;
use core::fmt::Display;
use core::num::NonZero;

use envparse::parse_env;

use hal::stack::Stacklike;

use crate::error::Result;
use crate::mem;
use crate::sched::{ThreadMap, thread};
use crate::types::list;

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
    pub fn new(uid: usize) -> Self {
        Self { uid }
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

#[allow(dead_code)]
pub struct Attributes {
    pub resrv_pgs: Option<NonZero<usize>>,
    pub address_space: Option<mem::vmm::AddressSpace>,
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
    pub fn new(id: UId, attrs: Attributes) -> Result<Self> {
        let address_space = match attrs.address_space {
            Some(addr_space) => addr_space,
            None => {
                let resrv_pgs = attrs.resrv_pgs.ok_or(kerr!(InvalidArgument))?;
                mem::vmm::AddressSpace::new(resrv_pgs.get())?
            }
        };

        Ok(Self {
            id,
            address_space,
            tid_cntr: 0,
            threads: list::List::new(),
        })
    }

    pub fn allocate_tid(&mut self) -> thread::Id {
        let tid = self.tid_cntr;
        self.tid_cntr += 1;
        thread::Id::new(tid, self.id)
    }

    pub fn allocate_stack(&mut self, attrs: &thread::Attributes) -> Result<hal::Stack> {
        let size = DEFAULTS.stack_pages * mem::pfa::PAGE_SIZE;
        let region = mem::vmm::Region::new(
            None,
            size,
            mem::vmm::Backing::Uninit,
            mem::vmm::Perms::Read | mem::vmm::Perms::Write,
        );
        let pa = self.address_space.map(region)?;

        Ok(unsafe {
            hal::Stack::new(hal::stack::Descriptor {
                top: pa + size,
                size: NonZero::new(size).unwrap(),
                entry: attrs.entry,
                fin: attrs.fin,
            })?
        })
    }

    pub fn register_thread<const N: usize>(
        &mut self,
        uid: thread::UId,
        storage: &mut ThreadMap<N>,
    ) -> Result<()> {
        self.threads.push_back(uid, storage)
    }

    #[allow(dead_code)]
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
