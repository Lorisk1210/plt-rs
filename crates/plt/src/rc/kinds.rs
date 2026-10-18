//! Typed values of individual rcParams with matplotlib's parsing rules.

use super::literal::Expr;
use super::value::{RcValue, split_list};
use crate::colors::Color;
use crate::legend::LegendLoc;
use crate::ticker::MinorDivisions;

/// A colour written as a name, `"none"` or hex code, which may omit the
/// `#` when it has six or eight digits (matplotlib's `validate_color`
/// before it evaluates tuple strings).
fn color_spec(text: &str) -> Option<Color> {
    if text.eq_ignore_ascii_case("none") {
        return Some(Color::Spec("none".to_owned()));
    }
    if matches!(text.len(), 6 | 8) {
        let hashed = format!("#{text}");
        if Color::parse(&hashed).is_ok() {
            return Some(Color::Spec(hashed));
        }
    }
    Color::parse(text).ok()
}

impl RcValue for Color {
    /// matplotlib's `validate_color`: `none` is normalised, six- or
    /// eight-digit hex may omit the `#`, and tuple strings are evaluated.
    fn parse_rc(text: &str) -> Result<Self, String> {
        if let Some(color) = color_spec(text) {
            return Ok(color);
        }
        let tuple = Expr::parse(text)
            .ok()
            .and_then(|expr| expr.as_sequence().map(<[Expr]>::to_vec))
            .and_then(|items| {
                items
                    .iter()
                    .map(Expr::as_number)
                    .collect::<Option<Vec<_>>>()
            });
        let color = match tuple.as_deref() {
            Some(&[r, g, b]) => Self::from((r, g, b)),
            Some(&[r, g, b, a]) => Self::from((r, g, b, a)),
            _ => return Err(format!("{text:?} does not look like a color arg")),
        };
        color
            .resolve_with(None, &[])
            .map(|_| color)
            .map_err(|err| err.to_string())
    }

    fn format_rc(&self) -> String {
        match self {
            Self::Spec(spec) => spec.clone(),
            Self::Rgba(c) => format!("({}, {}, {}, {})", c.r, c.g, c.b, c.a),
        }
    }
}

/// Declares an rc colour setting that also accepts keywords.
macro_rules! color_with_keywords {
    ($(#[$doc:meta])* $name:ident { $($(#[$vdoc:meta])* $variant:ident => [$($spelling:literal),+]),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq)]
        pub enum $name {
            $( $(#[$vdoc])* $variant, )+
            /// An explicit colour.
            Color(Color),
        }

        impl RcValue for $name {
            /// The keywords exactly as spelled (matplotlib's `_str_equal`),
            /// otherwise a colour.
            fn parse_rc(text: &str) -> Result<Self, String> {
                $( if [$($spelling),+].contains(&text) {
                    return Ok(Self::$variant);
                } )+
                Color::parse_rc(text).map(Self::Color)
            }

            fn format_rc(&self) -> String {
                match self {
                    $( Self::$variant => [$($spelling),+][0].to_owned(), )+
                    Self::Color(color) => color.format_rc(),
                }
            }
        }
    };
}

color_with_keywords! {
    /// A colour, or `auto` (derive it from another property).
    ColorOrAuto {
        /// Derived automatically.
        Auto => ["auto"],
    }
}

impl<T: Into<Color>> From<T> for ColorOrAuto {
    /// `"auto"` selects [`ColorOrAuto::Auto`]; anything else is a colour.
    fn from(value: T) -> Self {
        match value.into() {
            Color::Spec(spec) if spec == "auto" => Self::Auto,
            color => Self::Color(color),
        }
    }
}

impl ColorOrAuto {
    /// The explicit colour, or `fallback` when automatic.
    pub fn or<'a>(&'a self, fallback: &'a Color) -> &'a Color {
        match self {
            Self::Auto => fallback,
            Self::Color(color) => color,
        }
    }
}

color_with_keywords! {
    /// A colour, or `inherit` (use the parent's colour).
    ColorOrInherit {
        /// Inherited from the parent setting.
        Inherit => ["inherit"],
    }
}

color_with_keywords! {
    /// A colour, or `edge` (use the patch edge colour), for hatches.
    ColorOrEdge {
        /// The patch's edge colour.
        Edge => ["edge"],
    }
}

/// The colour of legend labels.
#[derive(Clone, Debug, PartialEq)]
pub enum LabelColor {
    /// Use the text colour (`None`).
    Unset,
    /// The colour of the corresponding line.
    LineColor,
    /// The marker face colour.
    MarkerFaceColor,
    /// The marker edge colour.
    MarkerEdgeColor,
    /// An explicit colour.
    Color(Color),
}

impl RcValue for LabelColor {
    /// matplotlib's `_validate_color_or_linecolor`: the keywords exactly
    /// as spelled, `none` in any case for the text colour, otherwise a
    /// colour by name or hex code (tuple strings are not evaluated).
    fn parse_rc(text: &str) -> Result<Self, String> {
        Ok(match text {
            "linecolor" => Self::LineColor,
            "markerfacecolor" | "mfc" => Self::MarkerFaceColor,
            "markeredgecolor" | "mec" => Self::MarkerEdgeColor,
            _ if text.eq_ignore_ascii_case("none") => Self::Unset,
            _ => Self::Color(
                color_spec(text)
                    .ok_or_else(|| format!("{text:?} does not look like a color arg"))?,
            ),
        })
    }

    fn format_rc(&self) -> String {
        match self {
            Self::Unset => "None".to_owned(),
            Self::LineColor => "linecolor".to_owned(),
            Self::MarkerFaceColor => "markerfacecolor".to_owned(),
            Self::MarkerEdgeColor => "markeredgecolor".to_owned(),
            Self::Color(color) => color.format_rc(),
        }
    }
}

/// `savefig.bbox`: the area `savefig` saves.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SavefigBbox {
    /// The whole figure (`standard`, which matplotlib stores as `None`).
    #[default]
    Standard,
    /// The tight box around what is drawn (`tight`).
    Tight,
}

impl RcValue for SavefigBbox {
    /// matplotlib's `validate_bbox`: `tight` or `standard`, in any case.
    fn parse_rc(text: &str) -> Result<Self, String> {
        match text.to_ascii_lowercase().as_str() {
            "tight" => Ok(Self::Tight),
            "standard" => Ok(Self::Standard),
            _ => Err(format!(
                "bbox should be 'tight' or 'standard', not {text:?}"
            )),
        }
    }

    fn format_rc(&self) -> String {
        match self {
            Self::Standard => "standard",
            Self::Tight => "tight",
        }
        .to_owned()
    }
}

/// The location names `legend.loc` takes (matplotlib's
/// `_validate_named_legend_loc`).
const LEGEND_LOCATIONS: [&str; 11] = [
    "best",
    "upper right",
    "upper left",
    "lower left",
    "lower right",
    "right",
    "center left",
    "center right",
    "lower center",
    "upper center",
    "center",
];

impl RcValue for LegendLoc {
    /// matplotlib's `_validate_legend_loc`: a location name in any case, a
    /// code from 0 to 10, or an `(x, y)` tuple.
    fn parse_rc(text: &str) -> Result<Self, String> {
        if let Some(name) = LEGEND_LOCATIONS
            .iter()
            .find(|name| name.eq_ignore_ascii_case(text))
        {
            return Ok(Self::Name((*name).to_owned()));
        }
        match Expr::parse(text) {
            Ok(Expr::Int(code)) if (0..=10).contains(&code) => Ok(Self::Code(code)),
            Ok(Expr::Tuple(items)) => match items.as_slice() {
                [x, y] => match (x.as_number(), y.as_number()) {
                    (Some(x), Some(y)) => Ok(Self::Point(x, y)),
                    _ => Err(format!("{text} is not a valid legend location")),
                },
                _ => Err(format!("{text} is not a valid legend location")),
            },
            _ => Err(format!("{text} is not a valid legend location")),
        }
    }

    fn format_rc(&self) -> String {
        match self {
            Self::Name(name) => name.clone(),
            Self::Code(code) => code.to_string(),
            Self::Point(x, y) => format!("({}, {})", x.format_rc(), y.format_rc()),
        }
    }
}

impl RcValue for MinorDivisions {
    /// matplotlib's `_validate_minor_tick_ndivs`: `auto` in any case or a
    /// non-negative integer.
    fn parse_rc(text: &str) -> Result<Self, String> {
        if text.eq_ignore_ascii_case("auto") {
            return Ok(Self::Auto);
        }
        i64::parse_rc(text)
            .ok()
            .and_then(|count| u32::try_from(count).ok())
            .map(Self::Count)
            .ok_or_else(|| "'tick.minor.ndivs' must be 'auto' or non-negative int".to_owned())
    }

    fn format_rc(&self) -> String {
        match self {
            Self::Auto => "auto".to_owned(),
            Self::Count(count) => count.to_string(),
        }
    }
}

/// Output font embedding for PDF/PostScript (`pdf.fonttype`, `ps.fonttype`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontType {
    /// Type 3 fonts: glyphs drawn as procedures (`3` / `type3`).
    Type3,
    /// TrueType (Type 42) embedding (`42` / `truetype`).
    TrueType,
}

impl RcValue for FontType {
    fn parse_rc(text: &str) -> Result<Self, String> {
        match text.to_ascii_lowercase().as_str() {
            "3" | "type3" => Ok(Self::Type3),
            "42" | "truetype" => Ok(Self::TrueType),
            _ => Err(format!(
                "supported fonttypes are 3 (type3) and 42 (truetype), not {text:?}"
            )),
        }
    }
    fn format_rc(&self) -> String {
        match self {
            Self::Type3 => "3",
            Self::TrueType => "42",
        }
        .to_owned()
    }
}

/// `ps.usedistiller`: post-processing of PostScript output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Distiller {
    /// No distilling (`False` / `None`).
    Off,
    /// Ghostscript.
    Ghostscript,
    /// xpdf.
    Xpdf,
}

impl RcValue for Distiller {
    fn parse_rc(text: &str) -> Result<Self, String> {
        match text.to_ascii_lowercase().as_str() {
            "none" | "false" => Ok(Self::Off),
            "ghostscript" => Ok(Self::Ghostscript),
            "xpdf" => Ok(Self::Xpdf),
            _ => Err(format!(
                "{text:?} is not None, False, 'ghostscript' or 'xpdf'"
            )),
        }
    }
    fn format_rc(&self) -> String {
        match self {
            Self::Off => "None",
            Self::Ghostscript => "ghostscript",
            Self::Xpdf => "xpdf",
        }
        .to_owned()
    }
}

/// `savefig.dpi`: the figure's own resolution or an explicit value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dpi {
    /// Use the figure's dpi.
    Figure,
    /// An explicit resolution.
    Value(f64),
}

impl RcValue for Dpi {
    fn parse_rc(text: &str) -> Result<Self, String> {
        if text == "figure" {
            Ok(Self::Figure)
        } else {
            f64::parse_rc(text)
                .map(Self::Value)
                .map_err(|_| format!("{text:?} is not 'figure' or a number"))
        }
    }
    fn format_rc(&self) -> String {
        match self {
            Self::Figure => "figure".to_owned(),
            Self::Value(dpi) => dpi.format_rc(),
        }
    }
}

/// `image.aspect`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Aspect {
    /// Square pixels.
    Equal,
    /// Fill the axes.
    Auto,
    /// A fixed height/width ratio of data units.
    Ratio(f64),
}

impl RcValue for Aspect {
    fn parse_rc(text: &str) -> Result<Self, String> {
        match text {
            "equal" => Ok(Self::Equal),
            "auto" => Ok(Self::Auto),
            _ => f64::parse_rc(text)
                .map(Self::Ratio)
                .map_err(|_| format!("{text:?} is not 'equal', 'auto' or a number")),
        }
    }
    fn format_rc(&self) -> String {
        match self {
            Self::Equal => "equal".to_owned(),
            Self::Auto => "auto".to_owned(),
            Self::Ratio(r) => r.format_rc(),
        }
    }
}

/// `axes.axisbelow`: where ticks and gridlines sit in the z-order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisBelow {
    /// Below everything (`True`, z-order 0.5).
    Below,
    /// Below lines but above patches (`line`, z-order 1.5).
    Line,
    /// Above everything (`False`, z-order 2.5).
    Above,
}

impl AxisBelow {
    /// The z-order matplotlib assigns to axis artists.
    pub const fn zorder(self) -> f64 {
        match self {
            Self::Below => 0.5,
            Self::Line => 1.5,
            Self::Above => 2.5,
        }
    }
}

impl RcValue for AxisBelow {
    fn parse_rc(text: &str) -> Result<Self, String> {
        match bool::parse_rc(text) {
            Ok(true) => Ok(Self::Below),
            Ok(false) => Ok(Self::Above),
            Err(_) if text == "line" => Ok(Self::Line),
            Err(_) => Err(format!(
                "{text:?} cannot be interpreted as True, False, or \"line\""
            )),
        }
    }
    fn format_rc(&self) -> String {
        match self {
            Self::Below => "True",
            Self::Line => "line",
            Self::Above => "False",
        }
        .to_owned()
    }
}

/// `hist.bins`: a bin count, a numpy binning rule or explicit edges.
#[derive(Clone, Debug, PartialEq)]
pub enum HistBins {
    /// This many equal-width bins.
    Count(usize),
    /// A numpy rule: `auto`, `sturges`, `fd`, `doane`, `scott`, `rice`, `sqrt`.
    Rule(String),
    /// Explicit bin edges.
    Edges(Vec<f64>),
}

impl RcValue for HistBins {
    fn parse_rc(text: &str) -> Result<Self, String> {
        const RULES: [&str; 7] = ["auto", "sturges", "fd", "doane", "scott", "rice", "sqrt"];
        if RULES.contains(&text) {
            return Ok(Self::Rule(text.to_owned()));
        }
        if let Ok(count) = text.parse::<usize>() {
            return Ok(Self::Count(count));
        }
        Vec::<f64>::parse_rc(text)
            .map(Self::Edges)
            .map_err(|_| format!("{text:?} is not a bin count, rule or list of edges"))
    }
    fn format_rc(&self) -> String {
        match self {
            Self::Count(n) => n.to_string(),
            Self::Rule(rule) => rule.clone(),
            Self::Edges(edges) => edges.format_rc(),
        }
    }
}

/// `boxplot.whiskers`: an IQR multiple or explicit percentiles.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Whiskers {
    /// Whiskers reach this many IQRs past the quartiles.
    Factor(f64),
    /// Whiskers at these lower/upper percentiles.
    Percentiles(f64, f64),
}

impl From<Whiskers> for crate::stats::Whis {
    fn from(whiskers: Whiskers) -> Self {
        match whiskers {
            Whiskers::Factor(factor) => Self::Factor(factor),
            Whiskers::Percentiles(low, high) => Self::Percentiles(low, high),
        }
    }
}

impl RcValue for Whiskers {
    fn parse_rc(text: &str) -> Result<Self, String> {
        match split_list(text)
            .map(f64::parse_rc)
            .collect::<Result<Vec<_>, _>>()?
            .as_slice()
        {
            [factor] => Ok(Self::Factor(*factor)),
            [lo, hi] => Ok(Self::Percentiles(*lo, *hi)),
            _ => Err(format!("{text:?} is not a float or a pair of floats")),
        }
    }
    fn format_rc(&self) -> String {
        match self {
            Self::Factor(f) => f.format_rc(),
            Self::Percentiles(lo, hi) => format!("{}, {}", lo.format_rc(), hi.format_rc()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rc_colours_normalise_like_matplotlib() {
        assert_eq!(Color::parse_rc("E5E5E5"), Ok(Color::Spec("#E5E5E5".into())));
        assert_eq!(Color::parse_rc("None"), Ok(Color::Spec("none".into())));
        assert_eq!(
            Color::parse_rc("(0.5, 0.5, 0.5)"),
            Ok(Color::from((0.5, 0.5, 0.5)))
        );
        assert!(Color::parse_rc("sparkly").is_err());
        assert_eq!(ColorOrAuto::parse_rc("auto"), Ok(ColorOrAuto::Auto));
        assert_eq!(LabelColor::parse_rc("mec"), Ok(LabelColor::MarkerEdgeColor));
        assert_eq!(LabelColor::parse_rc("None"), Ok(LabelColor::Unset));
    }

    #[test]
    fn small_kinds() {
        assert_eq!(FontType::parse_rc("truetype"), Ok(FontType::TrueType));
        assert_eq!(Dpi::parse_rc("figure"), Ok(Dpi::Figure));
        assert_eq!(AxisBelow::parse_rc("line").map(AxisBelow::zorder), Ok(1.5));
        assert_eq!(AxisBelow::parse_rc("True"), Ok(AxisBelow::Below));
        assert_eq!(HistBins::parse_rc("10"), Ok(HistBins::Count(10)));
        assert_eq!(HistBins::parse_rc("fd"), Ok(HistBins::Rule("fd".into())));
        assert_eq!(
            Whiskers::parse_rc("5, 95"),
            Ok(Whiskers::Percentiles(5.0, 95.0))
        );
        assert_eq!(Aspect::parse_rc("2"), Ok(Aspect::Ratio(2.0)));
    }
}
