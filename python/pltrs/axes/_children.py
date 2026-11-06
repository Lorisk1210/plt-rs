"""
The children of an Axes as `~.axes._base._AxesBase` lists them
(matplotlib's ``ax.collections``, ``ax.images``, ``ax.lines``,
``ax.patches``, ``ax.tables``, ``ax.texts``, ``ax.containers``,
``get_lines``, ``get_images`` and ``get_legend``). Ready-made artists are
added by `~.axes._base._AxesBase` (``add_line``, ``add_patch``,
``add_collection``, ``add_artist``).
"""

from .. import _handles, cbook
from .. import collections as mcoll, image as mimage, patches as mpatches
from .. import table as mtable
from .._native import axes as _axes, image as _native_image, plotting as _plotting
from .._native import table as _table, text as _text
from ..legend import Legend
from ..lines import Line2D
from ..text import Text
from ._artist_list import ArtistList


class ChildrenMixin:
    """The lists of the children of `~.axes.Axes`, and its containers."""

    @property
    def collections(self):
        return ArtistList._of("collections", lambda: _handles.wrap_all(
            _plotting.get_collections(self._native), mcoll.Collection, self.figure))

    @property
    def images(self):
        return ArtistList._of("images", lambda: _handles.wrap_all(
            _native_image.get_images(self._native), mimage.AxesImage,
            self.figure))

    @property
    def lines(self):
        return ArtistList._of("lines", lambda: _handles.wrap_all(
            _axes.get_lines(self._native), Line2D, self.figure))

    @property
    def patches(self):
        return ArtistList._of("patches", lambda: _handles.wrap_all(
            _plotting.get_patches(self._native), mpatches.Patch, self.figure))

    @property
    def tables(self):
        return ArtistList._of("tables", lambda: _handles.wrap_all(
            _table.axes_tables(self._native), mtable.Table, self.figure))

    @property
    def texts(self):
        return ArtistList._of("texts", lambda: _handles.wrap_all(
            _text.axes_texts(self._native), Text, self.figure))

    def get_legend(self):
        """Return the `.Legend` instance, or None if no legend is defined."""
        from .._native import legend as _legend
        return _handles.wrap(_legend.axes_legend(self._native), Legend, self.figure)

    @property
    def legend_(self):
        """The `.Legend` of the Axes, or None."""
        return self.get_legend()

    @legend_.setter
    def legend_(self, legend):
        # matplotlib's attribute: None takes the legend off the Axes, a
        # legend made for the Axes becomes its legend again.
        current = self.get_legend()
        if legend is current:
            return
        if legend is None:
            current.remove()
            return
        from .._native import legend as _legend
        if not _legend.set_axes_legend(self._native, legend._native):
            raise ValueError("legend_ must be a Legend made for this Axes")

    def get_images(self):
        r"""Return a list of `.AxesImage`\s contained by the Axes."""
        return cbook.silent_list('AxesImage', self.images)

    def get_lines(self):
        """Return a list of lines contained by the Axes."""
        return cbook.silent_list('Line2D', self.lines)

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
