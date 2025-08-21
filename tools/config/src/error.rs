use annotate_snippets::{Level, Message, Snippet};
use std::{borrow::Cow, fmt::Display, ops::Range, process::exit};
use thiserror::Error;
use toml_edit::TomlError;

use crate::toml_patch::Spanned;

use annotate_snippets as asn;

pub type Result<T> = std::result::Result<T, Error>;

pub fn fail_on_error<T>(res: Result<T>, diag: Option<&Diagnostic>) -> T {
    match (res, diag) {
        (Ok(value), _) => value,
        (Err(Error::InvalidToml(rep)), Some(diag)) => {
            let msg = diag.msg(&rep);
            log::error!("{}", asn::Renderer::styled().render(msg));
            exit(1);
        }
        (Err(error), _) => {
            log::error!("{error}");
            exit(1);
        }
    }
}

/// This is our global error type which contains all possible errors.
#[derive(Debug, Error)]
pub enum Error {
    InvalidToml(#[from] Report),
    Io(#[from] std::io::Error),
    Fmt(#[from] std::fmt::Error),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidToml(report) => write!(f, "{}", report),
            Error::Io(err) => write!(f, "{err}"),
            Error::Fmt(err) => write!(f, "{err}"),
            Error::Other(err) => write!(f, "{err}"),
        }
    }
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

#[derive(Debug, Error)]
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

            msg.snippet(snippet.annotations(self.annotations.iter().map(
                |(lvl, range, message)| match message {
                    Some(message) => lvl.span(range.clone()).label(message.as_ref()),
                    None => lvl.span(range.clone()),
                },
            )))
        } else {
            msg
        }
    }
}

impl From<TomlError> for Report {
    fn from(err: TomlError) -> Self {
        // cut of at the first newline
        let mut message = err.to_string();
        let _ = message.split_off(
            err.to_string()
                .find('\n')
                .unwrap_or_else(|| err.to_string().len()),
        );

        let mut report = Self {
            lvl: Level::Error,
            title: message.clone().into(),
            annotations: Vec::new(),
        };

        // Add annotation with the error message and span if available
        if let Some(span) = err.span() {
            report.add_annotation(
                Level::Error,
                span.start..span.end,
                None::<Cow<'static, str>>,
            );
        }

        report
    }
}

impl Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}
