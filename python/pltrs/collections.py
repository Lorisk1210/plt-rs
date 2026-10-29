"""
Collections of artists drawn together (matplotlib's ``collections``).
"""

from ._native import collections as _collections
from .artist import Artist


class Collection(Artist):
    """Base class for Collections."""

    def get_offsets(self):
        """Return the offsets for the collection."""
        return _collections.get_offsets(self._native)


class PathCollection(Collection):
    """A collection of `~.path.Path`\\s, as created by `~.Axes.scatter`."""


class PolyCollection(Collection):
    """A collection of polygons (as `~.Axes.fill_between` and `~.Axes.hexbin`
    make)."""


class FillBetweenPolyCollection(PolyCollection):
    """The polygons `~.Axes.fill_between` and `~.Axes.fill_betweenx` make."""


class LineCollection(Collection):
    """A collection of lines (as `~.Axes.hlines`, `~.Axes.vlines` and
    `~.Axes.eventplot` make)."""


class EventCollection(LineCollection):
    """A collection of locations along a single axis at which an "event"
    occurred (as `~.Axes.eventplot` makes)."""


class PatchCollection(Collection):
    """A generic collection of patches."""


class QuadMesh(Collection):
    """A quadrilateral mesh (as `~.Axes.pcolormesh` makes)."""
