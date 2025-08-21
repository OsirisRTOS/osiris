use std::collections::{HashMap, VecDeque};

use anyhow::anyhow;

use crate::category::ConfigCategory;
use crate::error::Result;
use crate::types::ConfigNode;
use crate::types::{ConfigKey, ConfigNodelike};

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
pub fn link_nodes(root: &mut ConfigCategory, new_nodes: Vec<ConfigNode>) {
    for node in new_nodes.into_iter() {
        // I dont care about performance in a config tool.
        let mut queue: VecDeque<(&mut ConfigCategory, String)> = VecDeque::new();
        queue.push_back((root, "".to_string()));

        while let Some((current_node, path)) = queue.pop_front() {
            // Append the current path to the current node's key
            let current_path = build_current_path(current_node, &path);

            let should_link = match node {
                ConfigNode::Category(ref cat) => should_link(&cat.parent, &cat.key, &current_path),
                ConfigNode::Option(ref opt) => should_link(&opt.parent, &opt.key, &current_path),
            };

            if should_link {
                current_node.children.push(node);
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

pub fn resolve_paths(root: &mut ConfigCategory) -> Result<()> {
    let mut queue = VecDeque::new();
    let mut id_map = HashMap::new();

    for child in &mut root.children {
        queue.push_back((child, "".to_string()));
    }

    while let Some((node, path)) = queue.pop_front() {
        let current_path = format!("{}.{}", path, node.key());

        // Resolve the parent for the current node
        node.set_parent(ConfigKey::Resolved {
            path: path.clone(),
            id: None,
        });

        id_map.insert(current_path.clone(), node.id());

        for child in &mut node.iter_children_mut() {
            queue.push_back((child, current_path.clone()));
        }
    }

    let mut queue = VecDeque::new();

    for child in &mut root.children {
        queue.push_back((child, "".to_string()));
    }

    while let Some((node, path)) = queue.pop_front() {
        let current_path = format!("{}.{}", path, node.key());
        let id = node.id();

        // Inefficient but it works for now.
        let drained_deps: Vec<_> = node.dependencies_drain().collect();
        for (mut dep, value) in drained_deps {
            match dep {
                ConfigKey::Resolved {
                    ref path,
                    ref mut id,
                } => *id = id_map.get(path).copied(),
                ConfigKey::Simple(ref key) => {
                    if let Some(id) = id_map.get(&format!("{}.{}", path, key)) {
                        dep = ConfigKey::Resolved {
                            path: format!("{}.{}", path, key),
                            id: Some(*id),
                        };
                    } else {
                        Err(anyhow!("Unresolved dependency: {key} in {path}"))?
                    }
                }
                ConfigKey::Qualified(ref key) => {
                    if let Some(id) = id_map.get(key) {
                        dep = ConfigKey::Resolved {
                            path: key.clone(),
                            id: Some(*id),
                        };
                    } else {
                        Err(anyhow!("Unresolved dependency: {key} in {path}"))?
                    }
                }
                _ => Err(anyhow!("Unresolved dependency: {dep:?} in {path}"))?,
            };

            node.add_dependency(dep, value);
        }

        for child in &mut node.iter_children_mut() {
            // Add parent node as dependency of the child
            child.add_dependency(
                ConfigKey::Resolved {
                    path: current_path.clone(),
                    id: Some(id),
                },
                None,
            );
            queue.push_back((child, current_path.clone()));
        }
    }

    Ok(())
}
