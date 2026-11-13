//! The coordinate system of an axes (matplotlib's projections).

use super::{Axes, AxesState};
use crate::polar::PolarState;
use crate::projections::{AxesOutline, Projection, ProjectionView};
use crate::spines::SpineSide;

/// The kind of coordinate system an axes draws in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProjectionKind {
    /// Cartesian x and y.
    #[default]
    Rectilinear,
    /// Angle and radius (see [`crate::polar`]).
    Polar,
    /// Longitude and latitude through the Aitoff projection (see
    /// [`crate::geo`]).
    Aitoff,
    /// Longitude and latitude through the Hammer projection.
    Hammer,
    /// Longitude and latitude through the Lambert azimuthal equal-area
    /// projection.
    Lambert,
    /// Longitude and latitude through the Mollweide projection.
    Mollweide,
    /// A projection defined by the program and registered under this name
    /// (see [`crate::projections`]).
    Registered(&'static str),
}

impl ProjectionKind {
    /// The built-in kinds of 2-D axes.
    pub const BUILTIN: [Self; 6] = [
        Self::Rectilinear,
        Self::Polar,
        Self::Aitoff,
        Self::Hammer,
        Self::Lambert,
        Self::Mollweide,
    ];

    /// matplotlib's projection name (`ax.name`).
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rectilinear => "rectilinear",
            Self::Polar => "polar",
            Self::Aitoff => "aitoff",
            Self::Hammer => "hammer",
            Self::Lambert => "lambert",
            Self::Mollweide => "mollweide",
            Self::Registered(name) => name,
        }
    }

    /// The kind called `name`: a built-in one, or one registered on this
    /// thread (see [`crate::projections::register`]).
    ///
    /// ```
    /// use pltrs::axes::ProjectionKind;
    /// assert_eq!(ProjectionKind::from_name("polar"), Some(ProjectionKind::Polar));
    /// assert_eq!(ProjectionKind::from_name("unheard-of"), None);
    /// ```
    pub fn from_name(name: &str) -> Option<Self> {
        Self::BUILTIN
            .into_iter()
            .find(|kind| kind.name() == name)
            .or_else(|| crate::projections::registered_name(name).map(Self::Registered))
    }
}

/// The projection of an axes with its settings.
#[derive(Clone, Debug, Default)]
pub(crate) enum ProjectionState {
    /// Cartesian x and y.
    #[default]
    Rectilinear,
    /// Angle and radius.
    Polar(PolarState),
    /// Angle and radius through the transform of a projection defined by
    /// the program (matplotlib's subclass of `PolarAxes` with a
    /// `PolarTransform` of its own, such as the radar chart example's):
    /// see [`Axes::install_projection`].
    ProgramPolar(PolarState, Box<dyn Projection>),
    /// A projection drawn through the [`Projection`] hooks: geographic
    /// axes, or one defined by the program.
    Custom(Box<dyn Projection>),
}

pltrs_core::archive_enum!(ProjectionState {
    Rectilinear,
    Polar(v0),
    ProgramPolar(v0, v1),
    Custom(v0),
});

impl ProjectionState {
    /// The kind of projection.
    pub(crate) fn kind(&self) -> ProjectionKind {
        match self {
            Self::Rectilinear => ProjectionKind::Rectilinear,
            Self::Polar(_) | Self::ProgramPolar(..) => ProjectionKind::Polar,
            // Built-in names are not registered: such a projection is the
            // built-in one.
            Self::Custom(custom) => ProjectionKind::BUILTIN
                .into_iter()
                .find(|kind| kind.name() == custom.name())
                .unwrap_or(ProjectionKind::Registered(custom.name())),
        }
    }

    /// Whether this is matplotlib's `rectilinear` projection itself.
    pub(crate) fn is_rectilinear(&self) -> bool {
        matches!(self, Self::Rectilinear)
    }

    /// Whether the axes is the unit square framed by its four side spines;
    /// curved projections draw their own outline and clip to their patch.
    pub(crate) fn is_rectangular(&self) -> bool {
        match self {
            Self::Rectilinear => true,
            Self::Polar(_) | Self::ProgramPolar(..) => false,
            Self::Custom(custom) => custom.outline() == AxesOutline::Rectangle,
        }
    }

    /// Whether the axes has a spine on `side` (matplotlib's
    /// `ax.spines` naming it): a projection drawn through the
    /// [`Projection`] hooks says which sides its class made spines of
    /// ([`Projection::spines`]).
    pub(crate) fn has_spine(&self, side: SpineSide) -> bool {
        self.custom()
            .is_none_or(|custom| custom.spines().contains(&side))
    }

    /// The projection drawn through the [`Projection`] hooks, if it is one.
    pub(crate) fn custom(&self) -> Option<&dyn Projection> {
        match self {
            Self::Custom(custom) => Some(&**custom),
            _ => None,
        }
    }

    /// The polar settings, if the axes is polar.
    pub(crate) fn polar(&self) -> Option<&PolarState> {
        match self {
            Self::Polar(settings) | Self::ProgramPolar(settings, _) => Some(settings),
            _ => None,
        }
    }

    /// The polar settings to change, if the axes is polar.
    pub(crate) fn polar_mut(&mut self) -> Option<&mut PolarState> {
        match self {
            Self::Polar(settings) | Self::ProgramPolar(settings, _) => Some(settings),
            _ => None,
        }
    }

    /// The projection of the program polar axes are drawn through, if
    /// they are.
    pub(crate) fn polar_program(&self) -> Option<&dyn Projection> {
        match self {
            Self::ProgramPolar(_, program) => Some(&**program),
            _ => None,
        }
    }

    /// The data ratio the projection gives in `view` (matplotlib's
    /// overridden `get_data_ratio`): 1 for polar and geographic axes.
    pub(crate) fn fixed_data_ratio(&self, view: &ProjectionView<'_>) -> Option<f64> {
        match self {
            Self::Rectilinear => None,
            Self::Polar(_) => Some(1.0),
            Self::ProgramPolar(_, program) => program.data_ratio(view).or(Some(1.0)),
            Self::Custom(custom) => custom.data_ratio(view),
        }
    }

    /// Whether the projection gives a data ratio of its own (see
    /// [`Projection::fixes_data_ratio`]): polar and geographic axes do.
    pub(crate) fn fixes_data_ratio(&self) -> bool {
        match self {
            Self::Rectilinear => false,
            Self::Polar(_) | Self::ProgramPolar(..) => true,
            Self::Custom(custom) => custom.fixes_data_ratio(),
        }
    }

    /// Whether the view limits refuse to change once the projection is set
    /// up (geographic axes show the whole globe).
    pub(crate) fn fixed_limits(&self) -> bool {
        self.custom().is_some_and(Projection::fixed_limits)
    }

    /// Whether both axes must keep linear scales.
    pub(crate) fn linear_only(&self) -> bool {
        self.custom().is_some_and(Projection::linear_only)
    }
}

impl Axes {
    /// The kind of coordinate system this axes draws in.
    pub fn projection(&self) -> ProjectionKind {
        self.raw.with::<AxesState, _>(|axes| axes.projection.kind())
    }
}
