//! The ticks of a 3-D axis: those drawn, every location with its label,
//! what tick helpers know of the axis, their marks and gridlines, and
//! ticking an axis of dates (matplotlib's
//! `get_ticklocs`, `get_ticklabels`, `get_ticklines`, `get_gridlines` and
//! `axis_date`, which `Axes3D` wraps as `get_zticks`, `get_zticklabels`,
//! `get_zticklines`, `get_zgridlines` and `zaxis_date`).

use super::axis_draw::{GridLine3D, TickLine3D, grid_lines, tick_lines};
use super::axis_handle::Axis3D;
use super::axis3d::{Axis3DState, AxisSpan};
use super::state::{Axes3DState, prepare_one};
use super::tick_sites::MinorTicks;
use crate::artist::{ArtistHandle, LayoutContext};
use crate::axes::{AxisName, Labelling, TickSet};
use crate::dates::TimeZone;
use crate::ticker::{AxisInfo, TickContext, TickerError, TickerSettings, drawable_ticks};
use crate::transforms::TransformContext;
use crate::units::UnitHandle;

impl Axis3DState {
    /// The locations the major (or `minor`) locator places for an axes
    /// `length_points` wide, as [`Axis3D::ticklocs`] gives them
    /// (matplotlib's `get_majorticklocs` and `get_minorticklocs`).
    fn level_locs(
        &self,
        span: &AxisSpan,
        length_points: f64,
        minor: bool,
    ) -> Result<Vec<f64>, TickerError> {
        let mut info = self.info(span, length_points);
        let settings = TickerSettings::current();
        let major = drawable_ticks(
            (self.tickers.major_locator).locate(&TickContext::new(&info, &settings))?,
        )?;
        if !minor {
            return Ok(major);
        }
        info.major_ticks.clone_from(&major);
        let locs = drawable_ticks(
            (self.tickers.minor_locator).locate(&TickContext::new(&info, &settings))?,
        )?;
        Ok(if self.remove_overlapping {
            self.warn_minor_overlaps(span, MinorTicks::Located);
            self.without_overlaps(locs, &major, span.view)
        } else {
            locs
        })
    }
}

impl Axis3D {
    /// The ticks the axis draws in the current view, after a layout pass
    /// (matplotlib's visible ticks of `_update_ticks`); a removed axes is
    /// ticked where it was in its figure.
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails, after the runtime warnings its
    /// arithmetic met, as matplotlib's locator issues them before it
    /// raises.
    pub fn ticks(&self) -> Result<TickSet, TickerError> {
        self.lay_out();
        let dir = self.dir;
        self.axes.with(|axes| {
            let axis = axes.axis(dir);
            axis.warn_failed_ticks();
            axis.ticks.clone()
        })
    }

    /// [`ticks`](Self::ticks) where matplotlib updates them
    /// (`_update_ticks`, which reading the tick labels runs), with the
    /// warning its minor ticks meet there (see `tick_sites`).
    ///
    /// # Errors
    ///
    /// As [`ticks`](Self::ticks).
    pub fn update_ticks(&self) -> Result<TickSet, TickerError> {
        let ticks = self.ticks()?;
        let dir = self.dir;
        self.axes.with(|axes| {
            let span = axes.limits.span(dir);
            axes.axis(dir).update_at_site(&span);
        });
        Ok(ticks)
    }

    /// [`ticks`](Self::ticks) when they can be computed, without the
    /// warnings of a failed computation: where a program makes tick
    /// objects without asking the locator (matplotlib's
    /// `get_major_ticks(numticks)`), nothing raises either.
    pub fn computed_ticks(&self) -> Option<TickSet> {
        self.lay_out();
        let dir = self.dir;
        self.axes.with(|axes| axes.axis(dir).ticks.clone().ok())
    }

    /// The number of the tick update that placed the axis' ticks last,
    /// when it labelled their major (or `minor`) level (see
    /// [`TickSet::labelled`]), without computing them; `None` otherwise.
    pub fn labelled(&self, minor: bool) -> Option<u64> {
        let dir = self.dir;
        self.axes
            .with(|axes| axes.axis(dir).ticks.as_ref().ok()?.labelled(minor))
    }

    /// What that update gave the formatter of the level (see
    /// [`TickSet::labelling`]), without computing the ticks.
    pub fn labelling(&self, minor: bool) -> Option<Labelling> {
        let dir = self.dir;
        self.axes
            .with(|axes| axes.axis(dir).ticks.as_ref().ok()?.labelling(minor))
    }

    /// Brings the layout of the axes up to date (a removed axes where it
    /// was in its figure).
    fn lay_out(&self) {
        let mut figure = self.axes.raw().figure_state().prepared();
        if !figure.is_attached(self.axes.id()) {
            prepare_one(&mut figure, self.axes.id(), None);
        }
    }

    /// Every major (or minor) tick location the locator places for the
    /// current view, inside it or not, whatever the axis shows; minor
    /// locations on major ticks are left out unless overlapping locations
    /// are kept (matplotlib's `Axis.get_ticklocs`, which asks the locator
    /// of the level only, and `Axes3D.get_zticks` for the z axis).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_zlim(0.0, 1.0)?;
    /// assert_eq!(ax.zaxis().ticklocs(false)?, [0.0, 0.2, 0.4, 0.6000000000000001, 0.8, 1.0]);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails.
    pub fn ticklocs(&self, minor: bool) -> Result<Vec<f64>, TickerError> {
        self.lay_out();
        let points = (self.axes.raw().figure_state().borrow())
            .axis_length_points(self.axes.id(), AxisName::X)
            .unwrap_or(0.0);
        let dir = self.dir;
        self.axes.with(|axes| {
            axes.axis(dir)
                .level_locs(&axes.limits.span(dir), points, minor)
        })
    }

    /// What a locator or formatter attached to this axis knows of it: its
    /// intervals, smallest positive datum, scale, major tick locations
    /// and tick space, along the width of the axes (matplotlib's helpers
    /// query their `Axis`, and every 3-D axis is an `XAxis`).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_zlim(0.0, 2.0)?;
    /// let info = ax.zaxis().info()?;
    /// assert_eq!(info.view_interval, (0.0, 2.0));
    /// assert!(info.three_d);
    /// assert_eq!(info.major_ticks, ax.zaxis().ticklocs(false)?);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails.
    pub fn info(&self) -> Result<AxisInfo, TickerError> {
        let major_ticks = self.ticklocs(false)?;
        let points = (self.axes.raw().figure_state().borrow())
            .axis_length_points(self.axes.id(), AxisName::X);
        let dir = self.dir;
        Ok(self.axes.with(|axes| AxisInfo {
            major_ticks,
            ..axes
                .axis(dir)
                .info(&axes.limits.span(dir), points.unwrap_or(0.0))
        }))
    }

    /// The labels of the [`ticklocs`](Self::ticklocs) on each side that
    /// shows them, those on the first side first (matplotlib's
    /// `Axis.get_ticklabels`; `get_zticklabels`, `get_zmajorticklabels`
    /// and `get_zminorticklabels` for the z axis).
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails.
    pub fn ticklabels(&self, minor: bool) -> Result<Vec<String>, TickerError> {
        let ticks = self.ticks()?;
        let dir = self.dir;
        let sides = self.axes.with(|axes| {
            let axis = axes.axis(dir);
            if minor {
                axis.minor_style.labels
            } else {
                axis.major_style.labels
            }
        });
        Ok(ticks.unclipped_labels(minor, sides))
    }

    /// The tick marks of every major (or minor) tick of the current view,
    /// in view or not, as the axes draw them after a layout pass
    /// (matplotlib's `Axis.get_ticklines`, `Axes3D.get_zticklines` for
    /// the z axis, whose tick objects matplotlib moves at every draw): one
    /// per tick, with its mark on each edge carrying the ticks.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_zlim(0.05, 0.95)?;
    /// let marks = ax.zaxis().ticklines(false)?;
    /// assert_eq!(marks.len(), ax.zaxis().ticklocs(false)?.len());
    /// assert!(!marks[0].in_view, "0 is out of view");
    /// assert!(marks.iter().all(|mark| mark.marks.len() == 1), "on one edge");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails.
    pub fn ticklines(&self, minor: bool) -> Result<Vec<TickLine3D>, TickerError> {
        self.ticks()?;
        Ok(self
            .with_layout(|axes, layout, extent| tick_lines(axes, layout, extent, self.dir, minor)))
    }

    /// The gridlines of every major tick of the current view, in view or
    /// not, where the axes draw them across the panes after a layout pass
    /// (matplotlib's `Axis.get_gridlines`, `Axes3D.get_zgridlines` for the
    /// z axis).
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.set_zlim(0.05, 0.95)?;
    /// let grid = ax.zaxis().gridlines()?;
    /// let drawn = grid.iter().filter(|line| line.in_view).count();
    /// assert_eq!(drawn, ax.zaxis().ticks()?.major.len());
    /// assert!(!grid[0].in_view && grid[1].in_view, "0 is out of view");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When a locator or formatter fails.
    pub fn gridlines(&self) -> Result<Vec<GridLine3D>, TickerError> {
        self.ticks()?;
        Ok(self.with_layout(|axes, layout, extent| grid_lines(axes, layout, extent, self.dir)))
    }

    /// Runs `f` on the axes, the figure's layout and the display extent of
    /// the axes; a default value when the axes is not laid out.
    fn with_layout<R: Default>(
        &self,
        f: impl FnOnce(&Axes3DState, LayoutContext<'_>, &pltrs_core::Bbox) -> R,
    ) -> R {
        let figure = self.axes.raw().figure_state().borrow();
        let axes = figure.store.get_as::<Axes3DState>(self.axes.id());
        match (axes, figure.axes_bbox(self.axes.id())) {
            (Some(axes), Some(extent)) => f(axes, LayoutContext::new(&figure), &extent),
            _ => R::default(),
        }
    }

    /// Ticks the axis as an axis of dates: its values are date
    /// numbers (days since the [epoch](crate::dates::get_epoch), see
    /// [`date2num`](crate::dates::date2num)), placed by an
    /// `AutoDateLocator` and labelled by the formatter of the axis'
    /// converter of dates (matplotlib's `Axis.axis_date`,
    /// `Axes3D.zaxis_date` for the z axis): the one `date.converter`
    /// chooses, or that of the kind a converter set with
    /// [`set_converter`](Self::set_converter) has
    /// ([`UnitConverter::DatesBy`](crate::units::UnitConverter::DatesBy)).
    /// Like matplotlib, the axis takes the converter plotting dates gives
    /// unless another one was set explicitly, replaces only the major
    /// locator and formatter that were not set explicitly, and leaves the
    /// limits as they are; the axes sharing the axis follow. A zone `tz`
    /// becomes the axis unit if it has none, so later plots keep it; the
    /// ticks are in the unit's zone, else (`None`) in the `timezone`
    /// rcParam's. As [`Axis::axis_date`](crate::axes::Axis::axis_date)
    /// does, they come only with that new unit or with a converter new to
    /// the axis, so an explicit converter of dates brings its ticks only
    /// with a zone.
    ///
    /// ```
    /// use pltrs::dates::TimeZone;
    /// use pltrs::figure::Figure;
    /// let ax = Figure::new().subplot_3d();
    /// ax.plot(&[0.0, 1.0], &[0.0, 1.0], &[19_000.0, 19_010.0])?;
    /// ax.zaxis().axis_date(&TimeZone::Utc);
    /// assert_eq!(ax.zaxis().ticklabels(false)?[0], "2022-01-08");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn axis_date<'a>(&self, tz: impl Into<Option<&'a TimeZone>>) -> &Self {
        self.make_date_axis(tz.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::dates::TimeZone;
    use crate::figure::Figure;
    use crate::mplot3d::Axis3;
    use crate::ticker::{MultipleLocator, NullFormatter};

    #[test]
    fn locations_and_labels_include_ticks_outside_the_view() {
        let ax = Figure::new().subplot_3d();
        ax.set_zlim(0.05, 0.95).unwrap();
        let axis = ax.zaxis();
        let locs = axis.ticklocs(false).unwrap();
        assert_eq!(locs.first(), Some(&0.0), "outside the view");
        assert_eq!(axis.ticks().unwrap().major.len(), locs.len() - 2);
        let labels = axis.ticklabels(false).unwrap();
        assert_eq!(labels.len(), locs.len());
        // Before the first draw the box is not square yet, and the axis
        // has room for a tick every 0.1, as matplotlib's has.
        assert_eq!(labels[1], "0.1");
        assert_eq!(
            axis.ticklocs(true).unwrap(),
            Vec::<f64>::new(),
            "no minor ticks"
        );
    }

    #[test]
    fn minor_locations_skip_the_major_ones_and_labels_follow_the_sides() {
        let ax = Figure::new().subplot_3d();
        ax.set_xlim(0.0, 1.0).unwrap();
        let axis = ax.xaxis();
        axis.set_minor_locator(MultipleLocator::new(0.1).unwrap());
        let minor = axis.ticklocs(true).unwrap();
        // matplotlib: -0.1, 0.1, 0.3, ..., 1.1 without 0.0, 0.2, ...
        assert_eq!(minor.len(), 7);
        assert!((minor[0] + 0.1).abs() < 1e-12 && (minor[6] - 1.1).abs() < 1e-12);
        ax.with_mut(|axes| axes.axis_mut(Axis3::X).major_style.labels.second = true);
        let labels = axis.ticklabels(false).unwrap();
        assert_eq!(labels.len(), 2 * axis.ticklocs(false).unwrap().len());
    }

    #[test]
    fn tick_lines_and_gridlines_follow_the_drawn_ticks() {
        use crate::mplot3d::AxisPosition;
        let ax = Figure::new().subplot_3d();
        ax.set_zlim(0.05, 0.95).unwrap();
        let axis = ax.zaxis();
        let marks = axis.ticklines(false).unwrap();
        let grid = axis.gridlines().unwrap();
        assert_eq!(marks.len(), axis.ticklocs(false).unwrap().len());
        let in_view = marks.iter().filter(|mark| mark.in_view).count();
        assert_eq!(in_view, axis.ticks().unwrap().major.len());
        assert_eq!(grid.len(), marks.len());
        assert!(grid.iter().all(|line| line.visible), "axes3d.grid");
        axis.set_ticks_position(AxisPosition::Both);
        let marks = axis.ticklines(false).unwrap();
        assert!(marks.iter().all(|mark| mark.marks.len() == 2));
        ax.grid(false);
        assert!(axis.gridlines().unwrap().iter().all(|line| !line.visible));
        let before = marks[1].marks[0];
        ax.view_init(10.0, 20.0, 0.0);
        assert_ne!(
            axis.ticklines(false).unwrap()[1].marks[0],
            before,
            "they move with the view"
        );
        assert_eq!(
            axis.ticklines(true).unwrap(),
            Vec::new(),
            "no minor ticks on z"
        );
        ax.xaxis()
            .set_minor_locator(MultipleLocator::new(0.1).unwrap());
        assert_ne!(ax.xaxis().ticklines(true).unwrap(), Vec::new());
    }

    #[test]
    fn date_axes_keep_explicit_tick_helpers() {
        let ax = Figure::new().subplot_3d();
        ax.set_zlim(19_000.0, 19_003.0).unwrap();
        ax.zaxis().axis_date(&TimeZone::Utc);
        assert_eq!(ax.zlim(), (19_000.0, 19_003.0), "limits stay");
        let labels = ax.zaxis().ticklabels(false).unwrap();
        assert_eq!(labels[..2], ["01-08 00", "01-08 12"], "twice a day");
        let ax = Figure::new().subplot_3d();
        ax.zaxis().set_major_formatter(NullFormatter);
        ax.zaxis().axis_date(&TimeZone::Utc);
        let labels = ax.zaxis().ticklabels(false).unwrap();
        assert!(labels.iter().all(String::is_empty), "{labels:?}");
        assert_eq!(ax.zaxis().ticklocs(false).unwrap()[1], 0.125, "date ticks");
    }
}
