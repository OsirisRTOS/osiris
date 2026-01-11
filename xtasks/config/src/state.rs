//! Configuration state management for the configuration system.
//!
//! This module handles the runtime state of configuration nodes, including:
//! - Node enable/disable states based on dependencies
//! - Configuration values and their validation
//! - Topological ordering for dependency resolution

use std::{cell::RefCell, collections::HashMap};

use anyhow::anyhow;
use toml_edit::{DocumentMut, ImDocument, Item, Key, Value};

use crate::{
    error::{Report, Result},
    macros::MacroEngine,
    option::ConfigOption,
    types::{Attribute, ConfigKey, ConfigNode, ConfigNodelike, ConfigType, ConfigValue},
};

use annotate_snippets as asn;

// ================================================================================================
// Configuration State
// ================================================================================================

/// Manages the runtime state of all configuration nodes.
///
/// Tracks which nodes are enabled, their current values, and handles dependency resolution
/// to ensure the configuration remains in a consistent state.
pub struct ConfigState<'a> {
    /// Whether each node is enabled or not.
    enabled: RefCell<HashMap<usize, bool>>,
    /// Topological order of nodes for dependency processing.
    ordered_rev: Vec<&'a ConfigNode>,
    /// Current values of each configuration option.
    values: HashMap<usize, ConfigValue>,
    /// Reference to original nodes for lookups.
    nodes: HashMap<usize, &'a ConfigNode>,
}

impl<'a> ConfigState<'a> {
    /// Creates a new configuration state from a root configuration node.
    ///
    /// This performs initial setup including:
    /// - Building node mappings
    /// - Computing topological order for dependencies
    /// - Setting initial values and enabled states
    pub fn new(root: &'a ConfigNode, engine: &MacroEngine) -> Result<Self> {
        let nodes = Self::compute_node_mapping(root);

        // Reverse the topological order.
        let mut ordered_rev = Self::compute_topological_order(&nodes)?;
        ordered_rev.reverse();

        let enabled = Self::compute_initial_enabled_state(root);
        let values = Self::compute_initial_values(&nodes, engine)?;

        let state = Self {
            enabled: RefCell::new(enabled),
            ordered_rev,
            values,
            nodes,
        };

        state.update_dependency_states()?;
        Ok(state)
    }

    // ================================================================================================
    // Public Interface
    // ================================================================================================

    /// Check if a node is visible (enabled) in the current state.
    pub fn visible(&self, id: &usize) -> bool {
        self.enabled.borrow().get(id).copied().unwrap_or(false)
    }

    /// Get the current value of a configuration option.
    pub fn value(&self, id: &usize) -> Option<&ConfigValue> {
        self.values.get(id)
    }

    /// Get the configuration node by ID.
    pub fn node(&self, id: &usize) -> Option<&ConfigNode> {
        self.nodes.get(id).copied()
    }

    /// Enable or disable a specific node.
    pub fn enable(&self, id: usize, enabled: bool) -> Result<()> {
        self.enabled.borrow_mut().insert(id, enabled);
        self.update_dependency_states()
    }

    /// Check if a node is currently enabled.
    pub fn enabled(&self, id: &usize) -> bool {
        self.enabled.borrow().get(id).copied().unwrap_or(false)
    }

    /// Update the value of a configuration option.
    pub fn update_value(&mut self, id: &usize, value: ConfigValue) -> Result<()> {
        self.values.insert(*id, value);
        self.update_dependency_states()
    }

    /// Get a copy of the reverse topologically ordered nodes.
    pub fn ordered_rev(&self) -> Vec<&ConfigNode> {
        self.ordered_rev.clone()
    }

    fn node_to_name(node: &ConfigNode) -> Option<String> {
        // Note: qualified paths start with a leading dot, we remove it.
        node.build_full_key()
            .map(|key| key.replace('.', "_").to_uppercase()[1..].to_string())
    }

    fn name_to_key(name: &str) -> Option<String> {
        name.to_lowercase()
            .replace('_', ".")
            .strip_prefix("osiris")
            .map(|s| s.to_string())
    }

    /// Serialize the current configuration state to a string format.
    pub fn serialize_into(&self, doc: &mut DocumentMut) -> Result<()> {
        let table = doc.entry("env").or_insert(toml_edit::table());

        if let Item::Table(table) = table {
            table.clear(); // Clear existing entries

            // Table exists, we can modify it
            for (id, value) in &self.values {
                if !self.enabled(id) {
                    continue;
                }

                let node = self
                    .nodes
                    .get(id)
                    .ok_or_else(|| anyhow!("Node with ID {} not found", id))?;

                if node.has_attribute(&Attribute::NoStore) {
                    continue;
                }

                let name = Self::node_to_name(node).ok_or_else(|| {
                    anyhow!("Failed to convert node to name for node with ID {}", id)
                })?;

                // We always convert to strings for environment variables.
                table.insert(
                    &format!("OSIRIS_{name}"),
                    Item::Value(Value::from(value.to_string())),
                );
            }
        }
        Ok(())
    }

    /// Computes initial values for all configuration options based on their types.
    fn compute_initial_values(
        nodes: &HashMap<usize, &ConfigNode>,
        engine: &MacroEngine,
    ) -> Result<HashMap<usize, ConfigValue>> {
        let mut initial_values = HashMap::new();

        for node in nodes.values() {
            if let ConfigNode::Option(ConfigOption { typ, .. }) = node {
                if let ConfigType::String(allowed, default) = typ {
                    match engine.exec(default) {
                        Ok(resolved) => {
                            if !allowed
                                .as_ref()
                                .map(|vals| vals.contains(&resolved))
                                .unwrap_or(true)
                            {
                                return Err(anyhow!(
                                "default value '{}' for option '{}' is not in allowed values: {:?}",
                                resolved,
                                node.build_full_key().unwrap_or_default(),
                                allowed
                            ).into());
                            }

                            initial_values.insert(node.id(), ConfigValue::String(resolved));
                            continue;
                        },
                        Err(err) => {
                            return Err(anyhow!(
                                "failed to expand macros in default value '{}' for option '{}': {}",
                                default,
                                node.build_full_key().unwrap_or_default(),
                                err
                            ).into());
                        }
                    }
                }

                initial_values.insert(node.id(), typ.clone().into());
            }
        }

        Ok(initial_values)
    }

    pub fn deserialize_from(
        doc: &ImDocument<String>,
        root: &'a ConfigNode,
        ignore: &Vec<String>,
    ) -> Result<Self> {
        let mut engine = MacroEngine::new();

        if let Some(triple) = doc.get("build").and_then(|b| b.get("target")) {
            if let Item::Value(Value::String(value)) = triple {
                engine = engine.with_target_triple(value.value().to_string())?;
            }
        } else {
            return Err(anyhow!("missing 'build.target'. Please specify the target triple in the configuration under 'build.target'.").into());
        }

        let mut state = Self::new(root, &engine)?;

        if let Some(table) = doc.get("env") {
            if let Item::Table(table) = table {
                'outer: for (key, item) in table.iter() {
                    let name = table.key(key).unwrap(); // Safe unwrap we iterate through the keys.

                    // Only process OSIRIS_ prefixed keys.
                    let key = match Self::name_to_key(name) {
                        Some(key) => key,
                        None => continue,
                    };

                    for ignored in ignore {
                        if key == *ignored {
                            log::warn!("ignoring config key: {}", key);
                            continue 'outer;
                        }
                    }

                    let node = state
                        .nodes
                        .iter()
                        .find(|(_, node)| match node {
                            ConfigNode::Option(opt) => opt.build_full_key() == Some(key.clone()),
                            ConfigNode::Category(_) => false,
                        })
                        .filter(|(_, node)| !node.has_attribute(&Attribute::NoStore))
                        .map(|(_, node)| node);

                    if let Some(ConfigNode::Option(opt)) = node {
                        let value: ConfigValue = match (item, &opt.typ) {
                            (Item::Value(Value::Boolean(value)), ConfigType::Boolean(_)) => {
                                Value::from(*value.value()).try_into()?
                            }
                            (Item::Value(Value::String(value)), ConfigType::String(_, _)) => {
                                Value::from(value.value()).try_into()?
                            }
                            (Item::Value(Value::Integer(value)), ConfigType::Integer(_, _)) => {
                                Value::from(*value.value()).try_into()?
                            }
                            (Item::Value(Value::Float(value)), ConfigType::Float(_, _)) => {
                                Value::from(*value.value()).try_into()?
                            }
                            (Item::Value(Value::String(value)), typ) => {
                                // If we expect a non-string type, try to parse the string.
                                match match typ {
                                    ConfigType::Boolean(_) => value
                                        .value()
                                        .parse::<bool>()
                                        .map(|parsed| Value::from(parsed).try_into())
                                        .map_err(|e| e.into()),
                                    ConfigType::Integer(_, _) => value
                                        .value()
                                        .parse::<i64>()
                                        .map(|parsed| Value::from(parsed).try_into())
                                        .map_err(|e| e.into()),
                                    ConfigType::Float(_, _) => value
                                        .value()
                                        .parse::<f64>()
                                        .map(|parsed| Value::from(parsed).try_into())
                                        .map_err(|e| e.into()),
                                    ConfigType::String(_, _) => {
                                        unreachable!("String type should have been handled earlier")
                                    }
                                }
                                .flatten()
                                {
                                    Ok(parsed) => parsed,
                                    Err(err) => {
                                        return Err(Report::from_spanned(
                                            asn::Level::Error,
                                            Some(name),
                                            item,
                                            err.to_string(),
                                        )
                                        .into());
                                    }
                                }
                            }
                            _ => {
                                return Err(Report::from_spanned(
                                    asn::Level::Error,
                                    Some(name),
                                    item,
                                    format!(
                                        "invalid item type, expected: {}, found: {}",
                                        opt.typ,
                                        item.type_name()
                                    ),
                                )
                                .into());
                            }
                        };

                        state.values.insert(opt.id(), value.into());
                    } else {
                        return Err(Report::from_spanned(
                            asn::Level::Error,
                            None::<&Key>,
                            name,
                            format!("couldn't find option for key: {}", key),
                        )
                        .into());
                    }
                }
            } else {
                return Err(Report::from_spanned(
                    asn::Level::Error,
                    None::<&Key>,
                    table,
                    format!(
                        "invalid type for env, expected table, found: {}",
                        table.type_name()
                    ),
                )
                .into());
            }
        }

        state.update_dependency_states()?;
        Ok(state)
    }

    // ================================================================================================
    // Private Implementation
    // ================================================================================================

    /// Updates node enabled states based on dependency satisfaction.
    ///
    /// Processes nodes in topological order to ensure dependencies
    /// are evaluated before their dependents.
    fn update_dependency_states(&self) -> Result<()> {
        let mut processing_order = self.ordered_rev();

        // Process nodes in topological order
        while let Some(node) = processing_order.pop() {
            let dependencies_satisfied = node.dependencies_iter().all(|(dep, required_value)| {
                match dep {
                    ConfigKey::Resolved { id: Some(id), .. } => {
                        (self.enabled(id) && *required_value == None)
                            || (self.enabled(id) && self.value(id) == required_value.as_ref())
                    }
                    _ => false, // Unresolved dependencies are considered unsatisfied
                }
            });

            // Update the node's enabled state based on dependency satisfaction
            self.enabled
                .borrow_mut()
                .insert(node.id(), dependencies_satisfied);
        }

        Ok(())
    }

    /// Creates a mapping from node IDs to their corresponding nodes.
    fn compute_node_mapping(root: &'a ConfigNode) -> HashMap<usize, &'a ConfigNode> {
        let mut mapping = HashMap::new();
        let mut stack = vec![root];

        while let Some(node) = stack.pop() {
            mapping.insert(node.id(), node);

            // Add all children to processing stack
            stack.extend(node.iter_children());
        }

        mapping
    }

    /// Initializes all nodes as enabled by default.
    fn compute_initial_enabled_state(root: &'a ConfigNode) -> HashMap<usize, bool> {
        let mut enabled_states = HashMap::new();
        let mut stack = vec![root];

        while let Some(node) = stack.pop() {
            enabled_states.insert(node.id(), true);

            // Add all children
            stack.extend(node.iter_children());
        }

        enabled_states
    }

    /// Computes topological ordering of nodes using Kahn's algorithm.
    ///
    /// This ensures that dependencies are always processed before their dependents.
    fn compute_topological_order(
        nodes: &HashMap<usize, &'a ConfigNode>,
    ) -> Result<Vec<&'a ConfigNode>> {
        let mut in_degree = HashMap::new();
        let mut dependents: HashMap<usize, Vec<usize>> = HashMap::new();

        for &node_id in nodes.keys() {
            in_degree.insert(node_id, 0);
            dependents.insert(node_id, Vec::new());
        }

        // Build dependency graph and calculate in-degrees
        for (&node_id, node) in nodes.iter() {
            for (dependency_key, _) in node.dependencies_iter() {
                match dependency_key {
                    ConfigKey::Resolved {
                        id: Some(dep_id), ..
                    } => {
                        // Node depends on dep_id, so create edge: dep_id -> node
                        dependents.entry(*dep_id).or_default().push(node_id);
                        *in_degree.entry(node_id).or_insert(0) += 1;
                    }
                    _ => {
                        return Err(
                            anyhow!("Unresolved dependency found: {:?}", dependency_key).into()
                        );
                    }
                }
            }
        }

        // Find root nodes (no dependencies)
        let mut processing_queue: Vec<usize> = in_degree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(&node_id, _)| node_id)
            .collect();

        let mut topological_order = Vec::new();

        // Process nodes in topological order
        while let Some(current_node_id) = processing_queue.pop() {
            let current_node = nodes.get(&current_node_id).ok_or_else(|| {
                anyhow!("Node {} not found during topological sort", current_node_id)
            })?;

            topological_order.push(*current_node);

            // Update dependents of the current node
            if let Some(node_dependents) = dependents.get(&current_node_id) {
                for &dependent_id in node_dependents {
                    if let Some(degree) = in_degree.get_mut(&dependent_id) {
                        *degree -= 1;

                        // If dependent has no more unprocessed dependencies, add to queue
                        if *degree == 0 {
                            processing_queue.push(dependent_id);
                        }
                    }
                }
            }
        }

        // Verify all nodes were processed (no cycles)
        if topological_order.len() != nodes.len() {
            return Err(anyhow!(
                "Circular dependency detected: processed {} nodes, expected {}",
                topological_order.len(),
                nodes.len()
            )
            .into());
        }

        Ok(topological_order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::category::ConfigCategory;

    fn opt(
        key: &str,
        id: usize,
        parent: ConfigKey,
        typ: ConfigType,
        depends_on: Vec<(ConfigKey, Option<ConfigValue>)>,
        attributes: Vec<Attribute>,
    ) -> ConfigNode {
        ConfigNode::Option(ConfigOption {
            parent: Some(parent),
            key: key.to_string(),
            id,
            name: key.to_string(),
            description: None,
            typ,
            attributes,
            depends_on,
        })
    }

    fn cat(
        key: &str,
        id: usize,
        parent: Option<ConfigKey>,
        children: Vec<ConfigNode>,
    ) -> ConfigNode {
        ConfigNode::Category(ConfigCategory {
            parent,
            key: key.to_string(),
            id,
            name: key.to_string(),
            description: None,
            depends_on: Vec::new(),
            attributes: Vec::new(),
            children,
        })
    }

    fn root_with(children: Vec<ConfigNode>) -> ConfigNode {
        cat(".", 0, None, children)
    }

    #[test]
    fn enables_and_disables_based_on_dependency() {
        let dep = ConfigKey::Resolved {
            path: ".opt1".to_string(),
            id: Some(1),
        };

        let opt1 = opt(
            "opt1",
            1,
            ConfigKey::Resolved {
                path: "".to_string(),
                id: Some(0),
            },
            ConfigType::Boolean(true),
            Vec::new(),
            Vec::new(),
        );

        let opt2 = opt(
            "opt2",
            2,
            ConfigKey::Resolved {
                path: "".to_string(),
                id: Some(0),
            },
            ConfigType::Boolean(false),
            vec![(dep, Some(ConfigValue::Boolean(true)))],
            Vec::new(),
        );

        let root = root_with(vec![opt1, opt2]);

        let mut state = ConfigState::new(&root, &MacroEngine::new()).unwrap();

        assert!(state.enabled(&2));

        // Changing dependency value to false should disable the dependent.
        state
            .update_value(&1, ConfigValue::Boolean(false))
            .unwrap();
        assert!(!state.enabled(&2));
    }

    #[test]
    fn detects_circular_dependency() {
        let opt1_dep = ConfigKey::Resolved {
            path: ".opt2".to_string(),
            id: Some(2),
        };
        let opt2_dep = ConfigKey::Resolved {
            path: ".opt1".to_string(),
            id: Some(1),
        };

        let opt1 = opt(
            "opt1",
            1,
            ConfigKey::Resolved {
                path: "".to_string(),
                id: Some(0),
            },
            ConfigType::Boolean(true),
            vec![(opt1_dep, None)],
            Vec::new(),
        );

        let opt2 = opt(
            "opt2",
            2,
            ConfigKey::Resolved {
                path: "".to_string(),
                id: Some(0),
            },
            ConfigType::Boolean(true),
            vec![(opt2_dep, None)],
            Vec::new(),
        );

        let root = root_with(vec![opt1, opt2]);

        let err = ConfigState::new(&root, &MacroEngine::new());
        assert!(err.is_err());
    }

    #[test]
    fn serialize_skips_disabled_and_no_store() {
        let base_parent = ConfigKey::Resolved {
            path: "".to_string(),
            id: Some(0),
        };

        let opt1 = opt(
            "opt1",
            1,
            base_parent.clone(),
            ConfigType::Boolean(true),
            Vec::new(),
            Vec::new(),
        );

        let opt2 = opt(
            "opt2",
            2,
            base_parent.clone(),
            ConfigType::String(None, "bar".to_string()),
            Vec::new(),
            vec![Attribute::NoStore],
        );

        let opt3_dep = ConfigKey::Resolved {
            path: ".opt1".to_string(),
            id: Some(1),
        };

        let opt3 = opt(
            "opt3",
            3,
            base_parent,
            ConfigType::Boolean(true),
            vec![(opt3_dep, Some(ConfigValue::Boolean(true)))],
            Vec::new(),
        );

        let root = root_with(vec![opt1, opt2, opt3]);
        let mut state = ConfigState::new(&root, &MacroEngine::new()).unwrap();

        // Flip opt1 to false so opt3 dependency is unsatisfied and opt3 is disabled.
        state
            .update_value(&1, ConfigValue::Boolean(false))
            .unwrap();

        let mut doc = DocumentMut::new();
        state.serialize_into(&mut doc).unwrap();

        let env = doc.get("env").and_then(|t| t.as_table()).unwrap();

        assert!(env.contains_key("OSIRIS_OPT1"));
        assert!(!env.contains_key("OSIRIS_OPT2")); // NoStore
        assert!(!env.contains_key("OSIRIS_OPT3")); // disabled
    }

    #[test]
    fn deserialize_parses_and_coerces_values() {
        let parent = ConfigKey::Resolved {
            path: "".to_string(),
            id: Some(0),
        };

        let opt_bool = opt(
            "feature",
            1,
            parent.clone(),
            ConfigType::Boolean(false),
            Vec::new(),
            Vec::new(),
        );

        let opt_int = opt(
            "size",
            2,
            parent,
            ConfigType::Integer(0..100, 0),
            Vec::new(),
            Vec::new(),
        );

        let root = root_with(vec![opt_bool, opt_int]);

        let toml = r#"
            [build]
            target = "x86_64-unknown-linux-gnu"

            [env]
            OSIRIS_FEATURE = "true"
            OSIRIS_SIZE = "42"
        "#;

        let doc = ImDocument::parse(toml.to_string()).unwrap();
        let state = ConfigState::deserialize_from(&doc, &root, &Vec::new()).unwrap();

        assert_eq!(state.value(&1), Some(&ConfigValue::Boolean(true)));
        assert_eq!(state.value(&2), Some(&ConfigValue::Integer(42)));
    }

    #[test]
    fn compute_node_mapping_builds_all_nodes() {
        let parent = ConfigKey::Resolved {
            path: "".to_string(),
            id: Some(0),
        };

        let opt1 = opt(
            "opt1",
            1,
            parent.clone(),
            ConfigType::Boolean(true),
            Vec::new(),
            Vec::new(),
        );

        let opt2 = opt(
            "opt2",
            2,
            parent,
            ConfigType::Boolean(false),
            Vec::new(),
            Vec::new(),
        );

        let root = root_with(vec![opt1, opt2]);
        let state = ConfigState::new(&root, &MacroEngine::new()).unwrap();

        assert!(state.node(&0).is_some());
        assert!(state.node(&1).is_some());
        assert!(state.node(&2).is_some());
        assert!(state.node(&999).is_none());
    }

    #[test]
    fn dependency_with_value_requirement() {
        let dep = ConfigKey::Resolved {
            path: ".level".to_string(),
            id: Some(1),
        };

        let opt_level = opt(
            "level",
            1,
            ConfigKey::Resolved {
                path: "".to_string(),
                id: Some(0),
            },
            ConfigType::String(Some(vec!["debug".into(), "info".into(), "error".into()]), "debug".into()),
            Vec::new(),
            Vec::new(),
        );

        let opt_verbose = opt(
            "verbose",
            2,
            ConfigKey::Resolved {
                path: "".to_string(),
                id: Some(0),
            },
            ConfigType::Boolean(false),
            vec![(dep, Some(ConfigValue::String("debug".into())))],
            Vec::new(),
        );

        let root = root_with(vec![opt_level, opt_verbose]);

        let mut state = ConfigState::new(&root, &MacroEngine::new()).unwrap();

        // Initially level="debug", so verbose should be enabled
        assert!(state.enabled(&2));

        // Change level to "info" - dependency unsatisfied
        state
            .update_value(&1, ConfigValue::String("info".into()))
            .unwrap();
        assert!(!state.enabled(&2));

        // Change level back to "debug" - dependency satisfied again
        state
            .update_value(&1, ConfigValue::String("debug".into()))
            .unwrap();
        assert!(state.enabled(&2));
    }

    #[test]
    fn serialize_with_different_value_types() {
        let parent = ConfigKey::Resolved {
            path: "".to_string(),
            id: Some(0),
        };

        let opt_bool = opt(
            "bool_opt",
            1,
            parent.clone(),
            ConfigType::Boolean(true),
            Vec::new(),
            Vec::new(),
        );

        let opt_int = opt(
            "int_opt",
            2,
            parent.clone(),
            ConfigType::Integer(0..100, 50),
            Vec::new(),
            Vec::new(),
        );

        let opt_float = opt(
            "float_opt",
            3,
            parent,
            ConfigType::Float(0.0..10.0, 3.14),
            Vec::new(),
            Vec::new(),
        );

        let root = root_with(vec![opt_bool, opt_int, opt_float]);
        let state = ConfigState::new(&root, &MacroEngine::new()).unwrap();

        let mut doc = DocumentMut::new();
        state.serialize_into(&mut doc).unwrap();

        let env = doc.get("env").and_then(|t| t.as_table()).unwrap();
        assert!(env.contains_key("OSIRIS_BOOL_OPT"));
        assert!(env.contains_key("OSIRIS_INT_OPT"));
        assert!(env.contains_key("OSIRIS_FLOAT_OPT"));

        assert_eq!(
            env.get("OSIRIS_BOOL_OPT")
                .and_then(|item| item.as_value())
                .and_then(|val| val.clone().try_into().ok()),
            Some(ConfigValue::String("true".to_string()))
        );

        assert_eq!(
            env.get("OSIRIS_INT_OPT")
                .and_then(|item| item.as_value())
                .and_then(|val| val.clone().try_into().ok()),
            Some(ConfigValue::String("50".to_string()))
        );

        assert_eq!(
            env.get("OSIRIS_FLOAT_OPT")
                .and_then(|item| item.as_value())
                .and_then(|val| val.clone().try_into().ok()),
            Some(ConfigValue::String("3.14".to_string()))
        );
    }

    #[test]
    fn name_to_key_conversion() {
        assert_eq!(
            ConfigState::<'_>::name_to_key("OSIRIS_FEATURE"),
            Some(".feature".to_string())
        );
        assert_eq!(
            ConfigState::<'_>::name_to_key("OSIRIS_LOG_LEVEL"),
            Some(".log.level".to_string())
        );
        assert_eq!(
            ConfigState::<'_>::name_to_key("OTHER_VAR"),
            None
        );
    }

    #[test]
    fn deserialize_ignores_ignored_keys() {
        let parent = ConfigKey::Resolved {
            path: "".to_string(),
            id: Some(0),
        };

        let opt = opt(
            "feature",
            1,
            parent,
            ConfigType::Boolean(false),
            Vec::new(),
            Vec::new(),
        );

        let root = root_with(vec![opt]);

        let toml = r#"
            [build]
            target = "x86_64-unknown-linux-gnu"

            [env]
            OSIRIS_FEATURE = "true"
        "#;

        let doc = ImDocument::parse(toml.to_string()).unwrap();
        // Ignore the 'feature' key during deserialization
        let state = ConfigState::deserialize_from(&doc, &root, &vec![".feature".to_string()]).unwrap();

        // The value should remain the default (false) since it was ignored
        assert_eq!(state.value(&1), Some(&ConfigValue::Boolean(false)));
    }

    #[test]
    fn enabled_and_visibility_match() {
        let parent = ConfigKey::Resolved {
            path: "".to_string(),
            id: Some(0),
        };

        let opt = opt(
            "test",
            1,
            parent,
            ConfigType::Boolean(true),
            Vec::new(),
            Vec::new(),
        );

        let root = root_with(vec![opt]);
        let state = ConfigState::new(&root, &MacroEngine::new()).unwrap();

        // enabled and visible should match for root children
        assert_eq!(state.enabled(&1), state.visible(&1));
    }
}


