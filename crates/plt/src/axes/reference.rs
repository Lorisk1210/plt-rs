//! Reference lines and spans: `axhline`, `axvline`, `axline`, `axhspan`,
//! `axvspan`.

use plt_core::Point;

use super::Axes;
use super::children::DataUpdate;
use crate::artist::{Artist, ArtistHandle};
use crate::lines::{AxLineThrough, InfiniteLine, Line2D, LineArtist, LineProps};
use crate::patches::{Patch, PatchArtist, PatchShape, Rectangle};
use crate::polar::GRIDLINE_INTERPOLATION_STEPS;
use crate::ticker::AxisName;
use crate::transforms::Transform;

impl Axes {
    /// A horizontal line at data height `y` across the whole axes
    /// (matplotlib's `axhline`).
    pub fn axhline(&self, y: f64, props: LineProps) -> Line2D {
        self.axhline_between(y, 0.0, 1.0, props)
    }

    /// A horizontal line at data height `y` from axes fraction `xmin` to
    /// `xmax`.
    pub fn axhline_between(&self, y: f64, xmin: f64, xmax: f64, props: LineProps) -> Line2D {
        let line = LineArtist::new(vec![xmin, xmax], vec![y, y], crate::rc::params());
        self.add_reference_line(line, props, self.trans_yaxis(), AxisName::Y, y)
    }

    /// A vertical line at data position `x` across the whole axes
    /// (matplotlib's `axvline`).
    pub fn axvline(&self, x: f64, props: LineProps) -> Line2D {
        self.axvline_between(x, 0.0, 1.0, props)
    }

    /// A vertical line at data position `x` from axes fraction `ymin` to
    /// `ymax`.
    pub fn axvline_between(&self, x: f64, ymin: f64, ymax: f64, props: LineProps) -> Line2D {
        let line = LineArtist::new(vec![x, x], vec![ymin, ymax], crate::rc::params());
        self.add_reference_line(line, props, self.trans_xaxis(), AxisName::X, x)
    }

    /// An infinite line through `xy1` and a second point or with a slope
    /// (matplotlib's `axline`).
    pub fn axline(&self, xy1: Point, through: AxLineThrough, props: LineProps) -> Line2D {
        let mut points = vec![xy1];
        if let AxLineThrough::Point(xy2) = through {
            points.push(xy2);
        }
        let (x, y) = points.iter().map(|p| (p.x, p.y)).unzip();
        let mut line = LineArtist::new(x, y, crate::rc::params());
        line.infinite = Some(InfiniteLine { xy1, through });
        props.apply(&mut line);
        line.base_mut()
            .transform
            .get_or_insert_with(|| self.trans_data());
        Line2D::from_raw(self.adopt(line, &DataUpdate::data(points)))
    }

    /// A horizontal band between data heights `ymin` and `ymax` across the
    /// whole axes (matplotlib's `axhspan`).
    pub fn axhspan(&self, ymin: f64, ymax: f64) -> Patch<Rectangle> {
        self.axhspan_between(ymin, ymax, 0.0, 1.0)
    }

    /// A horizontal band between data heights `ymin` and `ymax`, from axes
    /// fraction `xmin` to `xmax`.
    pub fn axhspan_between(&self, ymin: f64, ymax: f64, xmin: f64, xmax: f64) -> Patch<Rectangle> {
        let band = Rectangle::new(Point::new(xmin, ymin), xmax - xmin, ymax - ymin);
        bent_like_gridlines(self.add_patch_in(band, self.trans_yaxis(), (false, true)))
    }

    /// A vertical band between data positions `xmin` and `xmax` across the
    /// whole axes (matplotlib's `axvspan`).
    pub fn axvspan(&self, xmin: f64, xmax: f64) -> Patch<Rectangle> {
        self.axvspan_between(xmin, xmax, 0.0, 1.0)
    }

    /// A vertical band between data positions `xmin` and `xmax`, from axes
    /// fraction `ymin` to `ymax`.
    pub fn axvspan_between(&self, xmin: f64, xmax: f64, ymin: f64, ymax: f64) -> Patch<Rectangle> {
        let band = Rectangle::new(Point::new(xmin, ymin), xmax - xmin, ymax - ymin);
        bent_like_gridlines(self.add_patch_in(band, self.trans_xaxis(), (true, false)))
    }

    /// Adds a patch whose outline is given in `transform` coordinates,
    /// extending the data limits in the directions that are data values and
    /// autoscaling the `autoscale` directions.
    pub(crate) fn add_patch_in<S: PatchShape>(
        &self,
        shape: S,
        transform: Transform,
        autoscale: (bool, bool),
    ) -> Patch<S> {
        let mut patch = PatchArtist::new(shape, crate::rc::params());
        let update = DataUpdate {
            points: patch.data_points(),
            update: transform.data_directions(self.id()),
            autoscale,
        };
        patch.base_mut().transform = Some(transform);
        Patch::from_raw(self.adopt(patch, &update))
    }

    /// Adds a line spanning axes fractions along one axis at data value
    /// `at` along `axis`; autoscales `axis` only when `at` lies outside the
    /// current view (matplotlib's `axhline`/`axvline` rule).
    fn add_reference_line(
        &self,
        mut line: LineArtist,
        props: LineProps,
        transform: Transform,
        axis: AxisName,
        at: f64,
    ) -> Line2D {
        let (lo, hi) = match axis {
            AxisName::X => self.xlim(),
            AxisName::Y => self.ylim(),
        };
        let outside = at < lo.min(hi) || at > lo.max(hi);
        props.apply(&mut line);
        // matplotlib bends reference lines like gridlines on projections.
        line.interpolation_steps = GRIDLINE_INTERPOLATION_STEPS;
        line.recache();
        let points = line.drawn_points();
        line.base_mut().transform = Some(transform);
        let along_x = axis == AxisName::X;
        let update = DataUpdate {
            points,
            update: (along_x, !along_x),
            autoscale: (along_x && outside, !along_x && outside),
        };
        Line2D::from_raw(self.adopt(line, &update))
    }
}

/// A span whose outline bends with projections such as the polar one,
/// like a gridline (matplotlib sets `GRIDLINE_INTERPOLATION_STEPS` on the
/// paths of `axhspan` and `axvspan`).
fn bent_like_gridlines(span: Patch<Rectangle>) -> Patch<Rectangle> {
    span.raw().with_mut::<PatchArtist<Rectangle>, _>(|patch| {
        patch.interpolation_steps = GRIDLINE_INTERPOLATION_STEPS;
    });
    span
}
