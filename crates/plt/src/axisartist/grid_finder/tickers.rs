//! The extreme finder, grid locators and grid formatters of
//! matplotlib's `grid_finder` module.

use plt_core::path::DataLimits;
use plt_core::{Bbox, Point};

use super::{ExtremeFinder, GridError, GridFormatter, GridLevels, GridLocator};
use crate::axisartist::aux_transform::AuxTransform;
use crate::numeric::linspace;
use crate::spines::SpineSide;
use crate::ticker::{
    Formatter, Locator, Nbins, OffsetMode, ScalarFormatter, TickContext, TickerSettings,
};

/// `nx × ny` equispaced points of `bbox`, row by row (matplotlib's
/// `meshgrid` of two `linspace`s, flattened).
pub(crate) fn sample_grid(bbox: Bbox, nx: usize, ny: usize) -> Vec<Point> {
    let xs = linspace(bbox.x0(), bbox.x1(), nx);
    linspace(bbox.y0(), bbox.y1(), ny)
        .into_iter()
        .flat_map(|y| xs.iter().map(move |&x| Point::new(x, y)))
        .collect()
}

/// The bounding box of the finite `points` grown by `1 + 2 / nx` and
/// `1 + 2 / ny` around its centre (matplotlib's `Bbox.null()` updated from
/// the points, then `expanded`).
pub(crate) fn padded_extent(points: &[Point], nx: usize, ny: usize) -> Bbox {
    let mut limits = DataLimits::NULL;
    limits.update_from_points(points, true, true);
    limits
        .bbox
        .expanded(1.0 + 2.0 / nx as f64, 1.0 + 2.0 / ny as f64)
}

/// Samples `nx × ny` points of the box, maps them and pads their extent
/// by one sampling step on each side (matplotlib's
/// `ExtremeFinderSimple`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtremeFinderSimple {
    /// The number of samples along x.
    pub nx: usize,
    /// The number of samples along y.
    pub ny: usize,
}

impl ExtremeFinderSimple {
    /// A finder sampling `nx × ny` points.
    pub const fn new(nx: usize, ny: usize) -> Self {
        Self { nx, ny }
    }
}

impl ExtremeFinder for ExtremeFinderSimple {
    fn find_transformed_bbox(&self, transform: &AuxTransform, bbox: Bbox) -> Bbox {
        let points: Vec<Point> = sample_grid(bbox, self.nx, self.ny)
            .into_iter()
            .map(|p| transform.transform_point(p))
            .collect();
        padded_extent(&points, self.nx, self.ny)
    }
}

/// The ticker's [`MaxNLocator`](crate::ticker::MaxNLocator) as a grid
/// locator: nice levels with factor 1 (matplotlib's
/// `grid_finder.MaxNLocator`).
#[derive(Clone, Debug, PartialEq)]
pub struct MaxNLocator {
    locator: crate::ticker::MaxNLocator,
}

impl MaxNLocator {
    /// At most `nbins` intervals with the default steps.
    pub fn new(nbins: usize) -> Self {
        Self::from_locator(crate::ticker::MaxNLocator::new(Nbins::Fixed(nbins)))
    }

    /// The grid locator of a configured ticker locator (steps, integer,
    /// symmetric, prune).
    pub fn from_locator(locator: crate::ticker::MaxNLocator) -> Self {
        Self { locator }
    }
}

impl Default for MaxNLocator {
    /// `MaxNLocator()`: 10 bins.
    fn default() -> Self {
        Self::new(10)
    }
}

impl GridLocator for MaxNLocator {
    fn levels(&self, v1: f64, v2: f64) -> Result<GridLevels, GridError> {
        let settings = TickerSettings::from(&*crate::rc::params());
        let levels = self
            .locator
            .tick_values(v1, v2, &TickContext::detached(&settings))?;
        Ok(GridLevels {
            n: levels.len(),
            levels,
            factor: 1.0,
        })
    }
}

/// Fixed levels, those within the interval (matplotlib's
/// `grid_finder.FixedLocator`).
#[derive(Clone, Debug, PartialEq)]
pub struct FixedLocator {
    locs: Vec<f64>,
}

impl FixedLocator {
    /// Levels at `locs`.
    pub fn new(locs: impl Into<Vec<f64>>) -> Self {
        Self { locs: locs.into() }
    }
}

impl GridLocator for FixedLocator {
    fn levels(&self, v1: f64, v2: f64) -> Result<GridLevels, GridError> {
        let (low, high) = if v2 < v1 { (v2, v1) } else { (v1, v2) };
        let levels: Vec<f64> = self
            .locs
            .iter()
            .copied()
            .filter(|&l| low <= l && l <= high)
            .collect();
        Ok(GridLevels {
            n: levels.len(),
            levels,
            factor: 1.0,
        })
    }
}

/// Labels values like a `ScalarFormatter` without offset, in mathtext by
/// default (matplotlib's `FormatterPrettyPrint`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatterPrettyPrint {
    use_math_text: bool,
}

impl FormatterPrettyPrint {
    /// A formatter writing mathtext labels when `use_math_text`.
    pub const fn new(use_math_text: bool) -> Self {
        Self { use_math_text }
    }
}

impl Default for FormatterPrettyPrint {
    /// `FormatterPrettyPrint()`: mathtext labels.
    fn default() -> Self {
        Self::new(true)
    }
}

impl GridFormatter for FormatterPrettyPrint {
    fn labels(&self, _: SpineSide, _: f64, values: &[f64]) -> Result<Vec<String>, GridError> {
        let settings = TickerSettings::from(&*crate::rc::params());
        let formatter = ScalarFormatter::from_settings(&settings)
            .with_use_mathtext(self.use_math_text)
            .with_use_offset(OffsetMode::Off);
        Ok(formatter
            .format_ticks(values, &TickContext::detached(&settings))?
            .labels)
    }
}

/// Labels the levels found in a table, others through a fallback
/// formatter or not at all (matplotlib's `DictFormatter`).
#[derive(Debug)]
pub struct DictFormatter {
    format_dict: Vec<(f64, String)>,
    fallback: Option<Box<dyn GridFormatter>>,
}

impl DictFormatter {
    /// Labels `format_dict[value]` and, for values not in the table, the
    /// label of `fallback` (or none).
    pub fn new(
        format_dict: impl IntoIterator<Item = (f64, String)>,
        fallback: Option<Box<dyn GridFormatter>>,
    ) -> Self {
        Self {
            format_dict: format_dict.into_iter().collect(),
            fallback,
        }
    }
}

impl GridFormatter for DictFormatter {
    #[expect(
        clippy::float_cmp,
        reason = "matplotlib looks the values up as dictionary keys"
    )]
    fn labels(
        &self,
        direction: SpineSide,
        factor: f64,
        values: &[f64],
    ) -> Result<Vec<String>, GridError> {
        let fallback = match &self.fallback {
            Some(formatter) => formatter.labels(direction, factor, values)?,
            None => vec![String::new(); values.len()],
        };
        Ok(values
            .iter()
            .zip(fallback)
            .map(|(&value, fallback)| {
                self.format_dict
                    .iter()
                    .find(|(key, _)| *key == value)
                    .map_or(fallback, |(_, label)| label.clone())
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_levels_inside_either_order() {
        let locator = FixedLocator::new([0.0, 0.5, 1.5, 3.0]);
        let found = locator.levels(2.0, 0.0).unwrap();
        assert_eq!(
            (found.levels, found.n, found.factor),
            (vec![0.0, 0.5, 1.5], 3, 1.0)
        );
        let nice = MaxNLocator::new(4).levels(0.0, 10.0).unwrap();
        assert_eq!(nice.values(), [0.0, 2.5, 5.0, 7.5, 10.0]);
    }

    #[test]
    fn formatters_label_in_mathtext_or_from_a_table() {
        let pretty = FormatterPrettyPrint::default()
            .labels(SpineSide::Bottom, 1.0, &[-1.5, 0.0, 1.5])
            .unwrap();
        assert_eq!(pretty[0], "$\\mathdefault{\u{2212}1.5}$");
        let table = DictFormatter::new([(0.5, "half".to_owned())], None);
        let labels = table.labels(SpineSide::Left, 1.0, &[0.5, 1.0]).unwrap();
        assert_eq!(labels, ["half", ""]);
        let fallback = DictFormatter::new(
            [(0.5, "half".to_owned())],
            Some(Box::new(FormatterPrettyPrint::new(false))),
        );
        let labels = fallback.labels(SpineSide::Left, 1.0, &[0.5, 1.0]).unwrap();
        assert_eq!(labels[1], "1.0");
    }

    #[test]
    fn extremes_skip_points_mapped_to_nan() {
        let half_plane = AuxTransform::from_fns(
            |x, y| if x < 0.0 { (f64::NAN, y) } else { (x, y) },
            |x, y| (x, y),
        );
        let found = ExtremeFinderSimple::new(3, 3)
            .find_transformed_bbox(&half_plane, Bbox::from_extents(-1.0, 0.0, 1.0, 1.0));
        // The samples at x = 0 and 1 remain, padded by a third of the span.
        assert!((found.x0() + 1.0 / 3.0).abs() < 1e-12);
        assert!((found.x1() - 4.0 / 3.0).abs() < 1e-12);
    }
}
