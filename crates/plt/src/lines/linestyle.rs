//! Line styles and dash patterns (matplotlib's `linestyle`).

use std::fmt;
use std::str::FromStr;

use plt_render::Dashes;

use crate::rc::{Expr, RcParams, RcValue};

/// How a line is stroked.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum LineStyle {
    /// A continuous line (`"-"`, `"solid"`).
    #[default]
    Solid,
    /// Dashes (`"--"`, `"dashed"`), pattern `lines.dashed_pattern`.
    Dashed,
    /// Dash-dot (`"-."`, `"dashdot"`), pattern `lines.dashdot_pattern`.
    DashDot,
    /// Dots (`":"`, `"dotted"`), pattern `lines.dotted_pattern`.
    Dotted,
    /// No line at all (`"None"`, `"none"`, `" "`, `""`).
    None,
    /// An explicit on-off sequence in units of the line width.
    Dashes {
        /// Distance into the pattern where drawing starts.
        offset: f64,
        /// Alternating on/off lengths (an even number of entries).
        pattern: Vec<f64>,
    },
}

const NAMED: [(&str, &str, LineStyle); 4] = [
    ("-", "solid", LineStyle::Solid),
    ("--", "dashed", LineStyle::Dashed),
    ("-.", "dashdot", LineStyle::DashDot),
    (":", "dotted", LineStyle::Dotted),
];

impl LineStyle {
    /// An explicit on-off dash sequence.
    pub fn dashes(offset: f64, pattern: Vec<f64>) -> Self {
        Self::Dashes { offset, pattern }
    }

    /// Whether anything is drawn.
    pub fn is_visible(&self) -> bool {
        *self != Self::None
    }

    /// Whether the line is broken into dashes (matplotlib's `is_dashed`).
    pub fn is_dashed(&self) -> bool {
        !matches!(self, Self::Solid | Self::None)
    }

    /// The dash pattern before line-width scaling, with the offset
    /// normalised into the pattern's period (matplotlib's `_get_dash_pattern`).
    pub fn dash_pattern(&self, rc: &RcParams) -> Dashes {
        let (offset, pattern) = match self {
            Self::Solid | Self::None => return Dashes::solid(),
            Self::Dashed => (0.0, rc.lines_dashed_pattern.clone()),
            Self::DashDot => (0.0, rc.lines_dashdot_pattern.clone()),
            Self::Dotted => (0.0, rc.lines_dotted_pattern.clone()),
            Self::Dashes { offset, pattern } => (*offset, pattern.clone()),
        };
        let period: f64 = pattern.iter().sum();
        let offset = if period == 0.0 {
            offset
        } else {
            offset.rem_euclid(period)
        };
        Dashes { offset, pattern }
    }

    /// The dash pattern in points for a line `linewidth` points wide,
    /// scaled when `lines.scale_dashes` is on (matplotlib's `_scale_dashes`).
    pub fn scaled_dashes(&self, linewidth: f64, rc: &RcParams) -> Dashes {
        let dashes = self.dash_pattern(rc);
        if !rc.lines_scale_dashes {
            return dashes;
        }
        Dashes {
            offset: dashes.offset * linewidth,
            pattern: dashes.pattern.iter().map(|len| len * linewidth).collect(),
        }
    }

    /// Interprets a parsed rc/Python value: a style name, or an on-off
    /// sequence `(offset, (on, off, ...))` / `(on, off, ...)`.
    pub(crate) fn from_expr(expr: &Expr) -> Option<Self> {
        match expr {
            Expr::Str(name) => name.parse().ok(),
            other => {
                let items = other.as_sequence()?;
                let (offset, onoff) = match items {
                    [offset, onoff] if onoff.as_sequence().is_some() => {
                        (offset.as_number()?, onoff.as_sequence()?)
                    }
                    _ => (0.0, items),
                };
                let pattern: Vec<f64> = onoff.iter().map(Expr::as_number).collect::<Option<_>>()?;
                pattern
                    .len()
                    .is_multiple_of(2)
                    .then(|| Self::dashes(offset, pattern))
            }
        }
    }
}

/// The inverse of a dash pattern, used to fill the gaps with `gapcolor`
/// (matplotlib's `_get_inverse_dash_pattern`).
pub fn inverse_dashes(dashes: &Dashes) -> Dashes {
    let Some(&last) = dashes.pattern.last() else {
        return dashes.clone();
    };
    let mut pattern = Vec::with_capacity(dashes.pattern.len());
    pattern.push(last);
    pattern.extend_from_slice(&dashes.pattern[..dashes.pattern.len() - 1]);
    Dashes {
        offset: dashes.offset + last,
        pattern,
    }
}

impl FromStr for LineStyle {
    type Err = LineStyleError;

    /// Parses a style name (case-insensitive) or an on-off sequence.
    fn from_str(text: &str) -> Result<Self, LineStyleError> {
        if ["None", " ", ""].contains(&text) || text.eq_ignore_ascii_case("none") {
            return Ok(Self::None);
        }
        let named = NAMED
            .iter()
            .find(|(short, long, _)| *short == text || long.eq_ignore_ascii_case(text))
            .map(|(_, _, style)| style.clone());
        named
            .or_else(|| {
                Expr::parse(text)
                    .ok()
                    .and_then(|expr| Self::from_expr(&expr))
            })
            .ok_or_else(|| LineStyleError(text.to_owned()))
    }
}

impl fmt::Display for LineStyle {
    /// matplotlib's `get_linestyle()` spelling.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::None => f.write_str("None"),
            Self::Dashes { offset, pattern } => {
                let items: Vec<String> = pattern.iter().map(|v| format!("{v:?}")).collect();
                write!(f, "({offset:?}, ({}))", items.join(", "))
            }
            named => {
                let short = NAMED
                    .iter()
                    .find(|(_, _, s)| s == named)
                    .map_or("-", |(short, _, _)| short);
                f.write_str(short)
            }
        }
    }
}

/// An unrecognised line style.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LineStyleError(pub String);

impl fmt::Display for LineStyleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} is not a valid linestyle", self.0)
    }
}

impl std::error::Error for LineStyleError {}

impl RcValue for LineStyle {
    fn parse_rc(text: &str) -> Result<Self, String> {
        text.parse().map_err(|err: LineStyleError| err.to_string())
    }
    fn format_rc(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_and_sequences() {
        assert_eq!("--".parse(), Ok(LineStyle::Dashed));
        assert_eq!("DashDot".parse(), Ok(LineStyle::DashDot));
        assert_eq!("none".parse(), Ok(LineStyle::None));
        assert_eq!(
            "(0, (3, 1))".parse(),
            Ok(LineStyle::dashes(0.0, vec![3.0, 1.0]))
        );
        assert_eq!(
            "(5, 2, 1, 2)".parse(),
            Ok(LineStyle::dashes(0.0, vec![5.0, 2.0, 1.0, 2.0]))
        );
        assert!(
            "(1, 2, 3)".parse::<LineStyle>().is_err(),
            "odd on-off sequences are rejected"
        );
        assert!("wavy".parse::<LineStyle>().is_err());
    }

    #[test]
    fn display_round_trips() {
        for style in ["-", "--", "-.", ":", "None", "(1.0, (3.0, 1.0))"] {
            let parsed: LineStyle = style.parse().expect("valid");
            assert_eq!(parsed.to_string(), style);
        }
    }

    #[test]
    fn dash_patterns_scale_and_normalise() {
        let rc = RcParams::matplotlib();
        assert_eq!(LineStyle::Dashed.dash_pattern(&rc).pattern, vec![3.7, 1.6]);
        let scaled = LineStyle::Dotted.scaled_dashes(2.0, &rc);
        assert_eq!(scaled.pattern, vec![2.0, 3.3]);
        let wrapped = LineStyle::dashes(7.0, vec![2.0, 3.0]).dash_pattern(&rc);
        assert_eq!(wrapped.offset, 2.0);
        let negative = LineStyle::dashes(-1.0, vec![2.0, 3.0]).dash_pattern(&rc);
        assert_eq!(negative.offset, 4.0, "Python modulo keeps offsets positive");
        assert!(LineStyle::Solid.dash_pattern(&rc).is_solid());
    }

    #[test]
    fn inverse_pattern_fills_gaps() {
        let inverse = inverse_dashes(&Dashes {
            offset: 1.0,
            pattern: vec![3.0, 1.0, 2.0, 4.0],
        });
        assert_eq!(inverse.pattern, vec![4.0, 3.0, 1.0, 2.0]);
        assert_eq!(inverse.offset, 5.0);
    }
}
