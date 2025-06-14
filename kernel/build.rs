use std::{collections::HashMap, fs::File, path::Path};

extern crate rand;
extern crate syn;
extern crate walkdir;

use std::io::Write;
use syn::{Attribute, LitInt};
use walkdir::WalkDir;

extern crate cbindgen;

fn main() {
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../include/syscalls.map.gen.h");

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
        writeln!(file, "extern void entry_{}(void *svc_args); \\", name)?;
    }

    writeln!(file)?;

    writeln!(file, "#define IMPLEMENT_SYSCALLS()     \\")?;
    for (name, number) in syscalls {
        writeln!(
            file,
            "    DECLARE_SYSCALL(entry_{}, {})      \\",
            name, number
        )?;
    }

    writeln!(file)?;

    writeln!(file, "#endif //SYSCALLS_MAP_GEN_H")?;

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
                        return Err(meta.error(format!("invalid syscall number: {}", num)));
                    }

                    return Ok(());
                }

                Err(meta.error(format!("unknown attribute '{}'", meta.path.get_ident().unwrap())))
            });

            if let Err(e) = result {
                println!(
                    "cargo:warning=Failed to parse syscall arguments for `{}`, {}",
                    name, e
                );
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
                        println!("cargo:warning=Duplicate syscall handler: {}", name);
                        continue;
                    }

                    if numbers.contains_key(&num) {
                        println!(
                            "cargo:warning=Duplicate syscall number: {} for {}",
                            num, name
                        );
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
