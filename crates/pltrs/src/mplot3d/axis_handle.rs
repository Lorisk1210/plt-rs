//! Handles to the x, y and z axes of a 3-D axes (matplotlib's
//! `axis3d.Axis`): tick helpers, tick positions and labels, and the tick
//! parameters 3-D axes honour.

use std::any::Any;
use std::rc::Rc;

use super::axis3d::{Tickers3D, tick_style};
use super::coords::Axis3;
use super::error::Axes3DError;
use super::geometry::AxisPosition;
use super::handle::Axes3D;
use super::share::ShareKey;
use super::state::Axes3DState;
use crate::axes::{TickParams, TickStyle, TicklabelFormat, tick_label_formatter};
use crate::ticker::{
    AutoMinorLocator, AxisName, FixedLocator, Formatter, Locator, NullLocator, ScalarFormatter,
    TickerError, TickerSettings,
};
use crate::units::AxisValues;

/// One axis of a 3-D axes, from [`Axes3D::xaxis`], [`Axes3D::yaxis`],
/// [`Axes3D::zaxis`] or [`Axes3D::axis`].
///
/// Tick helpers are shared with the axes sharing the axis, as in
/// matplotlib.
///
/// ```
/// use pltrs::figure::Figure;
/// let ax = Figure::new().subplot_3d();
/// ax.zaxis().set_ticks([0.0, 0.5, 2.0]).set_ticklabels(["low", "mid", "high"])?;
/// assert_eq!(ax.zlim(), (-0.020833333333333332, 2.0));
/// let ticks = ax.zaxis().ticks()?;
/// assert_eq!(ticks.major[2].label, "high");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Axis3D {
    pub(super) axes: Axes3D,
    pub(super) dir: Axis3,
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
    pub fn tick_params(&self, params: TickParams) -> &Self {
        let dirs: &[Axis3] = match params.axis {
            None => &Axis3::ALL,
            Some(AxisName::X) => &[Axis3::X],
            Some(AxisName::Y) => &[Axis3::Y],
        };
        for &dir in dirs {
            self.axis(dir).set_tick_params(params.clone());
        }
        self
    }

    /// Changes how the default major tick labels of one axis (`None`:
    /// all three) are formatted (matplotlib's `ticklabel_format(axis=...)`
    /// with `'both'` meaning all three); `format.axis` is ignored. The
    /// axes sharing an axis follow.
    ///
    /// ```
    /// use pltrs::axes::TicklabelFormat;
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::Axis3;
    /// use pltrs::ticker::OffsetMode;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_xlim(1e7, 1e7 + 10.0)?;
    /// let plain = TicklabelFormat {
    ///     scientific: Some(false),
    ///     use_offset: Some(OffsetMode::Off),
    ///     ..TicklabelFormat::default()
    /// };
    /// ax.ticklabel_format(Some(Axis3::X), plain)?;
    /// let ticks = ax.xaxis().ticks()?;
    /// assert!(ticks.offset.is_empty());
    /// assert!(ticks.major.iter().any(|tick| tick.label == "10000002"));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When an affected axis does not label its ticks with a
    /// `ScalarFormatter`.
    pub fn ticklabel_format(
        &self,
        axis: Option<Axis3>,
        format: TicklabelFormat,
    ) -> Result<&Self, Axes3DError> {
        for dir in Axis3::ALL
            .into_iter()
            .filter(|&dir| axis.is_none_or(|only| only == dir))
        {
            let current = self.with(|axes| {
                let formatter: &dyn Any = axes.axis(dir).tickers.major_formatter.as_ref();
                formatter.downcast_ref::<ScalarFormatter>().cloned()
            });
            let formatter = current.ok_or(Axes3DError::NotScalarFormatter(dir))?;
            // Restyling the formatter, as matplotlib does in place, keeps
            // it the default.
            let formatter: Rc<dyn Formatter> = Rc::new(format.applied_to(formatter));
            self.axis(dir)
                .set_tickers(|t| t.major_formatter = Rc::clone(&formatter));
        }
        Ok(self)
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
    pub(super) fn set_tickers(&self, set: impl Fn(&mut Tickers3D)) -> &Self {
        let dir = self.dir;
        self.update_shared(|axes| set(&mut axes.axis_mut(dir).tickers));
        self
    }

    /// Places the major ticks with `locator` (matplotlib's
    /// `set_major_locator`).
    pub fn set_major_locator(&self, locator: impl Locator) -> &Self {
        let locator: Rc<dyn Locator> = Rc::new(locator);
        self.set_tickers(|t| {
            t.major_locator = Rc::clone(&locator);
            t.defaults.major.locator = false;
        })
    }

    /// Places the minor ticks with `locator`; 3-D axes draw minor ticks,
    /// their labels and gridlines like the major ones (matplotlib's
    /// `set_minor_locator`).
    pub fn set_minor_locator(&self, locator: impl Locator) -> &Self {
        let locator: Rc<dyn Locator> = Rc::new(locator);
        self.set_tickers(|t| {
            t.minor_locator = Rc::clone(&locator);
            t.defaults.minor.locator = false;
        })
    }

    /// Labels the major ticks with `formatter` (matplotlib's
    /// `set_major_formatter`).
    pub fn set_major_formatter(&self, formatter: impl Formatter) -> &Self {
        let formatter: Rc<dyn Formatter> = Rc::new(formatter);
        self.set_tickers(|t| {
            t.major_formatter = Rc::clone(&formatter);
            t.defaults.major.formatter = false;
        })
    }

    /// Labels the minor ticks with `formatter` (matplotlib's
    /// `set_minor_formatter`).
    pub fn set_minor_formatter(&self, formatter: impl Formatter) -> &Self {
        let formatter: Rc<dyn Formatter> = Rc::new(formatter);
        self.set_tickers(|t| {
            t.minor_formatter = Rc::clone(&formatter);
            t.defaults.minor.formatter = false;
        })
    }

    /// Puts the major ticks exactly at `ticks`, converted on this axis
    /// (numbers, dates, category names or user values), widening the view
    /// of this axis and of the axes sharing it to include them without
    /// changing their autoscaling (matplotlib's `Axis.set_ticks`).
    pub fn set_ticks(&self, ticks: impl Into<AxisValues>) -> &Self {
        let ticks = self.convert_units(ticks);
        self.include_ticks_in_view(&ticks);
        self.set_major_locator(FixedLocator::new(ticks))
    }

    /// Puts the minor ticks exactly at `ticks`, converted and widening the
    /// view as [`Axis3D::set_ticks`] does (matplotlib's `set_ticks(...,
    /// minor=True)`).
    pub fn set_minor_ticks(&self, ticks: impl Into<AxisValues>) -> &Self {
        let ticks = self.convert_units(ticks);
        self.include_ticks_in_view(&ticks);
        self.set_minor_locator(FixedLocator::new(ticks))
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
        self.set_level_ticklabels(true, labels.into_iter().map(Into::into).collect())
    }

    /// Labels the minor ticks with `labels`, like
    /// [`Axis3D::set_ticklabels`] does the major ones (matplotlib's
    /// `Axis.set_ticklabels(..., minor=True)`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.xaxis().set_minor_ticks([0.25]).set_minor_ticklabels(["quarter"])?;
    /// assert_eq!(ax.xaxis().ticks()?.minor[0].label, "quarter");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the minor ticks were set explicitly and their number differs
    /// from the number of labels.
    pub fn set_minor_ticklabels<S: Into<String>>(
        &self,
        labels: impl IntoIterator<Item = S>,
    ) -> Result<&Self, Axes3DError> {
        self.set_level_ticklabels(false, labels.into_iter().map(Into::into).collect())
    }

    /// Labels the major (or minor) ticks with `labels`.
    fn set_level_ticklabels(&self, major: bool, labels: Vec<String>) -> Result<&Self, Axes3DError> {
        let dir = self.dir;
        let locator = self.axes.with(|axes| {
            let tickers = &axes.axis(dir).tickers;
            Rc::clone(if major {
                &tickers.major_locator
            } else {
                &tickers.minor_locator
            })
        });
        let formatter = tick_label_formatter(locator.as_ref(), labels)?;
        Ok(self.set_tickers(|t| {
            if major {
                t.major_formatter = Rc::clone(&formatter);
                t.defaults.major.formatter = false;
            } else {
                t.minor_formatter = Rc::clone(&formatter);
                t.defaults.minor.formatter = false;
            }
        }))
    }

    /// Sets the distance of the axis label from the axis in points, on
    /// top of the fixed offset 3-D axes keep (matplotlib's `labelpad`).
    pub fn set_label_pad(&self, pad: f64) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).label.pad = pad);
        self
    }

    /// The label pads of the major ticks in the current view, one per
    /// location the major locator gives (inside the view or not), after
    /// a layout pass (matplotlib's `tick.get_pad()` for every tick of
    /// `get_major_ticks()`).
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails.
    pub fn major_tick_pads(&self) -> Result<Vec<f64>, TickerError> {
        self.ticks()?;
        let dir = self.dir;
        Ok(self.axes.with(|axes| axes.axis(dir).major_tick_pads()))
    }

    /// Gives the major ticks their own label pads, in the order of
    /// [`Axis3D::major_tick_pads`]: like matplotlib's `Tick` objects
    /// after `tick.set_pad`, the ticks past the end take the first pad.
    /// Tick parameters setting a pad give every tick that pad again.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// let pads = ax.yaxis().major_tick_pads()?;
    /// let shifted: Vec<f64> = pads.iter().enumerate().map(|(i, pad)| pad - i as f64).collect();
    /// ax.yaxis().set_major_tick_pads(shifted.clone());
    /// assert_eq!(ax.yaxis().major_tick_pads()?, shifted);
    /// # Ok::<(), pltrs::ticker::TickerError>(())
    /// ```
    pub fn set_major_tick_pads(&self, pads: Vec<f64>) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).major_pads = pads);
        self
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
            scale.name().has_own_minor_locator().then(|| {
                Rc::<dyn Locator>::from(
                    scale
                        .default_tickers(AxisName::X, &TickerSettings::current())
                        .minor_locator,
                )
            })
        });
        let locator = scale_minor.unwrap_or_else(|| Rc::new(AutoMinorLocator::new(None)));
        self.set_tickers(|t| {
            t.minor_locator = Rc::clone(&locator);
            t.defaults.minor.locator = false;
        })
    }

    /// Removes the minor ticks (matplotlib's `Axis.minorticks_off`).
    pub fn minorticks_off(&self) -> &Self {
        self.set_minor_locator(NullLocator)
    }

    /// Shows or hides the axis line (matplotlib's
    /// `axis.line.set_visible`).
    pub fn set_line_visible(&self, visible: bool) -> &Self {
        let dir = self.dir;
        self.axes
            .with_mut(|axes| axes.axis_mut(dir).line_visible = visible);
        self
    }

    /// Whether the axis line is drawn (matplotlib's
    /// `axis.line.get_visible`).
    pub fn line_visible(&self) -> bool {
        self.axes.with(|axes| axes.axis(self.dir).line_visible)
    }

    /// Chooses the box edges carrying the axis line, tick marks, tick
    /// labels and offset text (matplotlib's `set_ticks_position`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::mplot3d::AxisPosition;
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
    pub fn set_tick_params(&self, params: TickParams) -> &Self {
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
                *style = tick_style(major, &rc);
            }
            unreset.apply(sides, major, style, &rc);
        };
        let label_color = params.colors.as_ref().or(params.labelcolor.as_ref());
        let dir = self.dir;
        self.axes.with_mut(|axes| {
            let axis = axes.axis_mut(dir);
            if params.which.major() {
                apply(true, &mut axis.major_style);
                if params.reset || params.pad.is_some() {
                    axis.major_pads.clear();
                }
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
    use crate::ticker::{MaxNLocator, MultipleLocator, Nbins};

    #[test]
    fn fixed_ticks_widen_the_view_and_take_labels() {
        let ax = Figure::new().subplot_3d();
        ax.xaxis().set_ticks([-1.0, 3.0]);
        assert_eq!(ax.xlim(), (-1.0, 3.0));
        assert!(ax.autoscale_on(Axis3::X), "set_ticks leaves autoscaling on");
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
    fn fixed_ticks_convert_on_the_axis() {
        let ax = Figure::new().subplot_3d();
        ax.plot([0.0, 1.0], [0.0, 1.0], ["low", "high"]).unwrap();
        ax.zaxis().set_ticks(["high"]).set_minor_ticks(["low"]);
        assert_eq!(ax.zaxis().ticklocs(false).unwrap(), [1.0]);
        assert_eq!(ax.zaxis().ticklocs(true).unwrap(), [0.0]);
        assert_eq!(ax.zaxis().ticklabels(false).unwrap(), ["high"]);
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
        assert!(!ax.zaxis().line_visible());
        // Without tick labels the axes take less room.
        let hidden = fig.tight_bbox();
        assert!(hidden.width() < shown.width(), "{hidden:?} vs {shown:?}");
    }

    #[test]
    fn minor_ticks_are_computed_and_shared() {
        let fig = Figure::new();
        let a = fig.subplot_3d();
        let b = fig.subplot_3d();
        b.share_y(&a).unwrap();
        a.yaxis()
            .set_minor_locator(MultipleLocator::new(0.1).unwrap());
        assert_ne!(b.yaxis().ticks().unwrap().minor.len(), 0);
        b.yaxis().minorticks_off();
        assert_eq!(a.yaxis().ticks().unwrap().minor.len(), 0);
    }

    #[test]
    fn absurd_tick_counts_are_refused() {
        // Found by fuzzing: a million tick labels took minutes to draw.
        let ax = Figure::new().subplot_3d();
        let locator = |nbins| MaxNLocator::new(Nbins::Fixed(nbins));
        ax.zaxis().set_major_locator(locator(1 << 20));
        assert!(matches!(
            ax.zaxis().ticks(),
            Err(TickerError::TooManyTicks(_))
        ));
        ax.zaxis().set_major_locator(locator(5));
        assert!(ax.zaxis().ticks().is_ok());
    }

    #[test]
    fn tick_params_route_label_switches_like_matplotlib() {
        let ax = Figure::new().subplot_3d();
        ax.tick_params(TickParams {
            labelleft: Some(false),
            labelcolor: Some("red".into()),
            which: TickLevel::Both,
            ..TickParams::default()
        });
        let style = |dir: Axis3| ax.with(|axes| axes.axis(dir).major_style.labels.first);
        assert!(style(Axis3::X), "x labels follow labelbottom");
        assert!(!style(Axis3::Y) && !style(Axis3::Z));
        ax.zaxis().set_tick_params(TickParams {
            reset: true,
            ..TickParams::default()
        });
        assert!(style(Axis3::Z));
    }
}
