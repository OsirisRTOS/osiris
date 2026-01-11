use cargo_metadata::MetadataCommand;
use clap::Parser;
use object::{Object, ObjectSection};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "injector")]
#[command(about = "Patches runtime symbols into the kernel ELF", long_about = None)]
struct Cli {
    /// Path to the crate's manifest.
    manifest: PathBuf,
    
    /// Target triple. If not specified, will be read from .cargo/config.toml
    #[arg(short, long)]
    target: Option<String>,
}

fn extract_section(file_path: &PathBuf, section_name: &str) -> Result<Vec<u8>, String> {
    let mut file = File::open(file_path).map_err(|e| format!("failed to open file: {}", e))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("failed to read file: {}", e))?;

    let obj_file = object::File::parse(&*buffer)
        .map_err(|e| format!("failed to parse ELF file: {}", e))?;

    let section = obj_file
        .section_by_name(section_name)
        .ok_or_else(|| format!("section '{}' not found.", section_name))?;

    let data = section
        .data()
        .map_err(|e| format!("failed to read section data: {}", e))?;

    Ok(data.to_vec())
}

fn inject_section(file_path: &PathBuf, section_name: &str, new_data: &[u8]) -> Result<(), String> {
    let mut file = File::open(file_path).map_err(|e| format!("failed to open file: {}", e))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("failed to read file: {}", e))?;

    let obj_file = object::File::parse(&*buffer)
        .map_err(|e| format!("failed to parse ELF file: {}", e))?;

    let section = obj_file
        .section_by_name(section_name)
        .ok_or_else(|| format!("section '{}' not found.", section_name))?;

    let data = section
        .data()
        .map_err(|e| format!("failed to read section data: {}", e))?;

    // we compiled without support for runtime symbols, so silently ignore
    if data.len() <= 4 {
        log::warn!("binary compiled without runtime symbols support. skipping.");
        return Ok(());
    }

    if new_data.len() > data.len() {
        return Err(format!(
            "new data size exceeds original section size. {} > {}",
            new_data.len(),
            data.len()
        ));
    }

    let offset = section.file_range().ok_or("section has no file range")?.0;

    let mut file = OpenOptions::new()
        .write(true)
        .open(file_path)
        .map_err(|e| format!("failed to open file for writing: {}", e))?;

    file.seek(SeekFrom::Start(offset))
        .map_err(|e| format!("failed to seek to section offset: {}", e))?;

    file.write_all(new_data)
        .map_err(|e| format!("failed to write new data: {}", e))?;

    Ok(())
}

fn build_symtab_strtab_blob(symtab: &[u8], strtab: &[u8]) -> Vec<u8> {
    let header = (symtab.len() as u32).to_le_bytes();
    let mut blob = Vec::with_capacity(4 + symtab.len() + strtab.len());
    blob.extend_from_slice(&header);
    blob.extend_from_slice(symtab);
    blob.extend_from_slice(strtab);
    blob
}

fn inject(elf: &PathBuf) -> Result<(), String> {
    let symtab = extract_section(elf, ".symtab")?;
    let strtab = extract_section(elf, ".strtab")?;

    let blob = build_symtab_strtab_blob(&symtab, &strtab);

    inject_section(elf, ".syms_area", &blob)
}

fn get_target_from_cargo_config(manifest_dir: &PathBuf) -> Option<String> {
    let cargo_config = manifest_dir.join(".cargo").join("config.toml");
    
    if !cargo_config.exists() {
        return None;
    }
    
    let contents = std::fs::read_to_string(&cargo_config).ok()?;
    let config: toml::Value = toml::from_str(&contents).ok()?;
    
    config
        .get("build")
        .and_then(|build| build.get("target"))
        .and_then(|target| target.as_str())
        .map(|s| s.to_string())
}

fn extract_binaries(manifest: &PathBuf, target_triple: Option<String>) -> Result<Vec<PathBuf>, String> {
    let cmd = MetadataCommand::new()
        .manifest_path(manifest)
        .no_deps()
        .exec()
        .map_err(|e| format!("failed to get cargo metadata: {}", e))?;

    let manifest_dir = manifest.parent()
        .ok_or("invalid manifest path")?;
    
    let target = match target_triple {
        Some(t) => t,
        None =>
            get_target_from_cargo_config(&manifest_dir.to_path_buf())
            .ok_or("no target specified and no build.target found in .cargo/config.toml")?
    };

    if target == "host-tuple" {
        log::info!("host target detected, skipping injector.");
        std::process::exit(0);
    }
    
    log::info!("using target: {}", target);
    
    let package = cmd.root_package()
        .ok_or("no root package found")?;
    
    let mut binaries = Vec::new();
    
    for bin_target in &package.targets {
        if bin_target.kind.iter().any(|k| matches!(k, cargo_metadata::TargetKind::Bin)) {
            let binary_path = cmd.target_directory.join(&target);

            for entry in walkdir::WalkDir::new(&binary_path).into_iter().filter_map(Result::ok) {
                if entry.file_type().is_dir() {
                    let binary_path = entry.path().join(&bin_target.name);

                    if binary_path.exists() {
                        binaries.push(binary_path.into());
                    }
                }
            }
        }
    }
    
    if binaries.is_empty() {
        return Err("no binaries found".to_string());
    }
    
    Ok(binaries)
}

fn main() {
    logging::init();

    let cli = Cli::parse();

    let binaries = match extract_binaries(&cli.manifest, cli.target) {
        Ok(bins) => bins,
        Err(e) => {
            log::error!("{}", e);
            process::exit(1);
        }
    };
    
    for binary in binaries {
        log::info!("patching {:?}", binary);
        
        if let Err(e) = inject(&binary) {
            log::error!("failed to patch {:?}: {}", binary, e);
            process::exit(1);
        }
    }
}
