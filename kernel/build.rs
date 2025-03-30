use std::{collections::HashMap, fs::File, path::Path};

extern crate rand;
extern crate syn;
extern crate walkdir;

use cbindgen::LayoutConfig;
use std::io::Write;
use syn::{Attribute, LitInt};
use walkdir::WalkDir;

extern crate cbindgen;

fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");

    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    let mut config: cbindgen::Config = Default::default();

    config.no_includes = true;
    config.includes = vec![
        "stdint.h".to_string(),
        "stdbool.h".to_string(),
        "stdarg.h".to_string(),
    ];
    config.layout = LayoutConfig {
        packed: Some("__attribute__((packed))".to_string()),
        ..Default::default()
    };

    let bindings = cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .with_language(cbindgen::Language::C)
        .with_include_guard("KERNEL_H")
        .generate();

    match bindings {
        Ok(bindings) => {
            bindings.write_to_file("include/kernel/lib.h");
        }
        Err(e) => {
            panic!("Error generating bindings: {}", e);
        }
    }

    generate_syscall_map("src").expect("Failed to generate syscall map.");
}

fn generate_syscall_map<P: AsRef<Path>>(root: P) -> Result<(), std::io::Error> {
    let syscalls = collect_syscalls(root);

    let mut file = File::create("../include/syscalls.map.gen.h")?;

    writeln!(file, "#ifndef SYSCALLS_MAP_GEN_H")?;
    writeln!(file, "#define SYSCALLS_MAP_GEN_H")?;

    writeln!(file)?;

    writeln!(file, "#include <stdint.h>")?;

    writeln!(file)?;

    writeln!(
        file,
        "#define DECLARE_SYSCALL(name, num) case num: name(svc_args); break;"
    )?;

    writeln!(file)?;

    writeln!(file, "#define DECLARE_SYSCALLS() \\")?;
    for (name, _) in syscalls.clone() {
        writeln!(file, "extern void {}(void *svc_args); \\", name)?;
    }

    writeln!(file)?;

    writeln!(file, "#define IMPLEMENT_SYSCALLS()     \\")?;
    for (name, (number, _argc)) in syscalls {
        writeln!(file, "    DECLARE_SYSCALL({}, {})", name, number)?;
    }

    writeln!(file)?;

    writeln!(file, "#endif //SYSCALLS_MAP_GEN_H")?;

    Ok(())
}

fn is_syscall(attrs: &[Attribute]) -> Option<(u8, u8)> {
    let mut args = 0;
    let mut num = 0;

    for attr in attrs {
        if attr.path().is_ident("syscall_handler") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("args") {
                    let raw = meta.value()?;
                    let value: LitInt = raw.parse()?;
                    args = value.base10_parse()?;

                    if !(0..=4).contains(&args) {
                        return Err(meta.error("invalid number of arguments"));
                    }

                    return Ok(());
                }

                if meta.path.is_ident("num") {
                    let raw = meta.value()?;
                    let value: LitInt = raw.parse()?;
                    num = value.base10_parse()?;

                    if !(0..=255).contains(&num) {
                        return Err(meta.error("invalid syscall number"));
                    }

                    return Ok(());
                }

                Err(meta.error("unknown attribute"))
            })
            .expect("failed to parse attribute");
        }
    }

    if args == 0 || num == 0 {
        return None;
    }

    Some((num, args))
}

type SyscallData = (u8, u8);

fn collect_syscalls<P: AsRef<Path>>(root: P) -> HashMap<String, SyscallData> {
    let mut syscalls = HashMap::new();
    let mut numbers = HashMap::new();

    for entry in WalkDir::new(root) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        if entry.file_type().is_file() {
            let path = entry.path();

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

                if !item
                    .sig
                    .abi
                    .is_some_and(|abi| abi.name.is_some_and(|name| name.value() == "C"))
                {
                    continue;
                }

                if let Some((num, argc)) = is_syscall(&item.attrs) {
                    let name = item.sig.ident.to_string();

                    if syscalls.contains_key(&name) {
                        eprintln!("Duplicate syscall handler: {}", name);
                        continue;
                    }

                    syscalls.insert(name.clone(), (num, argc));
                    numbers.insert(num, name);
                }
            }
        }
    }

    syscalls
}
