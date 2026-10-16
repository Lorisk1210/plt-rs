//! MATLAB-style format strings such as `"ro--"` (matplotlib's
//! `_process_plot_format`).

use std::fmt;
use std::str::FromStr;

use super::{LineProps, LineStyle};
use crate::colors::{Color, is_color_like};
use crate::markers::Marker;
use crate::rc::RcParams;

/// Single-character marker codes allowed in format strings.
const MARKERS: &str = ".,ov^<>12348sp*hH+xDd|_PX";
/// Single-character colour codes allowed in format strings.
const COLORS: &str = "bgrcmykw";

/// The line style, marker and colour a format string selects.
///
/// A format that is a plain colour (`"red"`, `"0.5"`, `"C3"`) sets only the
/// colour. Otherwise the format is absolute: a part it leaves out is
/// turned off, except that a format with neither line style nor marker
/// draws `lines.linestyle`.
///
/// ```
/// use plt::lines::{FormatString, LineStyle};
/// let fmt: FormatString = "ro--".parse()?;
/// assert_eq!(fmt.linestyle, Some(LineStyle::Dashed));
/// assert_eq!(fmt.color, Some("r".into()));
/// # Ok::<(), plt::lines::FormatError>(())
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FormatString {
    /// The line style (`None` = unchanged).
    pub linestyle: Option<LineStyle>,
    /// The marker (`None` = unchanged).
    pub marker: Option<Marker>,
    /// The colour (`None` = from the property cycle).
    pub color: Option<Color>,
}

/// Why a format string is invalid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatError {
    /// The format string.
    pub format: String,
    /// What is wrong with it.
    pub reason: String,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} is not a valid format string ({})",
            self.format, self.reason
        )
    }
}

impl std::error::Error for FormatError {}

impl FormatString {
    /// Parses `format`, using `rc` for the default line style.
    ///
    /// # Errors
    ///
    /// When `format` contains an unknown character or repeats a part.
    pub fn parse_with(format: &str, rc: &RcParams) -> Result<Self, FormatError> {
        // "0" and "1" are grey levels but read as markers/digits here.
        if !["0", "1"].contains(&format) && is_color_like(format) {
            return Ok(Self {
                color: Some(Color::from(format)),
                ..Self::default()
            });
        }
        let error = |reason: &str| FormatError {
            format: format.to_owned(),
            reason: reason.to_owned(),
        };
        let mut linestyle: Option<LineStyle> = None;
        let mut marker: Option<Marker> = None;
        let mut color: Option<Color> = None;
        let mut rest = format;
        while let Some(c) = rest.chars().next() {
            let two = rest.get(..2).filter(|s| ["--", "-."].contains(s));
            if let Some(style) = two.or_else(|| "-: ".contains(c).then(|| &rest[..1])) {
                if linestyle.is_some() {
                    return Err(error("two linestyle symbols"));
                }
                linestyle = Some(style.parse().map_err(|_| error("bad linestyle"))?);
                rest = &rest[style.len()..];
            } else if MARKERS.contains(c) {
                if marker.is_some() {
                    return Err(error("two marker symbols"));
                }
                marker = Some(rest[..1].parse().map_err(|_| error("bad marker"))?);
                rest = &rest[1..];
            } else if COLORS.contains(c) {
                if color.is_some() {
                    return Err(error("two color symbols"));
                }
                color = Some(Color::from(&rest[..1]));
                rest = &rest[1..];
            } else if c == 'C' {
                let digits = rest[1..].bytes().take_while(u8::is_ascii_digit).count();
                if digits == 0 {
                    return Err(error("'C' must be followed by a number"));
                }
                color = Some(Color::from(&rest[..=digits]));
                rest = &rest[1 + digits..];
            } else {
                return Err(error(&format!("unrecognized character {c:?}")));
            }
        }
        if linestyle.is_none() && marker.is_none() {
            linestyle = Some(rc.lines_linestyle.clone());
        }
        Ok(Self {
            linestyle: Some(linestyle.unwrap_or(LineStyle::None)),
            marker: Some(marker.unwrap_or_default()),
            color,
        })
    }
}

impl FromStr for FormatString {
    type Err = FormatError;

    /// Parses with the current rc parameters.
    fn from_str(format: &str) -> Result<Self, FormatError> {
        Self::parse_with(format, &crate::rc::params())
    }
}

impl From<FormatString> for LineProps {
    fn from(format: FormatString) -> Self {
        Self {
            linestyle: format.linestyle,
            marker: format.marker,
            color: format.color,
            ..Self::default()
        }
    }
}

impl LineProps {
    /// The properties a format string such as `"ro--"` selects.
    ///
    /// # Errors
    ///
    /// When the format string is invalid.
    pub fn fmt(format: &str) -> Result<Self, FormatError> {
        format.parse::<FormatString>().map(Self::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markers::MarkerSymbol;

    fn parse(format: &str) -> Result<FormatString, FormatError> {
        FormatString::parse_with(format, &RcParams::matplotlib())
    }

    #[test]
    fn combines_colour_marker_and_style() {
        let fmt = parse("ro--").unwrap();
        assert_eq!(fmt.color, Some(Color::from("r")));
        assert_eq!(fmt.marker, Some(Marker::Symbol(MarkerSymbol::Circle)));
        assert_eq!(fmt.linestyle, Some(LineStyle::Dashed));
        let fmt = parse("C7x-").unwrap();
        assert_eq!(fmt.color, Some(Color::from("C7")));
        assert_eq!(fmt.linestyle, Some(LineStyle::Solid));
    }

    #[test]
    fn missing_parts_are_turned_off() {
        let markers_only = parse("g^").unwrap();
        assert_eq!(markers_only.linestyle, Some(LineStyle::None));
        let line_only = parse("b-.").unwrap();
        assert_eq!(line_only.marker, Some(Marker::default()));
        assert_eq!(line_only.linestyle, Some(LineStyle::DashDot));
    }

    #[test]
    fn plain_colours_set_only_the_colour() {
        for format in ["red", "r", "0.5", "C3", "#ff0000"] {
            let fmt = parse(format).unwrap();
            assert_eq!((fmt.linestyle, fmt.marker), (None, None), "{format}");
        }
        assert_eq!(
            parse("1").unwrap().marker,
            Some(Marker::Symbol(MarkerSymbol::TriDown))
        );
    }

    #[test]
    fn rejects_bad_formats() {
        assert!(parse("--:").is_err(), "two line styles");
        assert!(parse("os").is_err(), "two markers");
        assert!(parse("rg").is_err(), "two colours");
        assert!(parse("Cx").is_err(), "C without digits");
        assert!(parse("q").is_err());
    }
}
