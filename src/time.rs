use hal::Machinelike;

use crate::{sched, sync};

static TICKS: sync::atomic::AtomicU64 = sync::atomic::AtomicU64::new(0);

pub fn tick() -> u64 {
    TICKS.load(sync::atomic::Ordering::Acquire)
}

pub fn mono_now() -> u64 {
    // TODO: This will break on SMP systems without native u64 atomic store.
    sync::atomic::irq_free(|| hal::Machine::monotonic_now() )
}

pub fn mono_freq() -> u64 {
    hal::Machine::monotonic_freq()
}

/// cbindgen:ignore
/// cbindgen:no-export
#[unsafe(no_mangle)]
pub extern "C" fn systick_hndlr() {
    let tick = TICKS.fetch_add(1, sync::atomic::Ordering::Release) + 1;

    if sched::needs_reschedule(tick) {
        sched::reschedule();
    }
}
