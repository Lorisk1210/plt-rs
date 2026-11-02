"""
The interactive side of a figure: the observers of its Axes (matplotlib's
``Figure.add_axobserver``), functions called with the figure whenever its
Axes change (one is added, removed or made current), which the toolbars
use to keep their views; the release of the mouse an Axes grabs when it is
removed; the current Axes (``Figure.gca``, ``sca``), which subfigures
share with their root figure; and drawing single artists onto the canvas
for blitting (``Figure.draw_artist``).
"""

from . import _handles, cbook
from ._native import figure as _figure


class InteractionMixin:
    """
    The Axes observers, the current Axes and ``draw_artist`` of `.Figure`
    and `.SubFigure`; a subfigure shares the observers and the current Axes
    of its root figure, as in matplotlib.
    """

    @property
    def _axobservers(self):
        """The `.CallbackRegistry` of the observers."""
        root = self.get_figure(root=True)
        registry = root.__dict__.get("_axobserver_registry")
        if registry is None:
            registry = cbook.CallbackRegistry()
            root.__dict__["_axobserver_registry"] = registry
        return registry

    def add_axobserver(self, func):
        """Whenever the Axes state change, ``func(self)`` will be called."""
        # Connect a wrapper lambda and not func itself, to avoid it being
        # weakref-collected.
        self._axobservers.connect("_axes_change_event", lambda arg: func(arg))

    def _axes_changed(self):
        """Call the observers: the Axes of the figure changed."""
        self._axobservers.process("_axes_change_event", self)

    def sca(self, a):
        """Set the current Axes to be *a* and return *a*."""
        _figure.sca(self.get_figure(root=True)._native, a._native)
        self._axes_changed()
        return a

    def gca(self):
        """
        Get the current Axes.

        If there is currently no Axes on this Figure, a new one is created
        using `.Figure.add_subplot`.  (To test whether there is currently an
        Axes on a Figure, check whether ``figure.axes`` is empty.  To test
        whether there is currently a Figure on the pyplot figure stack, check
        whether `.pyplot.get_fignums()` is empty.)

        A subfigure shares the current Axes of its root figure, as in
        matplotlib.
        """
        from .axes import Axes
        root = self.get_figure(root=True)
        ax = _handles.wrap(_figure.current_axes(root._native), Axes, root)
        return ax if ax is not None else self.add_subplot()

    def _axes_removed(self, ax):
        """
        Tell the observers that *ax* was removed and release the mouse if
        *ax* grabs it (matplotlib's ``_remove_axes``).
        """
        self._axes_changed()
        self.get_figure(root=True).canvas.release_mouse(ax)

    def _forget_axobservers(self):
        """Drop the observers (`.Figure.clear` without *keep_observers*)."""
        self.get_figure(root=True).__dict__.pop("_axobserver_registry", None)

    def draw_artist(self, a):
        """
        Draw `.Artist` *a* only.
        """
        a.draw(self.canvas.get_renderer())
