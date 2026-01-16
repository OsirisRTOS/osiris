use crate::types::{Attribute, ConfigKey, ConfigNode, ConfigNodelike, ConfigValue};

/// Represents a category of configuration nodes
#[derive(Debug, Clone)]
pub struct ConfigCategory {
    pub parent: Option<ConfigKey>,
    pub key: String,
    /// The unique identifier of the node.
    pub id: usize,
    pub name: String,
    pub description: Option<String>,
    pub depends_on: Vec<(ConfigKey, Option<ConfigValue>)>,
    pub attributes: Vec<Attribute>,
    /// This subtree is condensed into a single node, which means the parent keys will not be resolved.
    pub children: Vec<ConfigNode>,
}

impl ConfigNodelike for ConfigCategory {
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
        Box::new(self.children.iter())
    }

    fn iter_children_mut(&mut self) -> Box<dyn Iterator<Item = &mut ConfigNode> + '_> {
        Box::new(self.children.iter_mut())
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

    fn has_attribute(&self, attribute: &crate::types::Attribute) -> bool {
        self.attributes.contains(attribute)
    }
}
