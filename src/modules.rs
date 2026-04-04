// Module declarations
include!(concat!(env!("OUT_DIR"), "/modules_kernel.rs"));

pub(crate) fn init_modules() {
    __init_modules()
}

pub(crate) fn exit_modules() {
    __exit_modules()
}
