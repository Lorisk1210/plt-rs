//! The keyword arguments shared by the contour functions.

use super::Levels;
use super::levels::is_log_locator;
use crate::colors::{Color, ColorizerOptions, Extend, Norm, NormKind, impl_colorize};
use crate::lines::LineStyle;
use crate::ticker::Locator;
use crate::transforms::Transform;

/// Options of [`Axes::contour`](crate::axes::Axes::contour) and friends
/// (matplotlib's contour keyword arguments).
#[derive(Debug, Default)]
pub struct ContourOptions {
    /// Which levels to draw.
    pub levels: Levels,
    /// The locator choosing automatic levels (default: `MaxNLocator(n + 1,
    /// min_n_ticks=1)`, or `LogLocator(numticks=n)` when log-scaled). A
    /// `LogLocator` makes the contour set log-scaled.
    pub locator: Option<Box<dyn Locator>>,
    /// Whether filled contours also cover values below the lowest and/or
    /// above the highest level (drawn in the colormap's under/over colours).
    pub extend: Extend,
    /// One colour for every level (band), or one per level (band), used
    /// instead of a colormap; with `extend`, two extra colours may give the
    /// under and over colours.
    pub colors: Option<Vec<Color>>,
    /// The colour mapping of the levels: the colormap (default
    /// `image.cmap`, exclusive with `colors`), the norm (default linear;
    /// a [`LogNorm`](crate::colors::LogNorm) also selects levels on a log
    /// scale and masks non-positive z on quad grids) and the limits
    /// (default the extreme levels), which override the norm's own, as in
    /// matplotlib.
    pub colorizer: ColorizerOptions,
    /// Opacity of lines and fills.
    pub alpha: Option<f64>,
    /// Line widths in points, cycled over the levels (default
    /// `contour.linewidth`, else `lines.linewidth`). Line contours only.
    pub linewidths: Option<Vec<f64>>,
    /// Line styles cycled over the levels (default: solid, except negative
    /// levels of a single-colour set). Line contours only.
    pub linestyles: Option<Vec<LineStyle>>,
    /// The style of negative levels of a single-colour set (default
    /// `contour.negative_linestyle`).
    pub negative_linestyles: Option<LineStyle>,
    /// Anti-aliasing (default: off for filled contours, `lines.antialiased`
    /// for lines).
    pub antialiased: Option<bool>,
    /// Hatch patterns cycled over the bands of filled contours (`None` =
    /// unhatched).
    pub hatches: Vec<Option<String>>,
    /// The coordinates of the grid (matplotlib's `transform`; default the
    /// axes' data coordinates). When they end in the data coordinates
    /// after a fixed affine map, the data limits and sticky edges cover
    /// the grid mapped into them.
    pub transform: Option<Transform>,
}

impl_colorize!(ContourOptions);

impl ContourOptions {
    /// Whether levels and colours are log-scaled: a `LogNorm` or a
    /// `LogLocator` (matplotlib's `logscale`).
    pub fn logscale(&self) -> bool {
        self.colorizer.norm.as_deref().is_some_and(is_log_norm)
            || self.locator.as_deref().is_some_and(is_log_locator)
    }
}

/// Whether `norm` is a `LogNorm` (a norm made from the log scale).
fn is_log_norm(norm: &dyn Norm) -> bool {
    matches!(norm.kind(), NormKind::Scaled(scale) if scale.name() == "log")
}
