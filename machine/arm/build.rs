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
/// `cargo::rustc-cfg` directives to enable conditional compilation for different
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
        println!("cargo::rustc-cfg=cm0");
    } else if target.starts_with("thumbv7m") {
        println!("cargo::rustc-cfg=cm3");
    } else if target.starts_with("thumbv7em") {
        println!("cargo::rustc-cfg=cm4");
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
                println!("cargo::rustc-cfg=feature=\"host\"");
                println!("cargo::warning=Building for host, skipping HAL build.");
                // Only build when we are not on the host
                return true;
            }
        }
        Err(e) => {
            println!("cargo::warning=Could not determine host triple: {e}");
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
        let entries: Vec<_> = (0..240)
            .map(|i| quote::format_ident!("__irq_{i}_handler"))
            .collect();

        quote::quote! {
            unsafe extern "C" {
                #(
                    fn #entries();
                )*
            }

            #[repr(C)]
            struct ExternalVectorTable {
                entries: [unsafe extern "C" fn(); 240],
            }

            #[unsafe(link_section = ".ivt.ext")]
            #[used]
            static EXTERNAL_VECTOR_TABLE: ExternalVectorTable = ExternalVectorTable {
                entries: [
                    #(#entries),*
                ],
            };
        }
        .to_string()
    }
}

// Device Tree Codegen ----------------------------------------------------------------------------

mod dt {
    use std::{
        fs,
        path::{Path, PathBuf},
        process::Command,
    };

    use crate::workspace_dir;

    /// Returns the compatible (vendor, name) tuple for the SoC node in the device tree.
    pub fn soc(dt: &dtgen::ir::DeviceTree) -> Vec<(&str, &str)> {
        let soc_node = dt
            .nodes
            .iter()
            .find(|n| n.name == "soc")
            .expect("Device tree must have a soc node");

        soc_node
            .compatible
            .iter()
            .filter_map(|s| {
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() == 2 {
                    Some((parts[0], parts[1]))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn generate_device_tree(
        dt: &dtgen::ir::DeviceTree,
        out: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let rust_content = dtgen::generate_rust(dt);
        std::fs::write(out.join("device_tree.rs"), rust_content)?;

        let ld_content =
            dtgen::generate_ld(dt).map_err(|e| format!("linker script generation failed: {e}"))?;
        std::fs::write(out.join("prelude.ld"), ld_content)?;
        println!("cargo::rustc-link-search=native={}", out.display());
        Ok(())
    }

    pub fn build_device_tree(
        out: &Path,
    ) -> Result<dtgen::ir::DeviceTree, Box<dyn std::error::Error>> {
        let dts = std::env::var("OSIRIS_TUNING_DTS")
            .expect("OSIRIS_TUNING_DTS environment variable not set");
        let workspace_root = workspace_dir().ok_or("Could not determine workspace root")?;
        let dts_path = workspace_root.join("boards").join(dts);
        println!("cargo::rerun-if-changed={}", dts_path.display());

        // dependencies SoC/HAL/pins
        let zephyr = Path::new(out).join("zephyr");
        let hal_stm32 = Path::new(out).join("hal_stm32");

        if !zephyr.exists() {
            sparse_clone(
                "https://github.com/zephyrproject-rtos/zephyr",
                &zephyr,
                // the west.yaml file is a manifest to manage/pin subprojects used for a specific zephyr
                // release
                &["include", "dts", "boards", "west.yaml"],
                Some("v4.3.0"),
            )?;
        }

        if !hal_stm32.exists() {
            // retrieve from manifest
            let hal_rev = get_hal_revision(&zephyr)?;
            println!("cargo:warning=Detected hal_stm32 revision: {hal_rev}");

            sparse_clone(
                "https://github.com/zephyrproject-rtos/hal_stm32",
                &hal_stm32,
                &["dts"],
                Some(&hal_rev),
            )?;
        }

        //let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("device_tree.rs");
        let include_paths = [
            zephyr.join("include"),
            zephyr.join("dts/arm/st"),
            zephyr.join("dts/arm/st/l4"),
            zephyr.join("dts"),
            zephyr.join("dts/arm"),
            zephyr.join("dts/common"),
            zephyr.join("boards/st"),
            hal_stm32.join("dts"),
            hal_stm32.join("dts/st"),
        ];
        let include_refs: Vec<&Path> = include_paths.iter().map(PathBuf::as_path).collect();

        for path in &include_paths {
            if !path.exists() {
                println!("cargo:warning=MISSING INCLUDE PATH: {:?}", path);
            }
        }

        Ok(dtgen::parse_dts(&dts_path, &include_refs)?)
    }

    fn get_hal_revision(zephyr_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
        let west_yml = fs::read_to_string(zephyr_path.join("west.yml"))?;
        let mut in_hal_stm32_block = false;

        for line in west_yml.lines() {
            let trimmed = line.trim();

            // Check if we've entered the hal_stm32 section
            if trimmed == "- name: hal_stm32" || trimmed == "name: hal_stm32" {
                in_hal_stm32_block = true;
                continue;
            }

            // If we are in the block, look for the revision
            if in_hal_stm32_block {
                if trimmed.starts_with("revision:") {
                    return Ok(trimmed.replace("revision:", "").trim().to_string());
                }

                // If we hit a new project name before finding a revision, something is wrong
                if trimmed.starts_with("- name:") || trimmed.starts_with("name:") {
                    in_hal_stm32_block = false;
                }
            }
        }

        Err("Could not find hal_stm32 revision in west.yml".into())
    }

    fn sparse_clone(
        url: &str,
        dest: &Path,
        paths: &[&str],
        revision: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Command::new("git")
            .args(["clone", "--filter=blob:none", "--no-checkout", url])
            .arg(dest)
            .status()?;

        Command::new("git")
            .args(["sparse-checkout", "init", "--cone"])
            .current_dir(dest)
            .status()?;

        Command::new("git")
            .arg("sparse-checkout")
            .arg("set")
            .args(paths)
            .current_dir(dest)
            .status()?;

        let mut checkout = Command::new("git");
        checkout.current_dir(dest).arg("checkout");

        if let Some(rev) = revision {
            checkout.arg(rev);
        }

        checkout.status()?;
        Ok(())
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
    let out = env::var("OUT_DIR").unwrap();
    let out = Path::new(&out);
    println!("cargo::rustc-link-search={}", out.display());

    let dt = dt::build_device_tree(out).unwrap_or_else(|e| {
        panic!("Failed to build device tree from DTS files: {e}");
    });

    if let Err(e) = dt::generate_device_tree(&dt, out) {
        panic!("Failed to generate device tree scripts: {e}");
    }

    for (vendor, name) in dt::soc(&dt) {
        let hal = Path::new(vendor).join(name);

        if hal.exists() {
            fail_on_error(generate_bindings(&out, &hal));
            fail_on_error(set_arm_core_cfg());

            let vector_code = vector_table::generate();

            if let Err(e) = fs::write(PathBuf::from(&out).join("vector_table.rs"), vector_code) {
                println!("cargo::error=Failed to write vector_table.rs: {e}");
                std::process::exit(1);
            }

            // Only build when we are not on the host
            if check_for_host() {
                return;
            }

            let build_dir = PathBuf::from(&out).join("build");

            // Build the HAL library
            let mut libhal_config = cmake::Config::new(&hal);
            libhal_config.generator("Ninja");
            libhal_config.define("OUT_DIR", out);

            for (vendor, name) in dt::soc(&dt) {
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
