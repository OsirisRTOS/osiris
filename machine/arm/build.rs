use std::env;

fn main() {
    let out = env::var("OUT_DIR").unwrap_or("src".to_string());

    let hal = env::var("HAL").unwrap_or("stm32l4xx".to_string());
    let board = env::var("BOARD").unwrap_or("nucleo".to_string());
    let _mcu = env::var("MCU").unwrap_or("STM32L4R5xx".to_string());

    let board_dir = format!("{}/{}", hal, board);

    let bindgen = bindgen::Builder::default()
        .header(format!("{}/{}/lib.h", hal, board))
        .use_core()
        .wrap_unsafe_ops(true)
        .generate()
        .expect("Unable to generate bindings");

    bindgen
        .write_to_file(format!("{}/bindings.rs", out))
        .expect("Couldn't write bindings!");

    println!("cargo:rerun-if-changed={}", board_dir);
    println!("cargo:rerun-if-env-changed=HAL");
    println!("cargo:rerun-if-env-changed=BOARD");
    println!("cargo:rerun-if-env-changed=MCU");
}
