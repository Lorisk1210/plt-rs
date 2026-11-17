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

The changes pltrs (Rust) makes (a setter, a plotting method) set no
``stale`` in Python: the figure of an artist given a callback of the
program's reports the artists they mark stale (`watch`), and at the end
of the native call each of them is set stale as matplotlib's setters set
it (`stale_reported`), which runs the chain.
"""

import weakref

from ._native import artist as _artist, register as _register

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
    figure's for a subfigure among its parent's or an artist added to a
    figure; none for a root figure, a subfigure being made or made alone,
    an artist in no figure, a part of another artist that only got its
    figure (a tick label, an axis label, a table cell, the Axes' patch,
    the offset box of an annotation box) or a figure's legend."""
    from . import _mirrors
    from .artist import Artist
    from .figure import Figure, FigureBase
    from .legend import Legend
    if isinstance(artist, Figure):
        return None
    if isinstance(artist, FigureBase):
        parent = artist.__dict__.get("_parent")
        listed = parent is not None and any(sub is artist for sub in parent.subfigs)
        return _stale_figure_callback if listed else None
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


#: The root figures reporting the artists their changes mark stale.
_watched = weakref.WeakSet()

#: Whether a figure was ever watched: until then nothing is reported, and
#: what reports need is skipped (`._mirrors.push`).
reporting = False


def set_by_program(artist):
    """Note that a program set a stale callback of its own on *artist*
    (see `_program_callbacks`), whose figure then reports its changes
    (`watch`)."""
    global _program_callbacks
    _program_callbacks = True
    watch(artist)


def watch(artist):
    """Have the root figure of *artist*, if it has one, report the artists
    its changes mark stale, which `stale_reported` sets stale."""
    global reporting
    figure = artist.get_figure(root=True)
    if figure is not None and figure not in _watched:
        reporting = True
        _watched.add(figure)
        _artist.report_stale(figure._native, True)


def stale_reported():
    """Set stale, as matplotlib's setters do, the artists pltrs (Rust)
    marked stale in the figures watched (`watch`) during the native call
    that ends, running their stale callbacks."""
    from ._handles import wrap_any
    for figure in list(_watched):
        root = _artist.id(figure._native)
        for native in _artist.take_stale_reports(figure._native):
            if _artist.id(native) == root:
                # A change to the figure's own state.
                figure.stale = True
                continue
            try:
                artist = wrap_any(native, figure)
            except AttributeError:
                # A part with no class of its own here (a tick).
                continue
            artist.stale = True


_register("stale_reported", stale_reported)


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
