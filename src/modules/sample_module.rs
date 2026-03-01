use macros::{kernelmod_exit, kernelmod_init, kernelmod_call};
use crate::error::PosixError;
// This is a sample kernel module. It doesn't do anything useful, but it serves as an example of how to write a kernel module and how to use the macros.
// The kernelmodule system offers 3 macros for integrating modules into the kernel. While building, the build system will then automatically generate a userspace API compatible to the currently installed kernelmodules.
// Below, stubs of the three types of methods can be found. Node that the signature needs to match the one provided in the samples and especially that thbe return type of the kernelmodule_call needs to be at most register sized.
// The macros are used to detect the modules during build. Below, the macros are commented out to avoid inclusion of this sample module.


//#[kernelmod_init]
pub fn init() {
    // This function is called once on kernel startup for each module. It should be used for initializing any state the module requires. Currently, there are no guarantees for the initialization order.
}

//#[kernelmod_exit]
pub fn exit() {
    // This function is called once on kernel shutdown for each module. It should be used for cleaning up any state the module requires. Currently, there are no guarantees for the exit order.
}

//#[kernelmod_call]
pub fn call(target: i32) -> Result<i32,PosixError> {
    // This function represents a kernel module call. The build system generates an equivalent userspace wrapper.
    // References are transmitted using pointers, slices and string slices are internally transmitted as pointers and lengths
    match target {
        1 => Ok(0),
        2 => Err(PosixError::EINVAL),
        _ => Err(PosixError::EPERM),
    }
}