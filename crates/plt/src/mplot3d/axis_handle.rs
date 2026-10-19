//! Handles to the x, y and z axes of a 3-D axes (matplotlib's
//! `axis3d.Axis`): tick helpers, tick positions and labels, and the tick
//! parameters 3-D axes honour.

use std::rc::Rc;

use super::axis3d::Tickers3D;
use super::coords::Axis3;
use super::error::Axes3DError;
use super::geometry::AxisPosition;
use super::handle::Axes3D;
use super::share::ShareKey;
use super::state::Axes3DState;
use crate::artist::ArtistHandle;
use crate::axes::{TickParams, TickSet, TickStyle, tick_label_formatter};
use crate::ticker::{AutoMinorLocator, AxisName, FixedLocator, Formatter, Locator, NullLocator};

/// One axis of a 3-D axes, from [`Axes3D::xaxis`], [`Axes3D::yaxis`],
/// [`Axes3D::zaxis`] or [`Axes3D::axis`].
///
/// Tick helpers are shared with the axes sharing the axis, as in
/// matplotlib.
///
/// ```
/// use plt::figure::Figure;
/// let ax = Figure::new().subplot_3d();
/// ax.zaxis().set_ticks(&[0.0, 0.5, 2.0]).set_ticklabels(["low", "mid", "high"])?;
/// assert_eq!(ax.zlim(), (-0.020833333333333332, 2.0));
/// let ticks = ax.zaxis().ticks()?;
/// assert_eq!(ticks.major[2].label, "high");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Axis3D {
    axes: Axes3D,
    dir: Axis3,
}

impl Axes3D {
    /// The axis along `dir`.
    pub fn axis(&self, dir: Axis3) -> Axis3D {
        Axis3D {
            axes: self.clone(),
            dir,
        }
    }

    /// The x axis.
    pub fn xaxis(&self) -> Axis3D {
        self.axis(Axis3::X)
    }

    /// The y axis.
    pub fn yaxis(&self) -> Axis3D {
        self.axis(Axis3::Y)
    }

    /// The z axis.
    pub fn zaxis(&self) -> Axis3D {
        self.axis(Axis3::Z)
    }

    /// Changes the tick parameters of the axes `params.axis` selects:
    /// all three when unset (matplotlib's `tick_params(axis='both')`),
    /// otherwise x or y; set the z axis alone with
    /// [`Axis3D::set_tick_params`] (matplotlib's `tick_params(axis='z')`).
    /// See [`Axis3D::set_tick_params`] for what 3-D axes honour.
    pub fn tick_params(&self, params: &TickParams) -> &Self {
        let dirs: &[Axis3] = match params.axis {
            None => &Axis3::ALL,
            Some(AxisName::X) => &[Axis3::X],
            Some(AxisName::Y) => &[Axis3::Y],
        };
        for &dir in dirs {
            self.axis(dir).set_tick_params(params);
        }
        self
    }
}

impl Axis3D {
    /// Which axis this is.
    pub fn dir(&self) -> Axis3 {
        self.dir
    }

    /// Applies `f` to this axis of this axes and of the axes sharing it.
    fn update_shared(&self, mut f: impl FnMut(&mut Axes3DState)) {
        self.axes.update_group(ShareKey::Axis(self.dir), &mut f);
    }

    /// Changes the tick helpers of this axis and of the axes sharing it.
    fn set_tickers(&self, set: impl Fn(&mut Tickers3D)) -> &Self {
        let dir = self.dir;
        self.update_shared(|axes| set(&mut axes.axis_mut(dir).tickers));
        self
    }

    /// Places the major ticks with `locator` (matplotlib's
    /// `set_major_locator`).
    pub fn set_major_locator(&self, locator: impl Locator) -> &Self {
        let locator: Rc<dyn Locator> = Rc::new(locator);
        self.set_tickers(|t| t.major_locator = Rc::clone(&locator))
    }

    /// Places the minor ticks with `locator`; 3-D axes draw minor ticks,
    /// their labels and gridlines like the major ones (matplotlib's
    /// `set_minor_locator`).
    pub fn set_minor_locator(&self, locator: impl Locator) -> &Self {
        let locator: Rc<dyn Locator> = Rc::new(locator);
        self.set_tickers(|t| t.minor_locator = Rc::clone(&locator))
    }

    /// Labels the major ticks with `formatter` (matplotlib's
    /// `set_major_formatter`).
    pub fn set_major_formatter(&self, formatter: impl Formatter) -> &Self {
        let formatter: Rc<dyn Formatter> = Rc::new(formatter);
        self.set_tickers(|t| t.major_formatter = Rc::clone(&formatter))
    }

    /// Labels the minor ticks with `formatter` (matplotlib's
    /// `set_minor_formatter`).
    pub fn set_minor_formatter(&self, formatter: impl Formatter) -> &Self {
        let formatter: Rc<dyn Formatter> = Rc::new(formatter);
        self.set_tickers(|t| t.minor_formatter = Rc::clone(&formatter))
    }

    /// Puts the major ticks exactly at `ticks`, widening the view of this
    /// axis and of the axes sharing it to include them without changing
    /// their autoscaling (matplotlib's `Axis.set_ticks`).
    pub fn set_ticks(&self, ticks: &[f64]) -> &Self {
        self.include_ticks_in_view(ticks);
        self.set_major_locator(FixedLocator::new(ticks.to_vec()))
    }

    /// Puts the minor ticks exactly at `ticks`, widening the view as
    /// [`Axis3D::set_ticks`] does (matplotlib's `set_ticks(...,
    /// minor=True)`).
    pub fn set_minor_ticks(&self, ticks: &[f64]) -> &Self {
        self.include_ticks_in_view(ticks);
        self.set_minor_locator(FixedLocator::new(ticks.to_vec()))
    }

    /// matplotlib's `_set_tick_locations` widening the view intervals.
    fn include_ticks_in_view(&self, ticks: &[f64]) {
        if ticks.is_empty() {
            return;
        }
        let low = ticks.iter().copied().fold(f64::INFINITY, f64::min);
        let high = ticks.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let dir = self.dir;
        self.update_shared(|axes| axes.include_in_view(dir, low, high));
    }

    /// Labels the major ticks with `labels`: one per fixed tick when the
    /// ticks were set explicitly, otherwise in order; no labels hide them
    /// (matplotlib's `Axis.set_ticklabels`).
    ///
    /// # Errors
    ///
    /// When the ticks were set explicitly and their number differs from
    /// the number of labels.
    pub fn set_ticklabels<S: Into<String>>(
        &self,
        labels: impl IntoIterator<Item = S>,
    ) -> Result<&Self, Axes3DError> {
        let labels: Vec<String> = labels.into_iter().map(Into::into).collect();
        let dir = self.dir;
        let locator = self
            .axes
            .with(|axes| Rc::clone(&axes.axis(dir).tickers.major_locator));
        let formatter = tick_label_formatter(locator.as_ref(), labels)?;
        Ok(self.set_tickers(|t| t.major_formatter = Rc::clone(&formatter)))
    }

    /// Turns minor ticks on with the locator matplotlib picks for the
    /// scale: the scale's own minor locator for log, symlog, asinh and
    /// logit axes, otherwise an `AutoMinorLocator` (matplotlib's
    /// `Axis.minorticks_on`).
    pub fn minorticks_on(&self) -> &Self {
        let dir = self.dir;
        let scale_minor = self.axes.with(|axes| {
            let axis = axes.axis(dir);
            let scale = axis.scale.scale();
            matches!(scale.name(), "log" | "symlog" | "asinh" | "logit").then(|| {
                Rc::<dyn Locator>::from(
                    scale
                        .default_tickers(AxisName::X, &axis.settings)
                        .minor_locator,
                )
            })
        });
        let locator = scale_minor.unwrap_or_else(|| Rc::new(AutoMinorLocator::new(None)));
        self.set_tickers(|t| t.minor_locator = Rc::clone(&locator))
    }

    /// Removes the minor ticks (matplotlib's `Axis.minorticks_off`).
    pub fn minorticks_off(&self) -> &Self {
        self.set_minor_locator(NullLocator)
    }

    /// The ticks the axis draws in the current view, after a layout pass
    /// (matplotlib's visible ticks of `_update_ticks`).
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails, with its message.
    pub fn ticks(&self) -> Result<TickSet, String> {
        self.axes.raw().figure_state().borrow_mut().prepare();
        let dir = self.dir;
        self.axes.with(|axes| axes.axis(dir).ticks.clone())
    }

    /// Shows or hides the axis line (matplotlib's
    /// `axis.line.set_visible`).
    pub fn set_line_visible(&self, visible: bool) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).line_visible = visible);
        self
    }

    /// Chooses the box edges carrying the axis line, tick marks, tick
    /// labels and offset text (matplotlib's `set_ticks_position`).
    ///
    /// ```
    /// use plt::figure::Figure;
    /// use plt::mplot3d::AxisPosition;
    /// let ax = Figure::new().subplot_3d();
    /// ax.zaxis().set_ticks_position(AxisPosition::Both);
    /// assert_eq!(ax.zaxis().ticks_position(), AxisPosition::Both);
    /// ```
    pub fn set_ticks_position(&self, position: AxisPosition) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).tick_position = position);
        self
    }

    /// The edges carrying the tick marks (matplotlib's
    /// `get_ticks_position`).
    pub fn ticks_position(&self) -> AxisPosition {
        self.axes.with(|axes| axes.axis(self.dir).tick_position)
    }

    /// Chooses the box edges carrying the axis label (matplotlib's
    /// `set_label_position`).
    pub fn set_label_position(&self, position: AxisPosition) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).label_position = position);
        self
    }

    /// The edges carrying the label (matplotlib's `get_label_position`).
    pub fn label_position(&self) -> AxisPosition {
        self.axes.with(|axes| axes.axis(self.dir).label_position)
    }

    /// Changes the tick parameters of this axis (matplotlib's
    /// `Axis.set_tick_params`); `params.axis` is ignored. The x axis
    /// takes the `bottom`/`top` switches, y and z the `left`/`right` ones,
    /// as `Axes3D.tick_params` routes them; `reset` restores the `xtick`
    /// defaults every 3-D tick uses.
    ///
    /// 3-D axes honour the tick colour, the label pad, size, colour,
    /// rotation and visibility (`labelbottom` or `labelleft`); like
    /// matplotlib they always draw the tick marks across the axis, as
    /// wide as `[xy]tick.*.width` say, and ignore the tick direction,
    /// length and width and the second label side.
    pub fn set_tick_params(&self, params: &TickParams) -> &Self {
        let rc = crate::rc::params();
        let sides = if self.dir == Axis3::X {
            AxisName::X
        } else {
            AxisName::Y
        };
        let unreset = TickParams {
            reset: false,
            ..params.clone()
        };
        let apply = |major: bool, style: &mut TickStyle| {
            if params.reset {
                *style = TickStyle::from_rc(AxisName::X, major, &rc);
            }
            unreset.apply(sides, major, style, &rc);
        };
        let label_color = params.colors.as_ref().or(params.labelcolor.as_ref());
        let dir = self.dir;
        self.axes.with_mut(|axes| {
            let axis = axes.axis_mut(dir);
            if params.which.major() {
                apply(true, &mut axis.major_style);
            }
            if params.which.minor() {
                apply(false, &mut axis.minor_style);
            }
            if let Some(color) = label_color {
                axis.offset_color = color.clone();
            }
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axes::TickLevel;
    use crate::figure::Figure;
    use crate::ticker::MultipleLocator;

    #[test]
    fn fixed_ticks_widen_the_view_and_take_labels() {
        let ax = Figure::new().subplot_3d();
        ax.xaxis().set_ticks(&[-1.0, 3.0]);
        assert_eq!(ax.xlim(), (-1.0, 3.0));
        assert!(ax.autoscale(Axis3::X), "set_ticks leaves autoscaling on");
        assert!(ax.xaxis().set_ticklabels(["a"]).is_err());
        ax.xaxis().set_ticklabels(["a", "b"]).unwrap();
        let labels: Vec<String> = ax
            .xaxis()
            .ticks()
            .unwrap()
            .major
            .into_iter()
            .map(|t| t.label)
            .collect();
        assert_eq!(labels, ["a", "b"]);
    }

    #[test]
    fn positions_choose_the_edges_drawn() {
        let fig = Figure::with_size(4.0, 3.0, 100.0);
        let ax = fig.subplot_3d();
        let shown = fig.tight_bbox();
        for axis in [ax.xaxis(), ax.yaxis(), ax.zaxis()] {
            axis.set_ticks_position(AxisPosition::None)
                .set_label_position(AxisPosition::Upper)
                .set_line_visible(false);
        }
        assert_eq!(ax.zaxis().ticks_position(), AxisPosition::None);
        assert_eq!(ax.zaxis().label_position(), AxisPosition::Upper);
        assert!(!ax.with(|axes| axes.axis(Axis3::Z).line_visible));
        // Without tick labels the axes take less room.
        let hidden = fig.tight_bbox();
        assert!(hidden.width() < shown.width(), "{hidden:?} vs {shown:?}");
    }

    #[test]
    fn minor_ticks_are_computed_and_shared() {
        let fig = Figure::new();
        let a = fig.subplot_3d();
        let b = fig.subplot_3d();
        b.sharey(&a).unwrap();
        a.yaxis()
            .set_minor_locator(MultipleLocator::new(0.1).unwrap());
        assert_ne!(b.yaxis().ticks().unwrap().minor.len(), 0);
        b.yaxis().minorticks_off();
        assert_eq!(a.yaxis().ticks().unwrap().minor.len(), 0);
    }

    #[test]
    fn tick_params_route_label_switches_like_matplotlib() {
        let ax = Figure::new().subplot_3d();
        ax.tick_params(&TickParams {
            labelleft: Some(false),
            labelcolor: Some("red".into()),
            which: TickLevel::Both,
            ..TickParams::default()
        });
        let style = |dir: Axis3| ax.with(|axes| axes.axis(dir).major_style.labels.first);
        assert!(style(Axis3::X), "x labels follow labelbottom");
        assert!(!style(Axis3::Y) && !style(Axis3::Z));
        ax.zaxis().set_tick_params(&TickParams {
            reset: true,
            ..TickParams::default()
        });
        assert!(style(Axis3::Z));
    }
}
