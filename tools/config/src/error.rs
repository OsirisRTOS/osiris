use annotate_snippets::{Level, Message, Snippet};
use std::{borrow::Cow, ops::Range};

use crate::toml_patch::Spanned;

pub type Result<T> = std::result::Result<T, Error>;

/// This is our global error type which contains all possible errors.
#[derive(Debug)]
pub enum Error {
    InvalidToml(Report),
    IoError(std::io::Error),
}

/// This struct is supposed to be filled by the functions parent to fill in context information about the error.
/// The function that actually encounters the error either fills in the error and prints the diagnostic
/// or returns a Result to the parent function which then fills in the error into it's own Diagnostic and prints it, etc.
#[derive(Debug)]
pub struct Diagnostic<'a> {
    /// The filename associated with the error.
    filepath: &'a str,
    /// The content of filename.
    content: Option<&'a str>,
}

impl<'a> Diagnostic<'a> {
    pub fn new(filepath: &'a str, content: Option<&'a str>) -> Self {
        Self { filepath, content }
    }

    pub fn msg<'b>(&'b self, report: &'b Report) -> Message<'b> {
        report.to_msg(self.content, self.filepath)
    }
}

#[derive(Debug)]
pub struct Report {
    lvl: Level,
    title: Cow<'static, str>,
    annotations: Vec<(Level, Range<usize>, Option<Cow<'static, str>>)>,
}

impl Report {
    pub fn new<S>(lvl: Level, title: S) -> Self
    where
        S: Into<Cow<'static, str>>,
    {
        Self {
            lvl,
            title: title.into(),
            annotations: Vec::new(),
        }
    }

    pub fn from_spanned<S>(
        lvl: Level,
        key: Option<&impl Spanned>,
        item: &impl Spanned,
        msg: S,
    ) -> Self
    where
        S: Into<Cow<'static, str>> + std::convert::From<&'static str> + Clone,
    {
        let mut report = Self::new(lvl, msg.clone());

        if let Some(span) = key.and_then(|k| k.span()) {
            report.add_annotation::<S>(Level::Help, span, Some("value defined in here".into()));
        }

        if let Some(span) = item.span() {
            report.add_annotation(lvl, span, Some(msg.into()));
        }

        report
    }

    pub fn add_annotation<S>(&mut self, lvl: Level, span: Range<usize>, message: Option<S>)
    where
        S: Into<Cow<'static, str>>,
    {
        self.annotations.push((lvl, span, message.map(Into::into)));
    }

    pub fn to_msg<'a>(&'a self, source: Option<&'a str>, origin: &'a str) -> Message<'a> {
        let msg = self.lvl.title(&self.title);

        if let Some(source) = source {
            let snippet = Snippet::source(source).fold(true).origin(origin);

            msg.snippet(snippet.annotations(self.annotations.iter().filter_map(
                |(lvl, range, message)| {
                    message
                        .as_ref()
                        .map(|message| lvl.span(range.clone()).label(message.as_ref()))
                },
            )))
        } else {
            msg
        }
    }
}
