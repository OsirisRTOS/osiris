use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::types::{ConfigCategory, ConfigKey};
use crate::{error::Result, types::ConfigNode};
use crate::parse;

pub fn load_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
    {
        let path = entry.path();

        if path.is_dir() {
            continue;
        }

        if path.extension().map(|e| e == "toml").unwrap_or(false)
            && path.file_name().is_some_and(|f| f == "options.toml")
        {
            files.push(path.to_path_buf());
        }
    }

    Ok(files)
}

fn should_link(parent: &Option<ConfigKey>, key: &str, path: &str) -> bool {
    match parent {
        Some(ConfigKey::Simple(p_key)) => p_key == key,
        Some(ConfigKey::Qualified(p_key)) => p_key == path,
        None => path == ".",
        _ => false,
    }
}

fn build_current_path(current_node: &ConfigCategory, path: &str) -> String {
    if current_node.key == "." {
        ".".to_string()
    } else {
        format!("{}.{}", path, current_node.key)
    }
}

// This works as follows:
// We get a set of nodes, which are either categories or options. Each of this nodes is an already fully resolved subtree.
// But we still need to link these nodes in our tree. Luckily we already process our files in a BFS fashion. Which means each new node just needs to be append to an already existing parent node.
// The nodes have a parent key, which is either a ConfigKey::Simple or ConfigKey::Qualified. Simple means we just search the nearest parent node in the tree with the same "name"/key. If this is not unique, we throw an error and the user has to use a fully qualified key.
// The fully qualified key is the path to the parent node, seperated by dots. If the parent is None this means its parent is the "root" node.
fn link_nodes(root: &mut ConfigCategory, new_nodes: &Vec<ConfigNode>) {
    for node in new_nodes {
        // I dont care about performance in a config tool.
        let mut queue: VecDeque<(&mut ConfigCategory, String)> = VecDeque::new();
        queue.push_back((root, "".to_string()));

        while let Some((current_node, path)) = queue.pop_front() {
            // Append the current path to the current node's key
            let current_path = build_current_path(current_node, &path);

            let should_link = match node {
                ConfigNode::Category(cat) => should_link(&cat.parent, &cat.key, &current_path),
                ConfigNode::Option(opt) => should_link(&opt.parent, &opt.key, &current_path),
            };

            if should_link {
                current_node.children.push(node.clone());
                break; // We found the right place to link the node, so we can stop searching
            }

            // If we reach here, we didn't find the parent node, so we need to continue searching
            for child in &mut current_node.children {
                match child {
                    ConfigNode::Category(cat) => {
                        if current_path == "." {
                            queue.push_back((cat, "".to_string()));
                        } else {
                            queue.push_back((cat, current_path.clone()));
                        }
                    }
                    _ => continue,
                }
            }
        }
    }
}

fn resolve_parents(root: &mut ConfigCategory) {
    let mut queue = VecDeque::new();

    for child in &mut root.children {
        queue.push_back((child, "".to_string()));
    }

    while let Some((node, path)) = queue.pop_front() {
        let current_path = format!("{}.{}", path, node.key());

        // Resolve the parent for the current node
        node.set_parent(ConfigKey::Resolved(current_path.clone()));

        for child in &mut node.iter_children_mut() {
            queue.push_back((child, current_path.clone()));
        }
    }
}


pub fn resolve_config(root: &Path) -> Result<ConfigNode> {
    let files = load_files(root)?;
   
    let mut root = ConfigCategory {
        parent: None,
        key: ".".to_string(),
        name: "Root".to_string(),
        description: None,
        depends_on: HashMap::new(),
        children: Vec::new(),
    };

    for file in files {
        let nodes = parse::parse_file(&file)?;
        link_nodes(&mut root, &nodes);
    }

    resolve_parents(&mut root);

    Ok(ConfigNode::Category(root))
}