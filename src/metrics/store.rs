use crate::sync::seqlock::Seqlock;

pub(crate) const SLOTS: usize = crate::sched::THREAD_COUNT;

#[derive(Debug, Clone, Copy)]
pub struct HeapSnapshot {
    pub total_bytes: usize,
    pub free_bytes: usize,
    pub used_bytes: usize,
    pub alloc_count: u64,
    pub free_count: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct StackSnapshot {
    pub total_bytes: usize,
    pub used_bytes: usize,
    pub free_bytes: usize,
    pub peak_used_bytes: usize,
}

static GLOBAL_HEAP: Seqlock<Option<HeapSnapshot>> = Seqlock::new(None);
static TASK_HEAPS: [Seqlock<Option<HeapSnapshot>>; SLOTS] = [const { Seqlock::new(None) }; SLOTS];
static THREAD_STACKS: [Seqlock<Option<StackSnapshot>>; SLOTS] =
    [const { Seqlock::new(None) }; SLOTS];

pub(crate) fn write_global_heap(s: HeapSnapshot) {
    GLOBAL_HEAP.write(Some(s));
}

pub(crate) fn write_task_heap(slot: usize, s: HeapSnapshot) {
    if slot < SLOTS {
        TASK_HEAPS[slot].write(Some(s));
    }
}

pub(crate) fn clear_task_heap(slot: usize) {
    if slot < SLOTS {
        TASK_HEAPS[slot].write(None);
    }
}

pub(crate) fn write_thread_stack(slot: usize, s: StackSnapshot) {
    if slot < SLOTS {
        THREAD_STACKS[slot].write(Some(s));
    }
}

pub(crate) fn clear_thread_stack(slot: usize) {
    if slot < SLOTS {
        THREAD_STACKS[slot].write(None);
    }
}

pub fn global_heap() -> Option<HeapSnapshot> {
    GLOBAL_HEAP.read()
}

pub fn task_heap(slot: usize) -> Option<HeapSnapshot> {
    if slot < SLOTS {
        TASK_HEAPS[slot].read()
    } else {
        None
    }
}

pub fn thread_stack(slot: usize) -> Option<StackSnapshot> {
    if slot < SLOTS {
        THREAD_STACKS[slot].read()
    } else {
        None
    }
}
