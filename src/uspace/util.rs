//! This module provides utility functions to declare and create tasks.

use crate::sched;

/// Declare a task with the given memory size, stack size, and name.
#[macro_export]
macro_rules! DECLARE_TASK {
    (mem_size: $mem_size:expr, stack_size: $stack_size:expr, name: $name:ident) => {
        const TASK_ ## $name ## _MEM_SIZE: usize = $mem_size;
        const TASK_ ## $name ## _STACK_SIZE: usize = $stack_size;
    };
}

/// A task finalizer which can be called when a task is finished, it will perform a reschedule.
pub extern "C" fn thread_finalizer() {
    let _ = hal::hprintln!("debug: thread finalizer called.");

    sched::reschedule();
}
