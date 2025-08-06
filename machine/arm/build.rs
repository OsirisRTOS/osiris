use std::env;

fn main() {
    let out = env::var("OUT_DIR").unwrap_or("src".to_string());

    let hal = env::var("HAL").unwrap_or("stm32l4xx".to_string());
    let board = env::var("BOARD").unwrap_or("nucleo".to_string());
    let mcu = env::var("MCU").unwrap_or("r5zi".to_string());

    let bindgen = bindgen::Builder::default()
        .header(format!("{hal}/interface/export.h"))
        .use_core()
        .wrap_unsafe_ops(true)
        .generate()
        .expect("Unable to generate bindings");

    bindgen
        .write_to_file(format!("{out}/bindings.rs"))
        .expect("Couldn't write bindings!");

    println!("cargo:rerun-if-changed={mcu}");
    println!("cargo:rerun-if-env-changed=HAL");
    println!("cargo:rerun-if-env-changed=BOARD");
    println!("cargo:rerun-if-env-changed=MCU");

    // Only build when we are not on the host
    if env::var("CARGO_FEATURE_HOST").is_ok() {
        println!("cargo:warning=Building for host, skipping HAL build.");
        return;
    }

    // Build the HAL library
    let libhal = cmake::Config::new(hal)
        .define("MCU", mcu.clone())
        .define("BOARD", board.clone())
        .define("OUT_DIR", out.clone())
        .build();
    println!("cargo:rustc-link-search=native={}", libhal.display());
    println!("cargo:linker-script={out}/link.ld");

    // Build the common library
    let common = cmake::Config::new("common").define("MCU", mcu).build();
    println!("cargo:rustc-link-search=native={}", common.display());
}
