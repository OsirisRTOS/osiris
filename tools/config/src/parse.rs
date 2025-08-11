use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;

use toml_edit::ImDocument;
use toml_edit::Item;
use toml_edit::Key;
use toml_edit::Table;
use toml_edit::TableLike;
use toml_edit::Value;

use annotate_snippets as asn;

use crate::error::Diagnostic;
use crate::error::Error;
use crate::error::Report;
use crate::error::Result;
use crate::toml_patch::Spanned;
use crate::types::ConfigCategory;
use crate::types::ConfigKey;
use crate::types::ConfigNode;
use crate::types::ConfigOption;
use crate::types::ConfigType;
use crate::types::ConfigValue;

fn parse_config_type_value(value: &Value, key: &Key, diag: &Diagnostic) -> Result<ConfigType> {
    match value {
        Value::String(s) => match s.value().to_ascii_lowercase().as_str() {
            "boolean" => Ok(ConfigType::Boolean),
            "string" => Ok(ConfigType::String(None)),
            "integer" => Ok(ConfigType::Integer(i64::MIN..i64::MAX)),
            "float" => Ok(ConfigType::Float(f64::MIN..f64::MAX)),
            _ => Err(Error::InvalidToml(Report::from_spanned(
                asn::Level::Error,
                Some(key),
                value,
                format!("unknown type {}", s.value()),
            ))),
        },
        Value::InlineTable(table) => {
            // If the value is an inline table, we parse it as a type.
            if let Some(Value::String(s)) = table.get("type") {
                match s.value().to_ascii_lowercase().as_str() {
                    "boolean" => Ok(ConfigType::Boolean),
                    "string" => {
                        if let Some(Value::Array(arr)) = table.get("allowed_values") {
                            let values: Vec<String> = arr
                                .iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect();
                            Ok(ConfigType::String(Some(values)))
                        } else {
                            Ok(ConfigType::String(None))
                        }
                    }
                    "integer" => {
                        if let (Some(Value::Integer(min)), Some(Value::Integer(max))) =
                            (table.get("min"), table.get("max"))
                        {
                            return Ok(ConfigType::Integer(*min.value()..*max.value()));
                        }

                        let msg = "integer type without range specified (Using max range). Please use 'type = 'Integer' instead.".to_string();
                        let report =
                            Report::from_spanned(asn::Level::Warning, Some(key), value, msg);
                        let msg = diag.msg(&report);
                        println!("{}", asn::Renderer::styled().render(msg));

                        Ok(ConfigType::Integer(i64::MIN..i64::MAX))
                    }
                    "float" => {
                        if let (Some(Value::Float(min)), Some(Value::Float(max))) =
                            (table.get("min"), table.get("max"))
                        {
                            return Ok(ConfigType::Float(*min.value()..*max.value()));
                        }
                        let msg = "float type without range specified. (Using max range). Please use 'type = 'Float' instead.".to_string();
                        let report =
                            Report::from_spanned(asn::Level::Warning, Some(key), value, msg);
                        let msg = diag.msg(&report);
                        println!("{}", asn::Renderer::styled().render(msg));
                        Ok(ConfigType::Float(f64::MIN..f64::MAX))
                    }
                    _ => Err(Error::InvalidToml(Report::from_spanned(
                        asn::Level::Error,
                        Some(key),
                        value,
                        format!("unknown type {}", s.value()),
                    ))),
                }
            } else {
                Err(Error::InvalidToml(Report::from_spanned(
                    asn::Level::Error,
                    Some(key),
                    value,
                    "expected a 'type' field in inline table".to_string(),
                )))
            }
        }
        _ => Err(Error::InvalidToml(Report::from_spanned(
            asn::Level::Error,
            Some(key),
            value,
            "expected a string value for 'type'".to_string(),
        ))),
    }
}

/// Determines the type of a configuration option based on the 'type' field.
///
/// # Arguments
/// * `value` - The 'type' field of the option.
/// * `key` - The key/identifier of the option.
/// * `diag` - Diagnostic information for error reporting.
fn parse_config_type(table: &Table, key: &Key, diag: &Diagnostic) -> Result<ConfigType> {
    // Check if our value is a table => it has tighter constraints on the type.
    // If it is a table, it must have a "type" key.
    if let Some((_, item)) = table.get_key_value("type") {
        match item {
            Item::Value(value) => parse_config_type_value(value, key, diag),
            _ => Err(Error::InvalidToml(Report::from_spanned(
                asn::Level::Error,
                Some(key),
                item,
                "expected 'type' to be an inline table or string.".to_string(),
            ))),
        }
    } else {
        Err(Error::InvalidToml(Report::from_spanned(
            asn::Level::Error,
            Some(key),
            table,
            "missing 'type' field".to_string(),
        )))
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

fn parse_config_depend(
    key: &Key,
    table: &(impl TableLike + Spanned),
) -> Result<(String, ConfigValue)> {
    if let Some(Item::Value(Value::String(dependency))) = table.get("key") {
        if let Some(value) = table.get("value") {
            let parsed_value: ConfigValue = value.into();

            if parsed_value == ConfigValue::Invalid {
                return Err(Error::InvalidToml(Report::from_spanned(
                    asn::Level::Error,
                    Some(key),
                    value,
                    "invalid value for 'value' field in dependency".to_string(),
                )));
            }

            Ok((dependency.clone().into_value(), parsed_value))
        } else {
            Err(Error::InvalidToml(Report::from_spanned(
                asn::Level::Error,
                Some(key),
                table,
                "missing 'value' field in dependency".to_string(),
            )))
        }
    } else {
        Err(Error::InvalidToml(Report::from_spanned(
            asn::Level::Error,
            Some(key),
            table,
            "missing 'key' field in dependency".to_string(),
        )))
    }
}

fn parse_config_depends(key: &Key, value: &Item) -> Result<HashMap<ConfigKey, ConfigValue>> {
    let mut map = HashMap::new();

    match value {
        Item::Value(Value::Array(v)) => {
            for item in v {
                if let Value::InlineTable(table) = item {
                    let (k, v) = parse_config_depend(key, table)?;
                    let k = parse_config_key(&k);
                    map.insert(k, v);
                } else {
                    return Err(Error::InvalidToml(Report::from_spanned(
                        asn::Level::Error,
                        Some(key),
                        value,
                        "expected to be an inline table".to_string(),
                    )));
                }
            }
        }
        Item::ArrayOfTables(v) => {
            for item in v {
                let (k, v) = parse_config_depend(key, item)?;
                let k = parse_config_key(&k);
                map.insert(k, v);
            }
        }
        _ => {
            return Err(Error::InvalidToml(Report::from_spanned(
                asn::Level::Error,
                Some(key),
                value,
                "expected to be an array of values or tables".to_string(),
            )));
        }
    }

    Ok(map)
}

fn parse_config_default(key: &Key, value: &Item, typ: &ConfigType) -> Result<ConfigValue> {
    match value {
        Item::Value(Value::Boolean(b)) => {
            if let ConfigType::Boolean = typ {
                Ok(ConfigValue::Boolean(*b.value()))
            } else {
                Err(Error::InvalidToml(Report::from_spanned(
                    asn::Level::Error,
                    Some(key),
                    value,
                    "default value for boolean type must be a boolean".to_string(),
                )))
            }
        }
        Item::Value(Value::String(s)) => {
            if let ConfigType::String(allowed_values) = typ {
                if let Some(allowed_values) = allowed_values {
                    if !allowed_values.contains(&s.value()) {
                        return Err(Error::InvalidToml(Report::from_spanned(
                            asn::Level::Error,
                            Some(key),
                            value,
                            format!(
                                "default value '{}' is not in the allowed values {:?}",
                                s.value(),
                                allowed_values
                            ),
                        )));
                    }
                }

                Ok(ConfigValue::String(s.to_string()))
            } else {
                Err(Error::InvalidToml(Report::from_spanned(
                    asn::Level::Error,
                    Some(key),
                    value,
                    "default value for string type must be a string".to_string(),
                )))
            }
        }
        Item::Value(Value::Integer(i)) => {
            if let ConfigType::Integer(range) = typ {
                if i.value() < &range.start || i.value() > &range.end {
                    return Err(Error::InvalidToml(Report::from_spanned(
                        asn::Level::Error,
                        Some(key),
                        value,
                        format!(
                            "default value for integer type is out of range: {} not in {:?}",
                            i.value(),
                            range
                        ),
                    )));
                }

                Ok(ConfigValue::Integer(*i.value()))
            } else {
                Err(Error::InvalidToml(Report::from_spanned(
                    asn::Level::Error,
                    Some(key),
                    value,
                    "default value for integer type must be an integer".to_string(),
                )))
            }
        }
        Item::Value(Value::Float(f)) => {
            if let ConfigType::Float(_) = typ {
                if f.value() < &f64::MIN || f.value() > &f64::MAX {
                    return Err(Error::InvalidToml(Report::from_spanned(
                        asn::Level::Error,
                        Some(key),
                        value,
                        format!(
                            "default value for float type is out of range: {} not in [{}, {}]",
                            f.value(),
                            f64::MIN,
                            f64::MAX
                        ),
                    )));
                }

                Ok(ConfigValue::Float(*f.value()))
            } else {
                Err(Error::InvalidToml(Report::from_spanned(
                    asn::Level::Error,
                    Some(key),
                    value,
                    "default value for float type must be a float".to_string(),
                )))
            }
        }
        _ => Err(Error::InvalidToml(Report::from_spanned(
            asn::Level::Error,
            Some(key),
            value,
            format!("default value for type {:?} must be a valid value", typ),
        ))),
    }
}

// This should get a root table (category/option) and if its a category it should walk down the tree and parse all subcategories and options.
// The parsed subcategories should be directly added to the parent category, resolved.
fn parse_config_category(
    key: &Key,
    table: &Table,
    parent: &Option<ConfigKey>,
    diag: &Diagnostic,
) -> Result<ConfigCategory> {
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
            let k = table.key(k).unwrap(); // Safe unwrap, we are iterating over the keys of the table.

            if let Item::Table(table) = v {
                if table.contains_key("type") {
                    // If the value is an option, we set the parent to pending, as we are registering it as a child of the current category.
                    parse_config_option(k, v, &Some(ConfigKey::Pending()), diag)
                        .map(ConfigNode::Option)
                } else {
                    // If the value is a category, we recursively parse it. We set the parent to pending, as we are registering it as a child of the current category.
                    parse_config_category(k, table, &Some(ConfigKey::Pending()), diag)
                        .map(ConfigNode::Category)
                }
            } else {
                Err(Error::InvalidToml(Report::from_spanned(
                    asn::Level::Error,
                    Some(key),
                    v,
                    "an option is expected to be a table".to_string(),
                )))
            }
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(ConfigCategory {
        parent: parent.clone(),
        key: key.to_string(),
        name,
        description,
        depends_on,
        children,
    })
}

fn parse_config_option(
    key: &Key,
    value: &Item,
    parent: &Option<ConfigKey>,
    diag: &Diagnostic,
) -> Result<ConfigOption> {
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

        let typ = parse_config_type(table, key, diag)?;

        let depends_on = if let Some(value) = table.get("depends_on") {
            parse_config_depends(key, value)?
        } else {
            HashMap::new()
        };

        let default = if let Some(value) = table.get("default") {
            parse_config_default(key, value, &typ)?
        } else {
            match typ {
                ConfigType::Boolean => ConfigValue::Boolean(false),
                ConfigType::String(_) => ConfigValue::String(String::new()),
                ConfigType::Integer(_) => ConfigValue::Integer(0),
                ConfigType::Float(_) => ConfigValue::Float(0.0),
            }
        };

        Ok(ConfigOption {
            parent: parent.clone(), // Parent will be set later
            key: key.to_string(),
            name,
            description,
            typ,
            depends_on,
            default,
        })
    } else {
        Err(Error::InvalidToml(Report::from_spanned(
            asn::Level::Error,
            Some(key),
            value,
            "expected to be a table".to_string(),
        )))
    }
}

/// We build this tree bottom-up, so we first load all the files and then parse them into ConfigOption structs as-well as ConfigCategory structs.
pub fn parse_file(path: &Path) -> Result<Vec<ConfigNode>> {
    let content = fs::read_to_string(path).map_err(Error::IoError)?;

    let path = path.to_string_lossy();
    let diag = Diagnostic::new(&path, Some(&content));

    let document = ImDocument::parse(&content).map_err(|e| {
        Error::IoError(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse TOML file '{path}': {e}"),
        ))
    })?;

    let mut opts = Vec::new();

    let mut parent: Option<ConfigKey> = None;

    // First, parse the metadata of our file.
    match document.get("metadata") {
        Some(Item::Table(meta_tbl)) => {
            if let Some(Item::Value(Value::String(name))) = meta_tbl.get("parent") {
                // If the metadata has a parent, we set the current parent to this category.
                parent = Some(parse_config_key(name.value()));
            }
        }
        Some(_) => {
            return Err(Error::IoError(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid metadata format in TOML file.",
            )));
        }
        None => {
            // If there is no parent, we assume the parent is the root category.
        }
    };

    // Now we iterate over the table and parse options and categories
    for (key, value) in document.iter().filter(|(k, _)| *k != "metadata") {
        let key = document.key(key).unwrap(); // Safe unwrap, we are iterating over the keys of the document.

        // Each category is a table with a name and n nested tables. Each nested table is an option or a subcategory.
        if let Item::Table(table) = value {
            if table.contains_key("type") {
                // Option
                match parse_config_option(key, value, &parent, &diag) {
                    Ok(option) => {
                        opts.push(ConfigNode::Option(option));
                    }
                    Err(Error::InvalidToml(report)) => {
                        let msg = diag.msg(&report);
                        println!("{}", asn::Renderer::styled().render(msg));
                    }
                    _ => {
                        unreachable!("Unexpected error while parsing option");
                    }
                }
            } else {
                // Category
                match parse_config_category(key, table, &parent, &diag) {
                    Ok(category) => {
                        opts.push(ConfigNode::Category(category));
                    }
                    Err(Error::InvalidToml(report)) => {
                        let msg = diag.msg(&report);
                        println!("{}", asn::Renderer::styled().render(msg));
                    }
                    _ => {
                        unreachable!("Unexpected error while parsing category");
                    }
                }
            }
        }
    }

    Ok(opts)
}
