use anyhow::{Context, Result};
use cmake::Config;
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

fn set_arm_core_cfg() -> Result<()> {
    // Add a rust cfg based on the target architecture. For thumbv7em we set "cortex-m4".
    let target = env::var("TARGET").context("TARGET environment variable not set")?;

    if target.starts_with("thumbv6m") {
        println!("cargo:rustc-cfg=cm0");
    } else if target.starts_with("thumbv7m") {
        println!("cargo:rustc-cfg=cm3");
    } else if target.starts_with("thumbv7em") {
        println!("cargo:rustc-cfg=cm4");
    }

    Ok(())
}

fn forward_env_vars(config: &mut Config) {
    let mut cnt = 0;

    for (key, var) in env::vars() {
        if key.starts_with("OSIRIS_") {
            // Instruct cargo to rerun the build script if this environment variable changes.
            println!("cargo:rerun-if-env-changed={key}");

        
            match var.as_str() {
                "0" | "false" | "off" => {
                    config.define(key, "0");
                },
                "1" | "true" | "on" => {
                    config.define(key, "1");
                }
                _ => {
                    config.define(key, var.as_str());
                }
            }

            cnt += 1;
        }
    }

    println!("cargo:info=Forwarded {cnt} OSIRIS_* environment variables to CMake");
}

fn forward_fpu_config(config: &mut Config) -> Result<()> {
    let target = env::var("TARGET").context("TARGET environment variable not set")?;
    
    // Check if FPU is enabled via config
    let fpu_enabled = env::var("OSIRIS_TUNING_ENABLEFPU")
        .map(|v| matches!(v.as_str(), "1" | "true" | "on"))
        .unwrap_or(false);
    
    // Determine FPU and float ABI based on target and config
    let (fpu_type, float_abi) = if target.starts_with("thumbv7em") {
        if fpu_enabled {
            // Hardware FPU available and enabled
            if target.contains("hf") {
                // Target has hard float ABI
                ("fpv4-sp-d16", "hard")
            } else {
                // Target has soft float ABI but we can still use hardware FPU
                ("fpv4-sp-d16", "softfp")
            }
        } else {
            return Ok(())
        }
    } else if target.starts_with("thumbv8m.main") {
        if fpu_enabled {
            if target.contains("hf") {
                ("fpv5-sp-d16", "hard")
            } else {
                ("fpv5-sp-d16", "softfp")
            }
        } else {
            return Ok(())
        }
    } else {
        // Cortex-M0/M0+/M3 - no FPU
        return Ok(())
    };

    config.cflag(format!("-mfpu={fpu_type}"));
    config.cflag(format!("-mfloat-abi={float_abi}"));
    config.asmflag(format!("-mfpu={fpu_type}"));
    config.asmflag(format!("-mfloat-abi={float_abi}"));

    println!("cargo:info=ARM FPU: {fpu_type}, Float ABI: {float_abi}");
    
    Ok(())
}

fn generate_bindings(out: &str, hal: &str) -> Result<()> {
    let bindgen = bindgen::Builder::default()
        .header(format!("{hal}/interface/export.h"))
        .use_core()
        .wrap_unsafe_ops(true)
        .generate()?;

    bindgen.write_to_file(format!("{out}/bindings.rs"))?;

    println!("cargo:rerun-if-changed={hal}"); 
    Ok(())
}

fn check_for_host() -> bool {
    // Enable host feature when target triplet is equal to host target
    match host_triple() {
        Ok(host) => {
            if host == env::var("TARGET").unwrap_or_default() {
                println!("cargo:rustc-cfg=feature=\"host\"");
                println!("cargo:warning=Building for host, skipping HAL build.");
                // Only build when we are not on the host
                return true;
            }
        }
        Err(e) => {
            println!("cargo:warning=Could not determine host triple: {e}");
        }
    }

    false
}

fn fail_on_error<T>(res: Result<T>) -> T {
    match res {
        Ok(val) => val,
        Err(e) => {
            println!("cargo:error={e}");
            std::process::exit(1);
        }
    }
}

fn main() {
    let out = env::var("OUT_DIR").unwrap_or("src".to_string());

    let hal = fail_on_error(env::var("OSIRIS_HAL").with_context(
        || "HAL environment variable not set. Please set it to the path of the ARM HAL.",
    ));

    fail_on_error(generate_bindings(&out, &hal));
    fail_on_error(set_arm_core_cfg());

    // Only build when we are not on the host
    if check_for_host() {
        return;
    }

    // Build the HAL library
    let mut libhal_config = cmake::Config::new(&hal);
    libhal_config.define("OUT_DIR", out.clone());
    libhal_config.always_configure(true);
    forward_env_vars(&mut libhal_config);
    fail_on_error(forward_fpu_config(&mut libhal_config));
    let libhal = libhal_config.build();

    println!("cargo:rustc-link-search=native={}", libhal.display());
    println!("cargo:linker-script={out}/link.ld");

    // Build the common library
    let mut common_config = cmake::Config::new("common");
    common_config.always_configure(true);
    forward_env_vars(&mut common_config);
    fail_on_error(forward_fpu_config(&mut common_config));
    let common = common_config.build();

    println!("cargo:rerun-if-changed=common");

    println!("cargo:rustc-link-search=native={}", common.display());
}
