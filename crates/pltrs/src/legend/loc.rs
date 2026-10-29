//! Where a legend goes (matplotlib's `loc`): parsed from matplotlib's
//! location names and codes where it is given.

use std::fmt;
use std::str::FromStr;

use super::LegendError;
use crate::offsetbox::AnchorLoc;

/// The figure edge an `"outside ..."` figure legend makes room at in
/// constrained layout (matplotlib's `_outside_loc`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OutsideEdge {
    /// Above the axes (`"outside upper ..."`).
    Upper,
    /// Below the axes (`"outside lower ..."`).
    Lower,
    /// Left of the axes (`"outside left ..."`).
    Left,
    /// Right of the axes (`"outside right ..."`).
    Right,
    /// No edge (`"outside center"`): the layout leaves no room.
    Center,
}

impl OutsideEdge {
    /// The edge named by the first word of `words`.
    fn from_word(words: &str) -> Self {
        match words.split_whitespace().next() {
            Some("upper") => Self::Upper,
            Some("lower") => Self::Lower,
            Some("left") => Self::Left,
            Some("right") => Self::Right,
            _ => Self::Center,
        }
    }

    /// matplotlib's word for the edge.
    fn word(self) -> &'static str {
        match self {
            Self::Upper => "upper",
            Self::Lower => "lower",
            Self::Left => "left",
            Self::Right => "right",
            Self::Center => "center",
        }
    }
}

/// Where a legend goes (matplotlib's `loc`): a location name or code, or
/// a point.
///
/// Names parse with [`str::parse`], matplotlib's spelling exactly, and
/// codes with [`LegendLoc::from_code`]; either way an unknown location is
/// refused there, with [`LegendError::InvalidLocation`]. Two rules depend
/// on the legend and are checked when it is made: `"best"` is for axes
/// legends only, and the `"outside ..."` names are for figure legends
/// only.
///
/// ```
/// use pltrs::legend::{LegendLoc, OutsideEdge};
/// use pltrs::offsetbox::AnchorLoc;
///
/// assert_eq!("upper left".parse(), Ok(LegendLoc::Anchored(AnchorLoc::UpperLeft)));
/// assert_eq!(LegendLoc::from_code(0), Ok(LegendLoc::Best));
/// let outside: LegendLoc = "outside right upper".parse()?;
/// assert_eq!(
///     outside,
///     LegendLoc::Outside { edge: OutsideEdge::Right, loc: AnchorLoc::UpperRight }
/// );
/// assert!("top middle".parse::<LegendLoc>().is_err());
/// # Ok::<(), pltrs::legend::LegendError>(())
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LegendLoc {
    /// The least crowded of the anchored positions (`"best"`, code 0);
    /// axes legends only.
    Best,
    /// An anchored position: `"upper right"` (code 1) to `"center"` (code
    /// 10).
    Anchored(AnchorLoc),
    /// A figure legend outside the axes (`"outside upper center"`, ...;
    /// figure legends only): constrained layout makes room for it at
    /// `edge`. The edge comes first where a corner is ambiguous:
    /// `"outside right upper"` makes room at the right, `"outside upper
    /// right"` at the top.
    Outside {
        /// The edge room is made at.
        edge: OutsideEdge,
        /// Where the legend sits in the figure.
        loc: AnchorLoc,
    },
    /// The lower-left corner of the legend, in fractions of the box given
    /// by `bbox_to_anchor` (by default the parent axes or figure).
    Point(f64, f64),
}

impl LegendLoc {
    /// The location with matplotlib's numeric `code`: `0` is `"best"`,
    /// `1` to `10` the anchored positions.
    ///
    /// # Errors
    ///
    /// When `code` is outside `0..=10`.
    pub fn from_code(code: i64) -> Result<Self, LegendError> {
        if code == 0 {
            return Ok(Self::Best);
        }
        AnchorLoc::from_code(code)
            .map(Self::Anchored)
            .ok_or_else(|| LegendError::InvalidLocation(code.to_string()))
    }

    /// matplotlib's numeric code of the location (what its legends keep
    /// as `_loc`): `0` for `"best"`, the anchored position's code for the
    /// others; `None` for a point.
    pub fn code(self) -> Option<u8> {
        match self {
            Self::Best => Some(0),
            Self::Anchored(loc) | Self::Outside { loc, .. } => Some(loc.code()),
            Self::Point(..) => None,
        }
    }

    /// `"outside {words}"`: the location `words` names, the vertical word
    /// last when the edge comes first (`"right upper"` is `"upper
    /// right"`), and the edge its first word other than a leading
    /// `"center"` names.
    fn outside(words: &str) -> Option<Self> {
        let edge = OutsideEdge::from_word(&words.replace("center ", ""));
        let mut parts: Vec<&str> = words.split_whitespace().collect();
        if parts.len() > 1 && matches!(parts[0], "right" | "left") {
            parts.reverse();
            parts.truncate(2);
        }
        let loc = parts.join(" ").parse().ok()?;
        Some(Self::Outside { edge, loc })
    }
}

impl FromStr for LegendLoc {
    type Err = LegendError;

    /// Parses `"best"`, an anchored position's name or an `"outside ..."`
    /// name, in matplotlib's spelling.
    fn from_str(name: &str) -> Result<Self, LegendError> {
        let parsed = match name.strip_prefix("outside ") {
            Some(words) => Self::outside(words),
            None if name == "best" => Some(Self::Best),
            None => name.parse().ok().map(Self::Anchored),
        };
        parsed.ok_or_else(|| LegendError::InvalidLocation(format!("{name:?}")))
    }
}

impl fmt::Display for LegendLoc {
    /// matplotlib's name of the location, or the point as `(x, y)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Best => f.write_str("best"),
            Self::Anchored(loc) => f.write_str(loc.name()),
            Self::Outside { edge, loc } => {
                let name = loc.name();
                match (edge, name.split_once(' ')) {
                    (OutsideEdge::Left | OutsideEdge::Right, Some((vertical, _)))
                        if vertical != "center" =>
                    {
                        write!(f, "outside {} {vertical}", edge.word())
                    }
                    _ => write!(f, "outside {name}"),
                }
            }
            Self::Point(x, y) => write!(f, "({x}, {y})"),
        }
    }
}

impl From<AnchorLoc> for LegendLoc {
    fn from(loc: AnchorLoc) -> Self {
        Self::Anchored(loc)
    }
}

impl From<(f64, f64)> for LegendLoc {
    fn from((x, y): (f64, f64)) -> Self {
        Self::Point(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outside(edge: OutsideEdge, loc: AnchorLoc) -> LegendLoc {
        LegendLoc::Outside { edge, loc }
    }

    #[test]
    fn names_and_codes_parse_like_matplotlib() {
        assert_eq!("best".parse(), Ok(LegendLoc::Best));
        assert_eq!("center".parse(), Ok(LegendLoc::Anchored(AnchorLoc::Center)));
        assert_eq!(
            LegendLoc::from_code(6),
            Ok(LegendLoc::Anchored(AnchorLoc::CenterLeft))
        );
        assert_eq!(LegendLoc::from_code(6).unwrap().code(), Some(6));
        assert_eq!(LegendLoc::Point(0.2, 0.3).code(), None);
        assert!("Upper Left".parse::<LegendLoc>().is_err());
        assert_eq!(
            LegendLoc::from_code(11),
            Err(LegendError::InvalidLocation("11".into()))
        );
        assert!(LegendLoc::from_code(-1).is_err());
    }

    #[test]
    fn outside_locations_name_an_edge() {
        for (name, edge, loc) in [
            (
                "outside upper center",
                OutsideEdge::Upper,
                AnchorLoc::UpperCenter,
            ),
            (
                "outside right upper",
                OutsideEdge::Right,
                AnchorLoc::UpperRight,
            ),
            (
                "outside center left",
                OutsideEdge::Left,
                AnchorLoc::CenterLeft,
            ),
            (
                "outside lower right",
                OutsideEdge::Lower,
                AnchorLoc::LowerRight,
            ),
            ("outside right", OutsideEdge::Right, AnchorLoc::Right),
            ("outside center", OutsideEdge::Center, AnchorLoc::Center),
        ] {
            assert_eq!(name.parse(), Ok(outside(edge, loc)), "{name}");
            assert_eq!(outside(edge, loc).to_string(), name);
        }
        assert_eq!(
            "outside left center".parse(),
            Ok(outside(OutsideEdge::Left, AnchorLoc::CenterLeft))
        );
        for refused in ["outside", "outside best", "outside upper", "outside left"] {
            assert_eq!(
                refused.parse::<LegendLoc>(),
                Err(LegendError::InvalidLocation(format!("{refused:?}")))
            );
        }
    }

    #[test]
    fn locations_display_as_matplotlib_names() {
        assert_eq!(LegendLoc::Best.to_string(), "best");
        assert_eq!(
            LegendLoc::from(AnchorLoc::LowerCenter).to_string(),
            "lower center"
        );
        assert_eq!(LegendLoc::from((0.5, 1.0)).to_string(), "(0.5, 1)");
    }
}
