use crate::{sched, sync::spinlock::SpinLocked};

// This variable is only allowed to be modified by the systick handler.
static TIME: SpinLocked<u64> = SpinLocked::new(0);

fn tick() {
    // Increment the global time counter.
    {
        let mut time = TIME.lock();
        *time += 1;
    }
}

/*
 * Returns the current time in milliseconds after boot.
 *
 */
pub fn time() -> u64 {
    if !hal::asm::are_interrupts_enabled() {
        // If interrupts are disabled, we can just read the time.
        return *TIME.lock();
    } else {
        let time;
        // We need to disable interrupts to ensure that systick is always able to lock the time.
        hal::asm::disable_interrupts();
        // Return the current time.
        {
            time = *TIME.lock();
        }
        hal::asm::enable_interrupts();
        // Now systick can be called again.
        return time;
    }
}

/// cbindgen:ignore
/// cbindgen:no-export
#[unsafe(no_mangle)]
pub extern "C" fn systick() {
    tick();

    let resched = { sched::tick_scheduler() };

    if resched {
        hal::sched::reschedule();
    }
}
