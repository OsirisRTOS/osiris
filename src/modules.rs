// Module declarations
include!(concat!(env!("OUT_DIR"), "/modules_kernel.rs"));


use crate::utils::KernelError;

trait KernelModule {
    fn init(&mut self) -> Result<(), KernelError>;
    fn exit(&mut self) -> Result<(), KernelError>;
    fn name(&self) -> &'static str;
}


fn init_modules() { 
    __init_modules()
}

fn exit_modules()  {
    __exit_modules()
}

