#![cfg_attr(target_os = "none", no_std)]
#![cfg(not(target_os = "none"))]

use std::{path::Path, process::exit};

use crate::{
    category::ConfigCategory,
    error::{Diagnostic, Error, Report},
    file::File,
    macros::MacroEngine,
    state::ConfigState,
    types::ConfigNode,
};

pub mod category;
pub mod error;
mod file;
pub mod macros;
pub mod option;
pub mod parse;
pub mod resolve;
pub mod state;
mod toml_patch;
pub mod types;
pub mod ui;

use toml_edit::{DocumentMut, ImDocument};

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
        attributes: Vec::new(),
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
                        log::error!("{}", error::msg_to_string(msg));
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

pub fn load_state<'node>(
    root: &'node ConfigNode,
    config: Option<&Path>,
    ignored: &Vec<String>,
) -> ConfigState<'node> {
    match config {
        Some(config) => match file::load_file(config) {
            Ok(File { path, content }) => {
                let path = path.to_string_lossy();
                let diag = Diagnostic::new(&path, Some(&content));

                match content.parse::<ImDocument<String>>().map_err(Report::from) {
                    Ok(doc) => error::fail_on_error(
                        ConfigState::deserialize_from(&doc, root, ignored),
                        Some(&diag),
                    ),
                    Err(rep) => {
                        let msg = diag.msg(&rep);
                        log::error!("{}", error::msg_to_string(msg));
                        exit(1);
                    }
                }
            }
            Err(e) => {
                log::error!("{e}");
                exit(1);
            }
        },
        None => error::fail_on_error(ConfigState::new(root, &MacroEngine::new()), None),
    }
}

pub fn load_toml_mut(toml: &Path) -> Result<DocumentMut, Error> {
    file::create_if_not_exists(&toml)?;
    let File { path, content } = file::load_file(&toml)?;

    let path = path.to_string_lossy();
    let diag = Diagnostic::new(&path, Some(&content));

    let doc = content
        .parse::<DocumentMut>()
        .map_err(Report::from)
        .map_err(|rep| {
            let msg = diag.msg(&rep);
            log::error!("{}", error::msg_to_string(msg));
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
            log::error!("{}", error::msg_to_string(msg));
            Error::InvalidToml(rep)
        })?;

    Ok(doc)
}

pub fn apply_preset(config: &mut DocumentMut, preset: &ImDocument<String>) -> Result<(), Error> {
    config.clear();

    for (key, value) in preset.iter() {
        config.insert(key, value.clone());
    }

    Ok(())
}
