use crate::types::{ConfigKey, ConfigNode, ConfigNodelike, ConfigType, ConfigValue};

/// Represents an option in the configuration tree (leaf node)
#[derive(Debug, Clone)]
pub struct ConfigOption {
    /// The link to the parent category, or root if None.
    pub parent: Option<ConfigKey>,
    /// The key of the option. Note: This already "links" implicitly to this struct, so no need for ConfigKey.
    pub key: String,
    /// The unique identifier of the node.
    pub id: usize,
    /// The human-readable name of the option.
    pub name: String,
    /// An optional description of the option.
    pub description: Option<String>,
    /// The type of the option and allowed values.
    pub typ: ConfigType,
    /// Depends on other options, which must be enabled or have a specific value for this option to be enabled.
    pub depends_on: Vec<(ConfigKey, Option<ConfigValue>)>,
}

impl ConfigNodelike for ConfigOption {
    fn key(&self) -> &str {
        &self.key
    }

    fn build_full_key(&self) -> Option<String> {
        let parent_path = match self.parent.as_ref()? {
            ConfigKey::Resolved { path, .. } => path,
            _ => return None,
        };

        Some(format!("{}.{}", parent_path, self.key))
    }

    fn id(&self) -> usize {
        self.id
    }

    fn parent(&self) -> Option<&ConfigKey> {
        self.parent.as_ref()
    }

    fn set_parent(&mut self, parent: ConfigKey) {
        self.parent = Some(parent);
    }

    fn iter_children(&self) -> Box<dyn Iterator<Item = &ConfigNode> + '_> {
        Box::new(std::iter::empty())
    }

    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ConfigNode> + '_> {
        Box::new(std::iter::empty())
    }

    fn dependencies_iter(
        &self,
    ) -> Box<dyn Iterator<Item = &(ConfigKey, Option<ConfigValue>)> + '_> {
        Box::new(self.depends_on.iter())
    }

    fn dependencies_iter_mut(
        &mut self,
    ) -> Box<dyn Iterator<Item = &mut (ConfigKey, Option<ConfigValue>)> + '_> {
        Box::new(self.depends_on.iter_mut())
    }

    fn add_dependency(&mut self, key: ConfigKey, value: Option<ConfigValue>) {
        self.depends_on.push((key, value));
    }

    fn dependencies_drain(
        &mut self,
    ) -> Box<dyn Iterator<Item = (ConfigKey, Option<ConfigValue>)> + '_> {
        Box::new(self.depends_on.drain(..))
    }
}

impl ConfigOption {
    pub fn default_value(&self) -> ConfigValue {
        self.typ.clone().into()
    }
}
