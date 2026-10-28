//! rcParams whose values are types that read their text with `FromStr` and
//! write it with `Display`: the keyword types of matplotlib's
//! `ValidateInStrings` parameters (the table's `validate::choice` picks
//! the spellings each parameter accepts), the font weight and stretch, and
//! the fonts of `mathtext.fallback`.

use plt_render::{PageOrientation, Paper, SvgFontType};
use plt_text::mathtext::{Fontset, MathFallback, MathFont};
use plt_text::tex::TexEngine;
use plt_text::{HAlign, VAlign};

use super::value::RcValue;
use crate::animation::{FrameFormat, HtmlRepr};
use crate::axes::{
    Shading, TickDirection, TickLevel, TitleLocation, WhichAxis, XLabelLoc, YLabelLoc,
};
use crate::contour::ContourAlgorithm;
use crate::figure::PgfTexSystem;
use crate::fonts::{FontStretch, FontWeight};
use crate::gui::{MacosxWindowMode, ToolbarKind};
use crate::image::{InterpolationStage, Origin};
use crate::markers::FillStyle;
use crate::mplot3d::RotationStyle;
use crate::text::TextHinting;
use crate::ticker::AutolimitMode;
use crate::units::DateConverter;

/// A name that a keyword type does not have: what parsing a keyword type
/// returns, such as an [`Interpolation`](crate::image::Interpolation),
/// an [`Origin`](crate::image::Origin), a
/// [`TickLevel`](crate::axes::TickLevel) or a
/// [`PgfTexSystem`](crate::figure::PgfTexSystem).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownName {
    /// What was being parsed (`"interpolation"`, ...).
    pub kind: &'static str,
    /// The unknown name.
    pub name: String,
}

impl std::fmt::Display for UnknownName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} is not a valid {}", self.name, self.kind)
    }
}

impl std::error::Error for UnknownName {}

/// Implements `Display` (the value's `name()`) and `FromStr` (the value of
/// `$all` with that name, else an [`UnknownName`] of `$kind`) for a
/// keyword type: the names matplotlib reads and writes. With
/// `ignore_case`, names parse in any ASCII case, as matplotlib's
/// case-insensitive validators read them.
///
macro_rules! keyword_names {
    ($ty:ty, $kind:literal, $all:expr) => {
        $crate::rc::keyword_names!(@impl $ty, $kind, $all, |a: &str, b: &str| a == b);
    };
    ($ty:ty, $kind:literal, $all:expr, ignore_case) => {
        $crate::rc::keyword_names!(@impl $ty, $kind, $all, str::eq_ignore_ascii_case);
    };
    (@impl $ty:ty, $kind:literal, $all:expr, $same:expr) => {
        impl ::std::fmt::Display for $ty {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.name())
            }
        }

        impl ::std::str::FromStr for $ty {
            type Err = $crate::rc::UnknownName;

            /// Parses matplotlib's name.
            fn from_str(name: &str) -> Result<Self, Self::Err> {
                $all.into_iter()
                    .find(|value| $same(value.name(), name))
                    .ok_or_else(|| $crate::rc::UnknownName {
                        kind: $kind,
                        name: name.to_owned(),
                    })
            }
        }
    };
}

pub(crate) use keyword_names;

/// rc values reuse the `FromStr`/`Display` pair of each type.
macro_rules! rc_via_str {
    ($($ty:ty),+ $(,)?) => {$(
        impl RcValue for $ty {
            fn parse_rc(text: &str) -> Result<Self, String> {
                text.parse::<Self>().map_err(|err| err.to_string())
            }
            fn format_rc(&self) -> String {
                self.to_string()
            }
        }
    )+};
}

rc_via_str!(
    AutolimitMode,
    ContourAlgorithm,
    DateConverter,
    FillStyle,
    FontStretch,
    FontWeight,
    Fontset,
    FrameFormat,
    HAlign,
    HtmlRepr,
    InterpolationStage,
    MacosxWindowMode,
    MathFallback,
    MathFont,
    Origin,
    PageOrientation,
    Paper,
    PgfTexSystem,
    RotationStyle,
    Shading,
    SvgFontType,
    TexEngine,
    TextHinting,
    TickDirection,
    TickLevel,
    TitleLocation,
    ToolbarKind,
    VAlign,
    WhichAxis,
    XLabelLoc,
    YLabelLoc,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_read_and_write_their_names() {
        assert_eq!(FillStyle::parse_rc("left"), Ok(FillStyle::Left));
        assert_eq!(FillStyle::Left.format_rc(), "left");
        assert_eq!(
            Paper::parse_rc("A4").map(|paper| paper.format_rc()),
            Ok("a4".to_owned())
        );
        assert!(TextHinting::parse_rc("hinted").is_err());
    }
}
