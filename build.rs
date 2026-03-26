use std::process::Command;
use std::{collections::HashMap, fs, fs::File, path::Path, path::PathBuf};

extern crate rand;
extern crate syn;
extern crate walkdir;

use cfg_aliases::cfg_aliases;
use quote::ToTokens;
use std::io::Write;
use syn::{Attribute, FnArg, LitInt, punctuated::Punctuated, token::Comma};
use walkdir::WalkDir;

extern crate cbindgen;

fn main() {
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=build.rs");

    generate_syscall_map("src/syscalls").expect("Failed to generate syscall map.");
    generate_syscalls_export("src/syscalls").expect("Failed to generate syscall exports.");

    cfg_aliases! {
        freestanding: { all(not(test), not(doctest), not(doc), not(kani), any(target_os = "none", target_os = "unknown")) },
    }
}

// Device Tree Codegen ----------------------------------------------------------------------------

fn generate_device_tree() -> Result<(), Box<dyn std::error::Error>> {
    let dts =
        std::env::var("OSIRIS_TUNING_DTS").unwrap_or_else(|_| "nucleo_l4r5zi.dts".to_string());
    println!("cargo::rerun-if-changed={dts}");

    let dts_path = std::path::Path::new("boards").join(dts);

    // dependencies SoC/HAL/pins
    let zephyr = Path::new(&std::env::var("OUT_DIR").unwrap()).join("zephyr");
    let hal_stm32 = Path::new(&std::env::var("OUT_DIR").unwrap()).join("hal_stm32");

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

    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("device_tree.rs");
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

    dtgen::run(&dts_path, &include_refs, &out)?;
    Ok(())
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

fn generate_syscalls_export<P: AsRef<Path>>(root: P) -> Result<(), std::io::Error> {
    let syscalls = collect_syscalls_export(root);

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("syscalls_export.rs");
    let mut file = File::create(out_path)?;

    writeln!(file, "// This file is @generated by build.rs. Do not edit!")?;

    for (name, (number, inputs)) in &syscalls {
        let mut args = &inputs.iter().fold("".to_owned(), |acc, arg| {
            acc + "," + &arg.into_token_stream().to_string()
        })[..];
        if !args.is_empty() {
            args = &args[1..];
        }
        let names = get_arg_names(args);
        writeln!(file)?;
        writeln!(file, "pub fn {name}({args}) {{")?;
        writeln!(file, "    hal::asm::syscall!({number}{names});")?;
        writeln!(file, "}}")?;
    }

    Ok(())
}

fn get_arg_names(args: &str) -> String {
    if args.is_empty() {
        return "".to_string();
    }
    let mut in_arg_name = true;

    ", ".to_owned()
        + &args.chars().fold("".to_owned(), |mut acc, char| {
            if char.eq(&' ') {
                in_arg_name = false;
                return acc;
            }
            if char.eq(&',') {
                in_arg_name = true;
                return acc + ", ";
            }
            if in_arg_name {
                acc.push(char);
            }
            acc
        })
}

fn generate_syscall_map<P: AsRef<Path>>(root: P) -> Result<(), std::io::Error> {
    let syscalls = collect_syscalls(root);

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("syscall_dispatcher.in");
    let mut file = File::create(out_path)?;

    writeln!(file, "// This file is @generated by build.rs. Do not edit!")?;
    writeln!(file)?;
    writeln!(file, "match number {{")?;

    for (name, number) in &syscalls {
        writeln!(file, "    {number} => entry_{name}(args),")?;
    }

    writeln!(
        file,
        "    _ => panic!(\"Unknown syscall number: {{}}\", number),"
    )?;
    writeln!(file, "}}")?;

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

type SyscallData = u16;

fn collect_syscalls<P: AsRef<Path>>(root: P) -> HashMap<String, SyscallData> {
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

type SyscallDataExport = (u16, Punctuated<FnArg, Comma>);

fn collect_syscalls_export<P: AsRef<Path>>(root: P) -> HashMap<String, SyscallDataExport> {
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
                        println!("cargo:warning=Duplicate syscall handler: {name}");
                        continue;
                    }

                    if numbers.contains_key(&num) {
                        println!("cargo:warning=Duplicate syscall number: {num} for {name}");
                        continue;
                    }

                    syscalls.insert(name.clone(), (num, item.sig.inputs));
                    numbers.insert(num, name);
                }
            }
        }
    }

    syscalls
}
