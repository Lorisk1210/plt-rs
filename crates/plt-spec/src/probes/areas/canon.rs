//! The canonical form of styling values shared by probe areas (see
//! `spec/reference/probes/test_rcsetup.py`): meaning rather than
//! representation, so that a colour is its RGBA and a line style its name
//! or dash pattern.

use plt::animation::{FrameFormat, HtmlRepr};
use plt::axes::{
    Shading, TickDirection, TickLevel, TitleLocation, WhichAxis, XLabelLoc, YLabelLoc,
};
use plt::colors::Color;
use plt::contour::ContourAlgorithm;
use plt::figure::{PageOrientation, Paper, PgfTexSystem, SvgFontType};
use plt::fonts::{FontSize, FontStretch, FontWeight, Fontset, MathFallback, MathFont};
use plt::gui::{MacosxWindowMode, ToolbarKind};
use plt::image::{InterpolationStage, Origin};
use plt::legend::LegendLoc;
use plt::lines::{LineStyle, MarkEvery};
use plt::markers::{FillStyle, Marker, MarkerKey, MarkerSymbol};
use plt::mplot3d::RotationStyle;
use plt::rc::{
    Aspect, AxisBelow, ColorOrAuto, ColorOrEdge, ColorOrInherit, CycleValue, Cycler, Distiller,
    Dpi, FontType, HistBins, LabelColor, LegendLocParam, SavefigBbox, Whiskers,
};
use plt::text::{HAlign, RotationMode, TexEngine, TextHinting, VAlign};
use plt::ticker::{AutolimitMode, MinorDivisions};
use plt::units::DateConverter;
use plt_render::{CapStyle, JoinStyle};

use crate::json::Json;

/// A value in the probes' canonical form.
pub(crate) trait Canon {
    /// The canonical JSON.
    fn canon(&self) -> Json;
}

fn text(value: &str) -> Json {
    Json::String(value.to_owned())
}

impl Canon for bool {
    fn canon(&self) -> Json {
        Json::Bool(*self)
    }
}

impl Canon for f64 {
    fn canon(&self) -> Json {
        Json::Number(*self)
    }
}

impl Canon for i64 {
    fn canon(&self) -> Json {
        Json::Integer(*self)
    }
}

impl Canon for String {
    fn canon(&self) -> Json {
        text(self)
    }
}

impl<T: Canon> Canon for Option<T> {
    fn canon(&self) -> Json {
        self.as_ref().map_or(Json::Null, Canon::canon)
    }
}

impl<T: Canon> Canon for Vec<T> {
    fn canon(&self) -> Json {
        Json::Array(self.iter().map(Canon::canon).collect())
    }
}

impl<T: Canon, const N: usize> Canon for [T; N] {
    fn canon(&self) -> Json {
        Json::Array(self.iter().map(Canon::canon).collect())
    }
}

/// Whether `spec` refers to the property cycle (`"C0"`, `"C12"`).
fn is_cycle_reference(spec: &str) -> bool {
    spec.strip_prefix('C')
        .is_some_and(|digits| !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// `[r, g, b, a]`, or the text of a cycle reference such as `"C3"`.
impl Canon for Color {
    fn canon(&self) -> Json {
        match self {
            Self::Spec(spec) if is_cycle_reference(spec) => text(spec),
            _ => match self.resolve_with(None, &[]) {
                Ok(c) => [c.r, c.g, c.b, c.a].canon(),
                Err(err) => text(&err.to_string()),
            },
        }
    }
}

impl Canon for ColorOrAuto {
    fn canon(&self) -> Json {
        match self {
            Self::Auto => text("auto"),
            Self::Color(color) => color.canon(),
        }
    }
}

impl Canon for ColorOrInherit {
    fn canon(&self) -> Json {
        match self {
            Self::Inherit => text("inherit"),
            Self::Color(color) => color.canon(),
        }
    }
}

impl Canon for ColorOrEdge {
    fn canon(&self) -> Json {
        match self {
            Self::Edge => text("edge"),
            Self::Color(color) => color.canon(),
        }
    }
}

impl Canon for LabelColor {
    fn canon(&self) -> Json {
        match self {
            // `test_rcsetup.py` reads `none`, however spelled, as `None`.
            Self::Unset => Json::Null,
            Self::Color(color) if color.is_none() => Json::Null,
            Self::LineColor => text("linecolor"),
            Self::MarkerFaceColor => text("markerfacecolor"),
            Self::MarkerEdgeColor => text("markeredgecolor"),
            Self::Color(color) => color.canon(),
        }
    }
}

impl Canon for LineStyle {
    fn canon(&self) -> Json {
        match self {
            Self::Solid => text("solid"),
            Self::Dashed => text("dashed"),
            Self::DashDot => text("dashdot"),
            Self::Dotted => text("dotted"),
            Self::None => text("none"),
            Self::Dashes { offset, pattern } => Json::Array(vec![offset.canon(), pattern.canon()]),
        }
    }
}

impl Canon for FontSize {
    fn canon(&self) -> Json {
        match self {
            Self::Relative(size) => text(size.name()),
            Self::Points(points) => points.canon(),
        }
    }
}

/// A keyword, or the number it was given as.
fn named(name: Option<&str>, value: u16) -> Json {
    name.map_or(Json::Integer(i64::from(value)), text)
}

impl Canon for FontWeight {
    fn canon(&self) -> Json {
        named(self.name(), self.value())
    }
}

impl Canon for FontStretch {
    fn canon(&self) -> Json {
        named(self.name(), self.value())
    }
}

/// The marker as matplotlib keeps it: its integer code or its text.
impl Canon for Marker {
    fn canon(&self) -> Json {
        match self {
            Self::Symbol(MarkerSymbol::Nothing) => text("None"),
            Self::Symbol(symbol) => {
                MarkerSymbol::all()
                    .find(|(_, s)| s == symbol)
                    .map_or(Json::Null, |(key, _)| match key {
                        MarkerKey::Char(c) => text(c),
                        MarkerKey::Code(code) => Json::Integer(code),
                    })
            }
            Self::Math(math) => text(math),
            other => text(&format!("{other:?}")),
        }
    }
}

impl Canon for CapStyle {
    fn canon(&self) -> Json {
        text(self.name())
    }
}

/// Keywords as matplotlib keeps them: by name.
macro_rules! canon_by_name {
    ($($ty:ty),+ $(,)?) => {$(
        impl Canon for $ty {
            fn canon(&self) -> Json {
                text(&self.to_string())
            }
        }
    )+};
}

canon_by_name!(
    AutolimitMode,
    ContourAlgorithm,
    DateConverter,
    FillStyle,
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

impl Canon for RotationMode {
    fn canon(&self) -> Json {
        text(match self {
            Self::Default => "default",
            Self::Anchor => "anchor",
            Self::XTick => "xtick",
            Self::YTick => "ytick",
        })
    }
}

impl Canon for JoinStyle {
    fn canon(&self) -> Json {
        text(self.name())
    }
}

impl Canon for FontType {
    fn canon(&self) -> Json {
        Json::Integer(match self {
            Self::Type3 => 3,
            Self::TrueType => 42,
        })
    }
}

impl Canon for Distiller {
    fn canon(&self) -> Json {
        match self {
            Self::Off => Json::Null,
            Self::Ghostscript => text("ghostscript"),
            Self::Xpdf => text("xpdf"),
        }
    }
}

impl Canon for Dpi {
    fn canon(&self) -> Json {
        match self {
            Self::Figure => text("figure"),
            Self::Value(dpi) => dpi.canon(),
        }
    }
}

impl Canon for Aspect {
    fn canon(&self) -> Json {
        match self {
            Self::Equal => text("equal"),
            Self::Auto => text("auto"),
            Self::Ratio(ratio) => ratio.canon(),
        }
    }
}

impl Canon for AxisBelow {
    fn canon(&self) -> Json {
        match self {
            Self::Below => Json::Bool(true),
            Self::Line => text("line"),
            Self::Above => Json::Bool(false),
        }
    }
}

impl Canon for HistBins {
    fn canon(&self) -> Json {
        match self {
            Self::Count(count) => Json::Integer(i64::try_from(*count).unwrap_or(i64::MAX)),
            Self::Rule(rule) => text(rule),
            Self::Edges(edges) => edges.canon(),
        }
    }
}

impl Canon for Whiskers {
    fn canon(&self) -> Json {
        match self {
            Self::Factor(factor) => factor.canon(),
            Self::Percentiles(low, high) => [*low, *high].canon(),
        }
    }
}

/// matplotlib keeps `standard` as `None`.
impl Canon for SavefigBbox {
    fn canon(&self) -> Json {
        match self {
            Self::Standard => Json::Null,
            Self::Tight => text("tight"),
        }
    }
}

/// matplotlib keeps `legend.loc` as given: a name, an integer code or a
/// pair.
impl Canon for LegendLocParam {
    fn canon(&self) -> Json {
        match (self.loc, self.loc.code()) {
            (LegendLoc::Point(x, y), _) => [x, y].canon(),
            (_, Some(code)) if self.by_code => i64::from(code).canon(),
            (loc, _) => text(&loc.to_string()),
        }
    }
}

impl Canon for MinorDivisions {
    fn canon(&self) -> Json {
        match self {
            Self::Auto => text("auto"),
            Self::Count(count) => Json::Integer(i64::from(*count)),
        }
    }
}

impl Canon for CycleValue {
    fn canon(&self) -> Json {
        match self {
            Self::Color(color) => color.canon(),
            Self::Number(number) => number.canon(),
            Self::LineStyle(style) => style.canon(),
            Self::Marker(marker) => marker.canon(),
            Self::Dashes(dashes) => dashes.canon(),
            Self::MarkEvery(markevery) => markevery.canon(),
            Self::Text(value) => text(value),
        }
    }
}

/// `None`, a step, a distance or a list, as Python's `_plain` leaves a
/// `markevery` value (pairs as lists).
impl Canon for Option<MarkEvery> {
    fn canon(&self) -> Json {
        let ints = |values: &[i64]| Json::Array(values.iter().map(|&v| Json::Integer(v)).collect());
        match self {
            None => Json::Null,
            Some(MarkEvery::Every { start: 0, step }) => Json::Integer(*step),
            Some(MarkEvery::Every { start, step }) => ints(&[*start, *step]),
            Some(MarkEvery::Indices(indices)) => ints(indices),
            Some(MarkEvery::Mask(mask)) => {
                Json::Array(mask.iter().map(|&m| Json::Bool(m)).collect())
            }
            Some(MarkEvery::Slice { start, stop, step }) => Json::Array(
                [start, stop, step]
                    .iter()
                    .map(|b| b.map_or(Json::Null, Json::Integer))
                    .collect(),
            ),
            Some(MarkEvery::Distance { start, step }) if *start == 0.0 => step.canon(),
            Some(MarkEvery::Distance { start, step }) => {
                Json::Array(vec![start.canon(), step.canon()])
            }
        }
    }
}

/// `{"cycler": [[property, [values...]], ...]}`, properties sorted.
impl Canon for Cycler {
    fn canon(&self) -> Json {
        let mut keys = self.keys().to_vec();
        keys.sort();
        let properties = keys
            .iter()
            .map(|key| {
                let values = self
                    .values(key)
                    .unwrap_or_default()
                    .into_iter()
                    .map(Canon::canon)
                    .collect();
                Json::Array(vec![text(key), Json::Array(values)])
            })
            .collect();
        Json::Object(vec![("cycler".to_owned(), Json::Array(properties))])
    }
}
