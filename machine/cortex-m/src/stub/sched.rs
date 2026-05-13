use std::ffi::c_void;

use hal_api::{
    Result,
    stack::{Descriptor, Stacklike},
};

/// A stub stack. The real Cortex-M implementation pushes a synthetic exception
/// frame onto a real stack. In host-test land we only need something that does
/// not panic and round-trips a stack pointer, so we use a single AtomicUsize-sized
/// cell as the "stack pointer" storage.
#[derive(Debug, Clone, Copy)]
pub struct StubStack {
    sp: *mut c_void,
}

unsafe impl Send for StubStack {}
unsafe impl Sync for StubStack {}

impl Stacklike for StubStack {
    type ElemSize = usize;
    type StackPtr = *mut c_void;

    unsafe fn new(_desc: Descriptor) -> Result<Self>
    where
        Self: Sized,
    {
        // Host tests never dereference the stack pointer. We give back a
        // non-null sentinel so any code that checks for null pointers behaves.
        Ok(Self {
            sp: 0x1 as *mut c_void,
        })
    }

    fn create_sp(&self, ptr: *mut c_void) -> Result<Self::StackPtr> {
        // The real implementation builds a synthetic exception frame. In tests
        // we just round-trip the pointer so save_ctx/ctx behave sensibly.
        Ok(ptr)
    }

    fn set_sp(&mut self, sp: Self::StackPtr) {
        self.sp = sp;
    }

    fn sp(&self) -> *mut c_void {
        self.sp
    }
}
