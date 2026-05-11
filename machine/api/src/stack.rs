use crate::{Result, mem::PhysAddr};
use core::{ffi::c_void, num::NonZero};

pub type EntryFn = extern "C" fn(*mut c_void);
pub type FinFn = extern "C" fn() -> !;

pub struct Descriptor {
    pub top: PhysAddr,
    pub size: NonZero<usize>,
    pub entry: EntryFn,
    pub ctx: *mut c_void,
    pub fin: Option<FinFn>,
}

/// Per-stack resource snapshot. Available when the `metrics` feature is enabled.
/// Backends that do not override `Stacklike::metrics` return all-zero values.
#[cfg(any(feature = "metrics", osiris_metrics))]
#[derive(Debug, Clone, Copy)]
pub struct StackMetrics {
    /// Total bytes allocated for this stack.
    pub total_bytes: usize,
    /// Bytes currently consumed (from stack top down to current SP).
    pub used_bytes: usize,
    /// Bytes still available for use.
    pub free_bytes: usize,
    /// Peak bytes ever used since the stack was created (high-water mark).
    pub peak_used_bytes: usize,
}

pub trait Stacklike {
    type ElemSize: Copy;
    type StackPtr;

    unsafe fn new(desc: Descriptor) -> Result<Self>
    where
        Self: Sized;

    fn create_sp(&self, ptr: *mut c_void) -> Result<Self::StackPtr>;
    fn set_sp(&mut self, sp: Self::StackPtr);

    fn sp(&self) -> *mut c_void;

    /// Returns a metrics snapshot for this stack.
    /// Backends that do not implement full metrics tracking return all-zero values.
    #[cfg(any(feature = "metrics", osiris_metrics))]
    fn metrics(&self) -> StackMetrics {
        StackMetrics {
            total_bytes: 0,
            used_bytes: 0,
            free_bytes: 0,
            peak_used_bytes: 0,
        }
    }

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
