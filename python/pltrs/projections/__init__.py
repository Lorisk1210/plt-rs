"""
Non-separable transforms that map from data space to screen space
(matplotlib's ``projections`` package): the registry of the Axes classes
``add_subplot(projection=...)`` makes.

The registry is matplotlib's (from matplotlib 3.11.2, under the Matplotlib
License). The built-in projections (polar and the geographic ones) are
pltrs (Rust)'s: their axes are pltrs (Rust) axes made with that projection.
"""

from .. import _api, axes, _docstring
from .geo import AitoffAxes, HammerAxes, LambertAxes, MollweideAxes
from .polar import PolarAxes


class ProjectionRegistry:
    """A mapping of registered projection names to projection classes."""

    def __init__(self):
        self._all_projection_types = {}

    def register(self, *projections):
        """Register a new set of projections."""
        for projection in projections:
            name = projection.name
            self._all_projection_types[name] = projection

    def get_projection_class(self, name, _error_cls=KeyError):
        """Get a projection class from its *name*."""
        return _api.getitem_checked(self._all_projection_types, _error_cls=_error_cls,
                                    projection=name)

    def get_projection_names(self):
        """Return the names of all projections currently registered."""
        return sorted(self._all_projection_types)


projection_registry = ProjectionRegistry()
projection_registry.register(
    axes.Axes,
    PolarAxes,
    AitoffAxes,
    HammerAxes,
    LambertAxes,
    MollweideAxes,
)


# The 3-D axes, whose toolkit imports the axes defined above.
from ..mpl_toolkits.mplot3d import Axes3D  # noqa: E402
projection_registry.register(Axes3D)
del Axes3D


def register_projection(cls):
    projection_registry.register(cls)


def get_projection_class(projection=None):
    """
    Get a projection class from its name.

    If *projection* is None, a standard rectilinear projection is returned.
    """
    if projection is None:
        projection = 'rectilinear'
    return projection_registry.get_projection_class(projection, _error_cls=ValueError)


get_projection_names = projection_registry.get_projection_names
_docstring.interpd.register(projection_names=get_projection_names())


# What matplotlib's module also has from its imports, which programs
# import from it.
from ..mpl_toolkits.mplot3d.axes3d import Axes3D  # noqa: E402, F401
