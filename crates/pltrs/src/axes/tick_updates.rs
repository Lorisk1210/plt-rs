//! Ticks the program computes: an axis whose tick helpers or scale run
//! code of the program's (a host language's formatter, locator or scale)
//! is ticked by the program's [`TickUpdater`], which runs matplotlib's
//! `Axis._update_ticks` with them.
//!
//! pltrs computes the ticks of its own helpers wherever it needs them,
//! as often as it needs them. Code of the program's sees how often and
//! with what it is called: matplotlib updates an axis' ticks three times
//! when it draws it, more when it measures it for a layout engine, and
//! programs observe it (a formatter counting its calls, a scale's
//! transform). So an axis whose helpers or scale call the program is
//! ticked by the program, through the figure's updater
//! ([`Figure::set_tick_updater`]), at matplotlib's call sites (see
//! `tick_sites`), with the figure lent to it
//! ([`FigureState::lend`]): its code reads the figure as it is laid out
//! and changes it at once. Elsewhere, the ticks it gave last stand as long
//! as they were computed for the same geometry.

use std::fmt;
use std::rc::Rc;

use pltrs_core::Bbox;

use super::axis::{AxisGeometry, AxisState};
use super::tick_set::{Tick, TickSet};
use super::{Axes, AxesState};
use crate::artist::{ArtistId, RawHandle};
use crate::figure::{Figure, FigureState};
use crate::ticker::{AxisInfo, AxisName, TickSpace, TickerError};
use crate::transforms::TransformContext;

/// The program's code ticking the axes whose tick helpers or scale call
/// it, as matplotlib's `Axis` ticks them, at matplotlib's call sites:
/// matplotlib updates an axis' ticks (`Axis._update_ticks`) three times
/// when it draws it, more when it measures it for a layout engine, and a
/// program observes how often its code is called. An axis ticks through
/// the updater when one of its locators, formatters or its scale says it
/// [`calls_program`](crate::ticker::Locator::calls_program), each time
/// with the figure free for the updater to read and change (pltrs (Rust)
/// lends it the figure it is laying out). pltrs (Python) runs
/// matplotlib's `_update_ticks` with the program's objects.
pub trait TickUpdater: fmt::Debug {
    /// One `Axis._update_ticks` of axis `axis` of `axes`, which `info`
    /// describes as it is now, computing the levels `levels` shows: the
    /// locations and labels of each level and which of them are drawn.
    ///
    /// # Errors
    ///
    /// When the program's helpers fail.
    fn update_ticks(
        &self,
        axes: &Axes,
        axis: AxisName,
        info: &AxisInfo,
        levels: TickLevels,
    ) -> Result<UpdatedTicks, TickerError>;

    /// The offset text of the axis' major formatter once its ticks are
    /// updated (matplotlib's `major.formatter.get_offset()`, which
    /// `Axis.draw` and `get_tightbbox` read).
    fn offset(&self, axes: &Axes, axis: AxisName, info: &AxisInfo) -> String;

    /// The spines of `axes`, of a projection the program defines (or
    /// whose spines it names: geographic axes), as matplotlib's
    /// `axes.spines` holds them, in order: measuring a spine updates the
    /// ticks of the axis registered with it.
    fn spines(&self, axes: &Axes) -> Vec<SpineAxis>;

    /// Whether axis `axis` of `axes`, whose class of the program's
    /// answers its view interval itself, is inverted, as that class
    /// answers (matplotlib's `Axis.get_inverted`, which `autoscale_view`
    /// asks through `set_xbound`); `None`, the default, when the updater
    /// cannot say, pltrs then answering itself.
    fn inverted(&self, axes: &Axes, axis: AxisName) -> Option<bool> {
        let _ = (axes, axis);
        None
    }
}

/// A spine of an axes as the tick updates see it (see
/// [`TickUpdater::spines`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpineAxis {
    /// Its name in `axes.spines` (`"left"`, `"polar"`, `"geo"`...).
    pub name: String,
    /// Whether it is shown and laid out (measured by a tight box).
    pub shown: bool,
    /// The axis registered with it, if any.
    pub axis: Option<AxisName>,
}

/// The levels of ticks an axis shows (matplotlib's `_tick_group_visible`
/// of its major and minor tick parameters): a level whose marks, labels
/// and gridlines are all off is not computed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TickLevels {
    /// Whether the major ticks are shown.
    pub major: bool,
    /// Whether the minor ticks are shown.
    pub minor: bool,
}

/// The ticks of one level that one update gives.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpdatedLevel {
    /// Every location of the level, inside the view or not (minor ones on
    /// major ticks left out unless the axis keeps them).
    pub locs: Vec<f64>,
    /// The label of each location.
    pub labels: Vec<String>,
    /// Whether each location is drawn: inside the view, through the
    /// axis' scale (matplotlib's `ticks_to_draw`).
    pub drawn: Vec<bool>,
}

/// What one update gives: each level computed, `None` for a level not
/// computed (not shown, or without a locator).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpdatedTicks {
    /// The major ticks.
    pub major: Option<UpdatedLevel>,
    /// The minor ticks.
    pub minor: Option<UpdatedLevel>,
}

/// The ticks of a level: every one, the positions of those drawn among
/// them, and those drawn.
type LevelTicks = (Vec<Tick>, Vec<usize>, Vec<Tick>);

impl UpdatedLevel {
    /// The ticks of the level, every one and those drawn with their
    /// positions among them.
    fn into_ticks(self) -> Result<LevelTicks, TickerError> {
        let count = self.locs.len();
        for (name, len) in [("labels", self.labels.len()), ("drawn", self.drawn.len())] {
            if len != count {
                return Err(TickerError::InvalidParameter {
                    name,
                    reason: format!("{len} {name} for {count} ticks"),
                });
            }
        }
        let all: Vec<Tick> = (self.locs.into_iter().zip(self.labels))
            .map(|(loc, label)| Tick { loc, label })
            .collect();
        let (index, drawn) = (all.iter().enumerate())
            .zip(self.drawn)
            .filter(|&(_, drawn)| drawn)
            .map(|((index, tick), _)| (index, tick.clone()))
            .unzip();
        Ok((all, index, drawn))
    }
}

impl UpdatedTicks {
    /// The tick set of the update, with the offset text `offset` (which
    /// matplotlib only reads where it draws or measures the axis).
    fn into_tick_set(self, offset: String) -> Result<TickSet, TickerError> {
        let mut ticks = TickSet {
            offset,
            ..TickSet::default()
        };
        if let Some(major) = self.major {
            ticks.major_locs.clone_from(&major.locs);
            (ticks.major_unclipped, ticks.major_index, ticks.major) = major.into_ticks()?;
        }
        if let Some(minor) = self.minor {
            ticks.minor_locs.clone_from(&minor.locs);
            (ticks.minor_unclipped, ticks.minor_index, ticks.minor) = minor.into_ticks()?;
            // The program gives the minor ticks off the major ones only.
            ticks.minor_all.clone_from(&ticks.minor);
        }
        Ok(ticks)
    }
}

/// What the program last ticked an axis for: while the axis is asked for
/// ticks with the same helpers, levels and geometry, they stand.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProgramTicked {
    geometry: AxisGeometry,
    levels: TickLevels,
    helpers: [usize; 4],
}

impl AxisState {
    /// Whether the axis' tick helpers or scale run code of the program's,
    /// or the program's class of the axis answers its view interval.
    pub(crate) fn calls_program(&self) -> bool {
        self.program_view
            || self.major_locator.calls_program()
            || self.minor_locator.calls_program()
            || self.major_formatter.calls_program()
            || self.minor_formatter.calls_program()
            || self.scale().calls_program()
    }

    /// The levels of ticks the axis shows.
    pub(crate) fn levels(&self) -> TickLevels {
        TickLevels {
            major: self.major_style.is_drawn(),
            minor: self.minor_style.is_drawn(),
        }
    }

    /// What the program ticks the axis for at `geometry`.
    fn program_record(&self, geometry: AxisGeometry) -> ProgramTicked {
        let address = |helper: *const ()| helper.addr();
        ProgramTicked {
            geometry,
            levels: self.levels(),
            helpers: [
                address(Rc::as_ptr(&self.major_locator).cast()),
                address(Rc::as_ptr(&self.minor_locator).cast()),
                address(Rc::as_ptr(&self.major_formatter).cast()),
                address(Rc::as_ptr(&self.minor_formatter).cast()),
            ],
        }
    }
}

/// The handle of `axes` in the figure of `cell`.
fn axes_handle(cell: &Rc<crate::figure::FigureCell>, axes: ArtistId) -> Axes {
    Axes::from_raw(RawHandle::new(Rc::clone(cell), axes))
}

impl FigureState {
    /// Whether axis `name` of `axes` is inverted as the program's class of
    /// the axis answers it, when that class answers its view interval
    /// itself (see [`TickUpdater::inverted`]), the figure lent meanwhile.
    pub(crate) fn program_inverted(&mut self, axes: ArtistId, name: AxisName) -> Option<bool> {
        let program = self
            .store
            .get_as::<AxesState>(axes)
            .is_some_and(|state| state.axis(name).program_view);
        let updater = self.tick_updater.as_ref().filter(|_| program)?;
        let updater = Rc::clone(updater);
        self.lend(|cell| updater.inverted(&axes_handle(cell, axes), name))
            .flatten()
    }

    /// The program's updater when it ticks axis `name` of `axes` (the
    /// axis' helpers or scale call the program) and can be lent the
    /// figure.
    pub(crate) fn program_ticker(
        &self,
        axes: ArtistId,
        name: AxisName,
    ) -> Option<Rc<dyn TickUpdater>> {
        let updater = self.tick_updater.as_ref()?;
        let state = self.store.get_as::<AxesState>(axes)?;
        (state.axis(name).calls_program() && self.lendable()).then(|| Rc::clone(updater))
    }

    /// The geometry axis `name` of `axes` is ticked at now and what its
    /// helpers know of it.
    fn tick_args(&self, axes: ArtistId, name: AxisName) -> Option<(AxisGeometry, AxisInfo)> {
        let extent = self.axes_bbox(axes)?;
        let tick_space = self.shared_tick_spaces(axes)[usize::from(name == AxisName::Y)];
        let state = self.store.get_as::<AxesState>(axes)?;
        let geometry = state.axis_geometry(name, &extent, self.dpi(), tick_space);
        Some((geometry, state.axis(name).info(&geometry)))
    }

    /// matplotlib's `Axis._update_ticks` of axis `name` of `axes` when the
    /// program ticks it: its updater ticks the axis where it is now, lent
    /// the figure. Returns whether it did.
    pub(crate) fn update_program_ticks(&mut self, axes: ArtistId, name: AxisName) -> bool {
        self.update_program_levels(axes, name, None)
    }

    /// [`update_program_ticks`](Self::update_program_ticks) for the
    /// `minor` level alone (or the major one), the other level's ticks
    /// kept as they were, as matplotlib's `set_ticklabels` places them.
    pub(crate) fn update_program_level(
        &mut self,
        axes: ArtistId,
        name: AxisName,
        minor: bool,
    ) -> bool {
        self.update_program_levels(axes, name, Some(minor))
    }

    /// [`update_program_ticks`](Self::update_program_ticks) for both levels
    /// the axis shows, or for the one level `only` names (`true` for the
    /// minor one).
    fn update_program_levels(
        &mut self,
        axes: ArtistId,
        name: AxisName,
        only: Option<bool>,
    ) -> bool {
        let Some(updater) = self.program_ticker(axes, name) else {
            return false;
        };
        let Some((geometry, info)) = self.tick_args(axes, name) else {
            return false;
        };
        let Some(shown) = self
            .store
            .get_as::<AxesState>(axes)
            .map(|state| state.axis(name).levels())
        else {
            return false;
        };
        let levels = match only {
            Some(minor) => TickLevels {
                major: !minor && shown.major,
                minor: minor && shown.minor,
            },
            None => shown,
        };
        let Some(updated) =
            self.lend(|cell| updater.update_ticks(&axes_handle(cell, axes), name, &info, levels))
        else {
            return false;
        };
        // The program may have removed the axes meanwhile.
        if let Some(state) = self.store.get_as_mut::<AxesState>(axes) {
            let axis = state.axis_mut(name);
            let offset = axis.ticks.as_ref().map(|ticks| ticks.offset.clone());
            let mut ticks =
                updated.and_then(|ticks| ticks.into_tick_set(offset.unwrap_or_default()));
            if let (Some(minor), Ok(ticks), Ok(before)) = (only, &mut ticks, &axis.ticks) {
                ticks.keep_level(before, !minor);
            }
            axis.install_ticks(ticks);
            axis.ticks_placed.set(true);
            // Ticks of one level stand for no update of the axis.
            axis.program_ticked = only.is_none().then(|| axis.program_record(geometry));
        }
        true
    }

    /// The spines of `axes` as the program names them (see
    /// [`TickUpdater::spines`]), when it ticks axes of this figure.
    pub(crate) fn program_spines(&mut self, axes: ArtistId) -> Option<Vec<SpineAxis>> {
        let updater = self.tick_updater.clone()?;
        self.lend(|cell| updater.spines(&axes_handle(cell, axes)))
    }

    /// The offset text of axis `name` of `axes` read from its major
    /// formatter when the program ticks it (matplotlib's
    /// `offsetText.set_text(major.formatter.get_offset())`).
    pub(crate) fn update_program_offset(&mut self, axes: ArtistId, name: AxisName) {
        let Some(updater) = self.program_ticker(axes, name) else {
            return;
        };
        let Some((_, info)) = self.tick_args(axes, name) else {
            return;
        };
        let Some(offset) = self.lend(|cell| updater.offset(&axes_handle(cell, axes), name, &info))
        else {
            return;
        };
        if let Some(state) = self.store.get_as_mut::<AxesState>(axes)
            && let Ok(ticks) = &mut state.axis_mut(name).ticks
        {
            ticks.offset = offset;
        }
    }

    /// Whether axis `name` of `axes` is animated, which matplotlib's draw
    /// leaves out (but printing).
    pub(crate) fn axis_left_out(&self, axes: ArtistId, name: AxisName) -> bool {
        !self.saving
            && (self.store.get_as::<AxesState>(axes)).is_some_and(|state| state.axis(name).animated)
    }

    /// Whether drawing the 2-D `axes` fails on the ticks of an axis it
    /// draws (a visible one its draw does not leave out), as matplotlib's
    /// `Axis._update_ticks` raises there.
    pub(crate) fn ticks_fail_draw(&self, axes: ArtistId) -> bool {
        self.store.get_as::<AxesState>(axes).is_some_and(|state| {
            [AxisName::X, AxisName::Y].into_iter().any(|name| {
                let axis = state.axis(name);
                axis.visible && axis.ticks.is_err() && !self.axis_left_out(axes, name)
            })
        })
    }

    /// The ticks of axis `name` of `axes`, at `extent` in display pixels
    /// at `dpi`, brought up to date where pltrs needs them: computed by
    /// its helpers, or for an axis the program ticks, those it gave last
    /// while they were for the same geometry, else the program's anew. An
    /// animated axis (see [`axis_left_out`](Self::axis_left_out)) is
    /// never ticked by the program meanwhile; measuring it
    /// (`get_tightbbox`) ticks it with its helpers.
    pub(crate) fn tick_axis_now(
        &mut self,
        axes: ArtistId,
        name: AxisName,
        extent: &Bbox,
        dpi: f64,
        tick_space: Option<TickSpace>,
    ) {
        if !self.axis_left_out(axes, name) && self.program_ticker(axes, name).is_some() {
            let current = self.store.get_as::<AxesState>(axes).map(|state| {
                let geometry = state.axis_geometry(name, extent, dpi, tick_space);
                let axis = state.axis(name);
                axis.program_ticked.as_ref() == Some(&axis.program_record(geometry))
            });
            if current == Some(true) || self.update_program_ticks(axes, name) {
                return;
            }
        }
        if let Some(state) = self.store.get_as_mut::<AxesState>(axes) {
            state.update_axis_ticks(name, extent, dpi, tick_space);
        }
    }

    /// [`tick_axis_now`](Self::tick_axis_now) for both axes of `axes`.
    pub(crate) fn tick_axes_now(
        &mut self,
        axes: ArtistId,
        extent: &Bbox,
        dpi: f64,
        shared: [Option<TickSpace>; 2],
    ) {
        for (name, tick_space) in [AxisName::X, AxisName::Y].into_iter().zip(shared) {
            self.tick_axis_now(axes, name, extent, dpi, tick_space);
        }
    }
}

impl Figure {
    /// Has `updater` tick the axes of the figure whose tick helpers or
    /// scale run code of the program's, at matplotlib's call sites (see
    /// [`TickUpdater`]); `None` removes it. It lives only while the figure
    /// does, and is not archived.
    pub fn set_tick_updater(&self, updater: Option<Rc<dyn TickUpdater>>) {
        self.state_mut().tick_updater = updater;
    }
}
