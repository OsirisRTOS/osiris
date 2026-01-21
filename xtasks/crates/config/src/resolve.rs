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
                ConfigNode::Category(ref cat) => {
                    should_link(&cat.parent, &current_node.key, &current_path)
                }
                ConfigNode::Option(ref opt) => {
                    should_link(&opt.parent, &current_node.key, &current_path)
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::option::ConfigOption;
    use crate::types::{ConfigType, ConfigValue};

    fn create_root() -> ConfigCategory {
        ConfigCategory {
            parent: None,
            key: ".".to_string(),
            id: 0,
            name: "Root".to_string(),
            description: None,
            depends_on: Vec::new(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    fn create_category(key: &str, id: usize, parent: Option<ConfigKey>) -> ConfigCategory {
        ConfigCategory {
            parent,
            key: key.to_string(),
            id,
            name: key.to_string(),
            description: None,
            depends_on: Vec::new(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }

    fn create_option(key: &str, id: usize, parent: Option<ConfigKey>) -> ConfigOption {
        ConfigOption {
            parent,
            key: key.to_string(),
            id,
            name: key.to_string(),
            description: None,
            typ: ConfigType::Boolean(false),
            attributes: Vec::new(),
            depends_on: Vec::new(),
        }
    }

    #[test]
    fn should_link_simple_key() {
        let parent = Some(ConfigKey::Simple("category".to_string()));
        assert!(should_link(&parent, "category", ".category"));
        assert!(!should_link(&parent, "other", ".category"));
    }

    #[test]
    fn should_link_qualified_key() {
        let parent = Some(ConfigKey::Qualified(".cat.subcat".to_string()));
        assert!(should_link(&parent, "subcat", ".cat.subcat"));
        assert!(!should_link(&parent, "subcat", ".cat.other"));
    }

    #[test]
    fn should_link_none_parent() {
        let parent = None;
        assert!(should_link(&parent, "anything", "."));
        assert!(!should_link(&parent, "anything", ".category"));
    }

    #[test]
    fn build_current_path_root() {
        let root = create_root();
        assert_eq!(build_current_path(&root, ""), ".");
    }

    #[test]
    fn build_current_path_category() {
        let cat = create_category("mycat", 1, None);
        assert_eq!(build_current_path(&cat, ""), ".mycat");
        assert_eq!(build_current_path(&cat, ".parent"), ".parent.mycat");
    }

    #[test]
    fn link_nodes_to_root() {
        let mut root = create_root();
        let cat = create_category("cat1", 1, None);
        let opt = create_option("opt1", 2, None);

        link_nodes(
            &mut root,
            vec![ConfigNode::Category(cat), ConfigNode::Option(opt)],
        );

        assert_eq!(root.children.len(), 2);
        assert_eq!(root.children[0].key(), "cat1");
        assert_eq!(root.children[1].key(), "opt1");
    }

    #[test]
    fn link_nodes_with_simple_key() {
        let mut root = create_root();
        let cat = create_category("parent", 1, None);
        root.children.push(ConfigNode::Category(cat));

        let child_cat = create_category("child", 2, Some(ConfigKey::Simple("parent".to_string())));
        link_nodes(&mut root, vec![ConfigNode::Category(child_cat)]);

        if let ConfigNode::Category(parent) = &root.children[0] {
            assert_eq!(parent.children.len(), 1);
            assert_eq!(parent.children[0].key(), "child");
            if let ConfigNode::Category(child) = &parent.children[0] {
                assert_eq!(child.id, 2);
            } else {
                panic!("Expected child to be a category");
            }
        } else {
            panic!("Expected category");
        }
    }

    #[test]
    fn link_nodes_with_qualified_key() {
        let mut root = create_root();
        let cat = create_category("parent", 1, None);
        root.children.push(ConfigNode::Category(cat));

        let child_opt = create_option("opt", 2, Some(ConfigKey::Qualified(".parent".to_string())));
        link_nodes(&mut root, vec![ConfigNode::Option(child_opt)]);

        if let ConfigNode::Category(parent) = &root.children[0] {
            assert_eq!(parent.children.len(), 1);
            assert_eq!(parent.children[0].key(), "opt");
        } else {
            panic!("Expected category");
        }
    }

    #[test]
    fn link_nodes_nested() {
        let mut root = create_root();

        let mut cat1 = create_category("cat1", 1, None);
        let cat2 = create_category("cat2", 2, None);
        cat1.children.push(ConfigNode::Category(cat2));
        root.children.push(ConfigNode::Category(cat1));

        let nested_opt = create_option(
            "opt",
            3,
            Some(ConfigKey::Qualified(".cat1.cat2".to_string())),
        );
        link_nodes(&mut root, vec![ConfigNode::Option(nested_opt)]);

        if let ConfigNode::Category(cat1) = &root.children[0] {
            if let ConfigNode::Category(cat2) = &cat1.children[0] {
                assert_eq!(cat2.children.len(), 1);
                assert_eq!(cat2.children[0].key(), "opt");
            } else {
                panic!("Expected nested category");
            }
        } else {
            panic!("Expected category");
        }
    }

    #[test]
    fn resolve_paths_simple() {
        let mut root = create_root();
        let cat = create_category("cat1", 1, Some(ConfigKey::Pending()));
        let opt = create_option("opt1", 2, Some(ConfigKey::Pending()));
        root.children.push(ConfigNode::Category(cat));
        root.children.push(ConfigNode::Option(opt));

        resolve_paths(&mut root).unwrap();

        // Check that parents are resolved
        if let ConfigNode::Category(cat) = &root.children[0] {
            match &cat.parent {
                Some(ConfigKey::Resolved { path, id }) => {
                    assert_eq!(path, "");
                    assert_eq!(id, &None);
                }
                _ => panic!("Expected resolved parent"),
            }
        }

        if let ConfigNode::Option(opt) = &root.children[1] {
            match &opt.parent {
                Some(ConfigKey::Resolved { path, id }) => {
                    assert_eq!(path, "");
                    assert_eq!(id, &None);
                }
                _ => panic!("Expected resolved parent"),
            }
        }
    }

    #[test]
    fn resolve_paths_with_children() {
        let mut root = create_root();
        let mut cat = create_category("cat1", 1, Some(ConfigKey::Pending()));
        let opt = create_option("opt1", 2, Some(ConfigKey::Pending()));
        cat.children.push(ConfigNode::Option(opt));
        root.children.push(ConfigNode::Category(cat));

        resolve_paths(&mut root).unwrap();

        if let ConfigNode::Category(cat) = &root.children[0] {
            if let ConfigNode::Option(opt) = &cat.children[0] {
                match &opt.parent {
                    Some(ConfigKey::Resolved { path, .. }) => {
                        assert_eq!(path, ".cat1");
                    }
                    _ => panic!("Expected resolved parent with ID"),
                }

                // Child should have parent as dependency
                assert_eq!(opt.depends_on.len(), 1);

                // Check that the dependency is the parent
                match &opt.depends_on[0].0 {
                    ConfigKey::Resolved { path, id } => {
                        assert_eq!(path, ".cat1");
                        assert_eq!(id, &Some(1));
                    }
                    _ => panic!("Expected resolved parent dependency"),
                }
            } else {
                panic!("Expected option");
            }
        } else {
            panic!("Expected category");
        }
    }

    #[test]
    fn resolve_paths_simple_dependency() {
        let mut root = create_root();

        let opt1 = create_option("opt1", 1, Some(ConfigKey::Pending()));
        let mut opt2 = create_option("opt2", 2, Some(ConfigKey::Pending()));
        opt2.depends_on.push((
            ConfigKey::Simple("opt1".to_string()),
            Some(ConfigValue::Boolean(true)),
        ));

        root.children.push(ConfigNode::Option(opt1));
        root.children.push(ConfigNode::Option(opt2));

        resolve_paths(&mut root).unwrap();

        if let ConfigNode::Option(opt2) = &root.children[1] {
            // Should have the opt1 dependency (no parent dependency for root children)
            assert_eq!(opt2.depends_on.len(), 1);

            // Check the opt1 dependency is resolved
            match &opt2.depends_on[0].0 {
                ConfigKey::Resolved { path, id } => {
                    assert_eq!(path, ".opt1");
                    assert_eq!(id, &Some(1));
                }
                _ => panic!("Expected resolved dependency"),
            }
        } else {
            panic!("Expected option");
        }
    }

    #[test]
    fn resolve_paths_qualified_dependency() {
        let mut root = create_root();

        let mut cat = create_category("cat1", 1, Some(ConfigKey::Pending()));
        let opt1 = create_option("opt1", 2, Some(ConfigKey::Pending()));
        cat.children.push(ConfigNode::Option(opt1));

        let mut opt2 = create_option("opt2", 3, Some(ConfigKey::Pending()));
        opt2.depends_on
            .push((ConfigKey::Qualified(".cat1.opt1".to_string()), None));

        root.children.push(ConfigNode::Category(cat));
        root.children.push(ConfigNode::Option(opt2));

        resolve_paths(&mut root).unwrap();

        if let ConfigNode::Option(opt2) = &root.children[1] {
            // Should have the opt1 dependency (no parent dependency for root children)
            assert_eq!(opt2.depends_on.len(), 1);

            match &opt2.depends_on[0].0 {
                ConfigKey::Resolved { path, id } => {
                    assert_eq!(path, ".cat1.opt1");
                    assert_eq!(id, &Some(2));
                }
                _ => panic!("Expected resolved qualified dependency"),
            }
        } else {
            panic!("Expected option");
        }
    }

    #[test]
    fn resolve_paths_unresolved_simple_dependency() {
        let mut root = create_root();

        let mut opt = create_option("opt1", 1, Some(ConfigKey::Pending()));
        opt.depends_on
            .push((ConfigKey::Simple("nonexistent".to_string()), None));
        root.children.push(ConfigNode::Option(opt));

        let result = resolve_paths(&mut root);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_paths_unresolved_qualified_dependency() {
        let mut root = create_root();

        let mut opt = create_option("opt1", 1, Some(ConfigKey::Pending()));
        opt.depends_on
            .push((ConfigKey::Qualified(".nonexistent.path".to_string()), None));
        root.children.push(ConfigNode::Option(opt));

        let result = resolve_paths(&mut root);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_paths_deeply_nested() {
        let mut root = create_root();

        let mut cat1 = create_category("cat1", 1, Some(ConfigKey::Pending()));
        let mut cat2 = create_category("cat2", 2, Some(ConfigKey::Pending()));
        let opt = create_option("opt1", 3, Some(ConfigKey::Pending()));

        cat2.children.push(ConfigNode::Option(opt));
        cat1.children.push(ConfigNode::Category(cat2));
        root.children.push(ConfigNode::Category(cat1));

        resolve_paths(&mut root).unwrap();

        if let ConfigNode::Category(cat1) = &root.children[0] {
            if let ConfigNode::Category(cat2) = &cat1.children[0] {
                if let ConfigNode::Option(opt) = &cat2.children[0] {
                    match &opt.parent {
                        Some(ConfigKey::Resolved { path, .. }) => {
                            assert_eq!(path, ".cat1.cat2");
                        }
                        _ => panic!("Expected deeply nested resolved parent"),
                    }

                    // Check the parent dependency
                    assert_eq!(opt.depends_on.len(), 1);
                    match &opt.depends_on[0].0 {
                        ConfigKey::Resolved { path, id } => {
                            assert_eq!(path, ".cat1.cat2");
                            assert_eq!(id, &Some(2));
                        }
                        _ => panic!("Expected resolved parent dependency"),
                    }
                } else {
                    panic!("Expected option");
                }
            } else {
                panic!("Expected cat2");
            }
        } else {
            panic!("Expected cat1");
        }
    }

    #[test]
    fn resolve_paths_multiple_dependencies() {
        let mut root = create_root();

        let opt1 = create_option("opt1", 1, Some(ConfigKey::Pending()));
        let opt2 = create_option("opt2", 2, Some(ConfigKey::Pending()));
        let mut opt3 = create_option("opt3", 3, Some(ConfigKey::Pending()));

        opt3.depends_on.push((
            ConfigKey::Simple("opt1".to_string()),
            Some(ConfigValue::Boolean(true)),
        ));
        opt3.depends_on.push((
            ConfigKey::Qualified(".opt2".to_string()),
            Some(ConfigValue::Boolean(false)),
        ));

        root.children.push(ConfigNode::Option(opt1));
        root.children.push(ConfigNode::Option(opt2));
        root.children.push(ConfigNode::Option(opt3));

        resolve_paths(&mut root).unwrap();

        if let ConfigNode::Option(opt3) = &root.children[2] {
            // 2 explicit dependencies (no parent for root children)
            assert_eq!(opt3.depends_on.len(), 2);

            // Verify both explicit dependencies are resolved
            for dep in &opt3.depends_on {
                match &dep.0 {
                    ConfigKey::Resolved { id, .. } => {
                        assert!(id.is_some());
                    }
                    _ => panic!("Expected resolved dependency"),
                }
            }

            // Verify first dependency is opt1
            match &opt3.depends_on[0].0 {
                ConfigKey::Resolved { path, id } => {
                    assert_eq!(path, ".opt1");
                    assert_eq!(id, &Some(1));
                }
                _ => panic!("Expected resolved opt1 dependency"),
            }

            // Verify second dependency is opt2
            match &opt3.depends_on[1].0 {
                ConfigKey::Resolved { path, id } => {
                    assert_eq!(path, ".opt2");
                    assert_eq!(id, &Some(2));
                }
                _ => panic!("Expected resolved opt2 dependency"),
            }
        } else {
            panic!("Expected option");
        }
    }
}
