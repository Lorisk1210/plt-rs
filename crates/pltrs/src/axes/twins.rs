//! Twinned axes (matplotlib's `twinx` and `twiny`): axes on top of one
//! another sharing one axis, of which only the bottom-most draws the
//! background.

use super::aspect::check_adjustable;
use super::sharing::share_free;
use super::{Adjustable, AspectError, Axes, AxesState, AxisName, GridOptions, TickLevel};
use crate::artist::{Artist, ArtistHandle};
use crate::figure::make_axes;
use crate::projections::{ProjectionError, ProjectionSpec, Projector};
use crate::spines::SpineSide;
use crate::units::UnitState;

impl Axes {
    /// This axes and the axes twinned with it by [`Axes::twinx`] and
    /// [`Axes::twiny`] (matplotlib's `_twinned_axes.get_siblings`).
    pub fn twins(&self) -> Vec<Self> {
        let ids = self.raw.figure_state().borrow().shared.twins(self.id());
        self.in_same_figure(ids)
    }

    /// A new axes on top of this one sharing its x axis, with its y axis
    /// on the right (matplotlib's `twinx`).
    ///
    /// # Errors
    ///
    /// As [`twin_with`](Self::twin_with).
    pub fn twinx(&self) -> Result<Self, AspectError> {
        self.twin_with(AxisName::X, 0.0)
    }

    /// A new axes on top of this one sharing its y axis, with its x axis
    /// on top (matplotlib's `twiny`).
    ///
    /// # Errors
    ///
    /// As [`twin_with`](Self::twin_with).
    pub fn twiny(&self) -> Result<Self, AspectError> {
        self.twin_with(AxisName::Y, 0.0)
    }

    /// A new axes on top of this one sharing axis `shared`, with its other
    /// axis on the far side, drawn `delta_zorder` above this one; of the
    /// twins only the bottom-most draws the background (matplotlib's
    /// `_make_twin_axes` and the `twinx`/`twiny(delta_zorder=...)` set-up).
    /// The twin of a removed axes is removed too, having no figure to go
    /// in.
    ///
    /// ```
    /// use pltrs::axes::{AspectError, AxisName, ProjectionKind};
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let twin = ax.twin_with(AxisName::X, 0.0)?;
    /// assert_eq!(ax.twins(), [ax.clone(), twin]);
    /// let globe = fig.subplot_with_projection(ProjectionKind::Mollweide)?;
    /// let refused = globe.twin_with(AxisName::X, 0.0);
    /// assert_eq!(refused, Err(AspectError::FixedDataRatio(ProjectionKind::Mollweide)));
    /// assert_eq!(fig.axes().len(), 3);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AspectError::FixedDataRatio`] when this axes fixes its data ratio,
    /// as polar and geographic axes do, so that twins cannot adjust their
    /// data limits to an aspect together (matplotlib's `ValueError` from
    /// `set_adjustable("datalim")`); no twin is made.
    pub fn twin_with(&self, shared: AxisName, delta_zorder: f64) -> Result<Self, AspectError> {
        self.raw
            .with::<AxesState, _>(|axes| check_adjustable(axes, Adjustable::Datalim))?;
        let twin = self.new_twin(shared);
        self.join_twin(&twin, shared, delta_zorder);
        Ok(twin)
    }

    /// [`twin_with`](Self::twin_with) making the twin with `projection`
    /// (matplotlib's `twinx(projection=..., **options)` and `twiny`).
    /// Like [`twin_with`](Self::twin_with), the twin adjusts its data
    /// limits to an aspect, as does this axes, which projections fixing
    /// their data ratio refuse.
    ///
    /// ```
    /// use pltrs::axes::{AxisName, ProjectionKind};
    /// use pltrs::figure::Figure;
    /// use pltrs::projections::ProjectionError;
    /// let fig = Figure::new();
    /// let ax = fig.subplot();
    /// let twin = ax.twin_with_projection(AxisName::X, 0.0, ProjectionKind::Rectilinear)?;
    /// assert_eq!(ax.twins(), [ax.clone(), twin]);
    /// let refused = ax.twin_with_projection(AxisName::X, 0.0, ProjectionKind::Polar);
    /// assert!(matches!(refused, Err(ProjectionError::Aspect(_))));
    /// assert_eq!(fig.axes().len(), 2);
    /// # Ok::<(), ProjectionError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Figure::subplot_with_projection`](crate::figure::Figure::subplot_with_projection),
    /// and [`ProjectionError::Aspect`] when this axes or the twin fixes
    /// its data ratio (no twin is made).
    pub fn twin_with_projection(
        &self,
        shared: AxisName,
        delta_zorder: f64,
        projection: impl Into<ProjectionSpec>,
    ) -> Result<Self, ProjectionError> {
        let projector = Projector::new(&projection.into())?;
        self.raw
            .with::<AxesState, _>(|axes| check_adjustable(axes, Adjustable::Datalim))?;
        let twin = projector.apply(self.new_twin(shared))?;
        if let Err(err) = twin
            .raw
            .with::<AxesState, _>(|axes| check_adjustable(axes, Adjustable::Datalim))
        {
            twin.remove();
            return Err(err.into());
        }
        self.join_twin(&twin, shared, delta_zorder);
        Ok(twin)
    }

    /// Makes `twin`, an axes made on its own at this axes' place and
    /// sharing axis `shared` with it, its twin drawn `delta_zorder` above
    /// it, as [`twin_with`](Self::twin_with) sets up the axes it makes
    /// (matplotlib's `_make_twin_axes` for `twinx(axes_class=...)`, whose
    /// axes the class' constructor makes).
    ///
    /// ```
    /// use pltrs::axes::AxisName;
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.add_axes([0.1, 0.1, 0.8, 0.8])?;
    /// let other = fig.add_axes([0.1, 0.1, 0.8, 0.8])?;
    /// other.share_x(&ax)?;
    /// ax.adopt_twin(&other, AxisName::X, -1.0)?;
    /// assert_eq!(ax.twins(), [ax.clone(), other.clone()]);
    /// assert!(other.patch_visible() && !ax.patch_visible());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`AspectError::FixedDataRatio`] when either axes fixes its data
    /// ratio; then nothing changes.
    pub fn adopt_twin(
        &self,
        twin: &Self,
        shared: AxisName,
        delta_zorder: f64,
    ) -> Result<&Self, AspectError> {
        for axes in [self, twin] {
            axes.raw
                .with::<AxesState, _>(|state| check_adjustable(state, Adjustable::Datalim))?;
        }
        self.join_twin(twin, shared, delta_zorder);
        Ok(self)
    }

    /// A new axes on top of this one sharing axis `shared` (the
    /// `add_subplot(spec, sharex=self)` of matplotlib's
    /// `_make_twin_axes`), not yet set up as a twin.
    fn new_twin(&self, shared: AxisName) -> Self {
        let (frame, position, spec) = self.raw.with::<AxesState, _>(|axes| {
            (axes.base().parent, axes.position, axes.subplot_spec.clone())
        });
        let twin = make_axes(self.raw().figure_state(), frame, position, spec);
        share_free(&twin, shared, self);
        twin
    }

    /// Makes `twin`, sharing axis `shared` with this axes, its twin drawn
    /// `delta_zorder` above it, with its other axis on the far side, and
    /// with a grid of its own only under `pltrs.axes.twin_grid` (which the
    /// modern looks turn off: that grid would cross this axes' data).
    fn join_twin(&self, twin: &Self, shared: AxisName, delta_zorder: f64) {
        let own = match shared {
            AxisName::X => AxisName::Y,
            AxisName::Y => AxisName::X,
        };
        let (zorder, autoscale, units) = self.raw.with::<AxesState, _>(|axes| {
            (
                axes.base().zorder,
                axes.axis(shared).autoscale.enabled,
                axes.axis(shared).unit_data(),
            )
        });
        let figure = self.raw().figure_state();
        figure.borrow_mut().shared.join_twins(self.id(), twin.id());
        self.store_adjustable(Adjustable::Datalim);
        twin.store_adjustable(Adjustable::Datalim);
        twin.set_zorder(zorder + delta_zorder);
        twin.update_twinned_patch_visibility();
        twin.raw.with_mut::<AxesState, _>(|axes| {
            axes.axis_mut(shared).autoscale.enabled = autoscale;
            axes.axis_mut(shared).visible = false;
            axes.axis_mut(shared)
                .set_unit_data((units.0.for_twin(), units.1));
            let axis = axes.axis_mut(own);
            axis.tick_to(true);
            axis.label.move_to(SpineSide::of(own, true));
        });
        self.raw
            .with_mut::<AxesState, _>(|axes| axes.axis_mut(own).tick_to(false));
        if !crate::rc::params().pltrs_axes_twin_grid {
            let options = GridOptions {
                which: TickLevel::Both,
                ..GridOptions::default()
            };
            twin.grid_with(false, options);
        }
    }

    /// Shows the background patch of the bottom-most axes of a group of
    /// twinned axes only (the lowest z-order, the first added among
    /// equals), and only if it draws its frame (matplotlib's
    /// `_update_twinned_axes_patch_visibility`, which matplotlib's
    /// `set_zorder` of an axes runs).
    pub fn update_twinned_patch_visibility(&self) {
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
    use crate::figure::Figure;
    use crate::prelude::*;
    use crate::spines::SpineSide;
    use crate::units::UnitConverter;

    #[test]
    fn twins_take_over_the_units_of_the_shared_axis() {
        let ax = Figure::new().subplot();
        ax.plot(["0", "1"], ["a", "b"]).unwrap();
        let (twin_x, twin_y) = (ax.twinx().unwrap(), ax.twiny().unwrap());
        assert_eq!(twin_x.xaxis().converter(), Some(UnitConverter::Categories));
        assert_eq!(twin_x.xaxis().own_converter(), None, "matplotlib's twins");
        assert!(twin_x.xaxis().have_units(), "the shared category names");
        assert_eq!(twin_x.xaxis().convert_units(["1", "0"]), [1.0, 0.0]);
        assert_eq!(
            twin_x.xaxis().own_converter(),
            Some(UnitConverter::Categories),
            "converting on the twin sets its own converter"
        );
        assert_eq!(twin_y.yaxis().convert_units(["b", "a"]), [1.0, 0.0]);
        assert!(!twin_x.yaxis().have_units());
    }

    #[test]
    fn twinx_puts_its_y_axis_on_the_right() {
        let fig = Figure::new();
        let ax = fig.subplot();
        ax.set_zorder(3.0);
        ax.plot([0.0, 10.0], [0.0, 1.0]).unwrap();
        let twin = ax.twinx().unwrap();
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
        let twin = ax.twiny().unwrap();
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
    fn a_twin_below_draws_the_background() {
        // matplotlib's test_twinning_patch_visibility_respects_delta_zorder.
        let ax = Figure::new().subplot();
        let below = ax.twin_with(AxisName::X, -1.0).unwrap();
        assert_eq!(below.zorder(), ax.zorder() - 1.0);
        assert!(below.patch_visible() && !ax.patch_visible());
        assert_eq!(ax.twins().len(), 2);
    }

    #[test]
    fn twins_move_and_reshape_together() {
        // matplotlib's set_position and set_box_aspect act on every twin.
        let ax = Figure::new().subplot();
        let twin = ax.twinx().unwrap();
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
        )))
        .unwrap();
        let ax = fig.subplot();
        ax.set_position([0.2, 0.2, 0.5, 0.5]);
        let twin = ax.twinx().unwrap();
        fig.layout();
        assert_eq!(ax.position(), twin.position());
        assert!(ax.position().width() > 0.5, "the layout took over");
    }

    #[test]
    fn axes_fixing_their_data_ratio_have_no_twins_as_in_matplotlib() {
        use crate::axes::{AspectError, ProjectionKind};

        let fig = Figure::new();
        let geo = fig
            .subplot_with_projection(ProjectionKind::Lambert)
            .unwrap();
        let refused = Err(AspectError::FixedDataRatio(ProjectionKind::Lambert));
        assert_eq!(geo.twinx(), refused);
        assert_eq!(geo.twiny(), refused);
        let polar = fig.polar_subplot();
        assert_eq!(
            polar.twinx(),
            Err(AspectError::FixedDataRatio(ProjectionKind::Polar))
        );
        assert_eq!(
            fig.axes(),
            [geo.clone(), (*polar).clone()],
            "no twin is made"
        );
        assert_eq!(geo.twins(), [geo]);
    }

    #[test]
    fn twins_leave_the_grid_to_their_host_only_when_asked() {
        let grids = |twin_grid: &str| {
            crate::rc::context(|| {
                crate::rc::set("axes.grid", "True").unwrap();
                crate::rc::set("axes.grid.which", "both").unwrap();
                crate::rc::set("pltrs.axes.twin_grid", twin_grid).unwrap();
                let ax = Figure::new().subplot();
                let twin = ax.twinx().unwrap();
                assert!(ax.has_grid(AxisName::Y, true), "the host keeps its grid");
                [true, false].map(|major| twin.has_grid(AxisName::Y, major))
            })
        };
        assert_eq!(grids("True"), [true, true], "matplotlib's twins");
        assert_eq!(grids("False"), [false, false]);
    }
}
