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
    option::ConfigOption,
    types::{ConfigKey, ConfigNode, ConfigNodelike, ConfigType, ConfigValue},
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
    pub fn new(root: &'a ConfigNode) -> Result<Self> {
        let nodes = Self::compute_node_mapping(root);

        // Reverse the topological order.
        let mut ordered_rev = Self::compute_topological_order(&nodes)?;
        ordered_rev.reverse();

        let enabled = Self::compute_initial_enabled_state(root);
        let values = Self::compute_initial_values(&nodes)?;

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
                let key = node
                    .build_full_key()
                    .ok_or_else(|| anyhow!("Failed to build full key for node {}", id))?;

                // Note: qualified paths start with a leading dot, we remove it.
                let key = key.to_uppercase().replace('.', "_")[1..].to_string();
                // We always convert to strings for environment variables.
                table.insert(
                    &format!("OSIRIS_{key}"),
                    Item::Value(Value::from(value.to_string())),
                );
            }
        }
        Ok(())
    }

    pub fn deserialize_from(doc: &ImDocument<String>, root: &'a ConfigNode) -> Result<Self> {
        let mut state = Self::new(root)?;

        if let Some(table) = doc.get("env") {
            if let Item::Table(table) = table {
                for (key, item) in table.iter() {
                    let key = table.key(key).unwrap(); // Safe unwrap we iterate through the keys.

                    // Remove the OSIRIS_ prefix if it exists. Otherwise skip the key.
                    let key_str = if let Some(stripped) = key.to_string().strip_prefix("OSIRIS_") {
                        stripped.to_string()
                    } else {
                        continue;
                    };

                    let key_str = format!(".{}", key_str.to_lowercase().replace('_', "."));

                    let node = state
                        .nodes
                        .iter()
                        .find(|(_, node)| match node {
                            ConfigNode::Option(opt) => {
                                opt.build_full_key() == Some(key_str.clone())
                            }
                            ConfigNode::Category(_) => false,
                        })
                        .map(|(_, node)| node);

                    if let Some(ConfigNode::Option(opt)) = node {
                        let value: ConfigValue = match (item, &opt.typ) {
                            (Item::Value(Value::Boolean(value)), ConfigType::Boolean(_)) => {
                                Value::from(*value.value()).into()
                            }
                            (Item::Value(Value::String(value)), ConfigType::String(_, _)) => {
                                Value::from(value.value()).into()
                            }
                            (Item::Value(Value::Integer(value)), ConfigType::Integer(_, _)) => {
                                Value::from(*value.value()).into()
                            }
                            (Item::Value(Value::Float(value)), ConfigType::Float(_, _)) => {
                                Value::from(*value.value()).into()
                            }
                            (Item::Value(Value::String(value)), typ) => {
                                // If we expect a non-string type, try to parse the string.
                                let res = match typ {
                                    ConfigType::Boolean(_) => {
                                        let parsed = value.value().parse::<bool>()?;
                                        Ok(Value::from(parsed).into())
                                    }
                                    ConfigType::Integer(_, _) => {
                                        let parsed = value.value().parse::<i64>()?;
                                        Ok(Value::from(parsed).into())
                                    }
                                    ConfigType::Float(_, _) => {
                                        let parsed = value.value().parse::<f64>()?;
                                        Ok(Value::from(parsed).into())
                                    }
                                    _ => Err(anyhow!("Invalid type conversion")),
                                };
                                res.map_err(|e| {
                                    Report::from_spanned(
                                        asn::Level::Error,
                                        Some(key),
                                        item,
                                        format!("invalid item type, expected: {}", e),
                                    )
                                })?
                            }
                            _ => {
                                return Err(Report::from_spanned(
                                    asn::Level::Error,
                                    Some(key),
                                    item,
                                    format!("invalid item type, expected: {}", opt.typ),
                                )
                                .into());
                            }
                        };

                        state.values.insert(opt.id(), value.into());
                    } else {
                        return Err(Report::from_spanned(
                            asn::Level::Error,
                            None::<&Key>,
                            key,
                            format!("couldn't find option for key: {}", key_str),
                        )
                        .into());
                    }
                }
            } else {
                return Err(Report::from_spanned(
                    asn::Level::Error,
                    None::<&Key>,
                    table,
                    format!("invalid type for env, expected table."),
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

    /// Computes initial values for all configuration options based on their types.
    fn compute_initial_values(
        nodes: &HashMap<usize, &ConfigNode>,
    ) -> Result<HashMap<usize, ConfigValue>> {
        let mut initial_values = HashMap::new();

        for node in nodes.values() {
            if let ConfigNode::Option(ConfigOption { typ, .. }) = node {
                initial_values.insert(node.id(), typ.clone().into());
            }
        }

        Ok(initial_values)
    }
}
