//! The contour set of one plot and the pipeline that computes it
//! (matplotlib's `ContourSet.__init__`).

use std::fmt;
use std::rc::Rc;

use plt_core::{Path, Point, Rgba};
use plt_render::Dashes;

use super::label::ContourLabel;
use super::levels::{self, LevelRequest};
use super::style::{self, LevelColoring, LineStyling};
use super::{ContourError, ContourOptions, QuadContourGenerator};
use crate::artist::{Artist, ArtistHandle};
use crate::axes::Axes;
use crate::collections::{Collection, CollectionArtist, CollectionKind, EdgeColors};
use crate::colors::{Color, Extend};
use crate::lines::LineStyle;
use crate::rc::RcParams;
use crate::ticker::TickerSettings;

/// Something matplotlib warns about while contouring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContourWarning {
    /// A log-scaled quad contour masked the non-positive values of z.
    NonPositiveMasked,
}

impl fmt::Display for ContourWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositiveMasked => f.write_str("Log scale: values of z <= 0 have been masked"),
        }
    }
}

/// A contour generator: the compound path of the lines at a level or of
/// the polygons between two levels.
pub(crate) trait Generator {
    /// The lines at `level`.
    fn lines(&mut self, level: f64) -> Path;

    /// The region `lower < z <= upper`.
    fn filled(&mut self, lower: f64, upper: f64) -> Result<Path, ContourError>;
}

impl Generator for QuadContourGenerator {
    fn lines(&mut self, level: f64) -> Path {
        Self::lines(self, level)
    }

    fn filled(&mut self, lower: f64, upper: f64) -> Result<Path, ContourError> {
        Self::filled(self, lower, upper)
    }
}

/// What a field contributes to a contour set (matplotlib's
/// `_process_args`).
#[derive(Debug)]
pub(crate) struct ContourSource<G> {
    pub(crate) generator: G,
    /// The extreme unmasked values (`None` when everything is masked).
    pub(crate) zmin: Option<f64>,
    pub(crate) zmax: Option<f64>,
    /// The corners of the coordinates' bounding box.
    pub(crate) data_lim: (Point, Point),
    /// Whether z holds booleans.
    pub(crate) boolean: bool,
    pub(crate) warnings: Vec<ContourWarning>,
}

/// The levels, colours and styles of a contour set.
#[derive(Clone, Debug)]
pub(crate) struct ContourState {
    pub(crate) filled: bool,
    pub(crate) levels: Vec<f64>,
    /// The levels with the `extend` sentinels.
    pub(crate) extended: Vec<f64>,
    pub(crate) layers: Vec<f64>,
    pub(crate) zmin: Option<f64>,
    pub(crate) zmax: Option<f64>,
    pub(crate) logscale: bool,
    pub(crate) extend: Extend,
    pub(crate) norm_limits: (Option<f64>, Option<f64>),
    /// One colour per level or band, before alpha.
    pub(crate) colors: Vec<Rgba>,
    pub(crate) alpha: Option<f64>,
    pub(crate) linewidths: Vec<f64>,
    pub(crate) linestyles: Vec<LineStyle>,
    pub(crate) warnings: Vec<ContourWarning>,
    pub(crate) labels: Vec<ContourLabel>,
    /// The label widths of the last `clabel` call, per labelled level.
    pub(crate) label_widths: Vec<f64>,
    pub(crate) rc: Rc<RcParams>,
}

/// A contour set computed but not yet added to an axes.
#[derive(Debug)]
pub(crate) struct ContourPlan {
    /// One collection, or one per band for hatched filled contours.
    pub(crate) collections: Vec<CollectionArtist>,
    /// The corners of the data, which the axes' limits must include.
    pub(crate) data_lim: (Point, Point),
    pub(crate) state: ContourState,
}

impl ContourPlan {
    /// Chooses the levels, traces the paths and styles them.
    pub(crate) fn new<G: Generator>(
        filled: bool,
        mut options: ContourOptions,
        mut source: ContourSource<G>,
    ) -> Result<Self, ContourError> {
        let rc = crate::rc::params();
        let logscale = options.logscale();
        let request = LevelRequest {
            levels: &options.levels,
            locator: options.locator.as_deref(),
            logscale,
            filled,
            extend: options.extend,
            boolean: source.boolean,
        };
        let levels = request.choose(source.zmin, source.zmax, &TickerSettings::from(&*rc))?;
        let extended = levels::extended(&levels, options.extend, logscale);
        let layers = levels::layers(&levels, &extended, filled, logscale);
        let coloring = LevelColoring {
            levels: &levels,
            layers: &layers,
            filled,
            logscale,
            extend: options.extend,
        };
        let plan = style::map_colors(&mut options, &coloring, &rc)?;
        let paths = if filled {
            levels::bands(&extended, source.zmin, logscale)
                .into_iter()
                .map(|(lower, upper)| source.generator.filled(lower, upper))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            levels
                .iter()
                .map(|&level| source.generator.lines(level))
                .collect()
        };
        let (linewidths, linestyles) = if filled {
            (Vec::new(), Vec::new())
        } else {
            let negative = options
                .negative_linestyles
                .clone()
                .unwrap_or_else(|| rc.contour_negative_linestyle.clone());
            let styling = LineStyling {
                levels: &levels,
                zrange: source.zmin.zip(source.zmax),
                monochrome: plan.monochrome,
                negative: &negative,
            };
            (
                style::linewidths(options.linewidths.as_deref(), levels.len(), &rc),
                style::linestyles(options.linestyles.as_deref(), &styling),
            )
        };
        let state = ContourState {
            filled,
            levels,
            extended,
            layers,
            zmin: source.zmin,
            zmax: source.zmax,
            logscale,
            extend: options.extend,
            norm_limits: plan.norm_limits,
            colors: plan.colors,
            alpha: options.alpha,
            linewidths,
            linestyles,
            warnings: source.warnings,
            labels: Vec::new(),
            label_widths: Vec::new(),
            rc,
        };
        let collections = state.collections(paths, &options, source.data_lim);
        Ok(Self {
            collections,
            data_lim: source.data_lim,
            state,
        })
    }
}

impl ContourState {
    /// The collections drawing `paths`: one, or one per band when filled
    /// contours are hatched (matplotlib then draws the bands one by one,
    /// each with its hatch).
    fn collections(
        &self,
        paths: Vec<Path>,
        options: &ContourOptions,
        data_lim: (Point, Point),
    ) -> Vec<CollectionArtist> {
        let colors: Vec<Color> = self.colors.iter().map(|&c| Color::from(c)).collect();
        if !(self.filled && options.hatches.iter().any(Option::is_some)) {
            return vec![self.collection(paths, colors, None, options.antialiased, data_lim)];
        }
        paths
            .into_iter()
            .enumerate()
            .map(|(i, path)| {
                let color = colors.get(i).cloned().into_iter().collect();
                let hatch = options.hatches[i % options.hatches.len()].clone();
                self.collection(vec![path], color, hatch, options.antialiased, data_lim)
            })
            .collect()
    }

    /// A collection drawing `paths` in `colors`: unstroked fills, or lines
    /// without faces above other collections.
    fn collection(
        &self,
        paths: Vec<Path>,
        colors: Vec<Color>,
        hatch: Option<String>,
        antialiased: Option<bool>,
        (mins, maxs): (Point, Point),
    ) -> CollectionArtist {
        let mut collection =
            CollectionArtist::new(CollectionKind::Generic, paths, Rc::clone(&self.rc));
        collection.hatch = hatch;
        if self.filled {
            collection.facecolors = Some(colors);
            collection.edgecolors = EdgeColors::Colors(vec![Color::none()]);
            // Not stroked, so anti-aliasing would only show seams.
            collection.antialiased = Some(vec![antialiased.unwrap_or(false)]);
        } else {
            collection.facecolors = Some(vec![Color::none()]);
            collection.edgecolors = EdgeColors::Colors(colors);
            collection.linewidths = Some(self.linewidths.clone());
            collection.linestyles.clone_from(&self.linestyles);
            collection.antialiased = antialiased.map(|a| vec![a]);
        }
        let base = collection.base_mut();
        base.alpha = self.alpha;
        if !self.filled {
            base.zorder = 2.0;
            "_nolegend_".clone_into(&mut base.label);
        }
        base.sticky_edges = (vec![mins.x, maxs.x], vec![mins.y, maxs.y]);
        collection
    }
}

/// A contour plot: what [`Axes::contour`], [`Axes::contourf`],
/// [`Axes::tricontour`] and [`Axes::tricontourf`] return (matplotlib's
/// `ContourSet`).
#[derive(Debug)]
pub struct ContourSet {
    axes: Axes,
    collections: Vec<Collection>,
    state: ContourState,
}

impl ContourSet {
    pub(crate) fn new(axes: Axes, collections: Vec<Collection>, state: ContourState) -> Self {
        Self {
            axes,
            collections,
            state,
        }
    }

    pub(crate) fn state(&self) -> &ContourState {
        &self.state
    }

    pub(crate) fn state_mut(&mut self) -> &mut ContourState {
        &mut self.state
    }

    /// The axes the contours are drawn in.
    pub fn axes(&self) -> &Axes {
        &self.axes
    }

    /// The collections drawing the contours: one, or one per band for
    /// hatched filled contours.
    pub fn collections(&self) -> &[Collection] {
        &self.collections
    }

    /// Whether these are filled contours.
    pub fn filled(&self) -> bool {
        self.state.filled
    }

    /// The contour levels.
    pub fn levels(&self) -> &[f64] {
        &self.state.levels
    }

    /// The value each level (lines) or band (fills) is coloured by: the
    /// levels, or the band midpoints (with the `extend` sentinels
    /// `±1e250`; geometric when log-scaled).
    pub fn layers(&self) -> &[f64] {
        &self.state.layers
    }

    /// The smallest unmasked z (`None` when all z is masked).
    pub fn zmin(&self) -> Option<f64> {
        self.state.zmin
    }

    /// The largest unmasked z.
    pub fn zmax(&self) -> Option<f64> {
        self.state.zmax
    }

    /// Whether levels and colours are log-scaled.
    pub fn logscale(&self) -> bool {
        self.state.logscale
    }

    /// Which ends are extended.
    pub fn extend(&self) -> Extend {
        self.state.extend
    }

    /// The norm's `(vmin, vmax)` after autoscaling to the levels.
    pub fn norm_limits(&self) -> (Option<f64>, Option<f64>) {
        self.state.norm_limits
    }

    /// The z interval `(lower, upper]` of every band of a filled contour:
    /// consecutive levels including the `extend` sentinels, the lowest
    /// bound lowered when it equals [`ContourSet::zmin`] so the minimum is
    /// included (matplotlib's `_get_lowers_and_uppers`).
    pub fn bands(&self) -> Vec<(f64, f64)> {
        levels::bands(&self.state.extended, self.state.zmin, self.state.logscale)
    }

    /// One colour per level (lines) or band (fills), alpha applied.
    pub fn colors(&self) -> Vec<Rgba> {
        self.state
            .colors
            .iter()
            .map(|&c| self.state.with_alpha(c))
            .collect()
    }

    /// The line width of every level in points (empty for fills).
    pub fn linewidths(&self) -> &[f64] {
        &self.state.linewidths
    }

    /// The line style of every level (empty for fills).
    pub fn linestyles(&self) -> &[LineStyle] {
        &self.state.linestyles
    }

    /// The dash pattern of every level, scaled by its line width
    /// (matplotlib's `get_linestyle`; solid lines have an empty pattern).
    pub fn dash_patterns(&self) -> Vec<Dashes> {
        self.state
            .linestyles
            .iter()
            .zip(&self.state.linewidths)
            .map(|(style, &width)| style.scaled_dashes(width, &self.state.rc))
            .collect()
    }

    /// One compound path per level (lines) or band (fills), as currently
    /// drawn (inline labels cut the lines).
    pub fn paths(&self) -> Vec<Path> {
        self.collections
            .iter()
            .flat_map(|c| c.raw().with::<CollectionArtist, _>(|c| c.paths.clone()))
            .collect()
    }

    /// What matplotlib would have warned about.
    pub fn warnings(&self) -> &[ContourWarning] {
        &self.state.warnings
    }

    /// The labels added by [`ContourSet::clabel`], in creation order.
    pub fn labels(&self) -> &[ContourLabel] {
        &self.state.labels
    }

    /// Replaces the paths (one per level or band) in the collections.
    pub(crate) fn set_paths(&self, paths: Vec<Path>) {
        if let [only] = self.collections.as_slice() {
            only.raw()
                .with_mut::<CollectionArtist, _>(|c| c.paths = paths);
            return;
        }
        for (collection, path) in self.collections.iter().zip(paths) {
            collection
                .raw()
                .with_mut::<CollectionArtist, _>(|c| c.paths = vec![path]);
        }
    }
}

impl ContourState {
    /// `color` with the set's alpha.
    pub(crate) fn with_alpha(&self, color: Rgba) -> Rgba {
        self.alpha.map_or(color, |a| color.with_alpha(a))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contour::{ContourNorm, Levels, QuadField};
    use crate::figure::Figure;
    use crate::numeric::Grid;

    /// z = row + column on a 3 × 4 grid.
    fn ramp() -> QuadField {
        QuadField::new(Grid::from_fn(3, 4, |i, j| (i + j) as f64))
    }

    fn levels(values: &[f64]) -> ContourOptions {
        ContourOptions {
            levels: Levels::Values(values.to_vec()),
            ..ContourOptions::default()
        }
    }

    #[test]
    fn filled_sets_report_bands_layers_and_limits() {
        let ax = Figure::new().subplot();
        let options = ContourOptions {
            extend: Extend::Max,
            ..levels(&[0.0, 2.0, 5.0])
        };
        let cs = ax.contourf(&ramp(), options).unwrap();
        assert!(cs.filled());
        assert_eq!(cs.bands(), [(-1.0, 2.0), (2.0, 5.0), (5.0, 1e250)]);
        assert_eq!(cs.layers(), [1.0, 3.5, f64::midpoint(5.0, 1e250)]);
        assert_eq!((cs.zmin(), cs.zmax()), (Some(0.0), Some(5.0)));
        assert_eq!(cs.paths().len(), 3);
        assert_eq!(cs.colors().len(), 3);
        assert!(cs.linewidths().is_empty() && cs.labels().is_empty());
        assert_eq!((ax.xlim(), ax.ylim()), ((0.0, 3.0), (0.0, 2.0)));
    }

    #[test]
    fn single_colour_lines_dash_negative_levels() {
        let ax = Figure::new().subplot();
        let field = QuadField::new(Grid::from_fn(3, 3, |i, j| (i + j) as f64 - 2.0));
        let options = ContourOptions {
            colors: Some(vec![Color::from("k")]),
            alpha: Some(0.5),
            ..levels(&[-1.0, 0.0, 1.0])
        };
        let cs = ax.contour(&field, options).unwrap();
        assert_eq!(
            cs.linestyles(),
            [LineStyle::Dashed, LineStyle::Solid, LineStyle::Solid]
        );
        assert_eq!(cs.linewidths(), [1.5; 3]);
        assert_eq!(cs.colors()[0], Rgba::new(0.0, 0.0, 0.0, 0.5));
        assert_eq!(cs.dash_patterns()[0].pattern.len(), 2);
        assert_eq!(cs.dash_patterns()[1].pattern, Vec::<f64>::new());
        assert_eq!(cs.norm_limits(), (Some(-1.0), Some(1.0)));
    }

    #[test]
    fn log_scaled_quads_mask_non_positive_values() {
        let ax = Figure::new().subplot();
        let field = QuadField::new(Grid::from_rows([[-1.0, 10.0], [100.0, 1000.0]]).unwrap());
        let options = ContourOptions {
            norm: ContourNorm::Log,
            ..ContourOptions::default()
        };
        let cs = ax.contour(&field, options).unwrap();
        assert!(cs.logscale());
        assert_eq!(cs.zmin(), Some(10.0));
        assert_eq!(cs.warnings(), [ContourWarning::NonPositiveMasked]);
        assert_eq!(cs.levels(), [1.0, 10.0, 100.0, 1000.0, 10000.0]);
    }

    #[test]
    fn invalid_levels_are_rejected() {
        let ax = Figure::new().subplot();
        assert_eq!(
            ax.contourf(&ramp(), levels(&[1.0])).unwrap_err(),
            ContourError::TooFewFilledLevels(1)
        );
        assert_eq!(
            ax.contour(&ramp(), levels(&[2.0, 1.0])).unwrap_err(),
            ContourError::LevelsNotIncreasing
        );
    }
}
