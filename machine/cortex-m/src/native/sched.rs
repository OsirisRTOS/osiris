//! Module: sched

use core::{
    ffi::c_void,
    num::NonZero,
    ops::{Add, AddAssign},
    ptr::NonNull,
};

use hal_api::{Result, stack::Descriptor};

// A default finalizer used if none is supplied: just spins forever.
#[inline(never)]
extern "C" fn default_finalizer() -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StackPtr {
    offset: usize,
}

impl StackPtr {
    fn as_ptr(&self, top: NonNull<u32>) -> NonNull<u32> {
        unsafe { top.sub(self.offset) }
    }

    fn checked_add(&self, rhs: usize) -> Option<Self> {
        self.offset.checked_add(rhs).map(|offset| Self { offset })
    }

    fn offset(&self) -> usize {
        self.offset
    }
}

impl AddAssign<usize> for StackPtr {
    fn add_assign(&mut self, rhs: usize) {
        self.offset += rhs;
    }
}

impl Add<usize> for StackPtr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            offset: self.offset + rhs,
        }
    }
}

/// A stack on arm is 4 byte aligned and grows downwards.
#[derive(Debug, Clone, Copy)]
pub struct ArmStack {
    /// The top of the stack (highest address).
    /// Safety: NonNull<u32> can safely be covariant over u32.
    top: NonNull<u32>,
    /// The current offset from the top of the stack
    sp: StackPtr,
    /// The size of the stack
    size: NonZero<usize>,
    /// High-water mark: largest sp offset ever recorded via set_sp.
    #[cfg(any(feature = "metrics", osiris_metrics))]
    peak_offset: usize,
}

impl ArmStack {
    fn does_fit(&self, size: usize) -> bool {
        size <= (self.size.get() - self.sp.offset()) * size_of::<u32>()
    }

    fn is_call_aligned(sp: StackPtr) -> bool {
        sp.offset.is_multiple_of(2)
    }

    fn in_bounds(&self, sp: *mut u32) -> Option<usize> {
        if let Some(sp) = NonNull::new(sp) {
            if sp > self.top {
                return None;
            }

            if sp < unsafe { self.top.sub(self.size.get()) } {
                return None;
            }

            return Some(unsafe { self.top.as_ptr().offset_from(sp.as_ptr()) as usize });
        }

        None
    }

    #[inline(always)]
    unsafe fn push(sp: &mut NonNull<u32>, value: u32) {
        unsafe {
            *sp = sp.sub(1);
            *sp.as_ptr() = value;
        };
    }

    fn push_irq_ret_fn(
        &mut self,
        f: extern "C" fn(*mut c_void),
        ctx: *mut c_void,
        fin: Option<extern "C" fn() -> !>,
    ) -> Result<()> {
        const FRAME_WORDS: usize = 18;
        const WORD: usize = core::mem::size_of::<u32>();

        // TODO: find out if this is Cortex-M4 specific
        const EXEC_RETURN_THREAD_PSP: u32 = 0xFFFFFFFD;
        // TODO: this is thumb specific
        const XPSR_THUMB: u32 = 1 << 24;

        let needed_size = FRAME_WORDS * WORD;

        if !self.does_fit(needed_size) {
            return Err(hal_api::PosixError::ENOMEM);
        }

        // We push an odd number of words, so if the stack is already call-aligned (DOUBLEWORD), we need to add padding.
        if !Self::is_call_aligned(self.sp) {
            self.sp = self.sp.checked_add(1).ok_or(hal_api::PosixError::EINVAL)?;
        }

        // Pushes a function context onto the stack, which will be executed when the IRQ returns.
        // The layout is as follows:
        // xPSR
        // PC (entry point)
        // LR (function to return after the thread is done)
        // R12 (scratch register)
        // R3 (argument to the function - 0)
        // R2 (argument to the function - 0)
        // R1 (argument to the function - 0)
        // R0 (argument to the function - ctx ptr)
        // LR (EXEC_RETURN)
        // R12 (dummy for alignment)
        // R11 - R4 (scratch - 0)

        unsafe {
            let mut write_index = self.sp.as_ptr(self.top);

            Self::push(&mut write_index, XPSR_THUMB);
            // Function pointer on arm is a 32bit address.
            Self::push(&mut write_index, f as usize as u32 | 1);
            let finalizer = fin.unwrap_or(default_finalizer);
            Self::push(&mut write_index, finalizer as usize as u32 | 1);

            // R12, R3, R2, R1
            for _ in 0..4 {
                Self::push(&mut write_index, 0);
            }
            // R0 = ctx pointer (delivered to the entry function as its
            // first argument per AAPCS).
            Self::push(&mut write_index, ctx as usize as u32);

            // Tells the hw to return to thread mode and use the PSP after the exception.
            Self::push(&mut write_index, EXEC_RETURN_THREAD_PSP);

            // R12 (dummy), R11 - R10
            for _ in 0..4 {
                Self::push(&mut write_index, 0);
            }

            // R8 - R4
            for _ in 0..5 {
                Self::push(&mut write_index, 0);
            }

            // We should have written exactly FRAME_WORDS words.
            debug_assert!(write_index == self.top.sub(self.sp.offset() + FRAME_WORDS));

            self.sp += FRAME_WORDS;
        }

        // The returned stack pointer must be call-aligned.
        debug_assert!(Self::is_call_aligned(self.sp));
        Ok(())
    }
}

#[cfg(all(test, any(feature = "metrics", osiris_metrics)))]
mod metrics_tests {
    use super::*;
    use core::num::NonZero;
    use hal_api::stack::{Descriptor, Stacklike};
    use hal_api::mem::PhysAddr;

    const STACK_WORDS: usize = 256;

    // Each test gets its own static buffer to avoid aliasing between parallel tests.
    static mut BUF_A: [u32; STACK_WORDS] = [0u32; STACK_WORDS];
    static mut BUF_B: [u32; STACK_WORDS] = [0u32; STACK_WORDS];

    fn make_stack(buf: &mut [u32; STACK_WORDS]) -> ArmStack {
        let top = unsafe { buf.as_mut_ptr().add(STACK_WORDS) };
        extern "C" fn entry() {}
        unsafe {
            ArmStack::new(Descriptor {
                top: PhysAddr::new(top as usize),
                size: NonZero::new(STACK_WORDS).unwrap(),
                entry,
                fin: None,
            })
            .unwrap()
        }
    }

    #[test]
    fn metrics_total_bytes_matches_size() {
        let stack = make_stack(unsafe { &mut BUF_A });
        let m = stack.metrics();
        let expected_total = STACK_WORDS * core::mem::size_of::<u32>();
        assert_eq!(m.total_bytes, expected_total);
        assert_eq!(m.total_bytes, m.used_bytes + m.free_bytes);
    }

    #[test]
    fn metrics_used_bytes_after_init() {
        // After new(), push_irq_ret_fn has consumed FRAME_WORDS (18) words.
        let stack = make_stack(unsafe { &mut BUF_A });
        let m = stack.metrics();
        let word = core::mem::size_of::<u32>();
        // Frame is 18 words; we allow for an optional alignment word.
        assert!(m.used_bytes >= 18 * word);
        assert!(m.used_bytes <= 20 * word);
        assert!(m.free_bytes < m.total_bytes);
    }

    #[test]
    fn metrics_peak_starts_at_zero() {
        // peak_offset is only updated through set_sp; new() increments sp directly.
        let stack = make_stack(unsafe { &mut BUF_A });
        assert_eq!(stack.metrics().peak_used_bytes, 0);
    }

    #[test]
    fn metrics_peak_tracks_high_water_mark() {
        let mut stack = make_stack(unsafe { &mut BUF_A });
        let word = core::mem::size_of::<u32>();

        // Simulate two context saves at increasing depths.
        let sp_deep = StackPtr { offset: 50 };
        stack.set_sp(sp_deep);
        assert_eq!(stack.metrics().peak_used_bytes, 50 * word);

        let sp_shallow = StackPtr { offset: 20 };
        stack.set_sp(sp_shallow);
        // Peak must not decrease.
        assert_eq!(stack.metrics().peak_used_bytes, 50 * word);
        assert_eq!(stack.metrics().used_bytes, 20 * word);
    }

    #[test]
    fn metrics_free_plus_used_equals_total() {
        let mut stack = make_stack(unsafe { &mut BUF_B });
        stack.set_sp(StackPtr { offset: 100 });
        let m = stack.metrics();
        assert_eq!(m.used_bytes + m.free_bytes, m.total_bytes);
    }
}

impl hal_api::stack::Stacklike for ArmStack {
    type ElemSize = u32;
    type StackPtr = StackPtr;

    unsafe fn new(desc: Descriptor) -> Result<Self>
    where
        Self: Sized,
    {
        let Descriptor {
            top,
            size,
            entry,
            ctx,
            fin,
        } = desc;

        // We expect a PhysAddr, which can be converted to a ptr on nommu.
        let top = NonNull::new(top.as_mut_ptr::<u32>())
            .ok_or(hal_api::PosixError::EINVAL)?;

        let mut stack = Self {
            top,
            sp: StackPtr { offset: 0 },
            size,
            #[cfg(any(feature = "metrics", osiris_metrics))]
            peak_offset: 0,
        };

        stack.push_irq_ret_fn(entry, ctx, fin)?;
        Ok(stack)
    }

    fn create_sp(&self, ptr: *mut c_void) -> Result<StackPtr> {
        if let Some(offset) = self.in_bounds(ptr as *mut u32) {
            return Ok(StackPtr { offset });
        }

        Err(hal_api::PosixError::EINVAL)
    }

    fn set_sp(&mut self, sp: StackPtr) {
        #[cfg(any(feature = "metrics", osiris_metrics))]
        if sp.offset > self.peak_offset {
            self.peak_offset = sp.offset;
        }
        self.sp = sp;
    }

    #[cfg(any(feature = "metrics", osiris_metrics))]
    fn metrics(&self) -> hal_api::stack::StackMetrics {
        let word = core::mem::size_of::<u32>();
        let total_bytes = self.size.get() * word;
        let used_bytes = self.sp.offset * word;
        hal_api::stack::StackMetrics {
            total_bytes,
            used_bytes,
            free_bytes: total_bytes - used_bytes,
            peak_used_bytes: self.peak_offset * word,
        }
    }

    fn sp(&self) -> *mut c_void {
        self.sp.as_ptr(self.top).as_ptr() as *mut c_void
    }
}
