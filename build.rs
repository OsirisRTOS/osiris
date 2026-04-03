use core::panic;
use std::process::Command;
use std::{collections::HashMap, fs, fs::File, path::Path, path::PathBuf};

extern crate rand;
extern crate syn;
extern crate walkdir;

use cfg_aliases::cfg_aliases;
use quote::format_ident;
use std::io::Write;
use syn::{Attribute, LitInt};
use walkdir::WalkDir;

extern crate cbindgen;

fn main() {
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=build.rs");
    let out_dir = std::env::var("OUT_DIR").unwrap();

    if gen_syscall_match(Path::new("src/syscalls"), Path::new(&out_dir)).is_err() {
        panic!("Failed to generate syscall match statement.");
    }

    let dt = build_device_tree(Path::new(&out_dir)).unwrap_or_else(|e| {
        panic!("Failed to build device tree from DTS files: {e}");
    });

    if let Err(e) = generate_device_tree(&dt, Path::new(&out_dir)) {
        panic!("Failed to generate device tree scripts: {e}");
    }

    cfg_aliases! {
        freestanding: { all(not(test), not(doctest), not(doc), not(kani), any(target_os = "none", target_os = "unknown")) },
    }
}

// Device Tree Codegen ----------------------------------------------------------------------------

fn generate_device_tree(dt: &dtgen::ir::DeviceTree, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let rust_content = dtgen::generate_rust(dt);
    std::fs::write(out.join("device_tree.rs"), rust_content)?;

    let ld_content = dtgen::generate_ld(dt).map_err(|e| format!("linker script generation failed: {e}"))?;
    std::fs::write(out.join("prelude.ld"), ld_content)?;
    println!("cargo::rustc-link-search=native={}", out.display());
    Ok(())
}

fn build_device_tree(out: &Path) -> Result<dtgen::ir::DeviceTree, Box<dyn std::error::Error>> {
    let dts =
        std::env::var("OSIRIS_TUNING_DTS").unwrap_or_else(|_| "nucleo_l4r5zi.dts".to_string());
    let dts_path = std::path::Path::new("boards").join(dts);
    println!("cargo::rerun-if-changed={}", dts_path.display());

    // dependencies SoC/HAL/pins
    let zephyr = Path::new(out).join("zephyr");
    let hal_stm32 = Path::new(out).join("hal_stm32");

    // clean state
    if zephyr.exists() {
        std::fs::remove_dir_all(&zephyr)?;
    }

    if hal_stm32.exists() {
        std::fs::remove_dir_all(&hal_stm32)?;
    }

    sparse_clone(
        "https://github.com/zephyrproject-rtos/zephyr",
        &zephyr,
        // the west.yaml file is a manifest to manage/pin subprojects used for a specific zephyr
        // release
        &["include", "dts", "boards", "west.yaml"],
        Some("v4.3.0"),
    )?;

    // retrieve from manifest
    let hal_rev = get_hal_revision(&zephyr)?;
    println!("cargo:warning=Detected hal_stm32 revision: {hal_rev}");

    sparse_clone(
        "https://github.com/zephyrproject-rtos/hal_stm32",
        &hal_stm32,
        &["dts"],
        Some(&hal_rev),
    )?;

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

// Syscalls ---------------------------------------------------------------------------------------

fn gen_syscall_match(root: &Path, out: &Path) -> Result<(), std::io::Error> {
    let syscalls = find_syscalls(root);
    let mut file = File::create(out.join("syscall_match.in"))?;

    let arms = syscalls.iter().map(|(name, number)| {
        let entry = format_ident!("entry_{}", name);
        quote::quote! {
            #number => #entry(args),
        }
    });

    let syscall_match = quote::quote! {
        // This match statement is @generated by build.rs. Do not edit.
        match number {
            #(#arms)*
            _ => panic!("Unknown syscall number: {}", number),
        }
    };

    writeln!(file, "{syscall_match}")?;
    Ok(())
}

fn is_syscall(attrs: &[Attribute], name: &str) -> Option<u16> {
    let mut num = 0;

    for attr in attrs {
        if attr.path().is_ident("syscall_handler") {
            let result = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("num") {
                    let raw = meta.value()?;
                    let value: LitInt = raw.parse()?;
                    num = value.base10_parse::<u16>()?;

                    if !(0..=255).contains(&num) {
                        return Err(meta.error(format!("invalid syscall number: {num}")));
                    }

                    return Ok(());
                }

                Err(meta.error(format!(
                    "unknown attribute '{}'",
                    meta.path.get_ident().unwrap()
                )))
            });

            if let Err(e) = result {
                println!("cargo::warning=Failed to parse syscall arguments for `{name}`, {e}");
                return None;
            }

            return Some(num);
        }
    }

    None
}

fn find_syscalls(root: &Path) -> HashMap<String, u16> {
    let mut syscalls = HashMap::new();
    let mut numbers = HashMap::new();

    for entry in WalkDir::new(&root) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if entry.file_type().is_file() {
            let path = entry.path();

            println!("Processing file: {}", path.display());

            let contents = match std::fs::read_to_string(path) {
                Ok(contents) => contents,
                Err(_) => continue,
            };

            let file = match syn::parse_file(&contents) {
                Ok(file) => file,
                Err(_) => continue,
            };

            for item in file.items {
                let item = match item {
                    syn::Item::Fn(item) => item,
                    _ => continue,
                };

                let name = item.sig.ident.to_string();

                if let Some(num) = is_syscall(&item.attrs, &name) {
                    if syscalls.contains_key(&name) {
                        println!("cargo::warning=Duplicate syscall handler: {name}");
                        continue;
                    }

                    if numbers.contains_key(&num) {
                        println!("cargo::warning=Duplicate syscall number: {num} for {name}");
                        continue;
                    }

                    syscalls.insert(name.clone(), num);
                    numbers.insert(num, name);
                }
            }
        }
    }

    syscalls
}
