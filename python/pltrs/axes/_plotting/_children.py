"""
The children of an Axes: its collections, patches and containers
(matplotlib's ``ax.collections``, ``ax.patches`` and ``ax.containers``).
Ready-made artists are added by `~.axes._base._AxesBase` (``add_line``,
``add_patch``, ``add_collection``, ``add_artist``).
"""

from ... import collections as mcoll, patches as mpatches
from ..._native import plotting as _plotting
from .._artist_list import ArtistList
from ._helpers import wrap_all


class ChildrenMixin:
    """``collections``, ``patches`` and ``containers``."""

    @property
    def collections(self):
        return ArtistList._of("collections", lambda: wrap_all(
            self, _plotting.get_collections(self._native), mcoll.Collection))

    @property
    def patches(self):
        return ArtistList._of("patches", lambda: wrap_all(
            self, _plotting.get_patches(self._native), mpatches.Patch))

    @property
    def containers(self):
        """The containers made by the plotting methods, in order."""
        return self.__dict__.setdefault("_containers", [])

    @containers.setter
    def containers(self, value):
        self.__dict__["_containers"] = value

    def add_container(self, container):
        """
        Add a `.Container` to the Axes' containers; return the container.
        """
        label = container.get_label()
        if not label:
            container.set_label('_container%d' % len(self.containers))
        self.containers.append(container)
        container._remove_method = self.containers.remove
        return container
