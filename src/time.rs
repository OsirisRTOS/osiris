use core::sync::atomic::Ordering;

use crate::sched;
use crate::sync::atomic::AtomicU64;

// This variable is only allowed to be modified by the systick handler.
static TIME: AtomicU64 = AtomicU64::new(0);

fn tick() {
    TIME.fetch_add(1, Ordering::Release);
}

/*
 * Returns the current time in milliseconds after boot.
 *
 */
#[allow(dead_code)]
pub fn time() -> u64 {
    TIME.load(Ordering::Acquire)
}

/// cbindgen:ignore
/// cbindgen:no-export
#[unsafe(no_mangle)]
pub extern "C" fn systick_hndlr() {
    let time = TIME.fetch_add(1, Ordering::Release) + 1;

    if sched::needs_reschedule(time) {
        sched::reschedule();
    }
}
