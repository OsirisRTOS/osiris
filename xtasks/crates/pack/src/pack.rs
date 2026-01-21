use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow, bail};
use cargo_metadata::MetadataCommand;

use crate::{
    bootinfo,
    elf::ElfInfo,
    image::{self},
};

struct BuildConfig {
    bin_name: String,
    target: Option<String>,
    workspace_root: PathBuf,
}

impl BuildConfig {
    fn find_root() -> Option<PathBuf> {
        if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
            if let Ok(metadata) = MetadataCommand::new()
                .manifest_path(format!("{}/Cargo.toml", manifest_dir))
                .no_deps()
                .exec()
            {
                // Workspace root contains the .cargo/config.toml if it exists.
                return Some(metadata.workspace_root.into_std_path_buf());
            }
        } else {
            // Try to find a .cargo/config.toml in current directory.
            return std::env::current_dir().ok();
        }

        None
    }

    fn from_path(path: &Path) -> Result<Self> {
        let path = Path::canonicalize(path)?;

        let mut target = None;
        let mut bin_name = None;

        let root = Self::find_root().unwrap_or(path.clone());
        let config = root.join(".cargo").join("config.toml");

        if config.exists() {
            let config_content = std::fs::read_to_string(&config)?;
            let config_toml: toml::Value = toml::from_str(&config_content)?;

            target = config_toml.get("build").and_then(|build_table| {
                build_table
                    .get("target")
                    .and_then(|target_value| target_value.as_str().map(|s| s.to_string()))
            });
        }

        let manifest = path.join("Cargo.toml");

        if manifest.exists()
            && let Ok(metadata) = MetadataCommand::new()
                .manifest_path(&manifest)
                .no_deps()
                .exec()
        {
            // Find package that matches the path.
            let package = metadata
                .packages
                .iter()
                .find(|p| p.manifest_path == manifest);
            bin_name = package.and_then(|p| {
                p.targets
                    .iter()
                    .find(|t| t.is_bin())
                    .map(|t| t.name.clone())
            });
        }

        if let Some(bin_name) = bin_name {
            log::info!("Detected binary name: {}", bin_name);

            Ok(Self {
                target,
                bin_name,
                workspace_root: root,
            })
        } else {
            bail!("failed to detect root binary name from Cargo.toml");
        }
    }
}

pub fn resolve_binary(binary: &str, target: &Option<String>, profile: &str) -> Result<ElfInfo> {
    let path = Path::new(binary);

    if !path.exists() {
        bail!("binary path {} does not exist.", path.display());
    }

    if path.is_dir() {
        // Try to detect the binary inside the target folder.
        if let Ok(config) = BuildConfig::from_path(path) {
            let target = target.as_ref().or(config.target.as_ref()).ok_or(anyhow!(
                "No target triple specified and none found in config."
            ))?;

            let binary = config
                .workspace_root
                .join("target")
                .join(target)
                .join(profile)
                .join(config.bin_name);
            return ElfInfo::from_path(&binary);
        }
    } else {
        return ElfInfo::from_path(path);
    }

    bail!("failed to resolve binary at {}.", path.display());
}

pub fn pack(init_info: &ElfInfo, kernel_info: &mut ElfInfo, out: &Path) -> Result<()> {
    let mut img = image::Image::new(kernel_info.base_paddr());

    // Add sections.
    img.add_section(kernel_info, image::SectionDescripter::Fixed)?;
    let init_section = img.add_elf(init_info, image::SectionDescripter::Loadable(None))?;

    // Patch bootinfo into kernel.
    let boot_info = bootinfo::BootInfo::new(img.paddr(), &init_section);
    kernel_info.patch_section(".bootinfo", 0, boot_info.inner())?;

    // Update kernel in image.
    img.update(kernel_info, 0)?;

    img.write(out)?;
    Ok(())
}
