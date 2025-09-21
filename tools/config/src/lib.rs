use std::{path::Path, process::exit};

use crate::{
    category::ConfigCategory,
    error::{Diagnostic, Error, Report},
    file::File,
    state::ConfigState,
    types::ConfigNode,
};

pub mod category;
pub mod error;
mod file;
pub mod logging;
pub mod option;
pub mod parse;
pub mod resolve;
pub mod state;
mod toml_patch;
pub mod types;
pub mod ui;

use annotate_snippets::{self as asn, Level};
use anyhow::anyhow;
use toml_edit::{DocumentMut, ImDocument, Key};

pub fn load_config(root: &Path, filename: &str) -> ConfigNode {
    let files = file::load_files(root, filename);

    let mut id = 0;

    let mut root = ConfigCategory {
        parent: None,
        key: ".".to_string(),
        id,
        name: "Root".to_string(),
        description: None,
        depends_on: Vec::new(),
        children: Vec::new(),
    };

    id += 1;

    let mut errored = false;

    for file in files {
        match file {
            Ok(File { path, content }) => {
                let path = path.to_string_lossy();
                let diag = Diagnostic::new(&path, Some(&content));
                match parse::parse_content(&content, &mut id, &diag) {
                    Ok(nodes) => {
                        resolve::link_nodes(&mut root, nodes);
                    }
                    Err(Error::InvalidToml(rep)) => {
                        let msg = diag.msg(&rep);
                        log::error!("{}", asn::Renderer::styled().render(msg));
                        errored = true;
                    }
                    Err(e) => {
                        log::error!("{e}");
                        errored = true;
                    }
                }
            }
            Err(e) => {
                log::error!("{e}");
                errored = true;
            }
        }
    }

    if let Err(e) = resolve::resolve_paths(&mut root) {
        log::error!("{e}");
        errored = true;
    }

    if errored {
        exit(1);
    }

    ConfigNode::Category(root)
}

pub fn load_state<'node>(root: &'node ConfigNode, config: Option<&Path>) -> ConfigState<'node> {
    match config {
        Some(config) => match file::load_file(config) {
            Ok(File { path, content }) => {
                let path = path.to_string_lossy();
                let diag = Diagnostic::new(&path, Some(&content));

                match content.parse::<ImDocument<String>>().map_err(Report::from) {
                    Ok(doc) => {
                        error::fail_on_error(ConfigState::deserialize_from(&doc, root), Some(&diag))
                    }
                    Err(rep) => {
                        let msg = diag.msg(&rep);
                        log::error!("{}", asn::Renderer::styled().render(msg));
                        exit(1);
                    }
                }
            }
            Err(e) => {
                log::error!("{e}");
                exit(1);
            }
        },
        None => error::fail_on_error(ConfigState::new(root), None),
    }
}

pub fn load_toml_mut(toml: &Path) -> Result<DocumentMut, Error> {
    let File { path, content } = file::load_file(&toml)?;

    let path = path.to_string_lossy();
    let diag = Diagnostic::new(&path, Some(&content));

    let doc = content
        .parse::<DocumentMut>()
        .map_err(Report::from)
        .map_err(|rep| {
            let msg = diag.msg(&rep);
            log::error!("{}", asn::Renderer::styled().render(msg));
            Error::InvalidToml(rep)
        })?;

    Ok(doc)
}

pub fn load_toml(toml: &Path) -> Result<ImDocument<String>, Error> {
    let File { path, content } = file::load_file(&toml)?;

    let path = path.to_string_lossy();
    let diag = Diagnostic::new(&path, Some(&content));

    let doc = content
        .parse::<ImDocument<String>>()
        .map_err(Report::from)
        .map_err(|rep| {
            let msg = diag.msg(&rep);
            log::error!("{}", asn::Renderer::styled().render(msg));
            Error::InvalidToml(rep)
        })?;

    Ok(doc)
}

pub fn apply_preset(config: &mut DocumentMut, preset: &ImDocument<String>) -> Result<(), Error> {
    // Iterate over the env section of the preset and apply each key-value pair to the config
    if let Some(preset_env) = preset.get("env") {
        let config_env = config.entry("env").or_insert(toml_edit::table());

        if let toml_edit::Item::Table(preset_table) = preset_env {
            if let toml_edit::Item::Table(config_table) = config_env {
                // Remove all existing keys starting with OSIRIS_ in the config env section
                config_table.retain(|key, _| !key.starts_with("OSIRIS_"));

                // Insert all keys from the preset env section
                for (key, value) in preset_table.iter() {
                    config_table[key] = value.clone();
                }

                Ok(())
            } else {
                return Err(Report::from_spanned(
                    Level::Error,
                    None::<&Key>,
                    config_env,
                    "expected 'env' to be a table.",
                )
                .into());
            }
        } else {
            return Err(Report::from_spanned(
                Level::Error,
                None::<&Key>,
                preset_env,
                "expected 'env' to be a table.",
            )
            .into());
        }
    } else {
        return Err(anyhow!("preset does not contain an 'env' section.").into());
    }
}
