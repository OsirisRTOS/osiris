use crate::args_from_raw;

pub struct InitTask {

}

impl InitTask {
    pub extern "C" fn main(argc: usize, argv: *const *const u8) {
        let args = args_from_raw!(argc, argv);

        let _ = hal::hprintln!("Hello, from init");
    }
}
