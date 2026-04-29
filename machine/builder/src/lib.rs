pub mod dt;

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

pub fn read_path_env(var: &str) -> std::path::PathBuf {
    let path = std::env::var(var).unwrap_or_else(|_| {
        panic!("Environment variable {var} not set. Please set it to the appropriate path.")
    });
    std::path::PathBuf::from(path)
}
