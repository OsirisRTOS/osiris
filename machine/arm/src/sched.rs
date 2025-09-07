//! Module: sched

use core::{
    ffi::c_void,
    num::NonZero,
    ops::{Add, AddAssign, Range},
    ptr::NonNull,
    fmt
};

use hal_api::{stack::StackDescriptor, Machinelike, Result};

use crate::print::{print, println};

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
pub struct ArmStack {
    /// The top of the stack (highest address).
    /// Safety: NonNull<u32> can safely be covariant over u32.
    top: NonNull<u32>,
    /// The current offset from the top of the stack (in 4 byte steps).
    sp: StackPtr,
    /// The size of the stack (in 4 byte steps).
    size: NonZero<usize>,
}

impl ArmStack {
    fn does_fit(&self, size: usize) -> bool {
        size <= (self.size.get() - self.sp.offset()) * size_of::<u32>()
    }

    fn is_call_aligned(sp: StackPtr) -> bool {
        (sp.offset % 2) == 0
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
        f: extern "C" fn(),
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
            return Err(hal_api::Error::OutOfMemory(needed_size));
        }

        // We push an odd number of words, so if the stack is already call-aligned (DOUBLEWORD), we need to add padding.
        if !Self::is_call_aligned(self.sp) {
            self.sp = self.sp.checked_add(1).ok_or(hal_api::Error::default())?;
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
        // R0 (argument to the function - 0)
        // LR (EXEC_RETURN)
        // R11 - R4 (scratch - 0)

        println!("Pushing IRQ return frame: sp offset {}, top: {:p}\n", self.sp.offset(), self.top);

        unsafe {
            let mut write_index = self.sp.as_ptr(self.top);

            Self::push(&mut write_index, XPSR_THUMB);
            // Function pointer on arm is a 32bit address.
            Self::push(&mut write_index, f as usize as u32 | 1);
            let finalizer = fin.unwrap_or(default_finalizer);
            Self::push(&mut write_index, finalizer as usize as u32 | 1);

            // R12 - R0
            for _ in 0..5 {
                Self::push(&mut write_index, 0);
            }

            // Tells the hw to return to thread mode and use the PSP after the exception.
            Self::push(&mut write_index, EXEC_RETURN_THREAD_PSP);

            // R12 (dummy for alignment), R11 - R4
            for _ in 0..9 {
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

impl hal_api::stack::Stacklike for ArmStack {
    type ElemSize = u32;
    type StackPtr = StackPtr;

    unsafe fn new(desc: StackDescriptor) -> Result<Self>
    where
        Self: Sized,
    {
        let StackDescriptor {
            top,
            size,
            entry,
            fin,
        } = desc;

        let mut stack = Self {
            top,
            sp: StackPtr {
                offset: 0,
            },
            size,
        };

        stack.push_irq_ret_fn(entry, fin)?;
        Ok(stack)
    }

    fn create_sp(&self, ptr: *mut c_void) -> Result<StackPtr> {
        if let Some(offset) = self.in_bounds(ptr as *mut u32) {
            return Ok(StackPtr { offset });
        }

        Err(hal_api::Error::OutOfBoundsPtr(
            ptr as usize,
            Range {
                start: self.top.as_ptr() as usize - self.size.get() * size_of::<u32>(),
                end: self.top.as_ptr() as usize,
            },
        ))
    }

    fn set_sp(&mut self, sp: StackPtr) {
        self.sp = sp;
    }

    fn sp(&self) -> *mut c_void {
        self.sp.as_ptr(self.top).as_ptr() as *mut c_void
    }
}
