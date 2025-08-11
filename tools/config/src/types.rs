


// A toml should first define the subcategory that it belongs to, in regards to the whole project. (Because many option files)
// Then it contains a list of options, each has a key, a human-readable name, an optional description, a type an optional value, a boolean to indicate if it is enabled or not and a list of dependencies (other options that must be enabled for this one to be enabled).

use std::{collections::HashMap, fmt::Display, iter::empty, ops::Range};

use toml_edit::{Item, Value};

/// Represents a node in the configuration tree
#[derive(Debug, Clone)]
pub enum ConfigNode {
    Category(ConfigCategory),
    Option(ConfigOption),
}

impl ConfigNode {
    pub fn parent(&self) -> Option<&ConfigKey> {
        match self {
            ConfigNode::Category(cat) => cat.parent.as_ref(),
            ConfigNode::Option(opt) => opt.parent.as_ref(),
        }
    }

    pub fn key(&self) -> &str {
        match self {
            ConfigNode::Category(cat) => &cat.key,
            ConfigNode::Option(opt) => &opt.key,
        }
    }

    pub fn set_parent(&mut self, parent: ConfigKey) {
        match self {
            ConfigNode::Category(cat) => cat.parent = Some(parent),
            ConfigNode::Option(opt) => opt.parent = Some(parent),
        }
    }

    pub fn iter_children(&self) -> Box<dyn Iterator<Item = &ConfigNode> + '_> {
        match self {
            ConfigNode::Category(cat) => Box::new(cat.children.iter()),
            ConfigNode::Option(_) => Box::new(empty()),
        }
    }

    pub fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ConfigNode> + '_> {
        match self {
            ConfigNode::Category(cat) => Box::new(cat.children.iter_mut()),
            ConfigNode::Option(_) => Box::new(empty()),
        }
    }
}

/// Represents a key which links to a configuration node in the tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConfigKey {
    /// Represents a simple key e.g. "foo"
    Simple(String),
    /// When a key is not unique in the whole tree, then for finding the right node, we try to resolve it to the "nearest" definition in the file system tree and print a warning.
    /// If this is not the wanted outcome, then a fully qualified key can be used. It has the form path.to.category where each dot is the next subcategory.
    Qualified(String),
    /// Nodes that are already placed into the correct subtree, but the absolute path is not known yet.
    Pending(),
    /// After the tree is fully resolved, the full path to the node is known.
    Resolved(String),
}

/// Represents a category of configuration nodes
#[derive(Debug, Clone)]
pub struct ConfigCategory {
    pub parent: Option<ConfigKey>,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub depends_on: HashMap<ConfigKey, ConfigValue>,
    /// This subtree is condensed into a single node, which means the parent keys will not be resolved.
    pub children: Vec<ConfigNode>,
}

#[derive(Debug, Clone)]
pub enum ConfigType {
    Boolean,
    String(Option<Vec<String>>), // Optional list of allowed values
    Integer(Range<i64>),         // Integer with a range
    Float(Range<f64>),           // Float with a range
}

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
            ConfigValue::Boolean(b) => write!(f, "{b}"),
            ConfigValue::String(s) => write!(f, "{s}"),
            ConfigValue::Integer(i) => write!(f, "{i}"),
            ConfigValue::Float(fl) => write!(f, "{fl}"),
            ConfigValue::Invalid => write!(f, "Invalid"),
        }
    }
}

impl From<&Item> for ConfigValue {
    fn from(item: &Item) -> Self {
        match item {
            Item::Value(Value::Boolean(b)) => ConfigValue::Boolean(*b.value()),
            Item::Value(Value::String(s)) => ConfigValue::String(s.to_string()),
            Item::Value(Value::Integer(i)) => ConfigValue::Integer(*i.value()),
            Item::Value(Value::Float(f)) => ConfigValue::Float(*f.value()),
            _ => ConfigValue::Invalid,
        }
    }
}

/// Represents an option in the configuration tree (leaf node)
#[derive(Debug, Clone)]
pub struct ConfigOption {
    /// The link to the parent category, or root if None.
    pub parent: Option<ConfigKey>,
    /// The key of the option. Note: This already "links" implicitly to this struct, so no need for ConfigKey.
    pub key: String,
    /// The human-readable name of the option.
    pub name: String,
    /// An optional description of the option.
    pub description: Option<String>,
    /// The type of the option and allowed values.
    pub typ: ConfigType,
    /// Depends on other options, which must be enabled or have a specific value for this option to be enabled.
    pub depends_on: HashMap<ConfigKey, ConfigValue>,
    pub default: ConfigValue,
}