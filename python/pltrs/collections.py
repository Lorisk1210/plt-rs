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
