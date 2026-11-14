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

from ._native import artist as _artist

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
    if (isinstance(axes, Artist) and axes is not artist) or _given_no_axes(artist):
        return _stale_axes_callback
    if (axes is artist and "_decoupled" not in artist.__dict__
            and not _artist.is_attached(artist._native)):
        # An Axes made for its figure and not added to it: matplotlib's
        # figure gives the callback when it adds the Axes (a removed one
        # keeps it).
        return None
    drawer = getattr(artist, "_drawer", None)
    if (artist.get_figure(root=True) is None or _mirrors.is_attached(artist)
            or isinstance(artist, Legend) or (drawer is not None and drawer() is not None)):
        return None
    return _stale_figure_callback


def _given_no_axes(artist):
    """Whether *artist* was given as its Axes something that is no artist,
    which matplotlib's stale callback fails to mark stale."""
    axes = artist.__dict__.get("_axes")
    # Every artist, and only an artist, has this module's mixin (tested
    # rather than `.Artist` to spare an import statement per call).
    return axes is not None and not isinstance(axes, StalenessMixin)


def default_stale_callback(artist):
    """
    The ``stale_callback`` of *artist* where no one set it: the function
    called with the artist and True when the artist goes stale
    (matplotlib's default marks its container stale), or None. matplotlib
    keeps the attribute in the instance, set when the artist joins its
    container; it reads here through `.Artist`'s ``__getattr__``, which a
    class of the program's ``super().stale_callback`` does not reach, as
    matplotlib's class has no such attribute.
    """
    return _default_callback(artist)


def set_by_program():
    """Note that a program set a stale callback of its own (see
    `_program_callbacks`)."""
    global _program_callbacks
    _program_callbacks = True


class StalenessMixin:
    """What setting ``stale`` runs (see `default_stale_callback` for
    ``stale_callback``)."""

    def _went_stale(self, val):
        """Run the stale callback as matplotlib's ``stale`` setter does,
        once a program set one, or for an artist given an Axes that is no
        artist (an animated artist runs none)."""
        if ((_program_callbacks or _given_no_axes(self)) and val
                and not self.get_animated()):
            callback = self.stale_callback
            if callback is not None:
                callback(self, val)
