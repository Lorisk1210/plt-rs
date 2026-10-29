//! Where the lines of a curvilinear grid run and where they cross the
//! axes box (matplotlib's `axisartist.grid_finder`): grid locators and
//! formatters, extreme finders, and the [`GridFinder`] combining them
//! with an [`AuxTransform`].

mod tickers;

use std::fmt;

use pltrs_core::{Bbox, Point};

use super::aux_transform::AuxTransform;
use crate::numeric::linspace;
use crate::spines::SpineSide;
use crate::ticker::TickerError;
use crate::ticker::pyfmt::FormatError;

pub use tickers::{
    DictFormatter, ExtremeFinderSimple, FormatterPrettyPrint, GridFixedLocator, GridMaxNLocator,
};
pub use tickers::{padded_extent, sample_grid};

/// Why grid levels, labels or lines could not be computed.
#[derive(Clone, Debug, PartialEq)]
pub enum GridError {
    /// A locator or formatter of the ticker failed.
    Ticker(TickerError),
    /// A label could not be formatted.
    Format(FormatError),
    /// The grid transform has no inverse.
    Singular,
    /// A grid step was asked for an interval beyond the step tables (an
    /// `IndexError` in matplotlib's `select_step_degree`).
    StepOutOfRange(f64),
    /// Grid levels could not be enumerated (non-finite bounds).
    InvalidLevels(String),
}

impl fmt::Display for GridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ticker(err) => write!(f, "{err}"),
            Self::Format(err) => write!(f, "{err}"),
            Self::Singular => f.write_str("the grid transform is not invertible"),
            Self::StepOutOfRange(dv) => write!(f, "no grid step for an interval of {dv}"),
            Self::InvalidLevels(why) => write!(f, "invalid grid levels: {why}"),
        }
    }
}

impl std::error::Error for GridError {}

impl From<TickerError> for GridError {
    fn from(err: TickerError) -> Self {
        Self::Ticker(err)
    }
}

impl From<FormatError> for GridError {
    fn from(err: FormatError) -> Self {
        Self::Format(err)
    }
}

/// The levels a grid locator chose: `levels[..n] / factor` are the grid
/// values, `factor` scales them to integers for the formatters (the
/// `(levs, n, factor)` triple of matplotlib's grid locators).
#[derive(Clone, Debug, PartialEq)]
pub struct GridLevels {
    /// The levels, scaled by `factor`.
    pub levels: Vec<f64>,
    /// How many of the levels are valid (a full cycle may repeat the
    /// first level at the end).
    pub n: usize,
    /// The scale of the levels.
    pub factor: f64,
}

impl GridLevels {
    /// The grid values: the first `n` levels divided by the factor.
    pub fn values(&self) -> Vec<f64> {
        self.levels
            .iter()
            .take(self.n)
            .map(|level| level / self.factor)
            .collect()
    }

    /// Every level divided by the factor (what floating axes tick).
    pub(crate) fn all_values(&self) -> Vec<f64> {
        self.levels
            .iter()
            .map(|level| level / self.factor)
            .collect()
    }
}

/// Chooses the grid levels of one curvilinear coordinate between `v1`
/// and `v2` (matplotlib's grid locator protocol, `locator(v1, v2)`).
pub trait GridLocator: fmt::Debug {
    /// The levels spanning `v1..v2`.
    fn levels(&self, v1: f64, v2: f64) -> Result<GridLevels, GridError>;
}

/// Labels grid levels (matplotlib's grid formatter protocol,
/// `formatter(direction, factor, values)`).
pub trait GridFormatter: fmt::Debug {
    /// One label per level of `values` (scaled by `factor`) ticked on the
    /// `direction` side.
    fn labels(
        &self,
        direction: SpineSide,
        factor: f64,
        values: &[f64],
    ) -> Result<Vec<String>, GridError>;
}

/// Approximates the range of curvilinear coordinates covering a box
/// (matplotlib's extreme finders).
pub trait ExtremeFinder: fmt::Debug {
    /// The bounding box of `bbox` mapped by `transform`, padded for the
    /// sampling (matplotlib's `_find_transformed_bbox`).
    fn find_transformed_bbox(&self, transform: &AuxTransform, bbox: Bbox) -> Bbox;
}

/// A crossing of a grid line with a side of the box: where, at which
/// angle in degrees (0 = moving right), on which level and with which
/// label.
#[derive(Clone, Debug, PartialEq)]
pub struct GridTick {
    /// The level of the grid line (scaled by the factor).
    pub level: f64,
    /// The crossing in data coordinates.
    pub loc: Point,
    /// The direction of the line there, in degrees.
    pub angle: f64,
    /// The label of the level.
    pub label: String,
}

/// The grid lines of one curvilinear coordinate and their crossings with
/// the four sides of the box.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CoordinateGrid {
    /// One polyline per grid value, in data coordinates.
    pub lines: Vec<Vec<Point>>,
    /// The crossings with the left, right, bottom and top sides.
    pub ticks: [Vec<GridTick>; 4],
}

impl CoordinateGrid {
    /// The crossings with `side`.
    pub fn side_ticks(&self, side: SpineSide) -> &[GridTick] {
        &self.ticks[side_index(side)]
    }
}

/// The sides in the order of [`CoordinateGrid::ticks`].
const SIDES: [SpineSide; 4] = [
    SpineSide::Left,
    SpineSide::Right,
    SpineSide::Bottom,
    SpineSide::Top,
];

/// The position of `side` in [`CoordinateGrid::ticks`].
fn side_index(side: SpineSide) -> usize {
    match side {
        SpineSide::Left => 0,
        SpineSide::Right => 1,
        SpineSide::Bottom => 2,
        SpineSide::Top => 3,
    }
}

/// Everything about the grid over a box (matplotlib's `grid_info`).
#[derive(Clone, Debug, PartialEq)]
pub struct GridInfo {
    /// The range of curvilinear coordinates covering the box.
    pub extremes: Bbox,
    /// The lines of constant first coordinate ("longitude").
    pub lon: CoordinateGrid,
    /// The lines of constant second coordinate ("latitude").
    pub lat: CoordinateGrid,
}

/// The optional parts of a [`GridFinder`]; `None` takes matplotlib's
/// default (`ExtremeFinderSimple(20, 20)`, `MaxNLocator()`,
/// `FormatterPrettyPrint()`).
#[derive(Debug, Default)]
pub struct GridFinderOptions {
    /// Finds the range of curvilinear coordinates in view.
    pub extreme_finder: Option<Box<dyn ExtremeFinder>>,
    /// Levels of the first coordinate.
    pub grid_locator1: Option<Box<dyn GridLocator>>,
    /// Levels of the second coordinate.
    pub grid_locator2: Option<Box<dyn GridLocator>>,
    /// Labels of the first coordinate.
    pub tick_formatter1: Option<Box<dyn GridFormatter>>,
    /// Labels of the second coordinate.
    pub tick_formatter2: Option<Box<dyn GridFormatter>>,
}

/// Computes the grid lines and ticks of a curvilinear grid (matplotlib's
/// `GridFinder`, the engine of `GridHelperCurveLinear`).
#[derive(Debug)]
pub struct GridFinder {
    transform: AuxTransform,
    extreme_finder: Box<dyn ExtremeFinder>,
    grid_locator1: Box<dyn GridLocator>,
    grid_locator2: Box<dyn GridLocator>,
    tick_formatter1: Box<dyn GridFormatter>,
    tick_formatter2: Box<dyn GridFormatter>,
}

/// The number of points of each raw grid line.
const LINE_POINTS: usize = 100;

impl GridFinder {
    /// A grid through `transform` (curvilinear to data coordinates).
    pub fn new(transform: AuxTransform, options: GridFinderOptions) -> Self {
        let locator = || -> Box<dyn GridLocator> { Box::new(GridMaxNLocator::default()) };
        let formatter = || -> Box<dyn GridFormatter> { Box::new(FormatterPrettyPrint::default()) };
        Self {
            transform,
            extreme_finder: options
                .extreme_finder
                .unwrap_or_else(|| Box::new(ExtremeFinderSimple::new(20, 20))),
            grid_locator1: options.grid_locator1.unwrap_or_else(locator),
            grid_locator2: options.grid_locator2.unwrap_or_else(locator),
            tick_formatter1: options.tick_formatter1.unwrap_or_else(formatter),
            tick_formatter2: options.tick_formatter2.unwrap_or_else(formatter),
        }
    }

    /// The transform from curvilinear to data coordinates.
    pub fn transform(&self) -> &AuxTransform {
        &self.transform
    }

    /// The extreme finder.
    pub fn extreme_finder(&self) -> &dyn ExtremeFinder {
        self.extreme_finder.as_ref()
    }

    /// The range of curvilinear coordinates covering the data box `bbox`.
    pub fn extremes(&self, bbox: Bbox) -> Result<Bbox, GridError> {
        let inverse = self.transform.inverted().ok_or(GridError::Singular)?;
        Ok(self.extreme_finder.find_transformed_bbox(&inverse, bbox))
    }

    /// The levels of the first (`1`) or second coordinate between `v1`
    /// and `v2`.
    pub(crate) fn levels(
        &self,
        coordinate: Coordinate,
        v1: f64,
        v2: f64,
    ) -> Result<GridLevels, GridError> {
        match coordinate {
            Coordinate::Lon => self.grid_locator1.levels(v1, v2),
            Coordinate::Lat => self.grid_locator2.levels(v1, v2),
        }
    }

    /// The labels of `levels` of one coordinate on the `direction` side.
    pub(crate) fn labels(
        &self,
        coordinate: Coordinate,
        direction: SpineSide,
        factor: f64,
        levels: &[f64],
    ) -> Result<Vec<String>, GridError> {
        match coordinate {
            Coordinate::Lon => self.tick_formatter1.labels(direction, factor, levels),
            Coordinate::Lat => self.tick_formatter2.labels(direction, factor, levels),
        }
    }

    /// The grid lines and their crossings with the data box `bbox`
    /// (matplotlib's `get_grid_info`).
    pub fn grid_info(&self, bbox: Bbox) -> Result<GridInfo, GridError> {
        let extremes = self.extremes(bbox)?;
        let lon = self.levels(Coordinate::Lon, extremes.x0(), extremes.x1())?;
        let lat = self.levels(Coordinate::Lat, extremes.y0(), extremes.y1())?;
        let (lon_lines, lat_lines) = self.raw_grid_lines(&lon.values(), &lat.values(), extremes);
        let expanded = bbox.expanded(1.0 + 2e-10, 1.0 + 2e-10);
        Ok(GridInfo {
            extremes,
            lon: self.coordinate_grid(Coordinate::Lon, &lon, lon_lines, expanded)?,
            lat: self.coordinate_grid(Coordinate::Lat, &lat, lat_lines, expanded)?,
        })
    }

    /// The lines of one coordinate with their labelled crossings.
    fn coordinate_grid(
        &self,
        coordinate: Coordinate,
        levels: &GridLevels,
        lines: Vec<Vec<Point>>,
        bbox: Bbox,
    ) -> Result<CoordinateGrid, GridError> {
        let mut ticks: [Vec<GridTick>; 4] = Default::default();
        for (line, &level) in lines.iter().zip(&levels.levels) {
            for (side_ticks, crossings) in ticks.iter_mut().zip(line_box_crossings(line, bbox)) {
                side_ticks.extend(crossings.into_iter().map(|(loc, angle)| GridTick {
                    level,
                    loc,
                    angle,
                    label: String::new(),
                }));
            }
        }
        for (side, side_ticks) in SIDES.into_iter().zip(&mut ticks) {
            let side_levels: Vec<f64> = side_ticks.iter().map(|tick| tick.level).collect();
            let labels = self.labels(coordinate, side, levels.factor, &side_levels)?;
            for (tick, label) in side_ticks.iter_mut().zip(labels) {
                tick.label = label;
            }
        }
        Ok(CoordinateGrid { lines, ticks })
    }

    /// The polylines of constant `lon_values` and `lat_values` through
    /// the extremes `bbox`, `LINE_POINTS` points each (matplotlib's
    /// `_get_raw_grid_lines`).
    pub(crate) fn raw_grid_lines(
        &self,
        lon_values: &[f64],
        lat_values: &[f64],
        bbox: Bbox,
    ) -> (Vec<Vec<Point>>, Vec<Vec<Point>>) {
        let lons = linspace(bbox.x0(), bbox.x1(), LINE_POINTS);
        let lats = linspace(bbox.y0(), bbox.y1(), LINE_POINTS);
        let line = |points: &mut dyn Iterator<Item = Point>| -> Vec<Point> {
            points.map(|p| self.transform.transform_point(p)).collect()
        };
        let lon_lines = lon_values
            .iter()
            .map(|&lon| line(&mut lats.iter().map(|&lat| Point::new(lon, lat))))
            .collect();
        let lat_lines = lat_values
            .iter()
            .map(|&lat| line(&mut lons.iter().map(|&lon| Point::new(lon, lat))))
            .collect();
        (lon_lines, lat_lines)
    }
}

/// The two curvilinear coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Coordinate {
    /// The first coordinate, located by `grid_locator1`.
    Lon,
    /// The second coordinate, located by `grid_locator2`.
    Lat,
}

/// Where the polyline `xys` crosses the sides of `bbox` and at which
/// angle (degrees, 0 = moving right), interpolated between the points on
/// either side: the crossings with the left, right, bottom and top sides
/// (matplotlib's `_find_line_box_crossings`).
pub fn line_box_crossings(xys: &[Point], bbox: Bbox) -> [Vec<(Point, f64)>; 4] {
    let (xmin, xmax) = (bbox.xmin(), bbox.xmax());
    let (ymin, ymax) = (bbox.ymin(), bbox.ymax());
    let along_x = |p: Point| (p.x, p.y);
    let along_y = |p: Point| (p.y, p.x);
    let crossings = |coords: &dyn Fn(Point) -> (f64, f64),
                     u0: f64,
                     inside: &dyn Fn(f64) -> bool,
                     (vmin, vmax): (f64, f64),
                     place: &dyn Fn(f64, f64) -> Point| {
        xys.windows(2)
            .filter_map(|pair| {
                let (u, v) = coords(pair[0]);
                let (u1, v1) = coords(pair[1]);
                if inside(u) == inside(u1) {
                    return None;
                }
                let crossing = v + (u0 - u) * (v1 - v) / (u1 - u);
                let (dx, dy) = (pair[1].x - pair[0].x, pair[1].y - pair[0].y);
                (vmin <= crossing && crossing <= vmax)
                    .then(|| (place(u0, crossing), dy.atan2(dx).to_degrees()))
            })
            .collect()
    };
    let on_x = |u: f64, v: f64| Point::new(u, v);
    let on_y = |u: f64, v: f64| Point::new(v, u);
    [
        crossings(&along_x, xmin, &|u| u > xmin, (ymin, ymax), &on_x),
        crossings(&along_x, xmax, &|u| u < xmax, (ymin, ymax), &on_x),
        crossings(&along_y, ymin, &|u| u > ymin, (xmin, xmax), &on_y),
        crossings(&along_y, ymax, &|u| u < ymax, (xmin, xmax), &on_y),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossings_are_interpolated_with_their_angle() {
        let line = [
            Point::new(-1.0, 0.5),
            Point::new(1.0, 0.5),
            Point::new(1.0, 2.0),
            Point::new(f64::NAN, 0.5),
        ];
        let [left, right, bottom, top] = line_box_crossings(&line, Bbox::UNIT);
        assert_eq!(left, [(Point::new(0.0, 0.5), 0.0)]);
        assert_eq!(right, [(Point::new(1.0, 0.5), 0.0)]);
        assert!(bottom.is_empty(), "no crossing towards the NaN point");
        assert_eq!(top, [(Point::new(1.0, 1.0), 90.0)]);
    }

    #[test]
    fn grid_info_labels_the_crossings_of_each_side() {
        let shear = AuxTransform::from_fns(|x, y| (x, y - x), |x, y| (x, y + x));
        let finder = GridFinder::new(shear, GridFinderOptions::default());
        let info = finder
            .grid_info(Bbox::from_extents(0.0, 0.0, 10.0, 10.0))
            .unwrap();
        let bottom = info.lon.side_ticks(SpineSide::Bottom);
        assert_eq!(bottom.len(), 7);
        assert_eq!(bottom[1].label, r"$\mathdefault{1.5}$");
        assert!((bottom[1].loc.x - 1.5).abs() < 1e-12);
        assert_eq!(bottom[1].angle, 90.0);
        assert!(info.lon.lines.iter().all(|line| line.len() == LINE_POINTS));
        let singular = GridFinder::new(
            AuxTransform::affine(pltrs_core::Affine2D::scaling(0.0, 1.0)),
            GridFinderOptions::default(),
        );
        assert_eq!(singular.grid_info(Bbox::UNIT), Err(GridError::Singular));
    }
}
