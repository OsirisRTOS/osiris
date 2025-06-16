pub mod kernelmods;

use alloc::boxed::Box;
use alloc::vec::Vec;
use crate::utils::KernelError;

trait KernelModule {
    fn init(&mut self) -> Result<(), KernelError>;
    fn exit(&mut self) -> Result<(), KernelError>;
    fn name(&self) -> &'static str;
}


fn init_modules() -> Result<(), KernelError> { 
    kernelmods::init_modules()
}

fn exit_modules() -> Result<(), KernelError> {
    kernelmods::exit_modules()
}

