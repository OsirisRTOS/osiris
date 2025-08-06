use std::collections::HashMap;
use std::fs;
use std::io;
use std::ops::Range;
use std::path::{Path, PathBuf};

use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode,
};
use log::warn;
use log::error;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use toml_edit::Document;
use toml_edit::Item;
use toml_edit::Value;
use walkdir::WalkDir;

use crate::error::Diagnostic;
use crate::error::Error;
use crate::error::Result;

mod error;

// A toml should first define the subcategory that it belongs to, in regards to the whole project. (Because many option files)
// Then it contains a list of options, each has a key, a human-readable name, an optional description, a type an optional value, a boolean to indicate if it is enabled or not and a list of dependencies (other options that must be enabled for this one to be enabled).

/// Represents a key which links to a configuration node in the tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ConfigKey {
    /// Represents a simple key e.g. "foo"
    Simple(String),
    /// When a key is not unique in the whole tree, then for finding the right node, we try to resolve it to the "nearest" definition in the file system tree and print a warning.
    /// If this is not the wanted outcome, then a fully qualified key can be used. It has the form path.to.category where each dot is the next subcategory.
    Qualified(String),
    /// After we evaluated our tree, we directly resolve the key to a node in the tree.
    Resolved(usize),
}

/// Represents a node in the configuration tree
#[derive(Debug, Clone)]
enum ConfigNode {
    Category(ConfigCategory),
    Option(ConfigOption),
}

/// Represents a category of configuration nodes
#[derive(Debug, Clone)]
struct ConfigCategory {
    parent: Option<ConfigKey>,
    key: String,
    name: String,
    description: Option<String>,
    depends_on: HashMap<ConfigKey, ConfigValue>,
    /// This subtree is condensed into a single node, which means the parent keys will not be resolved.
    children: Vec<ConfigNode>,
}

#[derive(Debug, Clone)]
enum ConfigType {
    Boolean,
    String(Option<Vec<String>>), // Optional list of allowed values
    Enum(Vec<String>),           // Enum with a list of possible values
    Integer(Range<i64>),         // Integer with a range
    Float(Range<f64>),           // Float with a range
}

#[derive(Debug, Clone, PartialEq)]
enum ConfigValue {
    Boolean(bool),
    String(String),
    Integer(i64),
    Float(f64),
    Enum(String),
    Invalid,
}

/// Represents an option in the configuration tree (leaf node)
#[derive(Debug, Clone)]
struct ConfigOption {
    /// The link to the parent category, or root if None.
    parent: Option<ConfigKey>,
    /// The key of the option. Note: This already "links" implicitly to this struct, so no need for ConfigKey.
    key: String,
    /// The human-readable name of the option.
    name: String,
    /// An optional description of the option.
    description: Option<String>,
    /// The type of the option and allowed values.
    typ: ConfigType,
    /// Depends on other options, which must be enabled or have a specific value for this option to be enabled.
    depends_on: HashMap<ConfigKey, ConfigValue>,
}

fn load_files(root: &Path) -> Result<Vec<PathBuf>> {
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

/// Determines the type of a configuration option based on the 'type' field.
///
/// # Arguments
/// * `value` - The 'type' field of the option.
/// * `key` - The key/identifier of the option.
/// * `diag` - Diagnostic information for error reporting.
fn parse_config_type(value: &Item, key: &str, diag: &Diagnostic) -> Result<ConfigType> {
    match value {
        // Check if our value is a table => it has tighter constraints on the type.
        Item::Table(table) => {
            // If it is a table, it must have a "type" key.
            if let Some((subkey, Item::Value(Value::String(s)))) = table.get_key_value("type") {
                match s.value().as_str() {
                    // Good old boolean
                    "boolean" => Ok(ConfigType::Boolean),

                    // String with optional allowed values
                    "string" => {
                        if let Some(Item::Value(Value::Array(arr))) = table.get("allowed_values") {
                            let values: Vec<String> = arr
                                .iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect();
                            Ok(ConfigType::String(Some(values)))
                        } else {
                            Ok(ConfigType::String(None))
                        }
                    }

                    // Enum with a list of possible values
                    "enum" => {
                        if let Some(Item::Value(Value::Array(arr))) = table.get("values") {
                            let values: Vec<String> = arr
                                .iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect();
                            Ok(ConfigType::Enum(values))
                        } else {
                            Ok(ConfigType::Enum(Vec::new()))
                        }
                    }

                    // Integer with a range
                    "integer" => {
                        if let Some(Item::Value(Value::Integer(min))) = table.get("min")
                            && let Some(Item::Value(Value::Integer(max))) = table.get("max")
                        {
                            return Ok(ConfigType::Integer(*min.value()..*max.value()));
                        }

                        warn!("{}", diag.with_warn(
                            &format!("Integer type of option '{key}' without range specified, using default range."),
                            Some(subkey)
                        ));

                        Ok(ConfigType::Integer(i64::MIN..i64::MAX))
                    }

                    // Float with a range
                    "float" => {
                        if let Some(Item::Value(Value::Float(min))) = table.get("min")
                            && let Some(Item::Value(Value::Float(max))) = table.get("max")
                        {
                            return Ok(ConfigType::Float(*min.value()..*max.value()));
                        }

                        warn!("{}", diag.with_warn(
                            &format!("Float type of option '{key}' without range specified, using default range."),
                            Some(subkey)
                        ));

                        Ok(ConfigType::Float(f64::MIN..f64::MAX))
                    }

                    // Unknown type
                    _ => Err(Error::Io(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Unknown type '{s}' for option '{key}'"),
                    ))),
                }
            } else {
                // If there is no type, we cannot determine the type of the option.
                Err(Error::Io(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Missing type for option {key}"),
                )))
            }
        }

        // If the value is not a table, we assume it is a simple type.
        Item::Value(Value::String(s)) => match s.value().to_lowercase().as_str() {
            // Simple types with default values/ranges/etc.
            "boolean" => Ok(ConfigType::Boolean),
            "string" => Ok(ConfigType::String(None)),
            "integer" => Ok(ConfigType::Integer(i64::MIN..i64::MAX)),
            "float" => Ok(ConfigType::Float(f64::MIN..f64::MAX)),

            // If the string is not a known type, we return an error.
            _ => Err(Error::Io(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unknown type '{s}' for option '{key}'"),
            ))),
        },

        // If the value is not a table or string, we cannot determine the type.
        // => Error
        _ => Err(Error::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unsupported value type for option '{key}'"),
        ))),
    }
}

fn parse_config_key(key: &str) -> ConfigKey {
    if key.contains('.') {
        // Qualified key
        ConfigKey::Qualified(key.to_string())
    } else {
        // Simple key
        ConfigKey::Simple(key.to_string())
    }
}

fn parse_config_depends(key: &str, value: &Item) -> Result<HashMap<ConfigKey, ConfigValue>> {
    if let Item::ArrayOfTables(arr) = value {
        let mut depends_on = HashMap::new();
        for table in arr {
            if let Some(Item::Value(Value::String(depend))) = table.get("key") {
                match table.get("value").cloned().map(|v| match v {
                    Item::Value(Value::Boolean(b)) => ConfigValue::Boolean(*b.value()),
                    Item::Value(Value::String(s)) => ConfigValue::String(s.into_value()),
                    Item::Value(Value::Integer(i)) => ConfigValue::Integer(*i.value()),
                    Item::Value(Value::Float(f)) => ConfigValue::Float(*f.value()),
                    _ => ConfigValue::Invalid,
                }) {
                    Some(value) if value != ConfigValue::Invalid => {
                        depends_on.insert(parse_config_key(depend.value()), value);
                    }
                    _ => {
                        return Err(Error::Io(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "Invalid dependency value for key '{depend}' in depends_on of option '{key}.'"
                            ),
                        )));
                    }
                }
            }
        }
        Ok(depends_on)
    } else {
        Err(Error::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("depends_on of option '{key}' is expected to be an array."),
        )))
    }
}

// This should get a root table (category/option) and if its a category it should walk down the tree and parse all subcategories and options.
// The parsed subcategories should be directly added to the parent category, resolved.
fn parse_config_category(key: &str, value: &Item) -> Result<ConfigCategory> {
    if let Item::Table(table) = value {
        let name = table
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(key)
            .to_string();

        let description = table
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        let depends_on = if let Some(value) = table.get("depends_on") {
            parse_config_depends(key, value)?
        } else {
            HashMap::new()
        };

        let children = table
            .iter()
            .filter(|(k, _)| *k != "name" && *k != "description" && *k != "depends_on")
            .map(|(k, v)| {
                if let Item::Table(table) = v {
                    match table.get("type") {
                        Some(Item::Value(Value::String(s))) if s.value() == "category" => {
                            // If the value is a category, we parse it as a subcategory
                            parse_config_category(k, v)
                                .map(ConfigNode::Category)
                                .map_err(|e| Error::Io(io::Error::new(io::ErrorKind::InvalidData, e.to_string())))
                        }
                        _ => {
                            // Otherwise it is an option
                            parse_config_option(k, v, &Diagnostic::new(PathBuf::from(""), None, 0))
                                .map(ConfigNode::Option)
                                .map_err(|e| Error::Io(io::Error::new(io::ErrorKind::InvalidData, e.to_string())))
                        }
                    }
                } else {
                    Err(Error::Io(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("The item '{k}' in category '{key}' is not a table."),
                    )))
                }
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(ConfigCategory {
            parent: None, // Parent will be set later
            key: key.to_string(),
            name,
            description,
            depends_on,
            children,
        })
    } else {
        Err(Error::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("The category '{key}' is not a table."),
        )))
    }
}

fn parse_config_option(key: &str, value: &Item, diag: &Diagnostic) -> Result<ConfigOption> {
    if let Item::Table(table) = value {
        let name = table
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(key)
            .to_string();

        let description = table
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        let typ = parse_config_type(value, key, diag)?;

        let depends_on = if let Some(value) = table.get("depends_on") {
            parse_config_depends(key, value)?
        } else {
            HashMap::new()
        };

        Ok(ConfigOption {
            parent: None, // Parent will be set later
            key: key.to_string(),
            name,
            description,
            typ,
            depends_on,
        })
    } else {
        Err(Error::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("The option '{key}' is not a table."),
        )))
    }
}

/// We build this tree bottom-up, so we first load all the files and then parse them into ConfigOption structs as-well as ConfigCategory structs.
fn parse_file(path: &Path, opts: &mut Vec<ConfigNode>) -> io::Result<()> {
    const FILE_CTX: usize = 2; // Context lines around the error
    let content = fs::read_to_string(path)?;

    let diag = Diagnostic::new(path, Some(&content), FILE_CTX);

    let document = Document::parse(&content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse TOML file '{}': {}", path.display(), e),
        )
    })?;

    // Each file can set a category, which is the parent of all options in this file.
    // If there are categories local to the file, they are all implicitly created in regards to the nesting of the option tables.
    let mut current_parent: Option<ConfigKey> = None;

    // First, parse the metadata of our file.
    match document.get("metadata") {
        Some(Item::Table(meta_tbl)) => {
            if let Some(Item::Value(Value::String(name))) = meta_tbl.get("parent") {
                // If the metadata has a parent, we set the current parent to this category.
                current_parent = Some(ConfigKey::Simple(name.clone().into_value()));
            }
        }
        Some(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid metadata format in TOML file.",
            ));
        }
        None => {
            // If there is no parent, we assume the parent is the root category.
        }
    };

    // Now we iterate over the table and parse options and categories
    for (key, value) in document.iter().filter(|(k, _)| *k != "metadata") {
        // Each category is a table with a name and n nested tables. Each nested table is an option or a subcategory.
        if let Item::Table(table) = value {
            match table.get("type") {
                None => {
                    // Category
                    match parse_config_category(key, value) {
                        Ok(mut category) => {
                            category.parent = current_parent.clone();
                            let category = ConfigNode::Category(category);
                            opts.push(category);
                        }
                        Err(e) => {
                            error!("Failed to parse category '{key}': {e}");
                        }
                    }
                }
                Some(value) => {
                    // Option
                    match parse_config_option(key, value, &diag) {
                        Ok(mut option) => {
                            option.parent = current_parent.clone();
                            let option = ConfigNode::Option(option);
                            opts.push(option);
                        }
                        Err(e) => {
                            error!("Failed to parse option '{key}': {e}");
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn main() -> Result<()> {
    // Initialize the terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load configuration files
    let root_path = PathBuf::from("config");
    let files = load_files(&root_path)?;

    // Parse each file and build the configuration tree
    let mut config_nodes = Vec::new();
    for file in files {
        parse_file(&file, &mut config_nodes)?;
    }

    // Here you would continue with your application logic, e.g., displaying the configuration tree.
    
    disable_raw_mode()?;
    Ok(())
}