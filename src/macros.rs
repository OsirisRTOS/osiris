
#[macro_export]
macro_rules! args_from_raw {
    ($argc:expr, $argv:expr) => {{
        let argc = $argc;
        let argv = $argv;

        if argc == 0 || argv.is_null() {
            &[]
        } else {
            unsafe { core::slice::from_raw_parts(argv, argc) }
        }
    }};
}