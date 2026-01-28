use macros::{kernel_deinit, kernel_init, kernelmod_call};

#[kernel_init]
pub(super) fn init() {
    
}

#[kernel_deinit]
pub(super) fn deinit() {
    
}


struct Test {
    a: i64,
    b: i64,
}
enum UnixError {
    Unknown = -1,
    InvalidArgument = -22,
    NotFound = -2,
}
#[kernelmod_call]
pub(super) fn call(target: i32) -> Result<i32,UnixError> {
    match target {
        1 => Ok(0),
        2 => Err(UnixError::InvalidArgument),
        _ => Err(UnixError::Unknown),
    }
}