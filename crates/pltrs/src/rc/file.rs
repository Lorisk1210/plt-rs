//! Reading rc files and style sheets (`key: value  # comment` lines).
//!
//! [`parse_rc_text`] reads trusted text (the bundled style sheets) and
//! stops at the first malformed line; [`RcFile`] reads files the way
//! matplotlib's `rc_params_from_file` does, skipping what it cannot use
//! and reporting it as [`RcWarning`]s.

use std::fmt;
use std::path::{Path, PathBuf};

use super::{RcError, RcParams};

/// One `key: value` assignment from an rc file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RcEntry {
    /// 1-based line number.
    pub line: usize,
    /// Parameter name.
    pub key: String,
    /// Value text (comment stripped, surrounding double quotes removed).
    pub value: String,
}

/// `line` without its comment, trimmed (matplotlib's
/// `cbook._strip_comment`): the comment starts at the first `#` outside
/// double quotes; single quotes do not protect a `#`.
///
/// ```
/// use pltrs::rc::strip_comment;
/// assert_eq!(strip_comment(r##"a : "#000000" # black"##), Ok(r##"a : "#000000""##));
/// assert_eq!(strip_comment("# only a comment"), Ok(""));
/// assert!(strip_comment(r#"grid.color: "aa"#).is_err());
/// ```
///
/// # Errors
///
/// [`RcError::UnclosedQuote`] when a double quote is not closed.
pub fn strip_comment(line: &str) -> Result<&str, RcError> {
    let mut in_quotes = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_quotes = !in_quotes,
            '#' if !in_quotes => return Ok(line[..i].trim()),
            _ => {}
        }
    }
    if in_quotes {
        return Err(RcError::UnclosedQuote(line.to_owned()));
    }
    Ok(line.trim())
}

/// What one line of rc text holds.
enum Line {
    /// Nothing but white space and comments.
    Blank,
    /// An assignment.
    Entry(RcEntry),
    /// Text without the `:` separating key and value.
    MissingColon,
}

/// The lines of rc text with their 1-based numbers and raw text, as
/// matplotlib's `_rc_params_in_file` reads them.
fn scan(text: &str) -> Result<Vec<(usize, &str, Line)>, RcError> {
    text.lines()
        .enumerate()
        .map(|(index, raw)| {
            let number = index + 1;
            let line = strip_comment(raw).map_err(|err| RcError::AtLine {
                line: number,
                source: Box::new(err),
            })?;
            let scanned = if line.is_empty() {
                Line::Blank
            } else if let Some((key, value)) = line.split_once(':') {
                let value = value.trim();
                let value = value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .unwrap_or(value);
                Line::Entry(RcEntry {
                    line: number,
                    key: key.trim().to_owned(),
                    value: value.to_owned(),
                })
            } else {
                Line::MissingColon
            };
            Ok((number, raw, scanned))
        })
        .collect()
}

/// Parses rc-file text into its assignments, mirroring matplotlib's
/// `_rc_params_in_file`: `#` starts a comment unless inside double quotes,
/// blank lines are skipped and a value wrapped in double quotes is
/// unwrapped. Unlike matplotlib, a line without a colon is an error.
pub fn parse_rc_text(text: &str) -> Result<Vec<RcEntry>, RcError> {
    scan(text)?
        .into_iter()
        .filter_map(|(line, raw, scanned)| match scanned {
            Line::Blank => None,
            Line::Entry(entry) => Some(Ok(entry)),
            Line::MissingColon => Some(Err(RcError::Syntax {
                line,
                text: raw.to_owned(),
            })),
        })
        .collect()
}

/// Why matplotlib skips a line of an rc file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RcWarningKind {
    /// The line has no `:` between key and value.
    MissingColon,
    /// The key was set before; the last value wins.
    DuplicateKey,
    /// No parameter has this name.
    UnknownKey(String),
    /// The value failed validation, for this reason.
    InvalidValue(String),
}

/// A line of an rc file that matplotlib reports and skips (or, for a
/// duplicate key, overrides).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RcWarning {
    /// The file, when the text came from one.
    pub path: Option<PathBuf>,
    /// 1-based line number.
    pub line: usize,
    /// The line as written.
    pub text: String,
    /// What is wrong with it.
    pub kind: RcWarningKind,
}

impl fmt::Display for RcWarning {
    /// matplotlib's log messages.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = self
            .path
            .as_ref()
            .map_or_else(|| "<text>".to_owned(), |path| path.display().to_string());
        let (line, text) = (self.line, &self.text);
        match &self.kind {
            RcWarningKind::MissingColon => {
                write!(f, "Missing colon in file {file:?}, line {line} ({text:?})")
            }
            RcWarningKind::DuplicateKey => {
                write!(f, "Duplicate key in file {file:?}, line {line} ({text:?})")
            }
            RcWarningKind::UnknownKey(key) => {
                write!(f, "Bad key {key} in file {file}, line {line} ({text:?})")
            }
            RcWarningKind::InvalidValue(reason) => write!(
                f,
                "Bad value in file {file:?}, line {line} ({text:?}): {reason}"
            ),
        }
    }
}

/// The valid settings of an rc file and the lines skipped on the way
/// (matplotlib's `rc_params_from_file(fname, use_default_template=False)`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RcFile {
    /// The file read, if any.
    pub path: Option<PathBuf>,
    /// The valid assignments, each key once with its last value, in the
    /// order the keys first appear.
    pub entries: Vec<RcEntry>,
    /// The lines skipped or overridden.
    pub warnings: Vec<RcWarning>,
}

impl RcFile {
    /// Reads rc text leniently: lines without a colon, unknown keys and
    /// invalid values are skipped with a warning, and a repeated key keeps
    /// its last value.
    ///
    /// ```
    /// use pltrs::rc::{RcFile, RcWarningKind};
    /// let file = RcFile::parse("lines.linewidth: 2\nfoo: bar\nlines.linewidth: 3\n")?;
    /// assert_eq!(file.entries.len(), 1);
    /// assert_eq!(file.entries[0].value, "3");
    /// assert_eq!(file.warnings[0].kind, RcWarningKind::DuplicateKey);
    /// assert_eq!(file.warnings[1].kind, RcWarningKind::UnknownKey("foo".into()));
    /// # Ok::<(), pltrs::rc::RcError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`RcError::AtLine`] with [`RcError::UnclosedQuote`] when a double
    /// quote is not closed, which matplotlib does not skip either.
    pub fn parse(text: &str) -> Result<Self, RcError> {
        let mut file = Self::default();
        let mut assigned: Vec<(RcEntry, &str)> = Vec::new();
        for (line, raw, scanned) in scan(text)? {
            let warn = |kind| RcWarning {
                path: None,
                line,
                text: raw.to_owned(),
                kind,
            };
            match scanned {
                Line::Blank => {}
                Line::MissingColon => file.warnings.push(warn(RcWarningKind::MissingColon)),
                Line::Entry(entry) => {
                    match assigned.iter_mut().find(|(seen, _)| seen.key == entry.key) {
                        Some(slot) => {
                            file.warnings.push(warn(RcWarningKind::DuplicateKey));
                            *slot = (entry, raw);
                        }
                        None => assigned.push((entry, raw)),
                    }
                }
            }
        }
        let mut scratch = RcParams::matplotlib();
        for (entry, raw) in assigned {
            let kind = match scratch.set(&entry.key, &entry.value) {
                Ok(()) => {
                    file.entries.push(entry);
                    continue;
                }
                Err(RcError::UnknownKey(key)) => RcWarningKind::UnknownKey(key),
                Err(err) => RcWarningKind::InvalidValue(err.to_string()),
            };
            file.warnings.push(RcWarning {
                path: None,
                line: entry.line,
                text: raw.to_owned(),
                kind,
            });
        }
        Ok(file)
    }

    /// Reads the rc file at `path` (a leading `~` stands for the home
    /// directory) as [`RcFile::parse`] does.
    ///
    /// # Errors
    ///
    /// [`RcError::Unreadable`] when the file cannot be read as UTF-8 text,
    /// and the errors of [`RcFile::parse`].
    pub fn read(path: impl AsRef<Path>) -> Result<Self, RcError> {
        let path = expand_user(path.as_ref());
        let text = std::fs::read_to_string(&path).map_err(|err| RcError::Unreadable {
            path: path.clone(),
            reason: err.to_string(),
        })?;
        let mut file = Self::parse(&text)?;
        for warning in &mut file.warnings {
            warning.path = Some(path.clone());
        }
        file.path = Some(path);
        Ok(file)
    }
}

/// `path` with a leading `~` replaced by the home directory (Python's
/// `os.path.expanduser`).
fn expand_user(path: &Path) -> PathBuf {
    match (path.strip_prefix("~"), std::env::var_os("HOME")) {
        (Ok(rest), Some(home)) => Path::new(&home).join(rest),
        _ => path.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_style_sheet_lines() {
        let text = "# heading\n\naxes.facecolor: E5E5E5  # grey\nfont.family: \"DejaVu Sans\"\naxes.prop_cycle: cycler('color', ['E24A33'])\n";
        let entries = parse_rc_text(text).expect("valid");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].value, "E5E5E5");
        assert_eq!(entries[0].line, 3);
        assert_eq!(entries[1].value, "DejaVu Sans");
        assert_eq!(entries[2].value, "cycler('color', ['E24A33'])");
    }

    #[test]
    fn rejects_lines_without_colon() {
        assert!(matches!(
            parse_rc_text("oops"),
            Err(RcError::Syntax { line: 1, .. })
        ));
    }

    #[test]
    fn comments_start_outside_double_quotes_only() {
        assert_eq!(strip_comment("a : no_comment"), Ok("a : no_comment"));
        assert_eq!(
            strip_comment(r#"a : "quoted str" # comment"#),
            Ok(r#"a : "quoted str""#)
        );
        assert_eq!(
            strip_comment(r##"a : ["#000000", "#FFFFFF"] # comment"##),
            Ok(r##"a : ["#000000", "#FFFFFF"]"##)
        );
        assert_eq!(
            strip_comment(r#"a : val  # a comment "with quotes""#),
            Ok("a : val")
        );
        assert_eq!(
            strip_comment("a: cycler('color', ['#E24A33'])"),
            Ok("a: cycler('color', ['"),
            "single quotes do not protect a #"
        );
        assert_eq!(
            strip_comment(r#"grid.color: "aa"#),
            Err(RcError::UnclosedQuote(r#"grid.color: "aa"#.into()))
        );
        assert!(matches!(
            parse_rc_text("a: 1\nb: \"x\n"),
            Err(RcError::AtLine { line: 2, .. })
        ));
    }

    #[test]
    fn files_skip_what_they_cannot_use() {
        let file = RcFile::parse(
            "oops\nlines.linewidth: wide\nimage.cmap: pink\nfoo: bar\nimage.cmap: gray\n",
        )
        .expect("only quotes are fatal");
        assert_eq!(file.entries.len(), 1);
        assert_eq!(
            (file.entries[0].line, file.entries[0].value.as_str()),
            (5, "gray"),
            "the last value, at its own line"
        );
        let kinds: Vec<(usize, &RcWarningKind)> = file
            .warnings
            .iter()
            .map(|warning| (warning.line, &warning.kind))
            .collect();
        // Reading problems in line order, then validation problems in the
        // order the keys first appear, as matplotlib logs them.
        assert_eq!(kinds[0], (1, &RcWarningKind::MissingColon));
        assert_eq!(kinds[1], (5, &RcWarningKind::DuplicateKey));
        assert!(matches!(kinds[2], (2, RcWarningKind::InvalidValue(_))));
        assert_eq!(kinds[3], (4, &RcWarningKind::UnknownKey("foo".into())));
        assert!(
            file.warnings[3]
                .to_string()
                .starts_with("Bad key foo in file <text>, line 4")
        );
    }

    #[test]
    fn reading_names_the_file() {
        let dir = std::env::temp_dir().join(format!("pltrs-rc-file-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temporary directory");
        let path = dir.join("bad.mplstyle");
        std::fs::write(&path, "foo: bar\n").expect("writable");
        let file = RcFile::read(&path).expect("readable");
        assert_eq!(file.warnings[0].path.as_deref(), Some(path.as_path()));
        assert!(file.warnings[0].to_string().contains("bad.mplstyle"));
        std::fs::remove_dir_all(&dir).expect("removable");
        assert!(matches!(
            RcFile::read(&path),
            Err(RcError::Unreadable { .. })
        ));
    }
}
