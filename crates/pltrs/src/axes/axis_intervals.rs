//! The intervals of one axis: its view and data limits, the smallest
//! positive datum and how many ticks fit along it (matplotlib's
//! `Axis.get_view_interval`, `set_view_interval`, `get_data_interval`,
//! `set_data_interval`, `get_minpos` and `get_tick_space`), and the
//! description of the axis tick helpers consult ([`Axis::info`]).

use pltrs_core::numeric::{py_max, py_min};

use super::axis_handle::Axis;
use super::state::AxesState;
use crate::artist::ArtistHandle;
use crate::ticker::{AxisInfo, AxisName, TickSpace, TickerError};

/// The interval `(old_min, old_max)` widened to include `vmin` and `vmax`,
/// keeping its orientation, as matplotlib's interval setters widen one
/// (`set_view_interval` and `set_data_interval` without `ignore`): with
/// Python's `min` and `max` over `vmin`, `vmax` and the old end, in that
/// order, which keep a NaN met first, and the reversed form for an
/// interval not increasing (NaN ends included).
pub(crate) fn widened((old_min, old_max): (f64, f64), vmin: f64, vmax: f64) -> (f64, f64) {
    let lowest = |end| py_min(py_min(vmin, vmax), end);
    let highest = |end| py_max(py_max(vmin, vmax), end);
    if old_min < old_max {
        (lowest(old_min), highest(old_max))
    } else {
        (highest(old_min), lowest(old_max))
    }
}

impl Axis {
    /// The view limits along this axis, autoscaled first, in the order
    /// they were set (matplotlib's `Axis.get_view_interval`).
    ///
    /// ```
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// ax.set_ylim(3.0, 1.0)?;
    /// assert_eq!(ax.yaxis().view_interval(), (3.0, 1.0));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn view_interval(&self) -> (f64, f64) {
        match self.name {
            AxisName::X => self.axes.xlim(),
            AxisName::Y => self.axes.ylim(),
        }
    }

    /// Sets the view limits along this axis to `(vmin, vmax)` when
    /// `ignore`, else widens them to include both, keeping their
    /// direction; autoscaling stays as it was and no limit callbacks run
    /// (matplotlib's `Axis.set_view_interval`).
    ///
    /// ```
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// ax.set_xlim(2.0, 0.0)?;
    /// ax.xaxis().set_view_interval(-1.0, 1.0, false);
    /// assert_eq!(ax.xlim(), (2.0, -1.0));
    /// ax.xaxis().set_view_interval(-1.0, 1.0, true);
    /// assert_eq!(ax.xlim(), (-1.0, 1.0));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    pub fn set_view_interval(&self, vmin: f64, vmax: f64, ignore: bool) -> &Self {
        self.axes.unstale_view();
        let name = self.name;
        self.axes.raw().with_mut::<AxesState, _>(|axes| {
            if ignore {
                axes.set_view_interval(name, (vmin, vmax));
            } else {
                axes.include_in_view(name, vmin, vmax);
            }
        });
        self.axes.raw().figure_state().borrow_mut().stale = true;
        self
    }

    /// The interval the data span along this axis (matplotlib's
    /// `Axis.get_data_interval`): infinite bounds while there are no data.
    pub fn data_interval(&self) -> (f64, f64) {
        let name = self.name;
        self.axes
            .raw()
            .with::<AxesState, _>(|axes| axes.data_interval(name).0)
    }

    /// Sets the data interval along this axis to `(vmin, vmax)` when
    /// `ignore`, else widens it to include both (matplotlib's
    /// `Axis.set_data_interval`); the smallest positive datum stays. The
    /// view follows at the next autoscale, which this does not ask for,
    /// as matplotlib's does not (cartopy sets the data and view limits of
    /// its maps so).
    ///
    /// ```
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// ax.xaxis().set_data_interval(1.0, 2.0, true);
    /// ax.xaxis().set_data_interval(0.0, 1.5, false);
    /// assert_eq!(ax.xaxis().data_interval(), (0.0, 2.0));
    /// assert_eq!(ax.xlim(), (0.0, 1.0));
    /// ```
    pub fn set_data_interval(&self, vmin: f64, vmax: f64, ignore: bool) -> &Self {
        let name = self.name;
        self.axes.raw().with_mut::<AxesState, _>(|axes| {
            if ignore {
                axes.set_data_interval(name, vmin, vmax);
            } else {
                axes.include_in_data(name, vmin, vmax);
            }
        });
        self
    }

    /// The smallest positive value plotted along this axis, infinite
    /// while there is none (matplotlib's `Axis.get_minpos`).
    pub fn minpos(&self) -> f64 {
        let name = self.name;
        self.axes
            .raw()
            .with::<AxesState, _>(|axes| axes.data_interval(name).1)
    }

    /// `values` through the axis' scale (matplotlib's
    /// `axis.get_transform().transform(values)`).
    ///
    /// ```
    /// use pltrs::scale::ScaleName;
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// ax.set_xscale(ScaleName::Log)?;
    /// assert_eq!(ax.xaxis().scaled(&[1.0, 100.0]), [0.0, 2.0]);
    /// assert_eq!(ax.yaxis().scaled(&[1.0, 100.0]), [1.0, 100.0], "linear");
    /// # Ok::<(), pltrs::scale::ScaleError>(())
    /// ```
    pub fn scaled(&self, values: &[f64]) -> Vec<f64> {
        let mut scaled = values.to_vec();
        // The map runs apart from the figure, which a map of the
        // program's may read and change.
        if let Some(map) = self.read(|axis| axis.map().cloned()) {
            map.forward_column(&mut scaled);
        }
        scaled
    }

    /// Maps every value of `values` in place through the axis' scale and
    /// back, as matplotlib's collections in data coordinates find their
    /// data limits (`get_datalim`): what the scale clips comes back
    /// clipped (a logarithm's 0 as `base^-1000`), the rest as the floating
    /// point arithmetic leaves it.
    pub(crate) fn round_trip(&self, values: &mut [f64]) {
        // The map runs apart from the figure, as in `scaled`.
        if let Some(map) = self.read(|axis| axis.map().cloned()) {
            map.forward_all(values);
            map.inverse_all(values);
        }
    }

    /// Rescales the view along this axis and the axes sharing it at the
    /// next layout, as after its major locator changed (matplotlib's
    /// `_request_autoscale_view`); `tight` replaces the tight setting of
    /// this axes.
    pub fn request_autoscale(&self, tight: Option<bool>) -> &Self {
        self.update_shared(|axis| axis.autoscale.stale = true);
        if let Some(tight) = tight {
            self.axes
                .raw()
                .with_mut::<AxesState, _>(|axes| axes.tight = tight);
        }
        self
    }

    /// How many ticks fit along this axis in the figure's current
    /// layout: its length in points over three (x) or two (y) times the
    /// tick label size (matplotlib's `Axis.get_tick_space`), none along an
    /// axes placed at NaN.
    ///
    /// ```
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// let (x, y) = (ax.xaxis().tick_space().count()?, ax.yaxis().tick_space().count()?);
    /// assert!(x > y / 2);
    /// ax.set_position([0.6, 0.1, -0.5, 0.5]);
    /// assert!(ax.xaxis().tick_space().count()? < 0);
    /// # Ok::<(), pltrs::ticker::TickerError>(())
    /// ```
    pub fn tick_space(&self) -> TickSpace {
        let state = self.axes.raw().figure_state().borrow();
        let points = state.axis_length_points(self.axes.id(), self.name);
        drop(state);
        self.read(|axis| axis.tick_space(points.unwrap_or(0.0)))
    }

    /// What a locator or formatter attached to this axis knows of it: its
    /// intervals, smallest positive datum, tick space, scale and major
    /// tick locations (matplotlib's helpers query their `Axis` for them).
    ///
    /// # Errors
    ///
    /// When the major locator fails.
    ///
    /// ```
    /// let ax = pltrs::figure::Figure::new().subplot();
    /// ax.set_xlim(0.0, 2.0)?;
    /// let info = ax.xaxis().info()?;
    /// assert_eq!(info.view_interval, (0.0, 2.0));
    /// assert_eq!(info.major_ticks.first(), Some(&0.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn info(&self) -> Result<AxisInfo, TickerError> {
        let major_ticks = self.ticklocs(false)?;
        let view_interval = self.view_interval();
        let (data_interval, minpos) = (self.data_interval(), self.minpos());
        let tick_space = self.tick_space();
        Ok(self.read(|axis| AxisInfo {
            name: self.name,
            view_interval,
            data_interval,
            minpos,
            shared_minpos: None,
            tick_space,
            major_ticks,
            scale: axis.scale().hint(),
            three_d: false,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::widened;
    use crate::figure::Figure;
    use pltrs_core::Point;

    #[test]
    fn data_intervals_follow_the_data_and_keep_minpos() {
        let ax = Figure::new().subplot();
        assert_eq!(
            ax.xaxis().data_interval(),
            (f64::INFINITY, f64::NEG_INFINITY)
        );
        ax.update_datalim(&[Point::new(0.5, 2.0), Point::new(3.0, -1.0)], true, true);
        assert_eq!(ax.xaxis().data_interval(), (0.5, 3.0));
        assert_eq!(ax.yaxis().data_interval(), (-1.0, 2.0));
        assert_eq!(ax.xaxis().minpos(), 0.5);
        ax.yaxis().set_data_interval(5.0, -5.0, true);
        assert_eq!(ax.yaxis().data_interval(), (5.0, -5.0));
        assert_eq!(ax.yaxis().minpos(), 2.0);
        // Widening keeps the orientation; a NaN first wins, as in Python.
        ax.yaxis().set_data_interval(7.0, 0.0, false);
        assert_eq!(ax.yaxis().data_interval(), (7.0, -5.0));
        ax.yaxis().set_data_interval(f64::NAN, 1.0, false);
        let (low, high) = ax.yaxis().data_interval();
        assert!(low.is_nan() && high.is_nan());
        ax.yaxis().set_data_interval(5.0, 4.0, false);
        assert_eq!(ax.yaxis().data_interval(), (5.0, 4.0));
    }

    #[test]
    fn widening_follows_pythons_min_and_max() {
        assert_eq!(widened((0.0, 1.0), 2.0, -1.0), (-1.0, 2.0));
        assert_eq!(widened((1.0, 0.0), 2.0, -1.0), (2.0, -1.0));
        let (low, high) = widened((0.0, 1.0), f64::NAN, 4.0);
        assert!(low.is_nan() && high.is_nan());
        assert_eq!(widened((0.0, 1.0), 0.5, f64::NAN), (0.0, 1.0));
        assert_eq!(widened((f64::NAN, f64::NAN), 5.0, 4.0), (5.0, 4.0));
    }

    #[test]
    fn view_intervals_widen_unless_ignored() {
        let ax = Figure::new().subplot();
        ax.set_xlim(0.0, 1.0).expect("finite limits");
        ax.xaxis().set_view_interval(0.25, 2.0, false);
        assert_eq!(ax.xaxis().view_interval(), (0.0, 2.0));
        ax.xaxis().set_view_interval(0.25, 0.5, true);
        assert_eq!(ax.xaxis().view_interval(), (0.25, 0.5));
    }
}
