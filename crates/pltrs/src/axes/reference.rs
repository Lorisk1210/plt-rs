//! Reference lines and spans: `axhline`, `axvline`, `axline`, `axhspan`,
//! `axvspan`.

use pltrs_core::Point;

use super::children::DataUpdate;
use super::{Axes, PlotError};
use crate::artist::{Artist, ArtistHandle};
use crate::lines::{AxLineThrough, InfiniteLine, Line2D, LineArtist, LineProps};
use crate::patches::{Patch, PatchArtist, PatchShape, Rectangle};
use crate::polar::GRIDLINE_INTERPOLATION_STEPS;
use crate::scale::ScaleName;
use crate::ticker::AxisName;
use crate::transforms::Transform;
use crate::units::{AxisValue, UnitSources};

impl Axes {
    /// A horizontal line at data height `y` (a number, date or category)
    /// across the whole axes (matplotlib's `axhline`).
    pub fn axhline(&self, y: impl Into<AxisValue>, props: LineProps) -> Line2D {
        self.axhline_between(y, 0.0, 1.0, props)
    }

    /// A horizontal line at data height `y` from axes fraction `xmin` to
    /// `xmax`.
    pub fn axhline_between(
        &self,
        y: impl Into<AxisValue>,
        xmin: f64,
        xmax: f64,
        props: LineProps,
    ) -> Line2D {
        let y = y.into();
        let sources = UnitSources::of_value(AxisName::Y, &y);
        let y = self.yaxis().convert_value(y);
        let mut line = LineArtist::new(vec![xmin, xmax], vec![y, y], crate::rc::params());
        line.unit_sources = sources;
        self.add_reference_line(line, props, self.trans_yaxis(), AxisName::Y, y)
    }

    /// A vertical line at data position `x` (a number, date or category)
    /// across the whole axes (matplotlib's `axvline`).
    pub fn axvline(&self, x: impl Into<AxisValue>, props: LineProps) -> Line2D {
        self.axvline_between(x, 0.0, 1.0, props)
    }

    /// A vertical line at data position `x` from axes fraction `ymin` to
    /// `ymax`.
    pub fn axvline_between(
        &self,
        x: impl Into<AxisValue>,
        ymin: f64,
        ymax: f64,
        props: LineProps,
    ) -> Line2D {
        let x = x.into();
        let sources = UnitSources::of_value(AxisName::X, &x);
        let x = self.xaxis().convert_value(x);
        let mut line = LineArtist::new(vec![x, x], vec![ymin, ymax], crate::rc::params());
        line.unit_sources = sources;
        self.add_reference_line(line, props, self.trans_xaxis(), AxisName::X, x)
    }

    /// An infinite line through `xy1` and a second point or with a slope
    /// (matplotlib's `axline`). Its points extend the data limits unless
    /// the line is given a transform of its own.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::lines::AxLineThrough;
    /// use pltrs::transforms::Point;
    /// let ax = Figure::new().subplot();
    /// ax.axline(Point::new(0.0, 0.0), AxLineThrough::Slope(1.0), LineProps::default())?;
    /// ax.set_xscale("log")?;
    /// assert!(ax.axline(Point::new(1.0, 1.0), AxLineThrough::Slope(1.0), LineProps::default()).is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a slope is given while either axis has a non-linear scale, as
    /// a slope is only meaningful on linear axes.
    pub fn axline(
        &self,
        xy1: Point,
        through: AxLineThrough,
        props: LineProps,
    ) -> Result<Line2D, PlotError> {
        if matches!(through, AxLineThrough::Slope(_))
            && (self.xscale() != ScaleName::Linear || self.yscale() != ScaleName::Linear)
        {
            return Err(PlotError::SlopeOnNonlinearScale);
        }
        let mut points = vec![xy1];
        if let AxLineThrough::Point(xy2) = through {
            points.push(xy2);
        }
        // matplotlib's `AxLine` is a line over the diagonal of the unit
        // square, which its transform maps onto the part of the line in
        // view: its data, path and extent are those of the diagonal.
        let mut line = LineArtist::new(vec![0.0, 1.0], vec![0.0, 1.0], crate::rc::params());
        line.infinite = Some(InfiniteLine { xy1, through });
        if props.base.transform.is_some() {
            points.clear();
        }
        props.apply(&mut line);
        line.base_mut()
            .transform
            .get_or_insert_with(|| self.trans_data());
        Ok(Line2D::from_raw(
            self.adopt(line, &DataUpdate::data(points)),
        ))
    }

    /// A horizontal band between data heights `ymin` and `ymax` (numbers,
    /// dates or categories) across the whole axes (matplotlib's
    /// `axhspan`).
    pub fn axhspan(
        &self,
        ymin: impl Into<AxisValue>,
        ymax: impl Into<AxisValue>,
    ) -> Patch<Rectangle> {
        self.axhspan_between(ymin, ymax, 0.0, 1.0)
    }

    /// A horizontal band between data heights `ymin` and `ymax`, from axes
    /// fraction `xmin` to `xmax`.
    pub fn axhspan_between(
        &self,
        ymin: impl Into<AxisValue>,
        ymax: impl Into<AxisValue>,
        xmin: f64,
        xmax: f64,
    ) -> Patch<Rectangle> {
        let yaxis = self.yaxis();
        let (ymin, ymax) = (yaxis.convert_value(ymin), yaxis.convert_value(ymax));
        let band = Rectangle::new(Point::new(xmin, ymin), xmax - xmin, ymax - ymin);
        bent_like_gridlines(self.add_patch_in(band, self.trans_yaxis(), (false, true)))
    }

    /// A vertical band between data positions `xmin` and `xmax` (numbers,
    /// dates or categories) across the whole axes (matplotlib's
    /// `axvspan`).
    pub fn axvspan(
        &self,
        xmin: impl Into<AxisValue>,
        xmax: impl Into<AxisValue>,
    ) -> Patch<Rectangle> {
        self.axvspan_between(xmin, xmax, 0.0, 1.0)
    }

    /// A vertical band between data positions `xmin` and `xmax`, from axes
    /// fraction `ymin` to `ymax`.
    pub fn axvspan_between(
        &self,
        xmin: impl Into<AxisValue>,
        xmax: impl Into<AxisValue>,
        ymin: f64,
        ymax: f64,
    ) -> Patch<Rectangle> {
        let xaxis = self.xaxis();
        let (xmin, xmax) = (xaxis.convert_value(xmin), xaxis.convert_value(xmax));
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
        Patch::from_raw(self.adopt_in_autoscale(patch, &update))
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
        Line2D::from_raw(self.adopt_in_autoscale(line, &update))
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

#[cfg(test)]
mod tests {
    use pltrs_core::Point;

    use crate::axes::PlotError;
    use crate::figure::Figure;
    use crate::lines::{AxLineThrough, LineProps};

    #[test]
    fn axlines_in_their_own_coordinates_leave_the_data_limits_alone() {
        let ax = Figure::new().subplot();
        let fractions = LineProps::default().transform(ax.trans_axes());
        ax.axline(Point::new(0.2, 0.0), AxLineThrough::Slope(0.5), fractions)
            .unwrap();
        assert!(ax.data_lim().bbox.x0().is_infinite());
        ax.axline(
            Point::new(1.0, 2.0),
            AxLineThrough::Point(Point::new(3.0, 4.0)),
            LineProps::default(),
        )
        .unwrap();
        let limits = ax.data_lim().bbox;
        assert_eq!((limits.x0(), limits.y1()), (1.0, 4.0));
    }

    #[test]
    fn slopes_need_linear_scales() {
        // matplotlib's test_axline_args.
        let ax = Figure::new().subplot();
        let slope = || {
            ax.axline(
                Point::ORIGIN,
                AxLineThrough::Slope(1.0),
                LineProps::default(),
            )
        };
        assert!(slope().is_ok());
        ax.set_yscale("log").unwrap();
        assert!(matches!(slope(), Err(PlotError::SlopeOnNonlinearScale)));
        let through = AxLineThrough::Point(Point::new(2.0, 3.0));
        assert!(
            ax.axline(Point::new(1.0, 1.0), through, LineProps::default())
                .is_ok()
        );
    }
}
