use std::ffi::c_void;

use hal_api::{
    Result,
    stack::{StackDescriptor, Stacklike},
};

pub struct TestingStack {}

impl Stacklike for TestingStack {
    type ElemSize = usize;
    type StackPtr = *mut c_void;

    unsafe fn new(_desc: StackDescriptor) -> Result<Self>
    where
        Self: Sized,
    {
        unimplemented!("Thread stacks are not implemented in testing");
    }

    fn create_sp(&self, _ptr: *mut c_void) -> Result<Self::StackPtr> {
        unimplemented!("Thread stacks are not implemented in testing");
    }

    fn set_sp(&mut self, _sp: Self::StackPtr) {
        unimplemented!("Thread stacks are not implemented in testing");
    }

    fn sp(&self) -> *mut c_void {
        unimplemented!("Thread stacks are not implemented in testing");
    }
}
