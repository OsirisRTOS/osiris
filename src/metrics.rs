//! Unified kernel metrics API.
//!
//! Enabled by the `metrics` Cargo feature. Provides:
//! - Global heap metrics via [`kernel_metrics`] / [`global_heap_metrics`]
//! - Per-task heap metrics via [`task_heap_metrics`]
//! - Per-thread stack metrics via [`thread_stack_metrics`]
//!
//! For full stack metrics, the backend crate's `metrics` feature must also be
//! enabled (e.g. `hal_cortex_m/metrics`). Without it, stack metrics return zeros.

use crate::mem;
use crate::mem::alloc::bestfit::AllocatorMetrics;
use crate::sched::{self, task, thread};

/// Aggregated snapshot of global kernel resources.
pub struct KernelMetrics {
    pub heap: AllocatorMetrics,
}

/// Returns a snapshot of global kernel metrics (heap only).
pub fn kernel_metrics() -> KernelMetrics {
    KernelMetrics {
        heap: mem::global_metrics(),
    }
}

/// Returns heap metrics for the global kernel allocator.
pub fn global_heap_metrics() -> AllocatorMetrics {
    mem::global_metrics()
}

/// Returns stack metrics for the thread identified by `tid`, or `None` if the
/// thread does not exist.
pub fn thread_stack_metrics(tid: thread::UId) -> Option<crate::hal::stack::StackMetrics> {
    sched::with(|sched| sched.thread_stack_metrics(tid))
}

/// Returns heap metrics for the address space owned by task `task_id`, or
/// `None` if the task does not exist.
pub fn task_heap_metrics(task_id: task::UId) -> Option<AllocatorMetrics> {
    sched::with(|sched| sched.task_heap_metrics(task_id))
}
