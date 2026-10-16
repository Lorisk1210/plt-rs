//! Font property values: sizes, weights and stretches
//! (the vocabularies of matplotlib's `font_manager`).

use std::fmt;
use std::str::FromStr;

use crate::rc::{RcParams, RcValue};

/// A relative font size keyword, scaled against `font.size`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelativeSize {
    /// `xx-small` (×0.579).
    XxSmall,
    /// `x-small` (×0.694).
    XSmall,
    /// `small` (×0.833).
    Small,
    /// `medium` (×1.0).
    Medium,
    /// `large` (×1.2).
    Large,
    /// `x-large` (×1.44).
    XLarge,
    /// `xx-large` (×1.728).
    XxLarge,
    /// `larger` (×1.2).
    Larger,
    /// `smaller` (×0.833).
    Smaller,
}

const RELATIVE_SIZES: [(&str, RelativeSize, f64); 9] = [
    ("xx-small", RelativeSize::XxSmall, 0.579),
    ("x-small", RelativeSize::XSmall, 0.694),
    ("small", RelativeSize::Small, 0.833),
    ("medium", RelativeSize::Medium, 1.0),
    ("large", RelativeSize::Large, 1.2),
    ("x-large", RelativeSize::XLarge, 1.44),
    ("xx-large", RelativeSize::XxLarge, 1.728),
    ("larger", RelativeSize::Larger, 1.2),
    ("smaller", RelativeSize::Smaller, 0.833),
];

impl RelativeSize {
    fn entry(self) -> (&'static str, f64) {
        RELATIVE_SIZES
            .iter()
            .find(|(_, size, _)| *size == self)
            .map(|(name, _, scale)| (*name, *scale))
            .expect("every relative size is in the table")
    }

    /// matplotlib's keyword.
    pub fn name(self) -> &'static str {
        self.entry().0
    }

    /// The factor applied to the base font size.
    pub fn scale(self) -> f64 {
        self.entry().1
    }
}

/// A font size: points, or a keyword relative to `font.size`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FontSize {
    /// A relative keyword such as `large`.
    Relative(RelativeSize),
    /// An absolute size in points.
    Points(f64),
}

impl FontSize {
    /// The size in points given the base size (`font.size`).
    pub fn to_points(self, base: f64) -> f64 {
        match self {
            Self::Relative(size) => base * size.scale(),
            Self::Points(points) => points,
        }
    }
}

impl From<f64> for FontSize {
    fn from(points: f64) -> Self {
        Self::Points(points)
    }
}

impl From<RelativeSize> for FontSize {
    fn from(size: RelativeSize) -> Self {
        Self::Relative(size)
    }
}

impl FromStr for FontSize {
    type Err = FontPropertyError;

    /// Parses a keyword (case-insensitive) or a number of points.
    fn from_str(text: &str) -> Result<Self, FontPropertyError> {
        let lower = text.to_ascii_lowercase();
        if let Some((_, size, _)) = RELATIVE_SIZES.iter().find(|(name, _, _)| *name == lower) {
            return Ok(Self::Relative(*size));
        }
        text.trim()
            .parse()
            .map(Self::Points)
            .map_err(|_| FontPropertyError::Size(text.to_owned()))
    }
}

impl fmt::Display for FontSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Relative(size) => f.write_str(size.name()),
            Self::Points(points) => write!(f, "{points:?}"),
        }
    }
}

/// Declares a numeric font property that also has CSS-style names.
macro_rules! named_scale {
    ($(#[$doc:meta])* $name:ident, $error:ident, [$(($keyword:literal, $value:literal)),+ $(,)?]) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub struct $name {
            value: u16,
            keyword: Option<&'static str>,
        }

        impl $name {
            const NAMES: &'static [(&'static str, u16)] = &[$(($keyword, $value)),+];

            /// A numeric value (100–900 for standard fonts).
            pub const fn numeric(value: u16) -> Self {
                Self { value, keyword: None }
            }

            /// Looks up a keyword such as `"bold"` (case-sensitive, as in matplotlib).
            pub fn from_name(name: &str) -> Option<Self> {
                Self::NAMES
                    .iter()
                    .find(|(keyword, _)| *keyword == name)
                    .map(|&(keyword, value)| Self { value, keyword: Some(keyword) })
            }

            /// The numeric value used for font matching.
            pub const fn value(self) -> u16 {
                self.value
            }

            /// The keyword it was given as, if any.
            pub const fn name(self) -> Option<&'static str> {
                self.keyword
            }
        }

        impl FromStr for $name {
            type Err = FontPropertyError;

            fn from_str(text: &str) -> Result<Self, FontPropertyError> {
                Self::from_name(text)
                    .or_else(|| text.trim().parse().ok().map(Self::numeric))
                    .ok_or_else(|| FontPropertyError::$error(text.to_owned()))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.keyword {
                    Some(keyword) => f.write_str(keyword),
                    None => write!(f, "{}", self.value),
                }
            }
        }
    };
}

named_scale!(
    /// Font weight: `normal` (400), `bold` (700), ... or a number.
    FontWeight,
    Weight,
    [
        ("ultralight", 100),
        ("light", 200),
        ("normal", 400),
        ("regular", 400),
        ("book", 400),
        ("medium", 500),
        ("roman", 500),
        ("semibold", 600),
        ("demibold", 600),
        ("demi", 600),
        ("bold", 700),
        ("heavy", 800),
        ("extra bold", 800),
        ("black", 900),
    ]
);

named_scale!(
    /// Font stretch: `normal` (500), `condensed` (300), ... or a number.
    FontStretch,
    Stretch,
    [
        ("ultra-condensed", 100),
        ("extra-condensed", 200),
        ("condensed", 300),
        ("semi-condensed", 400),
        ("normal", 500),
        ("semi-expanded", 600),
        ("semi-extended", 600),
        ("expanded", 700),
        ("extended", 700),
        ("extra-expanded", 800),
        ("ultra-expanded", 900),
    ]
);

impl FontWeight {
    /// `normal` (400).
    pub const NORMAL: Self = Self {
        value: 400,
        keyword: Some("normal"),
    };
    /// `bold` (700).
    pub const BOLD: Self = Self {
        value: 700,
        keyword: Some("bold"),
    };
}

impl FontStretch {
    /// `normal` (500).
    pub const NORMAL: Self = Self {
        value: 500,
        keyword: Some("normal"),
    };
}

/// A complete font description (matplotlib's `FontProperties`) with the
/// size resolved to points.
#[derive(Clone, Debug, PartialEq)]
pub struct FontProperties {
    /// Family names or generic families, in priority order.
    pub families: Vec<String>,
    /// Upright, italic or oblique.
    pub style: plt_text::FontStyle,
    /// Normal or small caps.
    pub variant: plt_text::FontVariant,
    /// Weight.
    pub weight: FontWeight,
    /// Stretch.
    pub stretch: FontStretch,
    /// Size in points.
    pub size: f64,
}

impl FontProperties {
    /// The defaults from `font.*` rcParams.
    pub fn from_rc(rc: &RcParams) -> Self {
        Self {
            families: rc.font_family.clone(),
            style: plt_text::FontStyle::parse(&rc.font_style)
                .unwrap_or(plt_text::FontStyle::Normal),
            variant: if rc.font_variant == "small-caps" {
                plt_text::FontVariant::SmallCaps
            } else {
                plt_text::FontVariant::Normal
            },
            weight: rc.font_weight,
            stretch: rc.font_stretch,
            size: rc.font_size,
        }
    }

    /// The same properties at another size, given relative to `font.size`.
    #[must_use]
    pub fn with_size(self, size: FontSize, base: f64) -> Self {
        Self {
            size: size.to_points(base),
            ..self
        }
    }

    /// The same properties with another weight.
    #[must_use]
    pub fn with_weight(self, weight: FontWeight) -> Self {
        Self { weight, ..self }
    }

    /// The font request used to pick font files, resolving generic family
    /// names through the `font.serif`, `font.sans-serif`, ... lists of `rc`.
    pub fn request(&self, rc: &RcParams) -> plt_text::FontRequest {
        let mut query = plt_text::FontQuery::families(self.families.iter().cloned());
        query.style = self.style;
        query.variant = self.variant;
        query.weight = plt_text::Weight(self.weight.value());
        query.stretch = plt_text::Stretch(self.stretch.value());
        query.size = self.size;
        plt_text::FontRequest {
            query,
            generics: plt_text::GenericFamilies {
                serif: rc.font_serif.clone(),
                sans_serif: rc.font_sans_serif.clone(),
                cursive: rc.font_cursive.clone(),
                fantasy: rc.font_fantasy.clone(),
                monospace: rc.font_monospace.clone(),
            },
        }
    }
}

/// An invalid font property value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontPropertyError {
    /// Not a size keyword or number.
    Size(String),
    /// Not a weight keyword or number.
    Weight(String),
    /// Not a stretch keyword or number.
    Stretch(String),
}

impl fmt::Display for FontPropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Size(text) => write!(f, "{text:?} is not a valid font size"),
            Self::Weight(text) => write!(f, "{text:?} is not a valid font weight"),
            Self::Stretch(text) => write!(f, "{text:?} is not a valid font stretch"),
        }
    }
}

impl std::error::Error for FontPropertyError {}

/// rc values reuse the `FromStr`/`Display` pair of each property.
macro_rules! rc_via_str {
    ($($ty:ty),+) => {$(
        impl RcValue for $ty {
            fn parse_rc(text: &str) -> Result<Self, String> {
                text.parse().map_err(|err: FontPropertyError| err.to_string())
            }
            fn format_rc(&self) -> String {
                self.to_string()
            }
        }
    )+};
}

rc_via_str!(FontSize, FontWeight, FontStretch);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(
            "Large".parse::<FontSize>().map(|s| s.to_points(10.0)),
            Ok(12.0)
        );
        assert_eq!(
            "xx-small".parse::<FontSize>().map(|s| s.to_points(10.0)),
            Ok(0.579 * 10.0)
        );
        assert_eq!("13.5".parse::<FontSize>(), Ok(FontSize::Points(13.5)));
        assert!("huge".parse::<FontSize>().is_err());
        assert_eq!(
            FontSize::Relative(RelativeSize::XLarge).to_string(),
            "x-large"
        );
    }

    #[test]
    fn weights_keep_their_names() {
        let bold: FontWeight = "bold".parse().expect("valid");
        assert_eq!((bold.value(), bold.name()), (700, Some("bold")));
        assert_eq!("600".parse::<FontWeight>().map(FontWeight::value), Ok(600));
        assert!(
            "Bold".parse::<FontWeight>().is_err(),
            "names are case-sensitive"
        );
        assert_eq!(
            "extra bold".parse::<FontWeight>().map(|w| w.to_string()),
            Ok("extra bold".into())
        );
    }

    #[test]
    fn stretches() {
        assert_eq!(
            "condensed".parse::<FontStretch>().map(FontStretch::value),
            Ok(300)
        );
        assert_eq!(FontStretch::NORMAL.to_string(), "normal");
    }
}
