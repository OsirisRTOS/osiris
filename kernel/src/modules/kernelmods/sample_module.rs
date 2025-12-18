use macros::{kernel_init, kernelmod_call};

#[kernel_init]
fn init() {
    
}

#[kernelmod_call]
fn call(target: i32) {
    
}