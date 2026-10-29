"""
The children of an Axes: its collections, patches and containers
(matplotlib's ``ax.collections``, ``ax.patches`` and ``ax.containers``),
and the adding of artists made before (``add_line``, ``add_patch``,
``add_collection``, ``add_artist``).
"""

from ... import collections as mcoll, patches as mpatches
from ..._native import artist as _artist, held as _held, plotting as _plotting
from ._helpers import wrap_all


class ChildrenMixin:
    """``collections``, ``patches``, ``containers``, and adding artists."""

    @property
    def collections(self):
        """The collections of the Axes, in the order they were added."""
        return wrap_all(self, _plotting.get_collections(self._native),
                        mcoll.Collection)

    @property
    def patches(self):
        """The patches of the Axes, in the order they were added."""
        return wrap_all(self, _plotting.get_patches(self._native), mpatches.Patch)

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

    def _adopt(self, artist, how, autolim=True):
        """
        Move *artist*, made before (or removed), into the Axes as the adding
        method *how* names (``"patch"``, ``"line"``, ``"collection"`` or
        ``"artist"``), and make its object the one of the new handle.
        """
        native = _held.adopt(self._native, artist._native, how, bool(autolim))
        artist._native = native
        artist._figure = self.figure
        artist.axes = self
        self.figure._artists[_artist.id(native)] = artist
        return artist

    def add_line(self, line):
        """
        Add a `.Line2D` to the Axes' lines; return the line.
        """
        return self._adopt(line, "line")

    def add_patch(self, p):
        """
        Add a `.Patch` to the Axes' patches; return the patch.
        """
        return self._adopt(p, "patch")

    def add_collection(self, collection, autolim=True):
        """
        Add a `.Collection` to the Axes; return the collection.

        Parameters
        ----------
        collection : `.Collection`
            The collection to add.
        autolim : bool
            Whether to update the data limits.
        """
        return self._adopt(collection, "collection", autolim)

    def add_artist(self, a):
        """
        Add an `.Artist` to the Axes; return the artist.

        Use `add_artist` only for artists for which there is no dedicated
        "add" method; the data limits are not updated.
        """
        return self._adopt(a, "artist")
