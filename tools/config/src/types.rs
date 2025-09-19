//! Configuration types and structures for the configuration system.
//!
//! This module defines the core types used throughout the configuration system:
//! - Configuration nodes (categories and options)
//! - Configuration keys and their resolution states
//! - Configuration types and values with validation

use std::{fmt::Display, ops::Range};

use enum_dispatch::enum_dispatch;
use toml_edit::{Item, Value};

use crate::{category::ConfigCategory, option::ConfigOption, state::ConfigState};

// ================================================================================================
// Configuration Node Types
// ================================================================================================

/// Represents a node in the configuration tree.
///
/// Each node can be either a category (containing other nodes) or an option (leaf node with a value).
#[derive(Debug, Clone)]
#[enum_dispatch(ConfigNodelike)]
pub enum ConfigNode {
    Category(ConfigCategory),
    Option(ConfigOption),
}

impl ConfigNode {
    /// Check if this node should be visible based on the current configuration state.
    pub fn visible(&self, state: &ConfigState) -> bool {
        state.visible(&self.id())
    }
}

/// Common interface for all configuration nodes.
#[enum_dispatch]
pub trait ConfigNodelike {
    fn key(&self) -> &str;
    fn build_full_key(&self) -> Option<String>;
    fn id(&self) -> usize;
    fn parent(&self) -> Option<&ConfigKey>;
    fn set_parent(&mut self, parent: ConfigKey);
    fn iter_children(&self) -> Box<dyn Iterator<Item = &ConfigNode> + '_>;
    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ConfigNode> + '_>;
    fn dependencies_iter(&self)
    -> Box<dyn Iterator<Item = &(ConfigKey, Option<ConfigValue>)> + '_>;
    fn dependencies_iter_mut(
        &mut self,
    ) -> Box<dyn Iterator<Item = &mut (ConfigKey, Option<ConfigValue>)> + '_>;
    fn add_dependency(&mut self, key: ConfigKey, value: Option<ConfigValue>);
    fn dependencies_drain(
        &mut self,
    ) -> Box<dyn Iterator<Item = (ConfigKey, Option<ConfigValue>)> + '_>;
}

// ================================================================================================
// Configuration Keys
// ================================================================================================

/// Represents a key that links to a configuration node in the tree.
///
/// Keys go through different resolution phases:
/// - Simple: Basic key name (e.g., "foo")
/// - Qualified: Fully qualified path (e.g., ".category.subcategory.option")
/// - Pending: Placed in tree but path not yet resolved
/// - Resolved: Full path known with optional ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConfigKey {
    /// Simple key name (e.g., "foo").
    /// May not be unique across the entire tree.
    Simple(String),

    /// Fully qualified key with dot-separated path (e.g., ".path.to.category").
    /// Used when simple keys are ambiguous.
    Qualified(String),

    /// Node placed in tree but absolute path not yet determined.
    Pending(),

    /// Fully resolved key with complete path and optional node ID.
    Resolved { path: String, id: Option<usize> },
}

// ================================================================================================
// Configuration Types and Values
// ================================================================================================

/// Defines the type and constraints for a configuration option.
#[derive(Debug, Clone)]
pub enum ConfigType {
    /// Boolean value with default
    Boolean(bool),

    /// String value with optional allowed values list and default
    String(Option<Vec<String>>, String),

    /// Integer value with valid range and default
    Integer(Range<i64>, i64),

    /// Float value with valid range and default
    Float(Range<f64>, f64),
}

impl Display for ConfigType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigType::Boolean(default) => {
                write!(f, "Boolean (default: {})", default)
            }
            ConfigType::String(allowed, default) => {
                if let Some(allowed) = allowed {
                    write!(
                        f,
                        "String (allowed: {:?}, default: \"{}\")",
                        allowed, default
                    )
                } else {
                    write!(f, "String (default: \"{}\")", default)
                }
            }
            ConfigType::Integer(range, default) => {
                write!(
                    f,
                    "Integer (range: {}..{}, default: {})",
                    range.start, range.end, default
                )
            }
            ConfigType::Float(range, default) => {
                write!(
                    f,
                    "Float (range: {:.2}..{:.2}, default: {:.2})",
                    range.start, range.end, default
                )
            }
        }
    }
}

/// Represents an actual configuration value.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    Boolean(bool),
    String(String),
    Integer(i64),
    Float(f64),
    Invalid,
}

impl Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigValue::Boolean(value) => write!(f, "{}", value),
            ConfigValue::String(value) => write!(f, "{}", value),
            ConfigValue::Integer(value) => write!(f, "{}", value),
            ConfigValue::Float(value) => write!(f, "{}", value),
            ConfigValue::Invalid => write!(f, "Invalid"),
        }
    }
}

// ================================================================================================
// Type Conversions
// ================================================================================================

impl From<&Item> for ConfigValue {
    fn from(item: &Item) -> Self {
        match item {
            Item::Value(Value::Boolean(b)) => ConfigValue::Boolean(*b.value()),
            Item::Value(Value::String(s)) => ConfigValue::String(s.value().to_string()),
            Item::Value(Value::Integer(i)) => ConfigValue::Integer(*i.value()),
            Item::Value(Value::Float(f)) => ConfigValue::Float(*f.value()),
            _ => ConfigValue::Invalid,
        }
    }
}

impl From<Value> for ConfigValue {
    fn from(value: Value) -> Self {
        match value {
            Value::Boolean(b) => ConfigValue::Boolean(b.into_value()),
            Value::String(s) => ConfigValue::String(s.into_value()),
            Value::Integer(i) => ConfigValue::Integer(i.into_value()),
            Value::Float(f) => ConfigValue::Float(f.into_value()),
            _ => ConfigValue::Invalid,
        }
    }
}

impl Into<Value> for ConfigValue {
    fn into(self) -> Value {
        match self {
            ConfigValue::Boolean(value) => Value::from(value),
            ConfigValue::String(value) => Value::from(value),
            ConfigValue::Integer(value) => Value::from(value),
            ConfigValue::Float(value) => Value::from(value),
            ConfigValue::Invalid => Value::from("Invalid".to_string()),
        }
    }
}

impl Into<String> for ConfigValue {
    fn into(self) -> String {
        match self {
            ConfigValue::Boolean(value) => value.to_string(),
            ConfigValue::String(value) => value,
            ConfigValue::Integer(value) => value.to_string(),
            ConfigValue::Float(value) => value.to_string(),
            ConfigValue::Invalid => "Invalid".to_string(),
        }
    }
}

impl From<ConfigType> for ConfigValue {
    fn from(config_type: ConfigType) -> Self {
        match config_type {
            ConfigType::Boolean(default) => ConfigValue::Boolean(default),
            ConfigType::String(_, default) => ConfigValue::String(default),
            ConfigType::Integer(_, default) => ConfigValue::Integer(default),
            ConfigType::Float(_, default) => ConfigValue::Float(default),
        }
    }
}

impl From<&str> for ConfigValue {
    fn from(s: &str) -> Self {
        // TODO type conversions
        ConfigValue::String(s.to_string())
    }
}
