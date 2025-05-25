use std::env;

fn main() {
    let out = env::var("OUT_DIR").unwrap_or("src".to_string());
    let hal_dir = env::var("C_HAL_DIR").unwrap_or("hal/".to_string());
    let dev_dir = env::var("DEVICE_DIR").unwrap_or("device/".to_string());
    let core_dir = env::var("CORE_DIR").unwrap_or("../cmsis/".to_string());
    let mcu = env::var("MCU").unwrap_or("STM32L4R5xx".to_string());

    const EXCLUDE_STR: &str = "HAL_MspInit, HAL_MspDeInit, HAL_UART_MspInit, HAL_UART_MspDeInit";

    let exclude = env::var("EXCLUDE").unwrap_or(EXCLUDE_STR.to_string());

    let bindgen = bindgen::Builder::default()
        .header(format!("{}/stm32l4xx_hal.h", hal_dir))
        .clang_arg(format!("-I{}", hal_dir))
        .clang_arg(format!("-I{}", dev_dir))
        .clang_arg(format!("-I{}", core_dir))
        .clang_arg(format!("-D{}", mcu))
        .use_core()
        .wrap_unsafe_ops(true)
        .blocklist_function(&exclude)
        .clang_macro_fallback()
        .generate()
        .expect("Unable to generate bindings");

    bindgen
        .write_to_file(format!("{}/bindings.rs", out))
        .expect("Couldn't write bindings!");

    // This generates bindings for our (currently) manually wrapped macros.
    let macros = bindgen::Builder::default()
        .header("macros/lib.h")
        .clang_arg(format!("-I{}/include", hal_dir))
        .clang_arg(format!("-I{}/include", dev_dir))
        .clang_arg(format!("-I{}/include", core_dir))
        .generate()
        .expect("Unable to generate macro bindings");

    macros
        .write_to_file(format!("{}/macros.rs", out))
        .expect("Couldn't write macro bindings!");

    println!("cargo:rerun-if-changed=macros/lib.h");
    println!("cargo:rerun-if-env-changed=HAL_DIR");
    println!("cargo:rerun-if-env-changed=DEVICE_DIR");
    println!("cargo:rerun-if-env-changed=CORE_DIR");
    println!("cargo:rerun-if-env-changed=MCU");
    println!("cargo:rerun-if-env-changed=EXCLUDE");
    println!("cargo:rerun-if-changed=bindgen.py");
}
