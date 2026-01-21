use std::ops::Range;

pub trait Spanned {
    fn span(&self) -> Option<Range<usize>>;
}

impl Spanned for toml_edit::Key {
    fn span(&self) -> Option<Range<usize>> {
        self.span()
    }
}

impl Spanned for toml_edit::Item {
    fn span(&self) -> Option<Range<usize>> {
        self.span()
    }
}

impl Spanned for toml_edit::Value {
    fn span(&self) -> Option<Range<usize>> {
        self.span()
    }
}

impl Spanned for toml_edit::InlineTable {
    fn span(&self) -> Option<Range<usize>> {
        self.span()
    }
}

impl Spanned for toml_edit::Table {
    fn span(&self) -> Option<Range<usize>> {
        self.span()
    }
}
