//! Marker specifications: what users ask for (`"o"`, `(5, 1, 30.0)`, ...).

use std::fmt;
use std::str::FromStr;

use plt_core::{Path, Point};

/// The named markers matplotlib accepts as single characters or integers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkerSymbol {
    /// `"."`
    Point,
    /// `","` — a single pixel.
    Pixel,
    /// `"o"`
    Circle,
    /// `"v"`
    TriangleDown,
    /// `"^"`
    TriangleUp,
    /// `"<"`
    TriangleLeft,
    /// `">"`
    TriangleRight,
    /// `"1"`
    TriDown,
    /// `"2"`
    TriUp,
    /// `"3"`
    TriLeft,
    /// `"4"`
    TriRight,
    /// `"8"`
    Octagon,
    /// `"s"`
    Square,
    /// `"p"`
    Pentagon,
    /// `"*"`
    Star,
    /// `"h"`
    Hexagon1,
    /// `"H"`
    Hexagon2,
    /// `"+"`
    Plus,
    /// `"x"`
    X,
    /// `"D"`
    Diamond,
    /// `"d"`
    ThinDiamond,
    /// `"|"`
    VLine,
    /// `"_"`
    HLine,
    /// `"P"`
    PlusFilled,
    /// `"X"`
    XFilled,
    /// `0`
    TickLeft,
    /// `1`
    TickRight,
    /// `2`
    TickUp,
    /// `3`
    TickDown,
    /// `4`
    CaretLeft,
    /// `5`
    CaretRight,
    /// `6`
    CaretUp,
    /// `7`
    CaretDown,
    /// `8`
    CaretLeftBase,
    /// `9`
    CaretRightBase,
    /// `10`
    CaretUpBase,
    /// `11`
    CaretDownBase,
    /// `"None"`, `"none"`, `" "`, `""` — no marker.
    Nothing,
}

/// Character codes in matplotlib's `MarkerStyle.markers` order.
const CHAR_CODES: [(&str, MarkerSymbol, &str); 25] = [
    (".", MarkerSymbol::Point, "point"),
    (",", MarkerSymbol::Pixel, "pixel"),
    ("o", MarkerSymbol::Circle, "circle"),
    ("v", MarkerSymbol::TriangleDown, "triangle_down"),
    ("^", MarkerSymbol::TriangleUp, "triangle_up"),
    ("<", MarkerSymbol::TriangleLeft, "triangle_left"),
    (">", MarkerSymbol::TriangleRight, "triangle_right"),
    ("1", MarkerSymbol::TriDown, "tri_down"),
    ("2", MarkerSymbol::TriUp, "tri_up"),
    ("3", MarkerSymbol::TriLeft, "tri_left"),
    ("4", MarkerSymbol::TriRight, "tri_right"),
    ("8", MarkerSymbol::Octagon, "octagon"),
    ("s", MarkerSymbol::Square, "square"),
    ("p", MarkerSymbol::Pentagon, "pentagon"),
    ("*", MarkerSymbol::Star, "star"),
    ("h", MarkerSymbol::Hexagon1, "hexagon1"),
    ("H", MarkerSymbol::Hexagon2, "hexagon2"),
    ("+", MarkerSymbol::Plus, "plus"),
    ("x", MarkerSymbol::X, "x"),
    ("D", MarkerSymbol::Diamond, "diamond"),
    ("d", MarkerSymbol::ThinDiamond, "thin_diamond"),
    ("|", MarkerSymbol::VLine, "vline"),
    ("_", MarkerSymbol::HLine, "hline"),
    ("P", MarkerSymbol::PlusFilled, "plus_filled"),
    ("X", MarkerSymbol::XFilled, "x_filled"),
];

/// Integer codes `0..=11` in order.
const INT_CODES: [(MarkerSymbol, &str); 12] = [
    (MarkerSymbol::TickLeft, "tickleft"),
    (MarkerSymbol::TickRight, "tickright"),
    (MarkerSymbol::TickUp, "tickup"),
    (MarkerSymbol::TickDown, "tickdown"),
    (MarkerSymbol::CaretLeft, "caretleft"),
    (MarkerSymbol::CaretRight, "caretright"),
    (MarkerSymbol::CaretUp, "caretup"),
    (MarkerSymbol::CaretDown, "caretdown"),
    (MarkerSymbol::CaretLeftBase, "caretleftbase"),
    (MarkerSymbol::CaretRightBase, "caretrightbase"),
    (MarkerSymbol::CaretUpBase, "caretupbase"),
    (MarkerSymbol::CaretDownBase, "caretdownbase"),
];

/// The spellings of "no marker".
const NOTHING_CODES: [&str; 4] = ["None", "none", " ", ""];

impl MarkerSymbol {
    /// The symbol for matplotlib's integer marker codes `0..=11`.
    pub fn from_code(code: i64) -> Option<Self> {
        usize::try_from(code)
            .ok()
            .and_then(|i| INT_CODES.get(i))
            .map(|(symbol, _)| *symbol)
    }

    /// matplotlib's descriptive name (`"circle"`, `"caretdown"`, ...).
    pub fn name(self) -> &'static str {
        if self == Self::Nothing {
            return "nothing";
        }
        CHAR_CODES
            .iter()
            .map(|(_, s, name)| (*s, *name))
            .chain(INT_CODES.iter().copied())
            .find(|(s, _)| *s == self)
            .map_or("nothing", |(_, name)| name)
    }

    /// Every symbol in matplotlib's `MarkerStyle.markers` order with the key
    /// matplotlib uses for it (a character or an integer code).
    pub fn all() -> impl Iterator<Item = (MarkerKey, Self)> {
        let chars = CHAR_CODES.iter().map(|(c, s, _)| (MarkerKey::Char(c), *s));
        let ints = INT_CODES
            .iter()
            .zip(0..)
            .map(|((s, _), i)| (MarkerKey::Code(i), *s));
        let nothing = NOTHING_CODES
            .iter()
            .map(|c| (MarkerKey::Char(c), Self::Nothing));
        chars.chain(ints).chain(nothing)
    }
}

/// How matplotlib spells a named marker: a string or an integer code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerKey {
    /// A character code such as `"o"`.
    Char(&'static str),
    /// An integer code `0..=11`.
    Code(i64),
}

/// How a regular polygon marker is drawn (second element of a marker tuple).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PolygonStyle {
    /// A filled regular polygon (`0`).
    Regular,
    /// A filled star (`1`).
    Star,
    /// An unfilled asterisk (`2`).
    Asterisk,
}

impl PolygonStyle {
    /// Parses the numeric style code used in marker tuples.
    pub fn from_code(code: i64) -> Option<Self> {
        Some(match code {
            0 => Self::Regular,
            1 => Self::Star,
            2 => Self::Asterisk,
            _ => return None,
        })
    }
}

/// A marker specification, matplotlib's `marker=` argument.
#[derive(Clone, Debug, PartialEq)]
pub enum Marker {
    /// A named marker.
    Symbol(MarkerSymbol),
    /// A regular polygon, star or asterisk with `sides` vertices, rotated by
    /// `angle` degrees (`(numsides, style, angle)` tuples).
    Polygon {
        /// Number of vertices.
        sides: usize,
        /// Polygon, star or asterisk.
        style: PolygonStyle,
        /// Rotation in degrees.
        angle: f64,
    },
    /// A closed polygon through the given vertices, normalised to fit.
    Vertices(Vec<Point>),
    /// An arbitrary path, normalised to fit.
    Path(Path),
    /// Mathtext such as `"$\clubsuit$"`, drawn as its outline centred and
    /// scaled to fit.
    Math(String),
}

impl Default for Marker {
    fn default() -> Self {
        Self::Symbol(MarkerSymbol::Nothing)
    }
}

impl From<MarkerSymbol> for Marker {
    fn from(symbol: MarkerSymbol) -> Self {
        Self::Symbol(symbol)
    }
}

impl From<Path> for Marker {
    fn from(path: Path) -> Self {
        Self::Path(path)
    }
}

impl Marker {
    /// Parses matplotlib's integer marker codes `0..=11`.
    pub fn from_code(code: i64) -> Result<Self, MarkerError> {
        MarkerSymbol::from_code(code)
            .map(Self::Symbol)
            .ok_or_else(|| MarkerError::Unrecognized(code.to_string()))
    }

    /// A `(numsides, style, angle)` polygon marker.
    pub fn polygon(sides: usize, style: PolygonStyle, angle: f64) -> Self {
        Self::Polygon {
            sides,
            style,
            angle,
        }
    }

    /// Whether this is the "no marker" marker.
    pub fn is_nothing(&self) -> bool {
        matches!(self, Self::Symbol(MarkerSymbol::Nothing))
    }
}

impl FromStr for Marker {
    type Err = MarkerError;

    /// Parses matplotlib's string marker codes (`"o"`, `"^"`, `"None"`,
    /// ...) and mathtext markers (`"$f$"`).
    fn from_str(code: &str) -> Result<Self, MarkerError> {
        if NOTHING_CODES.contains(&code) {
            return Ok(Self::Symbol(MarkerSymbol::Nothing));
        }
        if plt_text::mathtext::is_math_text(code) {
            return Ok(Self::Math(code.to_owned()));
        }
        CHAR_CODES
            .iter()
            .find(|(c, _, _)| *c == code)
            .map(|(_, symbol, _)| Self::Symbol(*symbol))
            .ok_or_else(|| MarkerError::Unrecognized(code.to_owned()))
    }
}

/// An invalid marker specification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MarkerError {
    /// Not a marker matplotlib knows.
    Unrecognized(String),
    /// Not one of `full`, `left`, `right`, `bottom`, `top`, `none`.
    InvalidFillStyle(String),
    /// A polygon marker needs at least one vertex.
    NoSides,
    /// A vertex or path marker without any finite extent.
    EmptyShape,
    /// A mathtext marker that cannot be typeset (the reason).
    Math(String),
}

impl fmt::Display for MarkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unrecognized(code) => write!(f, "unrecognized marker style {code:?}"),
            Self::InvalidFillStyle(style) => write!(
                f,
                "{style:?} is not a valid fillstyle; use full, left, right, bottom, top or none"
            ),
            Self::NoSides => f.write_str("polygon markers need at least one side"),
            Self::EmptyShape => f.write_str("marker shape has no extent"),
            Self::Math(why) => write!(f, "invalid mathtext marker: {why}"),
        }
    }
}

impl std::error::Error for MarkerError {}

/// Which part of a filled marker uses the face colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FillStyle {
    /// The whole marker.
    #[default]
    Full,
    /// The left half (the right half uses the alternate face colour).
    Left,
    /// The right half.
    Right,
    /// The bottom half.
    Bottom,
    /// The top half.
    Top,
    /// No fill: only the outline is drawn.
    None,
}

impl FillStyle {
    /// matplotlib's name for the fill style.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Left => "left",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Top => "top",
            Self::None => "none",
        }
    }

    /// Whether only half of the marker uses the face colour.
    pub const fn is_half(self) -> bool {
        matches!(self, Self::Left | Self::Right | Self::Bottom | Self::Top)
    }
}

impl fmt::Display for FillStyle {
    /// matplotlib's name ([`FillStyle::name`]).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for FillStyle {
    type Err = MarkerError;

    fn from_str(name: &str) -> Result<Self, MarkerError> {
        Ok(match name {
            "full" => Self::Full,
            "left" => Self::Left,
            "right" => Self::Right,
            "bottom" => Self::Bottom,
            "top" => Self::Top,
            "none" => Self::None,
            other => return Err(MarkerError::InvalidFillStyle(other.to_owned())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_character_and_integer_codes() {
        assert_eq!("o".parse(), Ok(Marker::Symbol(MarkerSymbol::Circle)));
        assert_eq!("None".parse(), Ok(Marker::Symbol(MarkerSymbol::Nothing)));
        assert_eq!(
            Marker::from_code(7),
            Ok(Marker::Symbol(MarkerSymbol::CaretDown))
        );
        assert!(
            "0".parse::<Marker>().is_err(),
            "integer codes are not strings"
        );
        assert!("oo".parse::<Marker>().is_err());
        assert!(Marker::from_code(12).is_err());
        assert!(Marker::from_code(-1).is_err());
    }

    #[test]
    fn names_follow_matplotlib() {
        assert_eq!(MarkerSymbol::ThinDiamond.name(), "thin_diamond");
        assert_eq!(MarkerSymbol::CaretUpBase.name(), "caretupbase");
        assert_eq!(MarkerSymbol::Nothing.name(), "nothing");
        assert_eq!(MarkerSymbol::all().count(), 41);
    }

    #[test]
    fn fill_styles() {
        assert_eq!("left".parse(), Ok(FillStyle::Left));
        assert!(
            "Full".parse::<FillStyle>().is_err(),
            "names are case-sensitive"
        );
        assert!(FillStyle::Top.is_half());
        assert!(!FillStyle::None.is_half());
        assert_eq!(FillStyle::Bottom.to_string(), "bottom");
    }
}
