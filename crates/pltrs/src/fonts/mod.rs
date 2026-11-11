//! Fonts (matplotlib's `font_manager`): the vocabularies of font sizes,
//! weights and stretches, [`FontProperties`] and the [`FontManager`] that
//! finds font files for them, among the fonts whose catalogue is kept in
//! the cache directory ([`keep_catalogue`]).

mod catalogue;
mod manager;
mod named_sizes;
mod properties;

use std::fmt;
use std::str::FromStr;

pub use catalogue::{catalogue_report, keep_catalogue};
pub use manager::{FontExt, FontListEntry, FontManager};
pub use named_sizes::{NamedSize, set_size_keywords};
pub use pltrs_text::fontconfig::FontconfigPattern;
pub use pltrs_text::mathtext::{Fontset, MathFallback, MathFont};
pub use pltrs_text::{CatalogueReport, CatalogueState};
pub use pltrs_text::{FontStyle, FontVariant};
pub use properties::{FontProperties, parse_math_fontfamily, parse_style, parse_variant};

use crate::rc::RcValue;

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

pltrs_core::archive_enum!(RelativeSize {
    XxSmall,
    XSmall,
    Small,
    Medium,
    Large,
    XLarge,
    XxLarge,
    Larger,
    Smaller,
});

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
    /// A keyword the program added (`font_manager.font_scalings`).
    Named(NamedSize),
}

pltrs_core::archive_enum!(FontSize {
    Relative(v0),
    Points(v0),
    Named(v0),
});

impl FontSize {
    /// The size in points given the base size (`font.size`).
    pub fn to_points(self, base: f64) -> f64 {
        match self {
            Self::Relative(size) => base * size.scale(),
            Self::Points(points) => points,
            Self::Named(size) => base * size.scale(),
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

    /// Parses a number of points (Python's `float`) or a keyword
    /// (case-sensitive, as `FontProperties.set_size` requires): one of
    /// matplotlib's, else one the program added.
    fn from_str(text: &str) -> Result<Self, FontPropertyError> {
        if let Ok(points) = text.trim().parse() {
            return Ok(Self::Points(points));
        }
        RELATIVE_SIZES
            .iter()
            .find(|(name, _, _)| *name == text)
            .map(|(_, size, _)| Self::Relative(*size))
            .or_else(|| NamedSize::find(text).map(Self::Named))
            .ok_or_else(|| FontPropertyError::Size(text.to_owned()))
    }
}

impl fmt::Display for FontSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Relative(size) => f.write_str(size.name()),
            Self::Points(points) => write!(f, "{points:?}"),
            Self::Named(size) => f.write_str(size.name()),
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

            /// A numeric value, which must lie in 0–1000.
            pub fn from_number(value: i64) -> Result<Self, FontPropertyError> {
                u16::try_from(value)
                    .ok()
                    .filter(|&value| value <= 1000)
                    .map(Self::numeric)
                    .ok_or_else(|| FontPropertyError::$error(value.to_string()))
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

            /// Parses a keyword or an integer in 0–1000 (Python's `int`).
            fn from_str(text: &str) -> Result<Self, FontPropertyError> {
                if let Some(named) = Self::from_name(text) {
                    return Ok(named);
                }
                text.trim()
                    .parse()
                    .map_err(|_| FontPropertyError::$error(text.to_owned()))
                    .and_then(Self::from_number)
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

        /// Archived as its keyword or number.
        impl pltrs_core::archive::Archive for $name {
            fn encode(
                &self,
                encoder: &mut pltrs_core::archive::Encoder<'_>,
            ) -> Result<(), pltrs_core::archive::ArchiveError> {
                encoder.write_str(&self.to_string());
                Ok(())
            }

            fn decode(
                decoder: &mut pltrs_core::archive::Decoder<'_>,
            ) -> Result<Self, pltrs_core::archive::ArchiveError> {
                let text = decoder.read_str()?;
                text.parse()
                    .map_err(|err: FontPropertyError| pltrs_core::archive::ArchiveError::invalid(err.to_string()))
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
        ("extra-extended", 800),
        ("ultra-expanded", 900),
        ("ultra-extended", 900),
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

/// An invalid font property value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontPropertyError {
    /// Not a size keyword or number.
    Size(String),
    /// Not a weight keyword or number.
    Weight(String),
    /// Not a stretch keyword or number.
    Stretch(String),
    /// Not `normal`, `italic` or `oblique`.
    Style(String),
    /// Not `normal` or `small-caps`.
    Variant(String),
    /// Not a mathtext fontset.
    MathFontFamily(String),
    /// A malformed fontconfig pattern.
    Pattern(String),
    /// A fontconfig pattern property `FontProperties` has no setter for.
    PatternKey(String),
}

impl fmt::Display for FontPropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Size(text) => write!(f, "{text:?} is not a valid font size"),
            Self::Weight(text) => write!(f, "{text:?} is not a valid font weight"),
            Self::Stretch(text) => write!(f, "{text:?} is not a valid font stretch"),
            Self::Style(text) => write!(f, "{text:?} is not a valid font style"),
            Self::Variant(text) => write!(f, "{text:?} is not a valid font variant"),
            Self::MathFontFamily(text) => {
                write!(f, "{text:?} is not a valid math font family")
            }
            Self::Pattern(why) => f.write_str(why),
            Self::PatternKey(key) => {
                write!(f, "font properties have no {key:?} to set from a pattern")
            }
        }
    }
}

impl std::error::Error for FontPropertyError {}

impl RcValue for FontSize {
    /// rc files spell size keywords case-insensitively.
    fn parse_rc(text: &str) -> Result<Self, String> {
        text.to_ascii_lowercase()
            .parse()
            .map_err(|err: FontPropertyError| err.to_string())
    }

    fn format_rc(&self) -> String {
        self.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(
            "large".parse::<FontSize>().map(|s| s.to_points(10.0)),
            Ok(12.0)
        );
        assert_eq!(
            "xx-small".parse::<FontSize>().map(|s| s.to_points(10.0)),
            Ok(0.579 * 10.0)
        );
        assert_eq!("13.5".parse::<FontSize>(), Ok(FontSize::Points(13.5)));
        assert_eq!(" 1e1 ".parse::<FontSize>(), Ok(FontSize::Points(10.0)));
        assert!("huge".parse::<FontSize>().is_err());
        assert!("Small".parse::<FontSize>().is_err());
        assert_eq!(
            FontSize::parse_rc("Small"),
            Ok(FontSize::Relative(RelativeSize::Small))
        );
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
        assert!("1001".parse::<FontWeight>().is_err());
        assert!("-1".parse::<FontWeight>().is_err());
        assert!("bold ".parse::<FontWeight>().is_err());
        assert_eq!(FontWeight::from_number(0).map(FontWeight::value), Ok(0));
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
