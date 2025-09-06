use core::{ffi::c_void, num::NonZero, ptr::NonNull};

use crate::Result;

pub struct StackDescriptor {
    pub top: NonNull<u32>,
    pub size: NonZero<usize>,
    pub entry: extern "C" fn(),
    pub fin: Option<extern "C" fn() -> !>,
}

pub trait Stacklike {
    type ElemSize: Copy;
    type StackPtr;

    unsafe fn new(desc: StackDescriptor) -> Result<Self> where Self: Sized;

    fn create_sp(&self, ptr: *mut c_void) -> Result<Self::StackPtr>;
    fn set_sp(&mut self, sp: Self::StackPtr);

    fn sp(&self) -> *mut c_void;

    //fn push_tinit<F, const N: usize>(&mut self, init: &ThreadInitializer<F, N, Self::ElemSize>) -> Result<CtxPtr>;

    // Pushes a function context onto the stack, which will be executed when the IRQ returns.
    //fn push_irq_ret_fn(&mut self, f: fn(), fin: Option<fn() -> !>) -> Result<Self::StackPtr>;
}

pub trait ThreadArgument: Send + 'static {}

impl<T> ThreadArgument for T where T: Send + 'static {}

/* 
macro_rules! impl_thread_arg {
    ($($t:ty),+) => { $(unsafe impl ThreadArgument for $t {})+ };
}

macro_rules! impl_thread_arg_tuples {
    ( $( $len:literal ),* $(,)? ) => {
        $(
            seq!(I in 0..$len {
                unsafe impl<#(T~I: ThreadArgument,)*> ThreadArgument for (#(T~I,)*) {}
            });
        )*
    }
}

impl_thread_arg!(u8,u16,u32,u64,u128,usize,i8,i16,i32,i64,i128,isize,bool,char);
impl_thread_arg_tuples!(1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16);


pub struct ThreadInitializer<F, const N: usize> {
     pub func: F,
     pub finalizer: Option<fn()>,
     pub args: [ElemSize; N],
}

impl<F, const N: usize, ElemSize: Copy + Into<usize>> ThreadInitializer<F, N, ElemSize> {
    pub fn new(func: F, finalizer: Option<fn()>, args: &[ElemSize; N]) -> Self {
        Self { func, finalizer, args: *args }
    }
}
*/