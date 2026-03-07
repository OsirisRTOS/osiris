//! This is the owner of all Tasks. It takes care of context switching between them.
//! The idea is that the Schedulers selects one of its threads to run, and then the Dipatcher takes care of context-switching to the associated Task. (e.g. setting up the address space)
//! If the thread is part of the same task as the currently running one, the Dispatcher does effectively nothing.
//! 
//! 

mod task;

use crate::types::array::IndexMap;

/* 
struct Dispatcher<const N: usize> {
    tasks: IndexMap<task::UId, task::Task, N>,
}*/