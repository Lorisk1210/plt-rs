//! Shared and twinned axes (matplotlib's `sharex`/`sharey`, `twinx`/`twiny`
//! and `label_outer`).

use std::rc::Rc;

use super::{Adjustable, Axes, AxesState, AxisName, PlotError};
use crate::artist::{Artist, ArtistHandle, RawHandle};
use crate::figure::insert_axes;
use crate::spines::SpineSide;

impl Axes {
    /// Shares the x axis with `other`: both then show the same x range
    /// and ticks, autoscaled from the data of both, and changing the
    /// limits or scale of one changes all (matplotlib's `sharex`).
    ///
    /// ```
    /// use plt::figure::Figure;
    /// let fig = Figure::new();
    /// let (top, bottom) = (fig.add_subplot(2, 1, 1)?, fig.add_subplot(2, 1, 2)?);
    /// bottom.share_x(&top)?;
    /// top.plot([0.0, 1.0], [0.0, 1.0])?;
    /// bottom.plot([3.0, 4.0], [0.0, 1.0])?;
    /// assert_eq!(bottom.xlim(), top.xlim());
    /// assert_eq!(top.xlim(), (-0.2, 4.2));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// When the x axis is already shared with a different axes, or `other`
    /// belongs to another figure.
    pub fn share_x(&self, other: &Self) -> Result<&Self, PlotError> {
        self.share(AxisName::X, other)
    }

    /// Shares the y axis with `other` (matplotlib's `sharey`); see
    /// [`Axes::share_x`].
    ///
    /// # Errors
    ///
    /// As [`Axes::share_x`].
    pub fn share_y(&self, other: &Self) -> Result<&Self, PlotError> {
        self.share(AxisName::Y, other)
    }

    /// Shares axis `name` with `other`.
    pub(crate) fn share(&self, name: AxisName, other: &Self) -> Result<&Self, PlotError> {
        let figure = self.raw().figure_state();
        if !Rc::ptr_eq(figure, other.raw().figure_state()) {
            return Err(PlotError::ForeignAxes);
        }
        figure
            .borrow_mut()
            .share_axis(name, self.id(), other.id())?;
        Ok(self)
    }

    /// This axes and the axes sharing axis `name` with it (matplotlib's
    /// `get_shared_x_axes().get_siblings(ax)`).
    ///
    /// ```
    /// use plt::figure::{Figure, Share};
    /// use plt::ticker::AxisName;
    /// let fig = Figure::new();
    /// let axs = fig.subplots_shared(1, 2, Share::None, Share::All)?;
    /// assert_eq!(axs[(0, 0)].shared_axes(AxisName::Y).len(), 2);
    /// assert_eq!(axs[(0, 0)].shared_axes(AxisName::X), [axs[(0, 0)].clone()]);
    /// # Ok::<(), plt::gridspec::GridSpecError>(())
    /// ```
    pub fn shared_axes(&self, name: AxisName) -> Vec<Self> {
        let figure = self.raw.figure_state();
        let ids = figure.borrow().siblings(name, self.id());
        ids.into_iter()
            .map(|id| Self::from_raw(RawHandle::new(Rc::clone(figure), id)))
            .collect()
    }

    /// This axes and the axes twinned with it by [`Axes::twinx`] and
    /// [`Axes::twiny`] (matplotlib's `_twinned_axes.get_siblings`).
    pub fn twins(&self) -> Vec<Self> {
        let figure = self.raw.figure_state();
        let ids = figure.borrow().shared.twins(self.id());
        ids.into_iter()
            .map(|id| Self::from_raw(RawHandle::new(Rc::clone(figure), id)))
            .collect()
    }

    /// A new axes on top of this one sharing its x axis, with its y axis
    /// on the right (matplotlib's `twinx`).
    #[must_use]
    pub fn twinx(&self) -> Self {
        self.twin_with(AxisName::X, 0.0)
    }

    /// A new axes on top of this one sharing its y axis, with its x axis
    /// on top (matplotlib's `twiny`).
    #[must_use]
    pub fn twiny(&self) -> Self {
        self.twin_with(AxisName::Y, 0.0)
    }

    /// A new axes on top of this one sharing axis `shared`, with its other
    /// axis on the far side, drawn `delta_zorder` above this one; of the
    /// twins only the bottom-most draws the background (matplotlib's
    /// `_make_twin_axes` and the `twinx`/`twiny(delta_zorder=...)` set-up).
    #[must_use]
    pub fn twin_with(&self, shared: AxisName, delta_zorder: f64) -> Self {
        let own = match shared {
            AxisName::X => AxisName::Y,
            AxisName::Y => AxisName::X,
        };
        let (frame, position, spec, zorder, autoscale, units) =
            self.raw.with::<AxesState, _>(|axes| {
                (
                    axes.base().parent,
                    axes.position,
                    axes.subplot_spec.clone(),
                    axes.base().zorder,
                    axes.axis(shared).autoscale.enabled,
                    axes.axis(shared).unit_data(),
                )
            });
        let figure = self.raw().figure_state();
        let frame = frame.expect("an axes always lives in a figure frame");
        let twin = insert_axes(figure, frame, position, spec);
        twin.share(shared, self)
            .expect("a new axes in this figure shares nothing yet");
        figure.borrow_mut().shared.join_twins(self.id(), twin.id());
        self.store_adjustable(Adjustable::Datalim);
        twin.store_adjustable(Adjustable::Datalim);
        twin.set_zorder(zorder + delta_zorder);
        twin.update_twinned_patch_visibility();
        twin.raw.with_mut::<AxesState, _>(|axes| {
            axes.axis_mut(shared).autoscale.enabled = autoscale;
            axes.axis_mut(shared).visible = false;
            axes.axis_mut(shared).set_unit_data(units);
            let axis = axes.axis_mut(own);
            axis.tick_to(true);
            axis.label.move_to(SpineSide::of(own, true));
        });
        self.raw
            .with_mut::<AxesState, _>(|axes| axes.axis_mut(own).tick_to(false));
        twin
    }

    /// Hides the tick labels, axis labels and offset texts that face
    /// neighbouring subplots, keeping only those on the outer edges of the
    /// grid (matplotlib's `label_outer`). Axes not placed on a grid are
    /// left alone.
    pub fn label_outer(&self) -> &Self {
        self.label_outer_with(false)
    }

    /// [`label_outer`](Self::label_outer), also removing the inner tick
    /// marks when `remove_inner_ticks` is set (matplotlib's
    /// `label_outer(remove_inner_ticks=True)`).
    pub fn label_outer_with(&self, remove_inner_ticks: bool) -> &Self {
        self.label_outer_axis(AxisName::X, remove_inner_ticks);
        self.label_outer_axis(AxisName::Y, remove_inner_ticks);
        self
    }

    /// What grids sharing axis `name` do to their axes (matplotlib's
    /// `_label_outer_xaxis(skip_non_rectangular_axes=True)`): the inner
    /// labels go, except on axes of curved projections, which have no
    /// inner sides.
    pub(crate) fn label_outer_shared(&self, name: AxisName) {
        let rectilinear = self
            .raw
            .with::<AxesState, _>(|axes| axes.projection.is_rectilinear());
        if rectilinear {
            self.label_outer_axis(name, false);
        }
    }

    /// matplotlib's `_label_outer_xaxis` / `_label_outer_yaxis`; with
    /// `remove_inner_ticks` the inner tick marks go too.
    fn label_outer_axis(&self, name: AxisName, remove_inner_ticks: bool) {
        self.raw.with_mut::<AxesState, _>(|axes| {
            let Some(spec) = axes.subplot_spec.clone() else {
                return;
            };
            let (inner_low, inner_high) = match name {
                AxisName::X => (!spec.is_last_row(), !spec.is_first_row()),
                AxisName::Y => (!spec.is_first_col(), !spec.is_last_col()),
            };
            let axis = axes.axis_mut(name);
            for (inner, high) in [(inner_high, true), (inner_low, false)] {
                if !inner {
                    continue;
                }
                let side = SpineSide::of(name, high);
                if axis.label.side == side {
                    axis.label.text.clear();
                }
                axis.restyle_ticks(|style| {
                    let (labels, ticks) = if high {
                        (&mut style.labels.second, &mut style.ticks.second)
                    } else {
                        (&mut style.labels.first, &mut style.ticks.first)
                    };
                    *labels = false;
                    if remove_inner_ticks {
                        *ticks = false;
                    }
                });
                if axis.offset_side == side {
                    axis.offset_visible = false;
                }
            }
        });
    }

    /// Shows the background patch of the bottom-most axes of a group of
    /// twinned axes only (the lowest z-order, the first added among
    /// equals), and only if it draws its frame (matplotlib's
    /// `_update_twinned_axes_patch_visibility`).
    pub(crate) fn update_twinned_patch_visibility(&self) {
        let figure = self.raw().figure_state();
        let twins = figure.borrow().shared.twins(self.id());
        if twins.len() < 2 {
            return;
        }
        let mut state = figure.borrow_mut();
        let order = |id| {
            let zorder = state.store.get(id).map_or(0.0, |a| a.base().zorder);
            (zorder, id)
        };
        let bottom = twins
            .iter()
            .copied()
            .min_by(|&a, &b| {
                let ((za, ia), (zb, ib)) = (order(a), order(b));
                za.total_cmp(&zb).then(ia.cmp(&ib))
            })
            .expect("a group of twins is not empty");
        for id in twins {
            if let Some(axes) = state.store.get_as_mut::<AxesState>(id) {
                axes.patch.visible = id == bottom && axes.decorations.frame_on;
            }
        }
    }

    /// Shows or hides the background patch (matplotlib's
    /// `ax.patch.set_visible`).
    pub fn set_patch_visible(&self, visible: bool) -> &Self {
        self.raw
            .with_mut::<AxesState, _>(|axes| axes.patch.visible = visible);
        self
    }

    /// Whether the background patch is drawn.
    pub fn patch_visible(&self) -> bool {
        self.raw.with::<AxesState, _>(|axes| axes.patch.visible)
    }
}

#[cfg(test)]
mod tests {
    use crate::artist::Artist;
    use crate::axes::{AxesState, AxisName};
    use crate::figure::{Figure, Share};
    use crate::prelude::*;
    use crate::spines::SpineSide;
    use crate::units::UnitConverter;

    #[test]
    fn twins_take_over_the_units_of_the_shared_axis() {
        let ax = Figure::new().subplot();
        ax.plot(["0", "1"], ["a", "b"]).unwrap();
        let (twin_x, twin_y) = (ax.twinx(), ax.twiny());
        assert_eq!(twin_x.xaxis().converter(), Some(UnitConverter::Categories));
        assert_eq!(twin_x.xaxis().convert_units(["1", "0"]), [1.0, 0.0]);
        assert_eq!(twin_y.yaxis().convert_units(["b", "a"]), [1.0, 0.0]);
        assert!(!twin_x.yaxis().have_units());
    }

    #[test]
    fn twinx_puts_its_y_axis_on_the_right() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_zorder(3.0);
        ax.plot([0.0, 10.0], [0.0, 1.0]).unwrap();
        let twin = ax.twinx();
        twin.plot([0.0, 20.0], [5.0, 6.0]).unwrap();
        assert_eq!(ax.xlim(), twin.xlim());
        assert_eq!(ax.xlim(), (-1.0, 21.0));
        assert_eq!(twin.position(), ax.position());
        assert_eq!(twin.zorder(), 3.0);
        assert!(!twin.xaxis().visible());
        assert_eq!(twin.yaxis().label_position(), SpineSide::Right);
        twin.raw().with::<AxesState, _>(|state| {
            assert!(!state.patch.visible);
            let style = &state.yaxis.major_style;
            assert!(style.ticks.second && !style.ticks.first);
            assert!(style.labels.second && !style.labels.first);
            assert_eq!(state.yaxis.offset_side, SpineSide::Right);
        });
    }

    #[test]
    fn twiny_puts_its_x_axis_on_top() {
        let ax = Figure::new().subplot();
        let twin = ax.twiny();
        assert!(!twin.yaxis().visible());
        assert_eq!(twin.xaxis().label_position(), SpineSide::Top);
        assert_eq!(
            twin.yaxis().set_label_position(SpineSide::Top).err(),
            Some(crate::axes::PlotError::WrongSide {
                axis: AxisName::Y,
                side: SpineSide::Top
            })
        );
    }

    #[test]
    fn label_outer_keeps_only_the_grid_edges() {
        let fig = Figure::new();
        let axs = fig.subplots(2, 2).unwrap();
        for ax in axs.as_slice() {
            ax.set_xlabel("x").set_ylabel("y").label_outer();
        }
        let labels = |row: usize, col: usize| {
            let ax = &axs[(row, col)];
            let shown = ax.raw().with::<AxesState, _>(|state| {
                (
                    state.xaxis.major_style.labels.first,
                    state.yaxis.major_style.labels.first,
                )
            });
            (ax.label(AxisName::X), ax.label(AxisName::Y), shown)
        };
        assert_eq!(labels(1, 0), ("x".into(), "y".into(), (true, true)));
        assert_eq!(labels(0, 0), (String::new(), "y".into(), (false, true)));
        assert_eq!(labels(1, 1), ("x".into(), String::new(), (true, false)));
        assert_eq!(labels(0, 1), (String::new(), String::new(), (false, false)));
    }

    #[test]
    fn label_outer_can_remove_the_inner_ticks_too() {
        let fig = Figure::new();
        let axs = fig.subplots(2, 1).unwrap();
        let ticks = |row: usize| {
            axs[(row, 0)]
                .raw()
                .with::<AxesState, _>(|state| state.xaxis.major_style.ticks.first)
        };
        axs[(0, 0)].label_outer();
        assert!(ticks(0), "label_outer keeps the tick marks");
        axs[(0, 0)].label_outer_with(true);
        assert!(!ticks(0) && ticks(1));
    }

    #[test]
    fn shared_subplots_hide_inner_tick_labels() {
        let fig = Figure::new();
        let axs = fig.subplots_shared(2, 1, Share::Row, Share::None).unwrap();
        let shown = |row: usize| {
            axs[(row, 0)]
                .raw()
                .with::<AxesState, _>(|state| state.xaxis.major_style.labels.first)
        };
        assert!(shown(0), "rows sharing x keep every label");
        let fig = Figure::new();
        let axs = fig.subplots_shared(2, 1, Share::Col, Share::None).unwrap();
        let shown = |row: usize| {
            axs[(row, 0)]
                .raw()
                .with::<AxesState, _>(|state| state.xaxis.major_style.labels.first)
        };
        assert!(!shown(0) && shown(1));
    }

    #[test]
    fn a_twin_below_draws_the_background() {
        // matplotlib's test_twinning_patch_visibility_respects_delta_zorder.
        let ax = Figure::new().subplot();
        let below = ax.twin_with(AxisName::X, -1.0);
        assert_eq!(below.zorder(), ax.zorder() - 1.0);
        assert!(below.patch_visible() && !ax.patch_visible());
        assert_eq!(ax.twins().len(), 2);
    }

    #[test]
    fn twins_move_and_reshape_together() {
        // matplotlib's set_position and set_box_aspect act on every twin.
        let ax = Figure::new().subplot();
        let twin = ax.twinx();
        ax.set_position([0.2, 0.2, 0.5, 0.5]);
        assert_eq!(twin.original_position(), ax.original_position());
        assert!(
            !ax.raw()
                .with::<AxesState, _>(|state| state.base().in_layout)
        );
        twin.set_box_aspect(Some(1.0)).unwrap();
        assert_eq!(ax.box_aspect(), Some(1.0));
        assert_eq!(twin.position(), ax.position());
    }

    #[test]
    fn twins_follow_constrained_layout_together() {
        // matplotlib's test_twin_stays_aligned_after_constrained_layout: the
        // host left the layout, the twin did not, and moves it along.
        let fig = Figure::new();
        fig.set_layout_engine(Some(crate::layout::LayoutEngine::Constrained(
            crate::layout::ConstrainedLayout::default(),
        )));
        let ax = fig.subplot();
        ax.set_position([0.2, 0.2, 0.5, 0.5]);
        let twin = ax.twinx();
        fig.layout();
        assert_eq!(ax.position(), twin.position());
        assert!(ax.position().width() > 0.5, "the layout took over");
    }

    #[test]
    fn shared_axes_list_the_group() {
        let fig = Figure::new();
        let axs = fig.subplots_shared(2, 2, Share::All, Share::Row).unwrap();
        assert_eq!(axs[(0, 0)].shared_axes(AxisName::X).len(), 4);
        assert_eq!(axs[(0, 0)].shared_axes(AxisName::Y).len(), 2);
        assert!(axs[(1, 1)].shared_axes(AxisName::Y).contains(&axs[(1, 0)]));
    }
}
