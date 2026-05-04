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
use core::panic;
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn check_cortex_m() -> bool {
    let target = env::var("TARGET").unwrap();

    let mut is_cortex_m = true;

    if target.starts_with("thumbv6m") {
        println!("cargo:rustc-cfg=cm0");
    } else if target.starts_with("thumbv7m") {
        println!("cargo:rustc-cfg=cm3");
    } else if target.starts_with("thumbv7em") {
        println!("cargo:rustc-cfg=cm4");
    } else if target.starts_with("thumbv8m.base") {
        println!("cargo:rustc-cfg=cm23");
    } else if target.starts_with("thumbv8m.main") {
        println!("cargo:rustc-cfg=cm33");
    } else {
        is_cortex_m = false;
    }

    if is_cortex_m {
        println!("cargo:rustc-cfg=cortex_m");
        return true;
    }

    false
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
    for (key, var) in env::vars() {
        if key.starts_with("OSIRIS_") {
            // Instruct cargo to rerun the build script if this environment variable changes.
            println!("cargo::rerun-if-env-changed={key}");

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
        }
    }
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

    println!("cargo::warning=ARM FPU: {fpu_type}, Float ABI: {float_abi}");

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
fn generate_bindings(out: &Path, hal: &Path) -> Result<()> {
    let bindgen = bindgen::Builder::default()
        .header(hal.join("interface").join("export.h").to_str().unwrap())
        .use_core()
        .wrap_unsafe_ops(true)
        .generate()?;

    bindgen.write_to_file(out.join("bindings.rs"))?;

    println!("cargo::rerun-if-changed={}", hal.display());
    Ok(())
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
            println!("cargo::error={e}");
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
        if let Ok(Value::Array(mut arr)) = serde_json::from_str::<Value>(data) {
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
fn workspace_dir() -> Option<PathBuf> {
    let output = Command::new("cargo")
        .args(["locate-project", "--workspace", "--message-format=plain"])
        .output()
        .ok()?;
    let path = String::from_utf8(output.stdout).expect("utf8");
    Some(PathBuf::from(path.trim()).parent()?.to_path_buf())
}

mod vector_table {
    pub fn generate() -> String {
        const LINES: usize = 240;

        let refs: Vec<_> = (0..LINES)
            .map(|i| quote::format_ident!("__irq_{i}_handler"))
            .collect();

        let defs: Vec<_> = refs.iter().enumerate().map(|(i, entry)| {
            quote::quote! {
                #[unsafe(no_mangle)]
                #[unsafe(naked)]
                unsafe extern "C" fn #entry() {
                   core::arch::naked_asm!(
                        "tst lr, #4",
                        "ite eq",
                        "mrseq r0, msp",
                        "mrsne r0, psp",
                        "mov r1, {vector}",
                        "b kernel_irq_handler",
                        vector = const #i,
                   );  
                }
            }
        }).collect();

        quote::quote! {
            #(#defs)*

            #[repr(C)]
            struct ExternalVectorTable {
                entries: [unsafe extern "C" fn(); #LINES],
            }

            #[unsafe(link_section = ".ivt.ext")]
            #[used]
            static EXTERNAL_VECTOR_TABLE: ExternalVectorTable = ExternalVectorTable {
                entries: [
                    #(#refs),*
                ],
            };
        }
        .to_string()
    }
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
    if !hal_builder::check_enabled("cortex-m") || !check_cortex_m() {
        return;
    }

    let dts = hal_builder::dt::check_dts()
        .expect("No DeviceTree specified. Set OSIRIS_DTS_PATH to specify.");
    let out = hal_builder::read_path_env("OUT_DIR");
    println!("cargo::rustc-link-search={}", out.display());

    let dt = hal_builder::dt::build_device_tree(&dts).unwrap_or_else(|e| {
        panic!("Failed to build device tree from DTS files: {e}");
    });

    if let Err(e) = hal_builder::dt::generate_device_tree(&dt, &out) {
        panic!("Failed to generate device tree scripts: {e}");
    }

    for (vendor, name) in hal_builder::dt::soc(&dt) {
        let hal = Path::new(vendor).join(name);

        if hal.exists() {
            fail_on_error(generate_bindings(&out, &hal));
            let vector_code = vector_table::generate();

            if let Err(e) = fs::write(PathBuf::from(&out).join("vector_table.rs"), vector_code) {
                println!("cargo::error=Failed to write vector_table.rs: {e}");
                std::process::exit(1);
            }

            let build_dir = PathBuf::from(&out).join("build");

            // Build the HAL library
            let mut libhal_config = cmake::Config::new(&hal);
            libhal_config.generator("Ninja");
            libhal_config.define("OUT_DIR", &out);

            for (vendor, name) in hal_builder::dt::soc(&dt) {
                if vendor == "st" {
                    libhal_config.cflag(format!("-D{}xx", name.to_uppercase()));
                }
                libhal_config.cflag(format!("-D{}", name.to_uppercase()));
            }

            libhal_config.always_configure(true);
            forward_env_vars(&mut libhal_config);
            fail_on_error(forward_fpu_config(&mut libhal_config));
            let libhal = libhal_config.build();

            println!("cargo::rustc-link-search=native={}", libhal.display());
            println!("cargo::rerun-if-changed={}/link.ld", out.display());

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

            println!("cargo::rerun-if-changed=common");
            println!("cargo::rustc-link-search=native={}", common.display());

            // Merge and export compile_commands.json for IDE integration
            let merged = merge_compile_commands(&[hal_cc, common_cc]);

            if let Some(project_root) = workspace_dir() {
                let out_file = project_root.join("compile_commands.json");
                fs::write(out_file, merged).expect("write merged compile_commands.json");
            } else {
                println!(
                    "cargo::warning=Could not determine workspace root, skipping compile_commands.json generation."
                );
            }

            return;
        }
    }

    panic!("No compatible SoC found in device tree");
}
