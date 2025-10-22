//! ARM build script for the Osiris project.
//!
//! This build script handles the compilation of ARM-specific components.
//!
//! The build process is target-aware and automatically configures itself based on:
//! - Target triple (thumbv6m, thumbv7m, thumbv7em, thumbv8m)
//! - FPU availability and configuration
//! - Environment variables prefixed with `OSIRIS_`
//!
//! # Build Artifacts
//!
//! - Static libraries for HAL and common components
//! - Linker script (link.ld) in OUT_DIR
//! - Rust FFI bindings (bindings.rs) in OUT_DIR
//! - IDE compile_commands.json in workspace root

use anyhow::{Context, Result};
use cmake::Config;
use std::{env, fs, path::PathBuf, process::Command};

/// Determines the host target triple by querying the Rust compiler.
///
/// This function executes `rustc -vV` and parses the output to extract
/// the host triple, which is used to detect when we're building for the
/// host platform versus a cross-compilation target.
///
/// # Returns
///
/// The host target triple as a string (e.g., "x86_64-unknown-linux-gnu")
///
/// # Errors
///
/// Returns an error if:
/// - `rustc` command execution fails
/// - Output cannot be parsed as UTF-8
/// - Host triple line is not found in compiler output
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

/// Sets ARM core-specific Rust configuration flags based on the target architecture.
///
/// This function analyzes the TARGET environment variable and emits appropriate
/// `cargo:rustc-cfg` directives to enable conditional compilation for different
/// ARM Cortex-M cores:
///
/// - `cm0` for Cortex-M0/M0+ (thumbv6m*)
/// - `cm3` for Cortex-M3 (thumbv7m*)  
/// - `cm4` for Cortex-M4/M7 (thumbv7em*)
///
/// These cfg flags can be used in Rust code with `#[cfg(cm4)]` for core-specific
/// optimizations or feature availability.
///
/// # Returns
///
/// `Ok(())` on success
///
/// # Errors
///
/// Returns an error if the TARGET environment variable is not set
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

/// Forwards OSIRIS_* environment variables to CMake configuration.
///
/// This function scans all environment variables and forwards any that start
/// with "OSIRIS_" to the CMake build system. Boolean-like values are normalized:
/// - "0", "false", "off" -> "0"
/// - "1", "true", "on" -> "1"  
/// - Other values are passed through unchanged
///
/// # Arguments
///
/// * `config` - The CMake configuration to modify
fn forward_env_vars(config: &mut Config) {
    let mut cnt = 0;

    for (key, var) in env::vars() {
        if key.starts_with("OSIRIS_") {
            // Instruct cargo to rerun the build script if this environment variable changes.
            println!("cargo:rerun-if-env-changed={key}");

            match var.as_str() {
                "0" | "false" | "off" => {
                    config.define(key, "0");
                }
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

/// Configures FPU settings for ARM targets based on architecture and user preferences.
///
/// This function determines the appropriate FPU type and float ABI based on:
/// - Target architecture capabilities
/// - OSIRIS_TUNING_ENABLEFPU environment variable
/// - Target triple float ABI suffix (hf for hard float)
///
/// The function automatically selects the correct FPU variant:
/// - **fpv4-sp-d16** for Cortex-M4/M7 (thumbv7em)
/// - **fpv5-sp-d16** for Cortex-M33 (thumbv8m.main)
///
/// Float ABI is chosen as:
/// - **hard**: Hardware FPU with hardware calling convention (hf targets)
/// - **softfp**: Hardware FPU with software calling convention (soft targets)
///
/// # Arguments
///
/// * `config` - Mutable reference to the CMake configuration to modify
///
/// # Returns
///
/// `Ok(())` on success, or if no FPU configuration is needed
///
/// # Errors
///
/// Returns an error if the TARGET environment variable is not set
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
            return Ok(());
        }
    } else if target.starts_with("thumbv8m.main") {
        if fpu_enabled {
            if target.contains("hf") {
                ("fpv5-sp-d16", "hard")
            } else {
                ("fpv5-sp-d16", "softfp")
            }
        } else {
            return Ok(());
        }
    } else {
        // Cortex-M0/M0+/M3 - no FPU
        return Ok(());
    };

    config.cflag(format!("-mfpu={fpu_type}"));
    config.cflag(format!("-mfloat-abi={float_abi}"));
    config.asmflag(format!("-mfpu={fpu_type}"));
    config.asmflag(format!("-mfloat-abi={float_abi}"));

    println!("cargo:info=ARM FPU: {fpu_type}, Float ABI: {float_abi}");

    Ok(())
}

/// Generates Rust FFI bindings from C header files using bindgen.
///
/// This function creates Rust bindings for the HAL's C interface by processing
/// the export.h header file. The generated bindings use core-only types and
/// wrap unsafe operations for improved safety.
///
/// # Arguments
///
/// * `out` - Output directory path for generated bindings.rs file
/// * `hal` - Path to HAL source directory containing interface/export.h
///
/// # Returns
///
/// `Ok(())` on successful binding generation
///
/// # Errors
///
/// Returns an error if:
/// - Header file cannot be found or parsed
/// - Binding generation fails
/// - Output file cannot be written
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

/// Detects if we're building for the host platform and enables host-only features.
///
/// This function compares the current target with the host triple to determine
/// if we're building for the host platform rather than cross-compiling. When
/// building for host, it enables the "host" feature and skips HAL compilation
/// since hardware-specific code isn't needed.
///
/// # Returns
///
/// `true` if building for host platform, `false` for cross-compilation
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

/// Error handling wrapper that converts Result failures to build script exit.
///
/// # Arguments
///
/// * `res` - Result to unwrap
///
/// # Returns
///
/// The success value from the Result
fn fail_on_error<T>(res: Result<T>) -> T {
    match res {
        Ok(val) => val,
        Err(e) => {
            println!("cargo:error={e}");
            std::process::exit(1);
        }
    }
}

/// Merges multiple compile_commands.json files into a single unified file.
///
/// This function takes JSON compilation database files and combines them into a single database.
/// Each input should be a JSON array of compilation commands.
///
/// # Arguments
///
/// * `files` - Slice of JSON strings representing compile_commands.json content
///
/// # Returns
///
/// A pretty-printed JSON string containing all compilation commands
fn merge_compile_commands(files: &[String]) -> String {
    use serde_json::Value;

    let mut entries = Vec::new();
    for data in files {
        if let Ok(Value::Array(mut arr)) = serde_json::from_str::<Value>(&data) {
            entries.append(&mut arr);
        }
    }
    serde_json::to_string_pretty(&entries).unwrap()
}

/// Determines the workspace root directory using cargo.
///
/// This function executes `cargo locate-project --workspace` to find the
/// workspace root directory, which is used for placing the merged
/// compile_commands.json file in the correct location.
///
/// # Returns
///
/// PathBuf pointing to the workspace root directory
fn workspace_dir() -> PathBuf {
    let output = Command::new("cargo")
        .args(["locate-project", "--workspace", "--message-format=plain"])
        .output()
        .expect("failed to run cargo locate-project");
    let path = String::from_utf8(output.stdout).expect("utf8");
    PathBuf::from(path.trim()).parent().unwrap().to_path_buf()
}

/// Main build script entry point.
///
/// This function orchestrates the entire build process:
///
/// 1. **Environment Setup**: Reads configuration from environment variables
/// 2. **Binding Generation**: Creates Rust FFI bindings from C headers  
/// 3. **Core Configuration**: Sets up ARM core-specific cfg flags
/// 4. **Host Detection**: Skips hardware builds when targeting host
/// 5. **HAL Compilation**: Builds hardware abstraction layer via CMake
/// 6. **Common Library**: Builds shared common components
/// 7. **IDE Integration**: Merges compilation databases for IDE support
///
/// The build process is conditional - hardware-specific components are only
/// built when cross-compiling for ARM targets, not when building for the host.
///
/// # Environment Variables Required
///
/// - `OSIRIS_ARM_HAL`: Path to ARM HAL source directory
/// - `TARGET`: Rust target triple (automatically set by cargo)
/// - `OUT_DIR`: Build output directory (automatically set by cargo)
///
/// # Panics
///
/// Exits with error code 1 if any critical build step fails
fn main() {
    let out = env::var("OUT_DIR").unwrap_or("src".to_string());

    let hal = fail_on_error(env::var("OSIRIS_ARM_HAL").with_context(
        || "OSIRIS_ARM_HAL environment variable not set. Please set it to the path of the ARM HAL.",
    ));

    fail_on_error(generate_bindings(&out, &hal));
    fail_on_error(set_arm_core_cfg());

    // Only build when we are not on the host
    if check_for_host() {
        return;
    }

    let build_dir = PathBuf::from(&out).join("build");

    // Build the HAL library
    let mut libhal_config = cmake::Config::new(&hal);
    libhal_config.generator("Ninja");
    libhal_config.define("OUT_DIR", out.clone());
    libhal_config.always_configure(true);
    forward_env_vars(&mut libhal_config);
    fail_on_error(forward_fpu_config(&mut libhal_config));
    let libhal = libhal_config.build();

    println!("cargo:rustc-link-search=native={}", libhal.display());
    println!("cargo:linker-script={out}/link.ld");

    // Extract compile commands for HAL
    let hal_cc = build_dir.join("compile_commands.json");
    let hal_cc = fs::read_to_string(hal_cc).unwrap_or_default();

    // Build the common library
    let mut common_config = cmake::Config::new("common");
    common_config.generator("Ninja");
    common_config.always_configure(true);
    forward_env_vars(&mut common_config);
    fail_on_error(forward_fpu_config(&mut common_config));
    let common = common_config.build();

    // Extract compile commands for common
    let common_cc = build_dir.join("compile_commands.json");
    let common_cc = fs::read_to_string(common_cc).unwrap_or_default();

    println!("cargo:rerun-if-changed=common");
    println!("cargo:rustc-link-search=native={}", common.display());

    // Merge and export compile_commands.json for IDE integration
    let merged = merge_compile_commands(&[hal_cc, common_cc]);

    let project_root = workspace_dir();
    let out_file = project_root.join("compile_commands.json");

    fs::write(out_file, merged).expect("write merged compile_commands.json");
}
