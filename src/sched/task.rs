//! Task management module.

use hal::common::sched::ThreadContext;

type TaskId = u32;

const TASK_MEMORY_SIZE: usize = 1024;

struct Task {
    id: TaskId,
    memory: TaskMemory,
    threads: Thread,
}

struct TaskMemory {
    mem: [u8; TASK_MEMORY_SIZE],
}

struct Thread {
    id: TaskId,
    state: ThreadState,
    context: ThreadContext,
}

enum ThreadState {
    Runs,
    Ready,
    Waits,
}
