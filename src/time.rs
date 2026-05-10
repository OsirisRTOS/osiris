use crate::hal::{self, Machinelike};

use crate::{sched, sync};

static TICKS: sync::atomic::AtomicU64 = sync::atomic::AtomicU64::new(0);

pub fn tick() -> u64 {
    TICKS.load(sync::atomic::Ordering::Acquire)
}

pub fn mono_now() -> u64 {
    // TODO: This will break on SMP systems without native u64 atomic store.
    sync::atomic::irq_free(|| hal::Machine::monotonic_now())
}

pub fn mono_freq() -> u64 {
    hal::Machine::monotonic_freq()
}

pub fn to_secs(cnt: u64, hz: u32, digits: u8) -> (u64, u64) {
    let secs = cnt / (hz as u64);
    let rem = cnt % (hz as u64);
    let frac = (rem * 10_u64.pow(digits as u32)) / (hz as u64);
    (secs, frac)
}

/// cbindgen:ignore
/// cbindgen:no-export
#[unsafe(no_mangle)]
pub extern "C" fn systick_hndlr() {
    let tick = TICKS.fetch_add(1, sync::atomic::Ordering::Release) + 1;

    sync::atomic::irq_free(|| {
        hal::Machine::do_tick();
    });

    if sched::needs_reschedule(tick) {
        sched::reschedule();
    }
}
