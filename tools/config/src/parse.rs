use toml_edit::ImDocument;
use toml_edit::Item;
use toml_edit::Key;
use toml_edit::Table;
use toml_edit::TableLike;
use toml_edit::Value;

use annotate_snippets as asn;

use crate::category::ConfigCategory;
use crate::error::Diagnostic;
use crate::error::Error;
use crate::error::Report;
use crate::error::Result;
use crate::option::ConfigOption;
use crate::toml_patch::Spanned;
use crate::types::ConfigKey;
use crate::types::ConfigNode;
use crate::types::ConfigType;
use crate::types::ConfigValue;

fn parse_config_type_value(value: &Value, key: &Key, diag: &Diagnostic) -> Result<ConfigType> {
    match value {
        Value::String(s) => match s.value().to_ascii_lowercase().as_str() {
            "boolean" => Ok(ConfigType::Boolean(false)),
            "string" => Ok(ConfigType::String(None, "".to_string())),
            "integer" => Ok(ConfigType::Integer(i64::MIN..i64::MAX, 0)),
            "float" => Ok(ConfigType::Float(f64::MIN..f64::MAX, 0.0)),
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
                    "boolean" => Ok(ConfigType::Boolean(false)),
                    "string" => {
                        if let Some(Value::Array(arr)) = table.get("allowed_values") {
                            let values: Vec<String> = arr
                                .iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect();
                            Ok(ConfigType::String(Some(values), "".to_string()))
                        } else {
                            Ok(ConfigType::String(None, "".to_string()))
                        }
                    }
                    "integer" => {
                        if let (Some(Value::Integer(min)), Some(Value::Integer(max))) =
                            (table.get("min"), table.get("max"))
                        {
                            return Ok(ConfigType::Integer(*min.value()..*max.value(), 0));
                        }

                        let msg = "integer type without range specified (Using max range). Please use 'type = 'Integer' instead.".to_string();
                        let report =
                            Report::from_spanned(asn::Level::Warning, Some(key), value, msg);
                        let msg = diag.msg(&report);
                        println!("{}", asn::Renderer::styled().render(msg));

                        Ok(ConfigType::Integer(i64::MIN..i64::MAX, 0))
                    }
                    "float" => {
                        if let (Some(Value::Float(min)), Some(Value::Float(max))) =
                            (table.get("min"), table.get("max"))
                        {
                            return Ok(ConfigType::Float(*min.value()..*max.value(), 0.0));
                        }
                        let msg = "float type without range specified. (Using max range). Please use 'type = 'Float' instead.".to_string();
                        let report =
                            Report::from_spanned(asn::Level::Warning, Some(key), value, msg);
                        let msg = diag.msg(&report);
                        println!("{}", asn::Renderer::styled().render(msg));
                        Ok(ConfigType::Float(f64::MIN..f64::MAX, 0.0))
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

fn parse_config_key(key: &str, span: &impl Spanned) -> Result<ConfigKey> {
    // A key is only allowed to contain a-z A-Z and .
    if !key
        .chars()
        .all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '.' | '0'..='9'))
    {
        return Err(Error::InvalidToml(
            Report::from_spanned(
                asn::Level::Error,
                None::<&Key>,
                span,
                format!("keys can only contain letters (a-zA-Z), digits (0-9), and dots (.)"),
            )
            .into(),
        ));
    }

    if key.contains('.') {
        // Qualified key
        Ok(ConfigKey::Qualified(key.to_string()))
    } else {
        // Simple key
        Ok(ConfigKey::Simple(key.to_string()))
    }
}

fn parse_config_depend(
    key: &Key,
    table: &(impl TableLike + Spanned),
) -> Result<(ConfigKey, Option<ConfigValue>)> {
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

            let key =
                parse_config_key(&dependency.clone().into_value(), table.get("key").unwrap())?;

            Ok((key, Some(parsed_value)))
        } else {
            let key =
                parse_config_key(&dependency.clone().into_value(), table.get("key").unwrap())?;
            Ok((key, None))
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

fn parse_config_depends(key: &Key, value: &Item) -> Result<Vec<(ConfigKey, Option<ConfigValue>)>> {
    let mut vec = Vec::new();

    match value {
        Item::Value(Value::Array(v)) => {
            for item in v {
                if let Value::InlineTable(table) = item {
                    let (k, v) = parse_config_depend(key, table)?;
                    vec.push((k, v));
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
                vec.push((k, v));
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

    Ok(vec)
}

fn parse_config_default(key: &Key, value: &Item, typ: &mut ConfigType) -> Result<()> {
    match value {
        Item::Value(Value::Boolean(b)) => {
            if let ConfigType::Boolean(default) = typ {
                *default = *b.value();
                Ok(())
            } else {
                Err(Report::from_spanned(
                    asn::Level::Error,
                    Some(key),
                    value,
                    "default value for boolean type must be a boolean".to_string(),
                )
                .into())
            }
        }
        Item::Value(Value::String(s)) => {
            if let ConfigType::String(allowed_values, default) = typ {
                if let Some(allowed_values) = allowed_values {
                    if !allowed_values.contains(s.value()) {
                        return Err(Report::from_spanned(
                            asn::Level::Error,
                            Some(key),
                            value,
                            format!(
                                "default value '{}' is not in the allowed values {:?}",
                                s.value(),
                                allowed_values
                            ),
                        )
                        .into());
                    }
                }

                *default = s.value().to_string();
                Ok(())
            } else {
                Err(Report::from_spanned(
                    asn::Level::Error,
                    Some(key),
                    value,
                    "default value for string type must be a string".to_string(),
                )
                .into())
            }
        }
        Item::Value(Value::Integer(i)) => {
            if let ConfigType::Integer(range, default) = typ {
                if i.value() < &range.start || i.value() > &range.end {
                    return Err(Report::from_spanned(
                        asn::Level::Error,
                        Some(key),
                        value,
                        format!(
                            "default value for integer type is out of range: {} not in {:?}",
                            i.value(),
                            range
                        ),
                    )
                    .into());
                }

                *default = *i.value();
                Ok(())
            } else {
                Err(Report::from_spanned(
                    asn::Level::Error,
                    Some(key),
                    value,
                    "default value for integer type must be an integer".to_string(),
                )
                .into())
            }
        }
        Item::Value(Value::Float(f)) => {
            if let ConfigType::Float(range, default) = typ {
                if f.value() < &range.start || f.value() > &range.end {
                    return Err(Report::from_spanned(
                        asn::Level::Error,
                        Some(key),
                        value,
                        format!(
                            "default value for float type is out of range: {} not in [{}, {}]",
                            f.value(),
                            f64::MIN,
                            f64::MAX
                        ),
                    )
                    .into());
                }

                *default = *f.value();
                Ok(())
            } else {
                Err(Report::from_spanned(
                    asn::Level::Error,
                    Some(key),
                    value,
                    "default value for float type must be a float".to_string(),
                )
                .into())
            }
        }
        _ => Err(Report::from_spanned(
            asn::Level::Error,
            Some(key),
            value,
            format!("default value for type {typ:?} must be a valid value"),
        )
        .into()),
    }
}

// This should get a root table (category/option) and if its a category it should walk down the tree and parse all subcategories and options.
// The parsed subcategories should be directly added to the parent category, resolved.
fn parse_config_category(
    key: &Key,
    table: &Table,
    parent: &Option<ConfigKey>,
    diag: &Diagnostic,
    next_id: &mut usize,
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
        Vec::new()
    };

    let children = table
        .iter()
        .filter(|(k, _)| *k != "name" && *k != "description" && *k != "depends_on")
        .map(|(k, v)| {
            let k = table.key(k).unwrap(); // Safe unwrap, we are iterating over the keys of the table.

            if let Item::Table(table) = v {
                // Ensure the key is valid
                parse_config_key(k, table)?;

                if table.contains_key("type") {
                    // If the value is an option, we set the parent to pending, as we are registering it as a child of the current category.
                    parse_config_option(k, v, &Some(ConfigKey::Pending()), diag, next_id)
                        .map(ConfigNode::Option)
                } else {
                    // If the value is a category, we recursively parse it. We set the parent to pending, as we are registering it as a child of the current category.
                    parse_config_category(k, table, &Some(ConfigKey::Pending()), diag, next_id)
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

    *next_id += 1;

    Ok(ConfigCategory {
        parent: parent.clone(),
        key: key.to_string(),
        id: *next_id - 1,
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
    next_id: &mut usize,
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

        let mut typ = parse_config_type(table, key, diag)?;

        let depends_on = if let Some(value) = table.get("depends_on") {
            parse_config_depends(key, value)?
        } else {
            Vec::new()
        };

        if let Some(value) = table.get("default") {
            parse_config_default(key, value, &mut typ)?
        }

        *next_id += 1;

        Ok(ConfigOption {
            parent: parent.clone(),
            key: key.to_string(),
            id: *next_id - 1,
            name,
            description,
            typ,
            depends_on,
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
pub fn parse_content(
    content: &str,
    next_id: &mut usize,
    diag: &Diagnostic,
) -> Result<Vec<ConfigNode>> {
    let document = ImDocument::parse(&content).map_err(Report::from)?;

    let mut opts = Vec::new();
    let mut parent: Option<ConfigKey> = None;

    // First, parse the metadata of our file.
    match document.get("metadata") {
        Some(Item::Table(meta_tbl)) => {
            if let Some(Item::Value(Value::String(name))) = meta_tbl.get("parent") {
                // If the metadata has a parent, we set the current parent to this category.
                parent = Some(parse_config_key(
                    name.value(),
                    meta_tbl.get("parent").unwrap(),
                )?);
            }
        }
        Some(item) => {
            return Err(Report::from_spanned(
                asn::Level::Error,
                None::<&Key>,
                item,
                "expected to be a table".to_string(),
            )
            .into());
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
                let option = parse_config_option(key, value, &parent, &diag, next_id)?;
                opts.push(ConfigNode::Option(option));
            } else {
                // Category
                let category = parse_config_category(key, table, &parent, &diag, next_id)?;
                opts.push(ConfigNode::Category(category));
            }
        }
    }

    Ok(opts)
}
