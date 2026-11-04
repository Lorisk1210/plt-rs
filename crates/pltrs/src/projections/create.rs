//! Making axes with a projection chosen by kind (matplotlib's
//! `add_subplot(projection=...)`, `add_axes(rect, projection=...)` and
//! `inset_axes(bounds, projection=...)`), and handing fresh axes over to a
//! projection defined by the program.

use std::any::Any;

use super::{AxesOutline, Projection, ProjectionError, ProjectionSpec, SharedOptions, registry};
use crate::artist::{ArtistHandle, DrawError};
use crate::axes::projection::ProjectionState;
use crate::axes::{Axes, AxesState, ProjectionKind};
use crate::figure::Figure;
use crate::geo::{GeoAxes, GeoProjection};
use crate::gridspec::SubplotSpec;
use crate::polar::PolarAxes;
use crate::spines::SpineSide;

/// The spine drawing an outline that is not a rectangle (matplotlib's
/// `spines['geo']` and the like); the other three spines are not drawn.
pub(crate) const OUTLINE_SPINE: SpineSide = SpineSide::Top;

/// What turns fresh rectilinear axes into axes of one projection; made
/// before the axes, so that an unknown projection adds no axes.
pub(crate) enum Projector {
    /// Rectilinear axes stay as they are.
    Rectilinear,
    /// Polar axes.
    Polar,
    /// Geographic axes through a map projection.
    Geo(GeoProjection),
    /// A projection defined by the program, made with these options.
    Custom {
        projection: Box<dyn Projection>,
        options: Option<SharedOptions>,
    },
}

impl Projector {
    /// The projector of `spec`; a registered kind gets its projection from
    /// the registry, made with the options of `spec`.
    pub(crate) fn new(spec: &ProjectionSpec) -> Result<Self, ProjectionError> {
        spec.check()?;
        Ok(match spec.kind() {
            ProjectionKind::Rectilinear => Self::Rectilinear,
            ProjectionKind::Polar => Self::Polar,
            ProjectionKind::Aitoff => Self::Geo(GeoProjection::Aitoff),
            ProjectionKind::Hammer => Self::Geo(GeoProjection::Hammer),
            ProjectionKind::Lambert => Self::Geo(GeoProjection::LAMBERT),
            ProjectionKind::Mollweide => Self::Geo(GeoProjection::Mollweide),
            ProjectionKind::Registered(name) => Self::Custom {
                projection: registry::create(name, spec.options().map(|options| &**options))?,
                options: spec.options().cloned(),
            },
        })
    }

    /// Turns the freshly made rectilinear `axes` into axes of this
    /// projection; axes a projection fails to set up are removed.
    pub(crate) fn apply(self, axes: Axes) -> Result<Axes, ProjectionError> {
        match self {
            Self::Rectilinear => {}
            Self::Polar => {
                PolarAxes::convert(axes.clone());
            }
            Self::Geo(projection) => {
                GeoAxes::convert(axes.clone(), projection);
            }
            Self::Custom {
                projection,
                options,
            } => {
                if let Err(err) = install(&axes, projection) {
                    axes.remove();
                    return Err(err);
                }
                axes.raw()
                    .with_mut::<AxesState, _>(|state| state.projection_options = options);
            }
        }
        Ok(axes)
    }
}

/// Lets `projection` set up the fresh rectilinear or polar `axes`, then
/// hands the axes over to it (matplotlib's projection `__init__` calling
/// `clear`): polar axes map their data through it and stay polar.
pub(crate) fn install(
    axes: &Axes,
    mut projection: Box<dyn Projection>,
) -> Result<(), ProjectionError> {
    projection
        .set_up(axes)
        .map_err(|err| ProjectionError::SetUp {
            projection: projection.name(),
            message: err.to_string(),
        })?;
    let outlined = projection.outline() != AxesOutline::Rectangle;
    let polar = axes.raw().with_mut::<AxesState, _>(|state| {
        if let ProjectionState::Polar(settings) = std::mem::take(&mut state.projection) {
            state.projection = ProjectionState::ProgramPolar(settings, projection);
            true
        } else {
            state.projection = ProjectionState::Custom(projection);
            false
        }
    });
    if polar {
        return Ok(());
    }
    if outlined {
        // The spine drawing an outline ignores `axes.spines.*`, like
        // matplotlib's circular spine.
        axes.spines()
            .get(OUTLINE_SPINE)
            .raw()
            .with_base_mut(|base| base.visible = true);
    }
    Ok(())
}

/// Hands the just cleared `axes` back to `projection`, which sets it up
/// again (matplotlib's `clear` of a projection); a failure leaves the axes
/// rectilinear and is reported when the figure is drawn.
pub(crate) fn reinstall(axes: &Axes, projection: Box<dyn Projection>) {
    if let Err(err) = install(axes, projection) {
        axes.raw()
            .figure_state()
            .borrow_mut()
            .pending_errors
            .push(DrawError::new(axes.id(), err.to_string()));
    }
}

impl Figure {
    /// Adds axes with `projection` filling the default subplot area
    /// (matplotlib's `fig.add_subplot(projection=...)`).
    ///
    /// ```
    /// use pltrs::axes::ProjectionKind;
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_with_projection(ProjectionKind::Mollweide)?;
    /// assert_eq!(ax.projection().name(), "mollweide");
    /// assert!(fig.subplot_with_projection(ProjectionKind::Registered("nowhere")).is_err());
    /// assert_eq!(fig.axes().len(), 1);
    /// # Ok::<(), pltrs::projections::ProjectionError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`ProjectionError::Unknown`] for a projection not registered on this
    /// thread and [`ProjectionError::Options`] for options it does not take
    /// (no axes are added either way), [`ProjectionError::SetUp`] when it
    /// fails to set the axes up.
    pub fn subplot_with_projection(
        &self,
        projection: impl Into<ProjectionSpec>,
    ) -> Result<Axes, ProjectionError> {
        Projector::new(&projection.into())?.apply(self.subplot())
    }

    /// Adds axes with `projection` in cell `index` (1-based, row-major) of
    /// an `nrows × ncols` grid (matplotlib's `fig.add_subplot(nrows, ncols,
    /// index, projection=...)`).
    ///
    /// # Errors
    ///
    /// As [`Figure::subplot_with_projection`], and
    /// [`ProjectionError::Grid`] when the grid is empty or `index` is
    /// outside it.
    pub fn add_subplot_with_projection(
        &self,
        nrows: usize,
        ncols: usize,
        index: usize,
        projection: impl Into<ProjectionSpec>,
    ) -> Result<Axes, ProjectionError> {
        let projector = Projector::new(&projection.into())?;
        projector.apply(self.add_subplot(nrows, ncols, index)?)
    }

    /// Adds axes with `projection` covering a grid cell or block
    /// (matplotlib's `fig.add_subplot(gs[...], projection=...)`).
    ///
    /// # Errors
    ///
    /// As [`Figure::subplot_with_projection`], and
    /// [`ProjectionError::Grid`] when the subplot parameters leave no room
    /// for the cell.
    pub fn add_subplot_spec_with_projection(
        &self,
        spec: &SubplotSpec,
        projection: impl Into<ProjectionSpec>,
    ) -> Result<Axes, ProjectionError> {
        let projector = Projector::new(&projection.into())?;
        projector.apply(self.add_subplot_spec(spec)?)
    }

    /// Adds axes with `projection` occupying `[left, bottom, width,
    /// height]` in figure fractions (matplotlib's `fig.add_axes(rect,
    /// projection=...)`).
    ///
    /// # Errors
    ///
    /// As [`Figure::subplot_with_projection`], and
    /// [`ProjectionError::Rect`] for a rectangle [`Figure::add_axes`]
    /// refuses.
    pub fn add_axes_with_projection(
        &self,
        rect: [f64; 4],
        projection: impl Into<ProjectionSpec>,
    ) -> Result<Axes, ProjectionError> {
        let projector = Projector::new(&projection.into())?;
        projector.apply(self.add_axes(rect)?)
    }

    /// Axes with `projection` occupying `[left, bottom, width, height]` in
    /// figure fractions, made outside the figure until it adds them
    /// (matplotlib's `PolarAxes(fig, rect)`; see [`Figure::removed_axes`]).
    ///
    /// ```
    /// use pltrs::axes::ProjectionKind;
    /// use pltrs::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.removed_axes_with_projection([0.1, 0.1, 0.8, 0.8], ProjectionKind::Polar)?;
    /// assert_eq!(ax.projection(), ProjectionKind::Polar);
    /// assert!(fig.axes().is_empty());
    /// # Ok::<(), pltrs::projections::ProjectionError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Figure::add_axes_with_projection`].
    pub fn removed_axes_with_projection(
        &self,
        rect: [f64; 4],
        projection: impl Into<ProjectionSpec>,
    ) -> Result<Axes, ProjectionError> {
        let projector = Projector::new(&projection.into())?;
        projector.apply(self.removed_axes(rect)?)
    }

    /// Axes with `projection` covering a grid cell or block, made outside
    /// the figure until it adds them (matplotlib's `PolarAxes(fig,
    /// subplotspec)`; see [`Figure::removed_subplot`]).
    ///
    /// # Errors
    ///
    /// As [`Figure::add_subplot_spec_with_projection`].
    pub fn removed_subplot_with_projection(
        &self,
        spec: &SubplotSpec,
        projection: impl Into<ProjectionSpec>,
    ) -> Result<Axes, ProjectionError> {
        let projector = Projector::new(&projection.into())?;
        projector.apply(self.removed_subplot(spec)?)
    }
}

impl Axes {
    /// Adds inset axes with `projection` at `[left, bottom, width,
    /// height]` in this axes' fractions (matplotlib's `inset_axes(bounds,
    /// projection=..., **options)`; see [`Axes::inset_axes`]).
    ///
    /// ```
    /// use pltrs::artist::ArtistHandle;
    /// use pltrs::axes::ProjectionKind;
    /// use pltrs::figure::Figure;
    /// let host = Figure::new().subplot();
    /// let inset = host.inset_axes_with_projection([0.6, 0.6, 0.35, 0.35], ProjectionKind::Polar)?;
    /// assert_eq!((inset.projection(), inset.zorder()), (ProjectionKind::Polar, 5.0));
    /// assert_eq!(host.child_axes(), [inset]);
    /// # Ok::<(), pltrs::projections::ProjectionError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// As [`Figure::subplot_with_projection`].
    pub fn inset_axes_with_projection(
        &self,
        bounds: [f64; 4],
        projection: impl Into<ProjectionSpec>,
    ) -> Result<Self, ProjectionError> {
        let projector = Projector::new(&projection.into())?;
        projector.apply(self.inset_axes(bounds))
    }

    /// Hands this rectilinear axes over to `projection`, a projection made
    /// for this axes alone, which sets it up first (matplotlib's
    /// `__init__` of a projection's class; a binding gives axes of a
    /// program's projection class a projection asking the program).
    ///
    /// Polar axes take a projection too (matplotlib's subclasses of
    /// `PolarAxes`, such as the radar chart example's): they stay polar,
    /// with their settings, ticks, labels and spines, but map their data
    /// through [`Projection::data_to_axes`] (matplotlib's `transData`
    /// without its final `transAxes`, made with the subclass'
    /// `PolarTransform`), and take their aspect ratio from
    /// [`Projection::data_ratio`] when it gives one; its other hooks are
    /// those of rectilinear axes, which polar axes do not ask. A clear
    /// sets the axes up again through it.
    ///
    /// ```
    /// use pltrs::figure::Figure;
    /// use pltrs::projections::{Projection, ProjectionView};
    /// use pltrs::transforms::ResolvedTransform;
    ///
    /// #[derive(Debug)]
    /// struct Plain;
    ///
    /// impl Projection for Plain {
    ///     fn name(&self) -> &'static str {
    ///         "plain"
    ///     }
    ///     fn data_to_axes(&self, view: &ProjectionView<'_>) -> ResolvedTransform {
    ///         view.rectilinear()
    ///     }
    /// }
    ///
    /// let ax = Figure::new().subplot();
    /// ax.install_projection(Plain)?;
    /// assert_eq!(ax.projection().name(), "plain");
    /// assert!(ax.install_projection(Plain).is_err(), "it has one");
    /// # Ok::<(), pltrs::projections::ProjectionError>(())
    /// ```
    ///
    /// # Errors
    ///
    /// [`ProjectionError::NotRectilinear`] when the axes already is drawn
    /// through a projection other than the polar one, or through one of
    /// the program's, [`ProjectionError::SetUp`] when `projection` fails
    /// to set it up; the axes stays as it was either way.
    pub fn install_projection(
        &self,
        projection: impl Projection,
    ) -> Result<&Self, ProjectionError> {
        let taken = self.raw().with::<AxesState, _>(|state| {
            matches!(
                state.projection,
                ProjectionState::Custom(_) | ProjectionState::ProgramPolar(..)
            )
        });
        let current = self.projection();
        if taken || !matches!(current, ProjectionKind::Rectilinear | ProjectionKind::Polar) {
            return Err(ProjectionError::NotRectilinear(current.name()));
        }
        install(self, Box::new(projection))?;
        Ok(self)
    }

    /// The projection this axes was made with, with its options
    /// (matplotlib's `_projection_init`).
    pub(crate) fn made_with(&self) -> ProjectionSpec {
        self.raw().with::<AxesState, _>(|state| {
            ProjectionSpec::new(state.projection.kind(), state.projection_options.clone())
        })
    }

    /// Reads the projection of this axes as `P` with `read`; `None` when
    /// the axes is not drawn through a `P`. `read` gets a copy, so the
    /// figure is not borrowed while it runs.
    pub fn with_projection<P: Projection + Clone, R>(
        &self,
        read: impl FnOnce(&P) -> R,
    ) -> Option<R> {
        self.projection_copy()
            .map(|projection: P| read(&projection))
    }

    /// Changes the projection of this axes as `P` with `change`, and draws
    /// the figure again with it; `None` when the axes is not drawn through
    /// a `P`. `change` works on a copy, which replaces the projection once
    /// it returns, so the figure is not borrowed while `change` runs.
    pub fn with_projection_mut<P: Projection + Clone, R>(
        &self,
        change: impl FnOnce(&mut P) -> R,
    ) -> Option<R> {
        let mut projection: P = self.projection_copy()?;
        let result = change(&mut projection);
        self.with_custom_projection_mut(|stored| {
            let stored: &mut dyn Any = stored;
            if let Some(stored) = stored.downcast_mut::<P>() {
                *stored = projection;
            }
        });
        Some(result)
    }

    /// A copy of the projection of this axes, if it is a `P`.
    fn projection_copy<P: Projection + Clone>(&self) -> Option<P> {
        self.raw().with::<AxesState, _>(|state| {
            let projection: &dyn Any = state.projection.custom()?;
            projection.downcast_ref::<P>().cloned()
        })
    }

    /// Changes the projection defined by the program of this axes with
    /// `change`, if it has one, and draws the figure again. `change` must
    /// not use the axes or its figure.
    pub(crate) fn with_custom_projection_mut<R>(
        &self,
        change: impl FnOnce(&mut dyn Projection) -> R,
    ) -> Option<R> {
        let custom = self
            .raw()
            .with::<AxesState, _>(|state| state.projection.custom().is_some());
        if !custom {
            return None;
        }
        self.raw()
            .with_mut::<AxesState, _>(|state| match &mut state.projection {
                ProjectionState::Custom(projection) => Some(change(&mut **projection)),
                _ => None,
            })
    }

    /// Takes the projection defined by the program out of this axes,
    /// leaving it rectilinear (or polar), if it has one.
    pub(crate) fn take_custom_projection(&self) -> Option<Box<dyn Projection>> {
        self.raw()
            .with_mut::<AxesState, _>(|state| match std::mem::take(&mut state.projection) {
                ProjectionState::Custom(projection) => Some(projection),
                ProjectionState::ProgramPolar(settings, projection) => {
                    state.projection = ProjectionState::Polar(settings);
                    Some(projection)
                }
                other => {
                    state.projection = other;
                    None
                }
            })
    }
}
