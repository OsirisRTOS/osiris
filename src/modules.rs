pub mod kernelmods;

use crate::utils::KernelError;

trait KernelModule {
    fn init(&mut self) -> Result<(), KernelError>;
    fn exit(&mut self) -> Result<(), KernelError>;
    fn name(&self) -> &'static str;
}


fn init_modules() { 
    kernelmods::init_modules()
}

fn exit_modules()  {
    kernelmods::exit_modules()
}

