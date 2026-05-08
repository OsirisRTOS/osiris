//! System-level controls (reset, shutdown future).

use super::bindings;

/// Hard-reset via CMSIS `NVIC_SystemReset`. Does not return — the
/// Cortex-M core restarts at the vector table on the next instruction
/// fetch after the AIRCR write.
pub fn reset() -> ! {
    unsafe { bindings::system_reset() }
}
