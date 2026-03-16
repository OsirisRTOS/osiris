use std::collections::HashMap;

// ================================================================================================
// DTS object attribute types
// ================================================================================================

#[derive(Debug, Clone)]
pub enum PropValue {
    Empty,
    U32(u32),
    U32Array(Vec<u32>),
    Str(String),
    StringList(Vec<String>),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub compatible: Vec<String>,
    pub reg: Option<(u64, u64)>, // (base, size)
    pub interrupts: Vec<u32>,
    pub phandle: Option<u32>,
    pub extra: HashMap<String, PropValue>,
    pub children: Vec<usize>, // indices into DeviceTree::nodes
    pub parent: Option<usize>,
}

#[allow(dead_code)]
impl Node {
    pub fn reg_base(&self) -> Option<u64> {
        self.reg.map(|(base, _)| base)
    }

    pub fn reg_size(&self) -> Option<u64> {
        self.reg.map(|(_, size)| size)
    }

    pub fn primary_compatible(&self) -> Option<&str> {
        self.compatible.first().map(|s| s.as_str())
    }

    pub fn is_compatible(&self, prefix: &str) -> bool {
        self.compatible.iter().any(|c| c.starts_with(prefix))
    }

    pub fn extra_u32(&self, key: &str) -> Option<u32> {
        match self.extra.get(key) {
            Some(PropValue::U32(v)) => Some(*v),
            _ => None,
        }
    }

    pub fn extra_u32_array(&self, key: &str) -> Option<&[u32]> {
        match self.extra.get(key) {
            Some(PropValue::U32Array(v)) => Some(v),
            _ => None,
        }
    }

    pub fn extra_str(&self, key: &str) -> Option<&str> {
        match self.extra.get(key) {
            Some(PropValue::Str(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn extra_string_list(&self, key: &str) -> Option<&[String]> {
        match self.extra.get(key) {
            Some(PropValue::StringList(v)) => Some(v),
            _ => None,
        }
    }
}

// ================================================================================================
// Raw devicetree as output from parsing in-memory DTB
// ================================================================================================

#[derive(Debug)]
pub struct DeviceTree {
    pub nodes: Vec<Node>,
    pub by_phandle: HashMap<u32, usize>,
    pub by_name: HashMap<String, usize>,
    pub root: usize,
}

#[allow(dead_code)]
impl DeviceTree {
    pub fn node(&self, idx: usize) -> &Node {
        &self.nodes[idx]
    }

    pub fn resolve_phandle(&self, phandle: u32) -> Option<&Node> {
        self.by_phandle.get(&phandle).map(|&idx| &self.nodes[idx])
    }

    pub fn resolve_phandle_idx(&self, phandle: u32) -> Option<usize> {
        self.by_phandle.get(&phandle).copied()
    }

    // iterate only direct children of a node.
    pub fn children(&self, idx: usize) -> impl Iterator<Item = (usize, &Node)> {
        self.nodes[idx]
            .children
            .iter()
            .map(|&child_idx| (child_idx, &self.nodes[child_idx]))
    }

    // walk all nodes depth-first, calling f for each (idx, node).
    pub fn walk(&self, mut f: impl FnMut(usize, &Node)) {
        self.walk_from(self.root, &mut f);
    }

    fn walk_from(&self, idx: usize, f: &mut impl FnMut(usize, &Node)) {
        f(idx, &self.nodes[idx]);
        for &child in &self.nodes[idx].children {
            self.walk_from(child, f);
        }
    }

    // model string from /model property or first compatible string.
    pub fn model(&self) -> String {
        let root = &self.nodes[self.root];
        if let Some(s) = root.extra_str("model") {
            return s.to_string();
        }
        root.compatible
            .first()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string())
    }

    // resolve stdout-path in /chosen to the first compatible string of that node.
    pub fn stdout_compat(&self) -> Option<String> {
        let chosen_idx = *self.by_name.get("chosen")?;
        let path = self.nodes[chosen_idx].extra_str("stdout-path")?.to_string();
        // strip optional baud suffix: "/soc/serial@deadbeef:115200" -> "/soc/serial@deadbeef"
        let path = path.split(':').next()?;
        // match by last path component
        let name = path.split('/').last()?;
        let idx = self.by_name.get(name)?;
        self.nodes[*idx].compatible.first().cloned()
    }
}
