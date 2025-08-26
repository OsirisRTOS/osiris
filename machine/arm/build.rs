use anyhow::{Context, Result};
use std::{env, process::Command};

fn host_triple() -> Result<String> {
    let out = Command::new("rustc")
        .arg("-vV")
        .output()
        .context("Failed to get host triple")?;

    let s = String::from_utf8(out.stdout)?;
    let triple = s
        .lines()
        .find_map(|l| l.strip_prefix("host: ").map(str::to_owned))
        .context("could not parse host triple")?;
    Ok(triple)
}

fn fail_on_error<T>(res: Result<T>) -> T {
    match res {
        Ok(val) => val,
        Err(e) => {
            println!("cargo:error={}", e);
            std::process::exit(1);
        }
    }
}

fn main() {
    let out = env::var("OUT_DIR").unwrap_or("src".to_string());

    let hal = fail_on_error(env::var("ARM_HAL").with_context(
        || "ARM_HAL environment variable not set. Please set it to the path of the ARM HAL.",
    ));
    let board = fail_on_error(
        env::var(format!("ARM_{}_BOARD", hal.to_uppercase())).with_context(|| {
            "ARM_{}_BOARD environment variable not set. Please set it to the board name."
                .replace("{}", &hal)
        }),
    );
    let mcu = fail_on_error(
        env::var(format!("ARM_{}_MCU", hal.to_uppercase())).with_context(|| {
            "ARM_{}_MCU environment variable not set. Please set it to the MCU name."
                .replace("{}", &hal)
        }),
    );

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
    println!("cargo:rerun-if-env-changed=ARM_HAL");
    println!("cargo:rerun-if-env-changed=ARM_{}_BOARD", hal.to_uppercase());
    println!("cargo:rerun-if-env-changed=ARM_{}_MCU", hal.to_uppercase());

    // Enable host feature when target triplet is equal to host target
    match host_triple() {
        Ok(host) => {
            if host == env::var("TARGET").unwrap_or_default() {
                println!("cargo:rustc-cfg=feature=\"host\"");
                println!("cargo:warning=Building for host, skipping HAL build.");
                // Only build when we are not on the host
                return;
            }
        }
        Err(e) => {
            println!("cargo:warning=Could not determine host triple: {}", e);
        }
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
