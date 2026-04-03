use std::process::Command;
use std::{collections::HashMap, fs, fs::File, io::Write, path::Path, path::PathBuf};

extern crate rand;
extern crate syn;
extern crate walkdir;

use cfg_aliases::cfg_aliases;
use quote::{format_ident, quote};
use syn::{Attribute, FnArg, LitInt, punctuated::Punctuated, token::Comma};
use walkdir::WalkDir;

extern crate cbindgen;

fn main() {
    println!("cargo::rerun-if-changed=src");
    println!("cargo::rerun-if-changed=build.rs");

    generate_syscall_map("src/syscalls").expect("Failed to generate syscall map.");
    generate_syscalls_export("src/syscalls").expect("Failed to generate syscall exports.");

    generate_device_tree().expect("Failed to generate device tree.");

    // Get linker script from environment variable
    if let Ok(linker_script) = std::env::var("DEP_HAL_LINKER_SCRIPT") {
        println!("cargo::rustc-link-arg=-T{linker_script}");
    } else {
        println!("cargo::warning=LD_SCRIPT_PATH environment variable not set.");
    }

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

type SyscallData = u16;
type SyscallDataExport = (u16, Punctuated<FnArg, Comma>);

fn generate_syscalls_export<P: AsRef<Path>>(root: P) -> Result<(), std::io::Error> {
    let syscalls = collect_syscalls(root);

    let functions = syscalls.iter().map(|(name, (number, inputs))| {
        let fn_name = format_ident!("{}", name);
        let arg_names: Vec<_> = inputs
            .iter()
            .filter_map(|arg| match arg {
                FnArg::Typed(pat_type) => match &*pat_type.pat {
                    syn::Pat::Ident(pat_ident) => Some(pat_ident.ident.clone()),
                    _ => None,
                },
                _ => None,
            })
            .collect();

        quote! {
            pub fn #fn_name(#inputs) {
                hal::asm::syscall!(#number #(, #arg_names)*);
            }
        }
    });

    let tokens = quote! { #(#functions)* };

    // publish
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("syscalls_export.rs");
    let mut file = File::create(out_path)?;

    write!(
        file,
        "// This file is @generated by build.rs. Do not edit!\n\n{}",
        prettyplease::unparse(&syn::parse2(tokens).unwrap())
    )?;
    Ok(())
}

fn generate_syscall_map<P: AsRef<Path>>(root: P) -> Result<(), std::io::Error> {
    let syscalls = collect_syscalls(root)
        .into_iter()
        // map from SyscallDataExport to SyscallData
        .map(|(name, (num, _))| (name, num))
        .collect::<HashMap<String, SyscallData>>();

    let arms = syscalls.iter().map(|(name, &number)| {
        let entry = format_ident!("entry_{}", name);

        // the wrapper context defines what type the literal correspondences to
        let literal = proc_macro2::Literal::u16_unsuffixed(number);
        quote! {
            #literal => #entry(args),
        }
    });

    let tokens = quote! {
        match number {
            #(#arms)*
            _ => panic!("Unknown syscall number: {}", number),
        }
    };

    // publish
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_path = Path::new(&out_dir).join("syscall_dispatcher.in");
    let mut file = File::create(out_path)?;

    // the token stream cannot be parsed since prettyplease cannot parse raw expressions
    write!(
        file,
        "// This file is @generated by build.rs. Do not edit!\n\n{}",
        tokens
    )?;
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

fn collect_syscalls<P: AsRef<Path>>(root: P) -> HashMap<String, SyscallDataExport> {
    let mut syscalls: HashMap<String, SyscallDataExport> = HashMap::new();
    let mut numbers: HashMap<u16, String> = HashMap::new();

    let entries = WalkDir::new(&root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file());

    for entry in entries {
        let path = entry.path();
        println!("Processing file: {}", path.display());

        let Ok(contents) = std::fs::read_to_string(entry.path()) else {
            continue;
        };

        let Ok(file) = syn::parse_file(&contents) else {
            continue;
        };

        for item in file.items.into_iter().filter_map(|i| match i {
            syn::Item::Fn(f) => Some(f),
            _ => None,
        }) {
            let name = item.sig.ident.to_string();
            let Some(num) = is_syscall(&item.attrs, &name) else {
                continue;
            };

            if syscalls.contains_key(&name) {
                println!("cargo::warning=Duplicate syscall handler: {name}");
                continue;
            }

            if numbers.contains_key(&num) {
                println!("cargo::warning=Duplicate syscall number: {num} for {name}");
                continue;
            }

            numbers.insert(num, name.clone());
            syscalls.insert(name, (num, item.sig.inputs));
        }
    }

    syscalls
}
