//! Floating axes (matplotlib's `axisartist.floating_axes`): axes showing
//! a fixed range of curvilinear coordinates, bounded by the grid lines of
//! its extremes, with a curved background and auxiliary axes to plot in
//! curvilinear coordinates.

use std::ops::Deref;
use std::rc::Rc;

use plt_core::{Bbox, Path, Point};

use super::aux_transform::AuxTransform;
use super::axes::{Axes, GridHelper};
use super::grid_finder::{
    CoordinateGrid, ExtremeFinder, GridError, GridFinder, GridFinderOptions, GridInfo,
};
use super::grid_helper_curvelinear::{self as curvelinear, AxisGrid, FloatingLine};
use crate::artist::{ArtistHandle, ClipPathSpec};
use crate::axes::{AspectError, AxesState};
use crate::rc::Aspect;
use crate::spines::SpineSide;
use crate::ticker::AxisName;
use crate::transforms::Transform;

/// An extreme finder always returning the same range (matplotlib's
/// `ExtremeFinderFixed`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtremeFinderFixed {
    extremes: Bbox,
}

impl ExtremeFinderFixed {
    /// The range `(x0, x1, y0, y1)` of curvilinear coordinates.
    pub const fn new((x0, x1, y0, y1): (f64, f64, f64, f64)) -> Self {
        Self {
            extremes: Bbox::from_extents(x0, y0, x1, y1),
        }
    }
}

impl ExtremeFinder for ExtremeFinderFixed {
    fn find_transformed_bbox(&self, _: &AuxTransform, _: Bbox) -> Bbox {
        self.extremes
    }
}

/// The grid helper of floating axes: a curvilinear grid over the fixed
/// range `extremes` (matplotlib's `floating_axes.GridHelperCurveLinear`).
/// The edges of the range are the axis lines; only the grid lines strictly
/// inside it are drawn.
#[derive(Debug)]
pub struct GridHelperCurveLinear {
    grid: Rc<curvelinear::GridHelperCurveLinear>,
    extremes: (f64, f64, f64, f64),
}

/// The grid of floating axes over the view (matplotlib's `_grid_info` of
/// `floating_axes.GridHelperCurveLinear`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FloatingGridInfo {
    /// The levels and labels of both coordinates over the extremes.
    pub(crate) grid: AxisGrid,
    /// The interior grid lines and the lines of the extremes.
    pub(crate) lines: GridInfo,
    /// The lines of the first coordinate at its two extremes.
    lon_edges: Vec<Vec<Point>>,
    /// The lines of the second coordinate at its two extremes.
    lat_edges: Vec<Vec<Point>>,
}

impl FloatingGridInfo {
    /// The line of the extremes along `side` (data coordinates).
    pub(crate) fn edge_line(&self, side: SpineSide) -> Vec<Point> {
        let (lines, index) = match side {
            SpineSide::Left => (&self.lon_edges, 0),
            SpineSide::Right => (&self.lon_edges, 1),
            SpineSide::Bottom => (&self.lat_edges, 0),
            SpineSide::Top => (&self.lat_edges, 1),
        };
        lines.get(index).cloned().unwrap_or_default()
    }
}

impl GridHelperCurveLinear {
    /// A grid through `aux_trans` over the curvilinear range `extremes =
    /// (x0, x1, y0, y1)`; the extreme finder of `options` is replaced by
    /// an [`ExtremeFinderFixed`].
    pub fn new(
        aux_trans: AuxTransform,
        extremes: (f64, f64, f64, f64),
        options: GridFinderOptions,
    ) -> Self {
        let options = GridFinderOptions {
            extreme_finder: Some(Box::new(ExtremeFinderFixed::new(extremes))),
            ..options
        };
        Self {
            grid: Rc::new(curvelinear::GridHelperCurveLinear::new(aux_trans, options)),
            extremes,
        }
    }

    /// The grid finder computing lines and ticks.
    pub fn grid_finder(&self) -> &GridFinder {
        self.grid.grid_finder()
    }

    /// The curvilinear range `(x0, x1, y0, y1)`.
    pub fn extremes(&self) -> (f64, f64, f64, f64) {
        self.extremes
    }

    /// The curvilinear grid underneath.
    pub(crate) fn curvelinear(&self) -> &Rc<curvelinear::GridHelperCurveLinear> {
        &self.grid
    }

    /// The edge of the range along `side` as a line of constant
    /// coordinate (matplotlib's `FixedAxisArtistHelper.__init__`).
    pub(crate) fn edge_line(&self, side: SpineSide) -> FloatingLine {
        let (x0, x1, y0, y1) = self.extremes;
        let (value, nth) = match side {
            SpineSide::Left => (x0, AxisName::X),
            SpineSide::Right => (x1, AxisName::X),
            SpineSide::Bottom => (y0, AxisName::Y),
            SpineSide::Top => (y1, AxisName::Y),
        };
        FloatingLine {
            nth,
            value,
            extremes: (f64::NEG_INFINITY, f64::INFINITY),
        }
    }

    /// The levels, labels and lines over the data box `view` (matplotlib's
    /// `_update_grid`).
    pub(crate) fn grid_info(&self, view: Bbox) -> Result<FloatingGridInfo, GridError> {
        let finder = self.grid_finder();
        let extremes = finder.extremes(view)?;
        let grid = AxisGrid::new(finder, extremes)?;
        let (lon_min, lat_min, lon_max, lat_max) = extremes.extents();
        let inside = |values: Vec<f64>, (low, high): (f64, f64)| -> Vec<f64> {
            values
                .into_iter()
                .filter(|&v| low < v && v < high)
                .collect()
        };
        let (lon_lines, lat_lines) = finder.raw_grid_lines(
            &inside(grid.lon.values(), (lon_min, lon_max)),
            &inside(grid.lat.values(), (lat_min, lat_max)),
            extremes,
        );
        let (lon_edges, lat_edges) =
            finder.raw_grid_lines(&[lon_min, lon_max], &[lat_min, lat_max], extremes);
        let mut lines = GridInfo {
            extremes,
            lon: CoordinateGrid::default(),
            lat: CoordinateGrid::default(),
        };
        lines.lon.lines = lon_lines;
        lines.lat.lines = lat_lines;
        Ok(FloatingGridInfo {
            grid,
            lines,
            lon_edges,
            lat_edges,
        })
    }
}

/// Axes showing the range of a [`GridHelperCurveLinear`]: its axis lines
/// are the edges of the range, its background the region they enclose,
/// and it keeps a data ratio of 1 (matplotlib's `FloatingAxes`, also
/// `FloatingSubplot`). It dereferences to the axisartist [`Axes`].
///
/// ```
/// use plt::axisartist::floating_axes::{FloatingAxes, GridHelperCurveLinear};
/// use plt::axisartist::grid_finder::GridFinderOptions;
/// use plt::axisartist::AuxTransform;
/// use plt::prelude::*;
/// use plt::transforms::Affine2D;
/// let fig = Figure::new();
/// let tr = AuxTransform::affine(Affine2D::scaling(2.0, 1.0));
/// let helper = GridHelperCurveLinear::new(tr.clone(), (0.0, 1.0, 0.0, 1.0), GridFinderOptions::default());
/// let ax = FloatingAxes::new(fig.subplot(), helper)?;
/// let (x0, x1) = ax.ax().xlim();
/// assert!((x0 + 0.02).abs() < 1e-12 && (x1 - 2.02).abs() < 1e-12);
/// let aux = ax.get_aux_axes(&tr);
/// aux.plot([0.0, 1.0], [0.0, 1.0])?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct FloatingAxes {
    axes: Axes,
}

/// matplotlib's `FloatingSubplot`, the same class as [`FloatingAxes`].
pub type FloatingSubplot = FloatingAxes;

impl Deref for FloatingAxes {
    type Target = Axes;

    fn deref(&self) -> &Axes {
        &self.axes
    }
}

/// The number of interpolation steps of the background outline.
const PATCH_STEPS: usize = 100;

impl FloatingAxes {
    /// Turns `ax` into floating axes over the range of `grid_helper`: axis
    /// artists along the edges of the range, the region they enclose as
    /// background and clip outline of the plotted artists, the limits
    /// fitted around it, and an equal aspect (matplotlib's
    /// `FloatingAxesBase.__init__` and `clear`).
    ///
    /// # Errors
    ///
    /// When the grid transform has no inverse.
    pub fn new(
        ax: crate::axes::Axes,
        grid_helper: GridHelperCurveLinear,
    ) -> Result<Self, FloatingAxesError> {
        let outline = grid_helper.outline();
        grid_helper.grid_finder().extremes(Bbox::UNIT)?;
        let axes = Axes::with_grid_helper(ax, GridHelper::Floating(grid_helper));
        let id = axes.ax().id();
        axes.ax().raw().with_mut::<AxesState, _>(|state| {
            state.patch_outline = Some((outline.clone(), Transform::Data(id)));
        });
        // The gridlines are clipped to the axes box, not to the outline
        // (matplotlib's `gridlines.set_clip_path(orig_patch)`, which turns
        // a rectangle into a clip box).
        if let Some(gridlines) = axes.gridlines() {
            gridlines.set_clip_path(Some(ClipPathSpec::Rectangle(Transform::Axes(id))));
        }
        let floating = Self { axes };
        floating.adjust_axes_lim();
        floating.ax().set_aspect(Aspect::Equal)?;
        Ok(floating)
    }

    /// The background outline in data coordinates.
    pub fn patch_path(&self) -> Path {
        self.ax()
            .raw()
            .with::<AxesState, _>(|state| state.patch_outline.clone())
            .map_or_else(Path::unit_rectangle, |(path, _)| path)
    }

    /// Fits the limits around the background outline with a 2 % margin
    /// (matplotlib's `adjust_axes_lim`).
    pub fn adjust_axes_lim(&self) {
        let bbox = self.patch_path().extents().expanded(1.02, 1.02);
        self.ax()
            .set_xlim(bbox.xmin(), bbox.xmax())
            .set_ylim(bbox.ymin(), bbox.ymax());
    }

    /// Gives `aux` the background patch of these axes (matplotlib's
    /// `aux_ax.patch = ax.patch`): what is plotted on `aux` from now on is
    /// clipped to the background outline instead of the axes box, and the
    /// patch is drawn again among the children of `aux` at its z-order
    /// ([`Axes::set_patch_zorder`](crate::axes::Axes::set_patch_zorder);
    /// matplotlib's gallery lowers it to 0.9 so that it stays below them).
    pub fn share_patch(&self, aux: &crate::axes::Axes) {
        let outline = self
            .ax()
            .raw()
            .with::<AxesState, _>(|state| state.patch_outline.clone());
        aux.raw().with_mut::<AxesState, _>(|state| {
            state.patch_outline = outline;
            state.patch.host_owned = true;
        });
    }
}

impl GridHelperCurveLinear {
    /// The region enclosed by the edges of the range, in data
    /// coordinates: the polygon of the extremes, interpolated, through the
    /// grid transform (matplotlib's `_gen_axes_patch` with its transform).
    fn outline(&self) -> Path {
        let (x0, x1, y0, y1) = self.extremes;
        let polygon = Path::polygon([
            Point::new(x0, y0),
            Point::new(x1, y0),
            Point::new(x1, y1),
            Point::new(x0, y1),
        ])
        .with_interpolation_steps(PATCH_STEPS);
        self.grid_finder().transform().transform_path(&polygon)
    }
}

/// Why floating axes could not be set up.
#[derive(Clone, Debug, PartialEq)]
pub enum FloatingAxesError {
    /// The grid transform has no inverse.
    Grid(GridError),
    /// The equal aspect was refused.
    Aspect(AspectError),
}

impl std::fmt::Display for FloatingAxesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Grid(err) => write!(f, "{err}"),
            Self::Aspect(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for FloatingAxesError {}

impl From<GridError> for FloatingAxesError {
    fn from(err: GridError) -> Self {
        Self::Grid(err)
    }
}

impl From<AspectError> for FloatingAxesError {
    fn from(err: AspectError) -> Self {
        Self::Aspect(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    fn quarter_annulus() -> GridHelperCurveLinear {
        GridHelperCurveLinear::new(
            AuxTransform::polar(),
            (0.5 * std::f64::consts::PI, 0.0, 2.0, 1.0),
            GridFinderOptions::default(),
        )
    }

    #[test]
    fn interior_lines_and_edges() {
        let helper = quarter_annulus();
        let info = helper.grid_info(Bbox::UNIT).unwrap();
        // With extremes given high to low no level lies strictly inside
        // them, as in matplotlib.
        assert!(info.lines.lon.lines.is_empty() && info.lines.lat.lines.is_empty());
        let outer = info.edge_line(SpineSide::Bottom);
        let last = outer.last().unwrap();
        assert!(
            (last.x - 2.0).abs() < 1e-12 && last.y.abs() < 1e-12,
            "r = 2 at θ = 0"
        );
        assert_eq!(helper.edge_line(SpineSide::Right).value, 0.0);
    }

    #[test]
    fn floating_axes_clip_to_their_outline() {
        let fig = Figure::new();
        let ax = FloatingAxes::new(fig.subplot(), quarter_annulus()).unwrap();
        let (x0, x1) = ax.ax().xlim();
        assert!((x0 + 0.02).abs() < 1e-12 && (x1 - 2.02).abs() < 1e-12);
        assert!(ax.patch_path().has_curves());
        let aux = ax.get_aux_axes(&AuxTransform::polar());
        ax.share_patch(&aux);
        let shared = aux
            .raw()
            .with::<AxesState, _>(|state| state.patch_outline.is_some());
        assert!(shared);
        assert!(fig.to_svg().is_ok());
    }

    #[test]
    fn a_shared_patch_is_drawn_again_among_the_children() {
        let fig = Figure::with_size(3.0, 3.0, 72.0);
        let ax = FloatingAxes::new(fig.subplot(), quarter_annulus()).unwrap();
        // The white background, and the figure's.
        let whites = |svg: &str| svg.matches("fill: #ffffff").count();
        let alone = whites(&fig.to_svg().unwrap());
        let aux = ax.get_aux_axes(&AuxTransform::polar());
        ax.share_patch(&aux);
        ax.ax().set_patch_zorder(0.9);
        assert_eq!(whites(&fig.to_svg().unwrap()), alone + 1);
    }
}
