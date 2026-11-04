"""
The callbacks run when an artist goes stale (matplotlib's
``Artist.stale_callback``).

pltrs (Rust) marks the containers of a stale artist stale itself, so the
callbacks matplotlib propagates staleness with need not run. They read as
matplotlib's (`_stale_axes_callback` for an artist in an Axes,
`_stale_figure_callback` for one in a figure), and once a program sets a
callback of its own on any artist, setting ``artist.stale = True`` runs the
chain of callbacks as matplotlib does: the artist's, which sets its Axes'
or its figure's ``stale``, which runs theirs, up to the program's.
"""

#: A callback nobody set: the artist's reads as matplotlib's default.
_DEFAULT = object()

#: Whether a program ever set a stale callback: until then, pltrs (Rust)'s
#: own propagation is all that setting ``stale`` does.
_program_callbacks = False


def _stale_axes_callback(self, val):
    """Mark the Axes of *self* stale (matplotlib's default callback of an
    artist in an Axes)."""
    if self.axes:
        self.axes.stale = val


def _stale_figure_callback(self, val):
    """Mark the (sub)figure of *self* stale (matplotlib's default callback
    of an Axes or of an artist of a figure)."""
    if (fig := self.get_figure(root=False)) is not None:
        fig.stale = val


def _default_callback(artist):
    """The callback matplotlib gives *artist* when it joins its Axes or
    figure: the Axes' for an artist given an Axes (a tick too), the
    figure's for a subfigure or an artist added to a figure; none for a
    root figure, an artist in no figure, a part of another artist that
    only got its figure (a tick label, an axis label, a table cell, the
    Axes' patch, the offset box of an annotation box) or a figure's
    legend."""
    from . import _mirrors
    from .artist import Artist
    from .figure import Figure, FigureBase
    from .legend import Legend
    if isinstance(artist, Figure):
        return None
    if isinstance(artist, FigureBase):
        return _stale_figure_callback
    axes = artist.axes
    if isinstance(axes, Artist) and axes is not artist:
        return _stale_axes_callback
    drawer = getattr(artist, "_drawer", None)
    if (artist.get_figure(root=True) is None or _mirrors.is_attached(artist)
            or isinstance(artist, Legend) or (drawer is not None and drawer() is not None)):
        return None
    return _stale_figure_callback


class StalenessMixin:
    """`stale_callback` of `.Artist`, and what setting ``stale`` runs."""

    @property
    def stale_callback(self):
        """
        The function called with the artist and True when the artist goes
        stale (matplotlib's default marks its container stale), or None.
        """
        callback = self.__dict__.get("_stale_callback", _DEFAULT)
        return _default_callback(self) if callback is _DEFAULT else callback

    @stale_callback.setter
    def stale_callback(self, callback):
        global _program_callbacks
        self._stale_callback = callback
        _program_callbacks = True

    def _went_stale(self, val):
        """Run the stale callback as matplotlib's ``stale`` setter does,
        once a program set one (an animated artist runs none)."""
        if _program_callbacks and val and not self.get_animated():
            callback = self.stale_callback
            if callback is not None:
                callback(self, val)
