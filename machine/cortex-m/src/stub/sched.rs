use std::ffi::c_void;

use hal_api::{
    Result,
    stack::{Descriptor, Stacklike},
};

/// Host-test stand-in for the real Cortex-M stack. The scheduler tests never
/// dereference `sp`, so we just round-trip a pointer and return a non-null
/// sentinel from `new` to keep null-pointer checks happy.
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
        Ok(Self {
            sp: 0x1 as *mut c_void,
        })
    }

    fn create_sp(&self, ptr: *mut c_void) -> Result<Self::StackPtr> {
        Ok(ptr)
    }

    fn set_sp(&mut self, sp: Self::StackPtr) {
        self.sp = sp;
    }

    fn sp(&self) -> *mut c_void {
        self.sp
    }
}
