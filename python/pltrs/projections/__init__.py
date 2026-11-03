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
        # Registered projections whose class is imported on first use:
        # name -> function returning the class.
        self._deferred = {}

    def register(self, *projections):
        """Register a new set of projections."""
        for projection in projections:
            name = projection.name
            self._all_projection_types[name] = projection
            self._deferred.pop(name, None)

    def _register_deferred(self, name, load):
        """Register the projection *name*, whose class *load* imports and
        returns when it is first asked for."""
        self._deferred[name] = load

    def get_projection_class(self, name, _error_cls=KeyError):
        """Get a projection class from its *name*."""
        if name in self._deferred and name not in self._all_projection_types:
            self.register(self._deferred[name]())
        return _api.getitem_checked(self._all_projection_types, _error_cls=_error_cls,
                                    projection=name)

    def get_projection_names(self):
        """Return the names of all projections currently registered."""
        return sorted({*self._all_projection_types, *self._deferred})


projection_registry = ProjectionRegistry()
projection_registry.register(
    axes.Axes,
    PolarAxes,
    AitoffAxes,
    HammerAxes,
    LambertAxes,
    MollweideAxes,
)


# The 3-D axes, whose toolkit imports the axes defined above: registered
# now, imported on first use.
def _axes3d():
    """`~mpl_toolkits.mplot3d.axes3d.Axes3D`, imported when a 3D projection
    is first asked for (matplotlib imports it with this module)."""
    from ..mpl_toolkits.mplot3d import Axes3D
    return Axes3D


projection_registry._register_deferred("3d", _axes3d)


def register_projection(cls):
    """Register the projection class *cls* under its ``name``."""
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
