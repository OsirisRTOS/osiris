pub fn check_enabled(backend: &str) -> bool {
    let machine = std::env::var("OSIRIS_MACHINE").unwrap_or_else(|_| {
        panic!("No machine backend specified. Please set the OSIRIS_MACHINE environment variable.")
    });
    if machine != backend {
        println!("cargo::rustc-cfg=disabled");
        false
    } else {
        true
    }
}
