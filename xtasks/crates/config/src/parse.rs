use toml_edit::ImDocument;
use toml_edit::Item;
use toml_edit::Key;
use toml_edit::Table;
use toml_edit::TableLike;
use toml_edit::Value;

use annotate_snippets as asn;

use crate::category::ConfigCategory;
use crate::error;
use crate::error::Diagnostic;
use crate::error::Error;
use crate::error::Report;
use crate::error::Result;
use crate::option::ConfigOption;
use crate::toml_patch::Spanned;
use crate::types::Attribute;
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
                        let min = if let Some(Value::Integer(min)) = table.get("min") {
                            *min.value()
                        } else {
                            i64::MIN
                        };

                        let max = if let Some(Value::Integer(max)) = table.get("max") {
                            *max.value()
                        } else {
                            i64::MAX
                        };

                        if max == i64::MAX && min == i64::MIN {
                            let msg = "integer type without range specified (Using max range). Please use 'type = 'Integer' instead.".to_string();
                            let report =
                                Report::from_spanned(asn::Level::Warning, Some(key), value, msg);
                            let msg = diag.msg(&report);
                            println!("{}", error::msg_to_string(msg));
                        }

                        return Ok(ConfigType::Integer(min..max, 0));
                    }
                    "float" => {
                        let min = if let Some(Value::Float(min)) = table.get("min") {
                            *min.value()
                        } else {
                            f64::MIN
                        };

                        let max = if let Some(Value::Float(max)) = table.get("max") {
                            *max.value()
                        } else {
                            f64::MAX
                        };

                        if max == f64::MAX && min == f64::MIN {
                            let msg = "float type without range specified. (Using max range). Please use 'type = 'Float' instead.".to_string();
                            let report =
                                Report::from_spanned(asn::Level::Warning, Some(key), value, msg);
                            let msg = diag.msg(&report);
                            println!("{}", error::msg_to_string(msg));
                        }
                        return Ok(ConfigType::Float(min..max, 0.0));
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
            if let Ok(parsed_value) = value.try_into() {
                let key =
                    parse_config_key(&dependency.clone().into_value(), table.get("key").unwrap())?;

                Ok((key, Some(parsed_value)))
            } else {
                return Err(Error::InvalidToml(Report::from_spanned(
                    asn::Level::Error,
                    Some(key),
                    value,
                    "invalid value for 'value' field in dependency".to_string(),
                )));
            }
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
    if let Err(_) = match value {
        Item::Value(Value::Boolean(b)) => {
            if let ConfigType::Boolean(default) = typ {
                *default = *b.value();
                Ok(())
            } else {
                Err(())
            }
        }
        Item::Value(Value::String(s)) => {
            if let ConfigType::String(_allowed_values, default) = typ {
                // TODO: Allowed values are checked at load stage.
                // At this stage we don't know the resolved macros yet.
                // But at load stage we forgot the span info, etc. :(
                *default = s.value().to_string();
                Ok(())
            } else {
                Err(())
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
                Err(())
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
                Err(())
            }
        }
        _ => Err(()),
    } {
        return Err(Error::InvalidToml(Report::from_spanned(
            asn::Level::Error,
            Some(key),
            value,
            format!(
                "invalid default: expected {}, found {}",
                typ.type_name(),
                value.type_name()
            ),
        )));
    }

    Ok(())
}

fn parse_config_attributes(key: &Key, value: &Item) -> Result<Vec<Attribute>> {
    let mut attributes = Vec::new();

    match value {
        Item::Value(Value::Array(attribs)) => {
            for attrib in attribs {
                if let Value::String(s) = attrib {
                    if let Ok(attrib) = s.value().as_str().try_into() {
                        attributes.push(attrib);
                    } else {
                        return Err(Report::from_spanned(
                            asn::Level::Error,
                            Some(key),
                            attrib,
                            format!("unknown attribute '{}'", s.value()),
                        )
                        .into());
                    }
                } else {
                    return Err(Report::from_spanned(
                        asn::Level::Error,
                        Some(key),
                        value,
                        format!(
                            "invalid attribute: expected string, found {}",
                            attrib.type_name()
                        ),
                    )
                    .into());
                }
            }

            Ok(attributes)
        }
        _ => {
            return Err(Report::from_spanned(
                asn::Level::Error,
                Some(key),
                value,
                format!(
                    "invalid attributes: expected array, found {}",
                    value.type_name()
                ),
            )
            .into());
        }
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

    let mut attributes = Vec::new();

    if let Some(value) = table.get("attributes") {
        attributes = parse_config_attributes(key, value)?;
    }

    const RESERVED_KEYS: [&str; 4] = ["name", "description", "depends_on", "attributes"];

    let children = table
        .iter()
        .filter(|(name, _)| !RESERVED_KEYS.contains(name))
        .map(|(name, value)| {
            let parent = key;
            let key = table.key(name).unwrap(); // Safe unwrap, we are iterating over the keys of the table.

            if let Item::Table(table) = value {
                // Ensure the key is valid
                parse_config_key(key, table)?;

                if table.contains_key("type") {
                    // If the value is an option, we set the parent to pending, as we are registering it as a child of the current category.
                    parse_config_option(key, value, &Some(ConfigKey::Pending()), diag, next_id)
                        .map(ConfigNode::Option)
                } else {
                    // If the value is a category, we recursively parse it. We set the parent to pending, as we are registering it as a child of the current category.
                    parse_config_category(key, table, &Some(ConfigKey::Pending()), diag, next_id)
                        .map(ConfigNode::Category)
                }
            } else {
                let mut report = Report::from_spanned(
                    asn::Level::Error,
                    Some(parent),
                    value,
                    format!(
                        "invalid config option '{}': expected table, found {}",
                        name,
                        value.type_name()
                    ),
                );

                if let Some(span) = key.span() {
                    report.add_annotation(
                        asn::Level::Help,
                        span,
                        Some(format!(
                            "only reserved keywords (e.g. name) are part of the category definition"
                        )),
                    );
                }

                Err(Error::InvalidToml(report))
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
        attributes,
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

        let mut attributes = Vec::new();

        if let Some(value) = table.get("attributes") {
            attributes = parse_config_attributes(key, value)?;
        }

        *next_id += 1;

        Ok(ConfigOption {
            parent: parent.clone(),
            key: key.to_string(),
            id: *next_id - 1,
            name,
            description,
            typ,
            attributes,
            depends_on,
        })
    } else {
        Err(Error::InvalidToml(Report::from_spanned(
            asn::Level::Error,
            Some(key),
            value,
            format!(
                "invalid config option: expected table, found {}",
                value.type_name()
            ),
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
            } else if meta_tbl.contains_key("parent") {
                return Err(Report::from_spanned(
                    asn::Level::Error,
                    None::<&Key>,
                    meta_tbl.get("parent").unwrap(),
                    "expected 'parent' to be a string".to_string(),
                )
                .into());
            }

            // if there are other keys in metadata, we error out.
            for (key, _) in meta_tbl.iter() {
                if key != "parent" {
                    return Err(Report::from_spanned(
                        asn::Level::Error,
                        None::<&Key>,
                        meta_tbl,
                        format!("unknown metadata key '{}'", key),
                    )
                    .into());
                }
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
                let option = parse_config_option(key, value, &parent, diag, next_id)?;
                opts.push(ConfigNode::Option(option));
            } else {
                // Category
                let category = parse_config_category(key, table, &parent, diag, next_id)?;
                opts.push(ConfigNode::Category(category));
            }
        }
    }

    Ok(opts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::fail_on_error;

    #[ctor::ctor]
    fn setup_logging() {
        logging::init();
    }

    #[allow(dead_code)]
    fn parse_expect_ok(content: &str) -> Vec<ConfigNode> {
        let mut next_id = 0;
        let diag = Diagnostic::new("test.toml", Some(content));
        fail_on_error(parse_content(content, &mut next_id, &diag), Some(&diag))
    }

    #[allow(dead_code)]
    fn parse_expect_err(content: &str) {
        let mut next_id = 0;
        let diag = Diagnostic::new("test.toml", Some(content));
        assert!(parse_content(content, &mut next_id, &diag).is_err());
    }

    #[test]
    fn parse_simple() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "An Option"
            type = "String"
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 1);

        if let ConfigNode::Option(opt) = &nodes[0] {
            assert_eq!(opt.name, "An Option");
            assert_eq!(opt.key, "option");
            assert_eq!(opt.id, 0);
            if let ConfigType::String(_, default) = &opt.typ {
                assert_eq!(default, "");
            } else {
                panic!("Expected String type");
            }
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn reject_invalid_metadata() {
        let content = r#"
            [metadata]
            name = "An Option"
            type = "String"
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn reject_invalid_parent() {
        let content = r#"
            [metadata]
            parent = 123
            
            [option]
            name = "An Option"
            type = "String"
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn parse_category_with_options() {
        let content = r#"
            [metadata]
            parent = "."
            
            [category]
            name = "Test Category"
            description = "A test category"
            
            [category.option1]
            name = "Option 1"
            type = "Boolean"
            default = true
            
            [category.option2]
            name = "Option 2"
            type = "Integer"
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 1);

        if let ConfigNode::Category(cat) = &nodes[0] {
            assert_eq!(cat.name, "Test Category");
            assert_eq!(cat.key, "category");
            assert_eq!(cat.children.len(), 2);
        } else {
            panic!("Expected Category node");
        }
    }

    #[test]
    fn parse_integer_with_range() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "Port Number"
            type = { type = "Integer", min = 1024, max = 65535 }
            default = 8080
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 1);

        if let ConfigNode::Option(opt) = &nodes[0] {
            if let ConfigType::Integer(range, default) = &opt.typ {
                assert_eq!(range.start, 1024);
                assert_eq!(range.end, 65535);
                assert_eq!(*default, 8080);
            } else {
                panic!("Expected Integer type with range");
            }
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn parse_integer_with_half_open_range() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "Samples"
            type = { type = "Integer", min = 1024 }
            default = 2048
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 1);

        if let ConfigNode::Option(opt) = &nodes[0] {
            if let ConfigType::Integer(range, default) = &opt.typ {
                assert_eq!(range.start, 1024);
                assert_eq!(range.end, i64::MAX);
                assert_eq!(*default, 2048);
            } else {
                panic!("Expected Integer type with range");
            }
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn reject_integer_default_out_of_range() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "Port Number"
            type = { type = "Integer", min = 1024, max = 65535 }
            default = 999
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn parse_string_with_allowed_values() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "Log Level"
            type = { type = "String", allowed_values = ["debug", "info", "warn", "error"] }
            default = "info"
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 1);

        if let ConfigNode::Option(opt) = &nodes[0] {
            if let ConfigType::String(Some(allowed), default) = &opt.typ {
                assert_eq!(allowed.len(), 4);
                assert!(
                    allowed.contains(&"info".to_string())
                        && allowed.contains(&"debug".to_string())
                        && allowed.contains(&"warn".to_string())
                        && allowed.contains(&"error".to_string())
                );
                assert_eq!(default, "info");
            } else {
                panic!("Expected String type with allowed values");
            }
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn parse_float_with_range() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "Temperature"
            type = { type = "Float", min = -273.15, max = 1000.0 }
            default = 20.5
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 1);

        if let ConfigNode::Option(opt) = &nodes[0] {
            if let ConfigType::Float(range, default) = &opt.typ {
                assert_eq!(range.start, -273.15);
                assert_eq!(range.end, 1000.0);
                assert_eq!(*default, 20.5);
            } else {
                panic!("Expected Float type with range");
            }
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn parse_option_with_dependencies() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "Dependent Option"
            type = "Boolean"
            depends_on = [
                { key = ".other.option", value = true },
                { key = "simplekey" }
            ]
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 1);

        if let ConfigNode::Option(opt) = &nodes[0] {
            assert_eq!(opt.depends_on.len(), 2);

            // First dependency with value
            if let ConfigKey::Qualified(key) = &opt.depends_on[0].0 {
                assert_eq!(key, ".other.option");
            } else {
                panic!("Expected qualified key");
            }
            assert!(opt.depends_on[0].1.is_some());

            // Second dependency without value
            if let ConfigKey::Simple(key) = &opt.depends_on[1].0 {
                assert_eq!(key, "simplekey");
            } else {
                panic!("Expected simple key");
            }
            assert!(opt.depends_on[1].1.is_none());
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn parse_option_with_attributes() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "Hidden Option"
            type = "String"
            attributes = ["hidden"]
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 1);

        if let ConfigNode::Option(opt) = &nodes[0] {
            assert_eq!(opt.attributes.len(), 1);
            assert_eq!(opt.attributes[0], Attribute::Hidden);
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn reject_invalid_attribute() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "Option"
            type = "String"
            attributes = ["invalid_attribute"]
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn parse_nested_categories() {
        let content = r#"
            [metadata]
            parent = "."
            
            [category]
            name = "Parent Category"
            
            [category.subcategory]
            name = "Sub Category"
            
            [category.subcategory.option]
            name = "Nested Option"
            type = "Boolean"
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 1);

        if let ConfigNode::Category(cat) = &nodes[0] {
            assert_eq!(cat.name, "Parent Category");
            assert_eq!(cat.children.len(), 1);

            if let ConfigNode::Category(subcat) = &cat.children[0] {
                assert_eq!(subcat.name, "Sub Category");
                assert_eq!(subcat.children.len(), 1);

                if let ConfigNode::Option(opt) = &subcat.children[0] {
                    assert_eq!(opt.name, "Nested Option");
                } else {
                    panic!("Expected nested Option");
                }
            } else {
                panic!("Expected nested Category");
            }
        } else {
            panic!("Expected Category node");
        }
    }

    #[test]
    fn parse_multiple_top_level_nodes() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option1]
            name = "First Option"
            type = "Boolean"
            
            [category1]
            name = "First Category"
            
            [option2]
            name = "Second Option"
            type = "String"
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 3);
    }

    #[test]
    fn reject_missing_type_field() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "Broken Option"
            default = "test"
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn reject_type_mismatch_in_default() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "Broken Option"
            type = "Boolean"
            default = "not a boolean"
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn reject_invalid_config_key_characters() {
        let content = r#"
            [metadata]
            parent = "invalid@key"
            
            [option]
            name = "Option"
            type = "String"
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn parse_qualified_parent_key() {
        let content = r#"
            [metadata]
            parent = ".parent.category.subcategory"
            
            [option]
            name = "Option"
            type = "String"
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 1);

        if let ConfigNode::Option(opt) = &nodes[0] {
            if let Some(ConfigKey::Qualified(key)) = &opt.parent {
                assert_eq!(key, ".parent.category.subcategory");
            } else {
                panic!("Expected qualified parent key");
            }
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn parse_category_with_dependencies() {
        let content = r#"
            [metadata]
            parent = "."
            
            [category]
            name = "Conditional Category"
            depends_on = [{ key = ".other.feature", value = true }]
            
            [category.option]
            name = "Option"
            type = "String"
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 1);

        if let ConfigNode::Category(cat) = &nodes[0] {
            assert_eq!(cat.depends_on.len(), 1);
        } else {
            panic!("Expected Category node");
        }
    }

    #[test]
    fn reject_unknown_type() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "Option"
            type = "UnknownType"
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn parse_all_basic_types() {
        let content = r#"
            [metadata]
            parent = "."
            
            [bool_opt]
            type = "Boolean"
            default = false
            
            [string_opt]
            type = "String"
            default = "test"
            
            [int_opt]
            type = "Integer"
            default = 42
            
            [float_opt]
            type = "Float"
            default = 3.14
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 4);
    }

    #[test]
    fn parse_empty_description() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            name = "Option"
            type = "String"
        "#;

        let nodes = parse_expect_ok(content);

        if let ConfigNode::Option(opt) = &nodes[0] {
            assert!(opt.description.is_none());
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn parse_option_defaults_to_key_as_name() {
        let content = r#"
            [metadata]
            parent = "."
            
            [my_option]
            type = "String"
        "#;

        let nodes = parse_expect_ok(content);

        if let ConfigNode::Option(opt) = &nodes[0] {
            assert_eq!(opt.name, "my_option");
            assert_eq!(opt.key, "my_option");
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn parse_category_defaults_to_key_as_name() {
        let content = r#"
            [metadata]
            parent = "."
            
            [mycat]
            description = "A category"
        "#;

        let nodes = parse_expect_ok(content);

        if let ConfigNode::Category(cat) = &nodes[0] {
            assert_eq!(cat.name, "mycat");
            assert_eq!(cat.key, "mycat");
        } else {
            panic!("Expected Category node");
        }
    }

    #[test]
    fn reject_float_default_out_of_range() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            type = { type = "Float", min = 0.0, max = 1.0 }
            default = 2.5
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn reject_option_as_bare_key() {
        let content = r#"
            [metadata]
            parent = "."
            
            option_value = 42
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn parse_multiple_attributes() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            type = "String"
            attributes = ["hidden", "deprecated"]
        "#;

        let nodes = parse_expect_ok(content);

        if let ConfigNode::Option(opt) = &nodes[0] {
            assert_eq!(opt.attributes.len(), 2);
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn reject_non_array_attributes() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            type = "String"
            attributes = "hidden"
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn reject_non_string_attribute_in_array() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            type = "String"
            attributes = ["hidden", 123]
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn parse_depends_on_without_value() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            type = "Boolean"
            depends_on = [{ key = ".other" }]
        "#;

        let nodes = parse_expect_ok(content);

        if let ConfigNode::Option(opt) = &nodes[0] {
            assert_eq!(opt.depends_on.len(), 1);
            assert!(opt.depends_on[0].1.is_none());
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn parse_integer_without_range() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            type = { type = "Integer" }
            default = 42
        "#;

        let nodes = parse_expect_ok(content);

        if let ConfigNode::Option(opt) = &nodes[0] {
            if let ConfigType::Integer(range, val) = &opt.typ {
                assert_eq!(range.start, i64::MIN);
                assert_eq!(range.end, i64::MAX);
                assert_eq!(*val, 42);
            } else {
                panic!("Expected Integer type");
            }
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn parse_float_without_range() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            type = { type = "Float" }
            default = 3.14
        "#;

        let nodes = parse_expect_ok(content);

        if let ConfigNode::Option(opt) = &nodes[0] {
            if let ConfigType::Float(range, val) = &opt.typ {
                assert_eq!(range.start, f64::MIN);
                assert_eq!(range.end, f64::MAX);
                assert_eq!(*val, 3.14);
            } else {
                panic!("Expected Float type");
            }
        } else {
            panic!("Expected Option node");
        }
    }

    #[test]
    fn reject_inline_table_without_type_field() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            type = { min = 0, max = 100 }
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn parse_complex_nested_structure() {
        let content = r#"
            [metadata]
            parent = "."
            
            [level1]
            name = "Level 1"
            
            [level1.level2]
            name = "Level 2"
            
            [level1.level2.level3]
            name = "Level 3"
            
            [level1.level2.level3.option]
            type = "Boolean"
        "#;

        let nodes = parse_expect_ok(content);
        assert_eq!(nodes.len(), 1);

        if let ConfigNode::Category(cat1) = &nodes[0] {
            assert_eq!(cat1.key, "level1");
            assert_eq!(cat1.children.len(), 1);

            if let ConfigNode::Category(cat2) = &cat1.children[0] {
                assert_eq!(cat2.key, "level2");
                assert_eq!(cat2.children.len(), 1);

                if let ConfigNode::Category(cat3) = &cat2.children[0] {
                    assert_eq!(cat3.key, "level3");
                    assert_eq!(cat3.children.len(), 1);

                    if let ConfigNode::Option(opt) = &cat3.children[0] {
                        assert_eq!(opt.key, "option");
                    } else {
                        panic!("Expected option at level 3");
                    }
                } else {
                    panic!("Expected category at level 3");
                }
            } else {
                panic!("Expected category at level 2");
            }
        } else {
            panic!("Expected category at level 1");
        }
    }

    #[test]
    fn reject_option_not_a_table() {
        let content = r#"
            [metadata]
            parent = "."
            
            [category]
            name = "Cat"
            description = "Desc"
            depends_on = []
            attributes = []
            hidden_field = "This should be caught"
        "#;

        parse_expect_err(content);
    }

    #[test]
    fn parse_string_with_empty_allowed_values() {
        let content = r#"
            [metadata]
            parent = "."
            
            [option]
            type = { type = "String", allowed_values = [] }
            default = "test"
        "#;

        let nodes = parse_expect_ok(content);

        if let ConfigNode::Option(opt) = &nodes[0] {
            if let ConfigType::String(Some(allowed), _) = &opt.typ {
                assert_eq!(allowed.len(), 0);
            } else {
                panic!("Expected String type with allowed values");
            }
        } else {
            panic!("Expected Option node");
        }
    }
}
