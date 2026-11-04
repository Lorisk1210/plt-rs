//! Where matplotlib updates the ticks of an axis (`Axis._update_ticks`),
//! for the axes the program ticks (see [`TickUpdater`](super::TickUpdater)):
//! the program's helpers are called at these sites, as often and in the
//! same order as matplotlib calls them.
//!
//! * `Axis.draw` and `Axis.get_tightbbox` update the axis' ticks, then
//!   place its label (`_update_label_position`), which updates the ticks
//!   of the same axis of every axes whose labels align with it, itself
//!   included, then measures the spine on the label's side, which updates
//!   the ticks of the axis it is registered with; then they read the
//!   major formatter's offset text.
//! * `Axes._update_title_position`, with a title, measures the x axis of
//!   every axes of its group (itself, its twins, its insets, the axes its
//!   titles align with) whose decorations are on top, and for each title
//!   with a text the y axis of each, to place the offset text.
//! * `Axes.draw` places its titles, then draws its x axis and its y axis.
//! * `Axes.get_tightbbox` measures its axes, places its titles, then
//!   measures its spines, each updating the ticks of the axis registered
//!   with it.
//!
//! Every site does nothing for an axis the program does not tick, and the
//! sites are only walked in figures with an updater.

use super::{AxesState, SpineAxis};
use crate::artist::{Artist, ArtistId};
use crate::figure::FigureState;
use crate::spines::{SpineArtist, SpineSide};
use crate::ticker::AxisName;

/// The axis matplotlib registers with the spine on `side` of rectilinear
/// axes.
fn rectilinear_spine_axis(side: SpineSide) -> AxisName {
    match side {
        SpineSide::Left | SpineSide::Right => AxisName::Y,
        SpineSide::Bottom | SpineSide::Top => AxisName::X,
    }
}

impl FigureState {
    /// Whether the program may tick axes of this figure, so that the sites
    /// are worth walking.
    pub(crate) fn ticks_by_program(&self) -> bool {
        self.tick_updater.is_some() && self.lendable()
    }

    /// The spines of `axes` in matplotlib's order (`axes.spines`), each
    /// with whether it is shown and laid out and the axis registered with
    /// it: the four of rectilinear axes, with their axes; for polar axes
    /// the outer one, registered with the radial axis, then the start,
    /// end and inner ones; for another projection those the program's
    /// class names.
    fn spine_axes(&mut self, axes: ArtistId) -> Vec<SpineAxis> {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return Vec::new();
        };
        if state.projection.custom().is_some() || state.projection.polar_program().is_some() {
            return self.program_spines(axes).unwrap_or_default();
        }
        let polar = state.polar().is_some();
        let sides: &[(SpineSide, &str)] = if polar {
            &[
                (SpineSide::Top, "polar"),
                (SpineSide::Left, "start"),
                (SpineSide::Right, "end"),
                (SpineSide::Bottom, "inner"),
            ]
        } else {
            &[
                (SpineSide::Left, "left"),
                (SpineSide::Right, "right"),
                (SpineSide::Bottom, "bottom"),
                (SpineSide::Top, "top"),
            ]
        };
        sides
            .iter()
            .filter_map(|&(side, name)| {
                let spine = state
                    .spines
                    .iter()
                    .filter_map(|&id| self.store.get_as::<SpineArtist>(id))
                    .find(|spine| spine.side == side)?;
                let axis = match (polar, side) {
                    (false, side) => Some(rectilinear_spine_axis(side)),
                    (true, SpineSide::Top) => Some(AxisName::Y),
                    (true, _) => None,
                };
                let base = spine.base();
                Some(SpineAxis {
                    name: name.to_owned(),
                    shown: base.visible && base.in_layout,
                    axis,
                })
            })
            .collect()
    }

    /// `Axis.draw` or `Axis.get_tightbbox` of the shown axis `name` of
    /// `axes`, as far as it ticks axes.
    pub(crate) fn program_axis_measured(&mut self, axes: ArtistId, name: AxisName) {
        self.update_program_ticks(axes, name);
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let label = &state.axis(name).label;
        if label.coords.is_none() {
            let side = label.side;
            let siblings = self.label_siblings(axes, name);
            for sibling in siblings {
                self.update_program_ticks(sibling, name);
            }
            // The spine named after the label's side, when the axes has
            // one.
            let spine = (self.spine_axes(axes).into_iter()).find(|spine| spine.name == side.name());
            if let Some(registered) = spine.and_then(|spine| spine.axis)
                && self.axis_shown(axes, registered)
            {
                self.update_program_ticks(axes, registered);
            }
        }
        self.update_program_offset(axes, name);
    }

    /// The axes whose `name` labels align with those of `axes`, itself
    /// included (matplotlib's `_align_label_groups[name].get_siblings`).
    fn label_siblings(&self, axes: ArtistId, name: AxisName) -> Vec<ArtistId> {
        match self.store.get(axes).and_then(|a| a.base().parent) {
            Some(frame) => {
                self.layout
                    .label_groups
                    .siblings(frame, crate::layout::Aligned::Labels(name), axes)
            }
            None => vec![axes],
        }
    }

    /// Whether axis `name` of `axes` is shown (matplotlib's
    /// `axis.get_visible()`).
    fn axis_shown(&self, axes: ArtistId, name: AxisName) -> bool {
        self.store
            .get_as::<AxesState>(axes)
            .is_some_and(|state| state.axis(name).visible)
    }

    /// `Axes._update_title_position` of `axes`, as far as it ticks axes.
    pub(crate) fn program_title_position(&mut self, axes: ArtistId) {
        let Some(state) = self.store.get_as::<AxesState>(axes) else {
            return;
        };
        let titles: Vec<ArtistId> = state.titles_centre_first().collect();
        if state.title_placement == super::state::TitlePlacement::Fixed
            || !titles.iter().any(|&title| self.has_text(title))
        {
            return;
        }
        // matplotlib's set of axes: each once.
        let mut group = vec![axes];
        for ax in self.title_group(axes) {
            if !group.contains(&ax) {
                group.push(ax);
            }
        }
        for &ax in &group {
            let on_top = self
                .store
                .get_as::<AxesState>(ax)
                .is_some_and(|state| state.axis(AxisName::X).decorations_on_top());
            if on_top && self.axis_shown(ax, AxisName::X) {
                self.program_axis_measured(ax, AxisName::X);
            }
        }
        for title in titles {
            if self.has_text(title) {
                for &ax in &group {
                    if self.axis_shown(ax, AxisName::Y) {
                        self.program_axis_measured(ax, AxisName::Y);
                    }
                }
            }
        }
    }

    /// `Axes.draw` of `axes` up to its artists, as far as it ticks axes:
    /// the title position, then its axes.
    pub(crate) fn program_axes_drawn(&mut self, axes: ArtistId) {
        if !self.ticks_by_program() || !self.axes_shown(axes) {
            return;
        }
        self.program_title_position(axes);
        if self.axes_axis_on(axes) {
            for name in [AxisName::X, AxisName::Y] {
                if self.axis_shown(axes, name) {
                    self.program_axis_measured(axes, name);
                }
            }
        }
    }

    /// `Axes.get_tightbbox` of `axes` (`for_layout_only` for a layout
    /// engine), as far as it ticks axes: its axes, the title position,
    /// then its spines `with_spines` (unless the program gave the artists
    /// to measure besides).
    pub(crate) fn program_axes_measured(
        &mut self,
        axes: ArtistId,
        for_layout_only: bool,
        with_spines: bool,
    ) {
        if !self.ticks_by_program() || !self.axes_shown(axes) {
            return;
        }
        let axis_on = self.axes_axis_on(axes);
        if axis_on {
            for name in [AxisName::X, AxisName::Y] {
                let measured = self.store.get_as::<AxesState>(axes).is_some_and(|state| {
                    let axis = state.axis(name);
                    axis.visible && (axis.in_layout || !for_layout_only)
                });
                if measured {
                    self.program_axis_measured(axes, name);
                }
            }
        }
        self.program_title_position(axes);
        let framed = self
            .store
            .get_as::<AxesState>(axes)
            .is_some_and(|state| state.decorations.framed());
        if framed && with_spines {
            for spine in self.spine_axes(axes) {
                if let Some(name) = spine.axis.filter(|_| spine.shown)
                    && self.axis_shown(axes, name)
                {
                    self.update_program_ticks(axes, name);
                }
            }
        }
    }

    /// Whether `axes` is shown.
    fn axes_shown(&self, axes: ArtistId) -> bool {
        self.store
            .get_as::<AxesState>(axes)
            .is_some_and(|state| state.base().visible)
    }

    /// Whether `axes` draws its axes (matplotlib's `axison`).
    fn axes_axis_on(&self, axes: ArtistId) -> bool {
        self.store
            .get_as::<AxesState>(axes)
            .is_some_and(|state| state.decorations.axis_on)
    }

    /// `Figure.draw`'s draws of the axes `axes` (in drawing order),
    /// `draws` times, as far as they tick axes.
    pub(crate) fn program_figure_drawn(&mut self, axes: &[ArtistId], draws: usize) {
        if !self.ticks_by_program() {
            return;
        }
        for _ in 0..draws {
            for &id in axes {
                self.program_axes_drawn(id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::axes::{
        Axes, SpineAxis, TickLevels, TickUpdater, TitleLocation, UpdatedLevel, UpdatedTicks,
    };
    use crate::figure::Figure;
    use crate::layout::LayoutEngine;
    use crate::ticker::{AxisInfo, AxisName, Locator, TickContext, TickerError};

    /// A locator of the program's (its calls are observed).
    #[derive(Debug)]
    struct Observed;

    impl Locator for Observed {
        fn tick_values(
            &self,
            _: f64,
            _: f64,
            _: &TickContext<'_>,
        ) -> Result<Vec<f64>, TickerError> {
            Ok(vec![0.5])
        }

        fn calls_program(&self) -> bool {
            true
        }
    }

    /// An updater recording the axes it ticks, which labels the tick with
    /// the axes' title (read from the figure) and retitles the axes.
    #[derive(Debug, Default)]
    struct Recording {
        updates: RefCell<Vec<AxisName>>,
    }

    impl TickUpdater for Recording {
        fn update_ticks(
            &self,
            axes: &Axes,
            axis: AxisName,
            _: &AxisInfo,
            _: TickLevels,
        ) -> Result<UpdatedTicks, TickerError> {
            self.updates.borrow_mut().push(axis);
            let title = axes.title(TitleLocation::Center);
            axes.set_title(format!("{}", self.updates.borrow().len()));
            Ok(UpdatedTicks {
                major: Some(UpdatedLevel {
                    locs: vec![0.5],
                    labels: vec![title],
                    drawn: vec![true],
                }),
                minor: None,
            })
        }

        fn offset(&self, _: &Axes, _: AxisName, _: &AxisInfo) -> String {
            String::new()
        }

        fn spines(&self, _: &Axes) -> Vec<SpineAxis> {
            Vec::new()
        }
    }

    /// A figure whose x axis the program ticks through `updater`.
    fn ticked(updater: &Rc<Recording>) -> (Figure, Axes) {
        let fig = Figure::with_size(4.0, 3.0, 50.0);
        let ax = fig.subplot();
        ax.xaxis().set_major_locator(Observed);
        fig.set_tick_updater(Some(Rc::clone(updater) as Rc<dyn TickUpdater>));
        (fig, ax)
    }

    #[test]
    fn a_draw_ticks_an_axis_three_times_lent_the_figure() {
        let updater = Rc::new(Recording::default());
        let (fig, ax) = ticked(&updater);
        fig.layout();
        // matplotlib's `Axis.draw`: the update, the label's siblings and
        // spine.
        assert_eq!(*updater.updates.borrow(), [AxisName::X; 3]);
        // The program read the figure and changed it at once.
        assert_eq!(ax.title(TitleLocation::Center), "3");
        let ticks = ax.xaxis().placed_ticks().expect("placed");
        assert_eq!(
            ticks.major[0].label, "2",
            "the title before the last update"
        );
    }

    #[test]
    fn a_print_with_a_layout_engine_ticks_as_matplotlib() {
        let updater = Rc::new(Recording::default());
        let (fig, _) = ticked(&updater);
        fig.set_layout_engine(LayoutEngine::tight())
            .expect("no colorbars");
        fig.to_png().expect("draws");
        // The engine's measure (the axis, then its two spines; without a
        // rectangle to fit, tight layout measures once), then the draw
        // before the print and the print.
        assert_eq!(updater.updates.borrow().len(), 5 + 2 * 3);
    }

    #[test]
    fn axes_ticked_by_pltrs_alone_are_not_given_to_the_updater() {
        let updater = Rc::new(Recording::default());
        let fig = Figure::new();
        fig.subplot();
        fig.set_tick_updater(Some(Rc::clone(&updater) as Rc<dyn TickUpdater>));
        fig.layout();
        assert!(updater.updates.borrow().is_empty());
    }
}
