//! Reading rc files and style sheets (`key: value  # comment` lines).

use super::RcError;

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

/// Parses rc-file text into its assignments, mirroring matplotlib's
/// `_rc_params_in_file`: `#` starts a comment unless quoted, blank lines
/// are skipped and a value wrapped in double quotes is unwrapped.
pub fn parse_rc_text(text: &str) -> Result<Vec<RcEntry>, RcError> {
    let mut entries = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line.split_once(':').ok_or_else(|| RcError::Syntax {
            line: index + 1,
            text: raw.to_owned(),
        })?;
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or(value);
        entries.push(RcEntry {
            line: index + 1,
            key: key.trim().to_owned(),
            value: value.to_owned(),
        });
    }
    Ok(entries)
}

/// Removes a trailing `#` comment that is not inside quotes.
fn strip_comment(line: &str) -> &str {
    let mut quote = None;
    for (i, c) in line.char_indices() {
        match (c, quote) {
            ('"' | '\'', None) => quote = Some(c),
            (c, Some(open)) if c == open => quote = None,
            ('#', None) => return &line[..i],
            _ => {}
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_style_sheet_lines() {
        let text = "# heading\n\naxes.facecolor: E5E5E5  # grey\nfont.family: \"DejaVu Sans\"\naxes.prop_cycle: cycler('color', ['#E24A33'])\n";
        let entries = parse_rc_text(text).expect("valid");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].value, "E5E5E5");
        assert_eq!(entries[0].line, 3);
        assert_eq!(entries[1].value, "DejaVu Sans");
        assert_eq!(
            entries[2].value, "cycler('color', ['#E24A33'])",
            "quoted '#' is kept"
        );
    }

    #[test]
    fn rejects_lines_without_colon() {
        assert!(matches!(
            parse_rc_text("oops"),
            Err(RcError::Syntax { line: 1, .. })
        ));
    }
}
