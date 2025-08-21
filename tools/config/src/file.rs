use std::path::{Path, PathBuf};

use anyhow::Context;
use walkdir::WalkDir;

use crate::error::Result;

pub struct File {
    pub path: PathBuf,
    pub content: String,
}

pub fn load_file(path: &Path) -> Result<File> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read file {}", path.display()))?;

    Ok(File {
        path: path.to_path_buf(),
        content,
    })
}

pub fn load_files(root: &Path) -> Vec<Result<File>> {
    let mut files = Vec::new();

    let mut entries = WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .collect::<Vec<_>>();

    entries.sort_by_key(|entry| entry.depth());

    for entry in entries {
        let path = entry.path();

        if path.is_dir() {
            continue;
        }

        if path.extension().map(|e| e == "toml").unwrap_or(false)
            && path.file_name().is_some_and(|f| f == "options.toml")
        {
            let file = load_file(&path);
            files.push(file);
        }
    }

    files
}
