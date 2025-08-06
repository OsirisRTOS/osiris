use colored::Colorize;
use std::{
    fmt::Display,
    ops::Range,
    path::PathBuf,
};
use thiserror::Error;
use toml_edit::Key;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug, Clone)]
pub struct TomlError {
    /// The error message.
    pub msg: String,
    /// The key that caused the error, if any.
    pub key: Option<Key>,
}

/// This is our global error type which contains all possible errors.
#[derive(Error, Debug)]
pub enum Error<'a> {
    #[error("{0}")]
    InvalidToml(#[from] Report<'a>),
}

/// This struct is supposed to be filled by the functions parent to fill in context information about the error.
/// The function that actually encounters the error either fills in the error and prints the diagnostic
/// or returns a Result to the parent function which then fills in the error into it's own Diagnostic and prints it, etc.
#[derive(Debug)]
pub struct Diagnostic<'a> {
    /// The filename associated with the error.
    filepath: PathBuf,
    /// The content of filename.
    content: Option<&'a str>,
    /// The amount of context lines to show around the error.
    ctx: usize,
}

impl<'a> Diagnostic<'a> {
    pub fn new<P: Into<PathBuf>>(filepath: P, content: Option<&'a str>, ctx: usize) -> Self {
        Self {
            filepath: filepath.into(),
            content,
            ctx,
        }
    }

    pub fn without(&self) -> Report {
        Report {
            diag: self,
            msg: None,
            snippet: None,
        }
    }

    fn extract_snippet(&self, range: Range<usize>) -> Option<(Range<usize>, usize, Range<usize>)> {
        if let Some(content) = self.content {
            // Calculate the line number of the error based on the range.
            let line_num = content[..=range.start].lines().count().saturating_sub(1);
            // Add our context lines around the error.

            let ctx = Range {
                start: line_num.saturating_sub(self.ctx),
                end: line_num.saturating_add(self.ctx),
            };

            // Range should be relative to the start of the error line.
            let range = Range {
                start: range.start
                    - content
                        .lines()
                        .take(line_num)
                        .map(|l| l.len() + 1)
                        .sum::<usize>(),
                end: range.end
                    - content
                        .lines()
                        .take(line_num)
                        .map(|l| l.len() + 1)
                        .sum::<usize>(),
            };

            Some((ctx, line_num - self.ctx, range))
        } else {
            None
        }
    }

    pub fn with_err(&self, err: Error, key: Option<&Key>) -> Report {
        let mut snippet = None;
        // We want to extract the position of the item that caused the error. And then log that line with some surrounding context.
        if let Some(key) = key
            && let Some(range) = key.span()
        {
            snippet = self.extract_snippet(range);
        }

        Report {
            diag: self,
            msg: Some(ReportType::Error(err)),
            snippet,
        }
    }

    pub fn with_warn(&self, warning: &'a str, key: Option<&Key>) -> Report {
        let mut snippet = None;
        // We want to extract the position of the item that caused the warning. And then log that line with some surrounding context.
        if let Some(key) = key
            && let Some(range) = key.span()
        {
            snippet = self.extract_snippet(range);
        }

        Report {
            diag: self,
            msg: Some(ReportType::Warning(warning)),
            snippet,
        }
    }
}

#[derive(Debug)]
enum ReportType<'a> {
    Error(&'a str),
    Warning(&'a str),
}

#[derive(Debug)]
pub struct Report<'a> {
    diag: &'a Diagnostic<'a>,
    msg: Option<ReportType<'a>>,
    /// This is the line num range of the whole snippet, the line number of the error, and the absolute range of the error in the document.
    snippet: Option<(Range<usize>, usize, Range<usize>)>,
}

impl<'a> Display for Report<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (prefix, msg, color) = match &self.msg {
            Some(ReportType::Error(err)) => ("error:".red().bold(), err.to_string(), "red"),
            Some(ReportType::Warning(warning)) => {
                ("warning:".yellow().bold(), warning.to_string(), "yellow")
            }
            None => (
                "note:".white().bold(),
                "No message provided".to_string(),
                "white",
            ),
        };

        let location_arrow = "-->".blue().bold();

        writeln!(f, "{prefix} {msg}")?;
        write!(f, "   {location_arrow} {}", self.diag.filepath.display(),)?;

        if let (Some((ctx_range, err_line_num, err_range)), Some(content)) =
            (&self.snippet, self.diag.content)
        {
            writeln!(f, ":{}", err_line_num + 1)?;

            let pipe = "|".blue().bold();
            writeln!(f, "    {pipe}")?;

            // Extract the snippet lines from the original content using the context range.
            let snippet_lines: Vec<&str> = content
                .lines()
                .skip(ctx_range.start)
                .take(ctx_range.end - ctx_range.start + 1)
                .collect();

            // Print each line of the snippet with its correct line number.
            for (i, line) in snippet_lines.iter().enumerate() {
                writeln!(f, "{:3} {pipe} {line}", line.blue().bold())?;

                if i == *err_line_num {
                    let start_col = err_range.start;
                    let highlight_len = (err_range.end - err_range.start).max(1);

                    // Add padding to align the carets under the error.
                    let padding = " ".repeat(start_col);
                    let carets = "^".repeat(highlight_len);
                    let colored_carets = if color == "red" {
                        carets.red().bold()
                    } else {
                        carets.yellow().bold()
                    };

                    // Print the underline on a new line, aligned with the error.
                    writeln!(f, "    {pipe} {padding}{colored_carets}")?;
                    writeln!(f, "    {pipe}")?;
                }
            }
        }
        writeln!(f)?;

        Ok(())
    }
}
