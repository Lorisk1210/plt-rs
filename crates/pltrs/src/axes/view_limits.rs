//! Explicit view limits and bounds, the autoscaling switches, margins and
//! sticky edges (matplotlib's `set_xlim`, `set_xbound`, `autoscale`,
//! `margins` and `use_sticky_edges`).

use super::limits::oriented;
use super::{Axes, AxesState, AxisName, PlotError};
use crate::artist::ArtistHandle;
use crate::scale::ScaleName;
use crate::ticker::AutolimitMode;

/// New view limits for one axis (matplotlib's `set_xlim(left, right,
/// auto=...)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    /// The new left (bottom) limit; `None` keeps the current one.
    pub low: Option<f64>,
    /// The new right (top) limit; `None` keeps the current one.
    pub high: Option<f64>,
    /// Turns autoscaling of the axis on or off; `None` leaves it
    /// (matplotlib's `auto`, off by default).
    pub auto: Option<bool>,
}

impl Limits {
    /// Both limits, turning autoscaling off.
    pub const fn new(low: f64, high: f64) -> Self {
        Self {
            low: Some(low),
            high: Some(high),
            auto: Some(false),
        }
    }
}

impl Default for Limits {
    /// No change of the limits; autoscaling off.
    fn default() -> Self {
        Self {
            low: None,
            high: None,
            auto: Some(false),
        }
    }
}

impl Axes {
    /// Sets the view limits of one axis and of the axes sharing it
    /// (matplotlib's `set_xlim`/`set_ylim`), and returns them as set:
    /// singular limits are expanded, limits a log axis cannot show keep
    /// their current value, and `low > high` inverts the axis.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::{AxisName, Limits};
    /// let ax = Figure::new().subplot();
    /// ax.plot([0.0, 10.0], [0.0, 5.0])?;
    /// let limits = Limits { high: Some(20.0), ..Limits::default() };
    /// assert_eq!(ax.set_limits_with(AxisName::X, limits)?, (-0.5, 20.0));
    /// assert!(!ax.autoscale_on(Some(AxisName::X)));
    /// # Ok::<(), pltrs::axes::PlotError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a limit is NaN or infinite.
    pub fn set_limits_with(&self, axis: AxisName, limits: Limits) -> Result<(f64, f64), PlotError> {
        let set = self.set_limits_unobserved(axis, limits, true)?;
        // matplotlib's `xlim_changed`/`ylim_changed` observers.
        crate::colorbar::follow_view(self);
        super::run_limit_callbacks(self.raw.figure_state());
        Ok(set)
    }

    /// Sets the view limits of one axis as [`Axes::set_limits_with`] does,
    /// but of this axes alone and without telling anyone (matplotlib's
    /// `set_xlim(..., emit=False)`): the axes sharing the axis keep their
    /// limits and no limit callback runs.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::axes::{AxisName, Limits};
    /// let fig = Figure::new();
    /// let (top, bottom) = (fig.add_subplot(2, 1, 1)?, fig.add_subplot(2, 1, 2)?);
    /// bottom.share_x(&top)?;
    /// top.set_limits_quietly(AxisName::X, Limits::new(2.0, 3.0))?;
    /// assert_eq!((top.xlim(), bottom.xlim()), ((2.0, 3.0), (0.0, 1.0)));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a limit is NaN or infinite.
    pub fn set_limits_quietly(
        &self,
        axis: AxisName,
        limits: Limits,
    ) -> Result<(f64, f64), PlotError> {
        self.set_limits_unobserved(axis, limits, false)
    }

    /// [`Axes::set_limits_with`] without telling the colorbar drawn in
    /// this axes, which sets its own limits this way; with `emit`, the
    /// axes sharing the axis follow and the limit callbacks are queued.
    fn set_limits_unobserved(
        &self,
        axis: AxisName,
        limits: Limits,
        emit: bool,
    ) -> Result<(f64, f64), PlotError> {
        if [limits.low, limits.high]
            .into_iter()
            .flatten()
            .any(|limit| !limit.is_finite())
        {
            return Err(PlotError::NonFiniteLimit(axis));
        }
        let log = self
            .raw
            .with::<AxesState, _>(|a| a.axis(axis).scale().name() == ScaleName::Log);
        let replaced = |limit: Option<f64>| limit.is_none_or(|v| log && v <= 0.0);
        if replaced(limits.low) || replaced(limits.high) {
            // matplotlib reads the current limits (autoscaled first) only
            // when it needs them.
            self.unstale_view();
        }
        let current = self.raw.with::<AxesState, _>(|a| a.view_interval(axis));
        let requested = (
            limits.low.unwrap_or(current.0),
            limits.high.unwrap_or(current.1),
        );
        self.raw.figure_state().borrow_mut().set_axis_limits(
            self.id(),
            axis,
            requested,
            limits.auto,
            emit,
        );
        Ok(self.raw.with::<AxesState, _>(|a| a.view_interval(axis)))
    }

    /// Sets the view limits of one axis, and of the axes sharing it, and
    /// turns its autoscaling off (matplotlib's `set_xlim(low, high)`); see
    /// [`Axes::set_limits_with`].
    ///
    /// # Errors
    ///
    /// When a limit is NaN or infinite; then the view is unchanged.
    pub fn set_limits(&self, axis: AxisName, low: f64, high: f64) -> Result<&Self, PlotError> {
        self.set_limits_with(axis, Limits::new(low, high))?;
        Ok(self)
    }

    /// [`Axes::set_limits`] for a colorbar drawing into this axes: its
    /// own extensions need not follow.
    pub(crate) fn set_view_limits(
        &self,
        axis: AxisName,
        low: f64,
        high: f64,
    ) -> Result<&Self, PlotError> {
        self.set_limits_unobserved(axis, Limits::new(low, high), true)?;
        Ok(self)
    }

    /// The view limits of one axis in increasing order, whatever its
    /// orientation (matplotlib's `get_xbound`/`get_ybound`).
    pub fn bound(&self, axis: AxisName) -> (f64, f64) {
        let (v0, v1) = match axis {
            AxisName::X => self.xlim(),
            AxisName::Y => self.ylim(),
        };
        oriented((v0, v1), false)
    }

    /// Sets the lower and upper view bound of one axis, keeping its
    /// orientation and autoscaling switch; `None` keeps a bound
    /// (matplotlib's `set_xbound`/`set_ybound`).
    ///
    /// # Errors
    ///
    /// When a bound is NaN or infinite.
    pub fn set_bound(
        &self,
        axis: AxisName,
        lower: Option<f64>,
        upper: Option<f64>,
    ) -> Result<&Self, PlotError> {
        let (old_lower, old_upper) = self.bound(axis);
        let inverted = self.axis_of(axis).inverted();
        let (low, high) = oriented(
            (lower.unwrap_or(old_lower), upper.unwrap_or(old_upper)),
            inverted,
        );
        self.set_limits_with(
            axis,
            Limits {
                low: Some(low),
                high: Some(high),
                auto: None,
            },
        )?;
        Ok(self)
    }

    /// Whether one axis (or, for `None`, every axis) follows the data
    /// (matplotlib's `get_autoscalex_on`, `get_autoscale_on`).
    pub fn autoscale_on(&self, axis: Option<AxisName>) -> bool {
        self.raw.with::<AxesState, _>(|a| match axis {
            Some(name) => a.axis(name).autoscale.enabled,
            None => a.xaxis.autoscale.enabled && a.yaxis.autoscale.enabled,
        })
    }

    /// Turns autoscaling of one axis on or off without autoscaling now
    /// (matplotlib's `set_autoscalex_on`/`set_autoscaley_on`).
    pub fn set_axis_autoscale_on(&self, axis: AxisName, on: bool) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|a| a.axis_mut(axis).autoscale.enabled = on);
        self
    }

    /// Turns autoscaling of both axes on or off without autoscaling now
    /// (matplotlib's `set_autoscale_on`).
    pub fn set_autoscale_on(&self, on: bool) -> &Self {
        for name in [AxisName::X, AxisName::Y] {
            self.set_axis_autoscale_on(name, on);
        }
        self
    }

    /// Autoscales one axis (or both) from now on (matplotlib's
    /// `autoscale(enable=True, axis=..., tight=...)`); see
    /// [`Axes::autoscale_with`].
    pub fn autoscale(&self, axis: Option<AxisName>, tight: Option<bool>) -> &Self {
        self.autoscale_with(Some(true), axis, tight)
    }

    /// matplotlib's `autoscale(enable, axis, tight)`: `enable` turns
    /// autoscaling of `axis` (both when `None`) on or off, and the axes
    /// that autoscale are rescaled at the next layout; `None` leaves the
    /// switches alone and rescales both axes. `tight` drops their margins
    /// and replaces the tight setting, which skips rounding the view out
    /// to ticks.
    pub fn autoscale_with(
        &self,
        enable: Option<bool>,
        axis: Option<AxisName>,
        tight: Option<bool>,
    ) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| {
            for name in [AxisName::X, AxisName::Y] {
                let scale = match enable {
                    None => true,
                    Some(on) if axis.is_none_or(|only| only == name) => {
                        a.axis_mut(name).autoscale.enabled = on;
                        on
                    }
                    Some(_) => false,
                };
                if !scale {
                    continue;
                }
                let autoscale = &mut a.axis_mut(name).autoscale;
                if tight == Some(true) {
                    autoscale.margin = 0.0;
                }
                autoscale.stale = true;
                if let Some(tight) = tight {
                    a.tight = tight;
                }
            }
        });
        self
    }

    /// Whether autoscaling stops the margins at the sticky edges of the
    /// plotted artists (matplotlib's `use_sticky_edges`).
    pub fn use_sticky_edges(&self) -> bool {
        self.raw.with::<AxesState, _>(|a| a.use_sticky_edges)
    }

    /// Lets the margins cross sticky edges (`false`) or not; the view
    /// follows at the next autoscaling (matplotlib's `use_sticky_edges`).
    pub fn set_use_sticky_edges(&self, on: bool) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|a| a.use_sticky_edges = on);
        self
    }

    /// Whether autoscaling rounds the view of the x and y axes out to
    /// ticks (the `axes.autolimit_mode` the axes was created under).
    pub fn autolimit_mode(&self) -> (AutolimitMode, AutolimitMode) {
        self.raw.with::<AxesState, _>(|a| {
            (
                a.xaxis.settings.axes_autolimit_mode,
                a.yaxis.settings.axes_autolimit_mode,
            )
        })
    }

    /// Rounds the autoscaled view of both axes out to ticks or not, and
    /// autoscales them again: matplotlib's `axes.autolimit_mode`, which it
    /// reads whenever it autoscales, as when `ax.autoscale_view()` runs
    /// under `rc_context({'axes.autolimit_mode': ...})`.
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// use pltrs::ticker::AutolimitMode;
    /// let ax = Figure::new().subplot();
    /// ax.plot([0.3, 4.7], [0.3, 4.7])?;
    /// ax.set_autolimit_mode(AutolimitMode::RoundNumbers);
    /// assert_eq!(ax.xlim(), (0.0, 5.0));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn set_autolimit_mode(&self, mode: AutolimitMode) -> &Self {
        self.raw.with_mut::<AxesState, _>(|a| {
            for name in [AxisName::X, AxisName::Y] {
                a.axis_mut(name).settings.axes_autolimit_mode = mode;
            }
            a.request_autoscale(true, true);
        });
        self
    }

    /// The autoscaling margins `(x, y)` as fractions of the data span
    /// (matplotlib's `margins()`).
    pub fn margins(&self) -> (f64, f64) {
        (self.margin(AxisName::X), self.margin(AxisName::Y))
    }

    /// The autoscaling margin of one axis (matplotlib's `get_xmargin`).
    pub fn margin(&self, axis: AxisName) -> f64 {
        self.raw
            .with::<AxesState, _>(|a| a.axis(axis).autoscale.margin)
    }

    /// Sets the padding added around the data of one axis when
    /// autoscaling, as a fraction of the data span; negative margins clip
    /// the data (matplotlib's `set_xmargin`/`set_ymargin`).
    ///
    /// # Errors
    ///
    /// When the margin is -0.5 or less, or not finite.
    pub fn set_margin(&self, axis: AxisName, margin: f64) -> Result<&Self, PlotError> {
        if !(margin.is_finite() && margin > -0.5) {
            return Err(PlotError::InvalidMargin(margin));
        }
        self.raw.with_mut::<AxesState, _>(|a| {
            let autoscale = &mut a.axis_mut(axis).autoscale;
            autoscale.margin = margin;
            autoscale.stale = true;
        });
        Ok(self)
    }

    /// Sets the margins of the axes given (`None` keeps one) and stops
    /// autoscaling from rounding the view out to ticks (matplotlib's
    /// `margins(x, y)`).
    ///
    /// ```
    /// use pltrs::prelude::*;
    /// let ax = Figure::new().subplot();
    /// ax.plot([1.0, 10.0], [1.0, 10.0])?;
    /// ax.set_margins(1.0, None)?;
    /// assert_eq!(ax.xlim(), (-8.0, 19.0));
    /// assert_eq!(ax.margins(), (1.0, 0.05));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a margin is -0.5 or less, or not finite; then no margin
    /// changes.
    pub fn set_margins(
        &self,
        x: impl Into<Option<f64>>,
        y: impl Into<Option<f64>>,
    ) -> Result<&Self, PlotError> {
        let margins = [(AxisName::X, x.into()), (AxisName::Y, y.into())];
        if let Some(bad) = margins
            .iter()
            .filter_map(|&(_, margin)| margin)
            .find(|m| !(m.is_finite() && *m > -0.5))
        {
            return Err(PlotError::InvalidMargin(bad));
        }
        for (axis, margin) in margins {
            if let Some(margin) = margin {
                self.set_margin(axis, margin)?;
            }
        }
        self.raw.with_mut::<AxesState, _>(|a| a.tight = true);
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::figure::Figure;

    fn line() -> Axes {
        let ax = Figure::new().subplot();
        ax.plot([0.0, 10.0], [0.0, 5.0]).expect("valid data");
        ax
    }

    #[test]
    fn one_sided_limits_keep_the_other_side() {
        let ax = line();
        let high = Limits {
            high: Some(20.0),
            ..Limits::default()
        };
        assert_eq!(ax.set_limits_with(AxisName::X, high), Ok((-0.5, 20.0)));
        let low = Limits {
            low: Some(1.0),
            auto: None,
            ..Limits::default()
        };
        assert_eq!(ax.set_limits_with(AxisName::Y, low), Ok((1.0, 5.25)));
        assert!(!ax.autoscale_on(Some(AxisName::X)));
        assert!(ax.autoscale_on(Some(AxisName::Y)), "auto=None keeps it");
    }

    #[test]
    fn non_finite_limits_are_rejected() {
        let ax = line();
        for bad in [f64::NAN, f64::INFINITY] {
            let limits = Limits {
                low: Some(bad),
                ..Limits::default()
            };
            assert_eq!(
                ax.set_limits_with(AxisName::X, limits),
                Err(PlotError::NonFiniteLimit(AxisName::X))
            );
        }
        for (left, right) in [(f64::NAN, 3.0), (1.0, f64::INFINITY)] {
            assert_eq!(
                ax.set_xlim(left, right).err(),
                Some(PlotError::NonFiniteLimit(AxisName::X))
            );
        }
        assert_eq!(
            ax.set_limits(AxisName::Y, f64::NEG_INFINITY, 1.0).err(),
            Some(PlotError::NonFiniteLimit(AxisName::Y))
        );
        assert_eq!(
            (ax.xlim(), ax.ylim()),
            ((-0.5, 10.5), (-0.25, 5.25)),
            "the view is unchanged"
        );
    }

    #[test]
    fn non_positive_log_limits_keep_the_current_value() {
        let ax = Figure::new().subplot();
        ax.set_xscale("log").expect("a known scale");
        let before = ax.xlim();
        ax.set_xlim(-1.0, 10.0).expect("finite limits");
        assert_eq!(ax.xlim(), (before.0, 10.0));
    }

    #[test]
    fn bounds_follow_the_orientation() {
        let ax = Figure::new().subplot();
        ax.set_bound(AxisName::X, Some(2.0), Some(3.0)).unwrap();
        ax.set_bound(AxisName::X, None, Some(4.0)).unwrap();
        assert_eq!((ax.bound(AxisName::X), ax.xlim()), ((2.0, 4.0), (2.0, 4.0)));
        ax.xaxis().set_inverted(true);
        ax.set_bound(AxisName::X, Some(3.0), None).unwrap();
        assert_eq!((ax.bound(AxisName::X), ax.xlim()), ((3.0, 4.0), (4.0, 3.0)));
    }

    #[test]
    fn margins_are_validated() {
        let ax = line();
        assert_eq!(
            ax.set_margins(-0.5, None).err(),
            Some(PlotError::InvalidMargin(-0.5))
        );
        assert_eq!(
            ax.set_margins(0.2, f64::NAN).err().map(|e| e.to_string()),
            Some("margin must be finite and greater than -0.5, got NaN".to_owned())
        );
        assert_eq!(ax.margins(), (0.05, 0.05), "nothing changed");
        ax.set_margins(-0.2, 0.5).unwrap();
        assert_eq!(ax.margins(), (-0.2, 0.5));
        let (x0, x1) = ax.xlim();
        assert!((x0 - 2.0).abs() < 1e-12 && (x1 - 8.0).abs() < 1e-12);
    }

    #[test]
    fn autoscale_switches() {
        let ax = line();
        assert_eq!(ax.ylim(), (-0.25, 5.25));
        ax.autoscale_with(Some(false), Some(AxisName::Y), None);
        assert!(ax.autoscale_on(Some(AxisName::X)) && !ax.autoscale_on(None));
        ax.plot([0.0, 20.0], [0.0, 50.0]).expect("valid data");
        assert_eq!(ax.ylim(), (-0.25, 5.25), "y stays put");
        ax.autoscale_with(None, None, Some(true));
        assert!(
            !ax.autoscale_on(Some(AxisName::Y)),
            "enable=None keeps switches"
        );
        assert_eq!((ax.xlim(), ax.ylim()), ((0.0, 20.0), (-0.25, 5.25)));
    }

    #[test]
    fn sticky_edges_can_be_ignored() {
        let ax = Figure::new().subplot();
        ax.bar([0.0, 1.0], [1.0, 2.0], crate::axes::BarOptions::default())
            .expect("valid bars");
        assert_eq!(ax.ylim().0, 0.0);
        ax.set_use_sticky_edges(false).autoscale(None, None);
        assert!(ax.ylim().0 < 0.0 && !ax.use_sticky_edges());
    }
}
