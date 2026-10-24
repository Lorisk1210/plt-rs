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

/// Lets `projection` set up the fresh rectilinear `axes`, then hands the
/// axes over to it (matplotlib's projection `__init__` calling `clear`).
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
    axes.raw().with_mut::<AxesState, _>(|state| {
        state.projection = ProjectionState::Custom(projection);
    });
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
            .push(DrawError {
                artist: axes.id(),
                message: err.to_string(),
            });
    }
}

impl Figure {
    /// Adds axes with `projection` filling the default subplot area
    /// (matplotlib's `fig.add_subplot(projection=...)`).
    ///
    /// ```
    /// use plt::axes::ProjectionKind;
    /// use plt::figure::Figure;
    /// let fig = Figure::new();
    /// let ax = fig.subplot_with_projection(ProjectionKind::Mollweide)?;
    /// assert_eq!(ax.projection().name(), "mollweide");
    /// assert!(fig.subplot_with_projection(ProjectionKind::Registered("nowhere")).is_err());
    /// assert_eq!(fig.axes().len(), 1);
    /// # Ok::<(), plt::projections::ProjectionError>(())
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
}

impl Axes {
    /// Adds inset axes with `projection` at `[left, bottom, width,
    /// height]` in this axes' fractions (matplotlib's `inset_axes(bounds,
    /// projection=..., **options)`; see [`Axes::inset_axes`]).
    ///
    /// ```
    /// use plt::artist::ArtistHandle;
    /// use plt::axes::ProjectionKind;
    /// use plt::figure::Figure;
    /// let host = Figure::new().subplot();
    /// let inset = host.inset_axes_with_projection([0.6, 0.6, 0.35, 0.35], ProjectionKind::Polar)?;
    /// assert_eq!((inset.projection(), inset.zorder()), (ProjectionKind::Polar, 5.0));
    /// assert_eq!(host.child_axes(), [inset]);
    /// # Ok::<(), plt::projections::ProjectionError>(())
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

    /// The projection this axes was made with, with its options
    /// (matplotlib's `_projection_init`).
    pub(crate) fn made_with(&self) -> ProjectionSpec {
        self.raw().with::<AxesState, _>(|state| {
            ProjectionSpec::new(state.projection.kind(), state.projection_options.clone())
        })
    }

    /// Reads the projection of this axes as `P` with `read`; `None` when
    /// the axes is not drawn through a `P`. `read` must not use the axes
    /// or its figure.
    pub fn with_projection<P: Projection, R>(&self, read: impl FnOnce(&P) -> R) -> Option<R> {
        self.raw().with::<AxesState, _>(|state| {
            let projection: &dyn Any = state.projection.custom()?;
            projection.downcast_ref::<P>().map(read)
        })
    }

    /// Changes the projection of this axes as `P` with `change`, and draws
    /// the figure again with it; `None` when the axes is not drawn through
    /// a `P`. `change` must not use the axes or its figure.
    pub fn with_projection_mut<P: Projection, R>(
        &self,
        change: impl FnOnce(&mut P) -> R,
    ) -> Option<R> {
        self.with_custom_projection_mut(|projection| {
            let projection: &mut dyn Any = projection;
            projection.downcast_mut::<P>().map(change)
        })
        .flatten()
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
    /// leaving it rectilinear, if it has one.
    pub(crate) fn take_custom_projection(&self) -> Option<Box<dyn Projection>> {
        self.raw()
            .with_mut::<AxesState, _>(|state| match std::mem::take(&mut state.projection) {
                ProjectionState::Custom(projection) => Some(projection),
                other => {
                    state.projection = other;
                    None
                }
            })
    }
}
