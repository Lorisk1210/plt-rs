"""
Legend handlers that run in Python, as matplotlib runs every handler
(``handler.legend_artist(legend, orig_handle, fontsize, handlebox)``).

pltrs (Rust) has the handlers of the package's classes
(``pltrs::legend``), which make the artists of an entry without calling
Python. A handler runs in Python when pltrs (Rust)'s would not draw as it
does: a handler of a class the program defines (or one overriding a
method of a package class), one configured beyond what pltrs (Rust)'s
handlers take (parameters other than its class' defaults, an update
function of the program's), and the handler of a handle pltrs (Rust)
cannot show (an object that is no artist), or of a tuple holding such a
handle. It makes the artists of its entry in a handle box
(`.DrawingArea`) as matplotlib's legend makes it, which pltrs (Rust) then
draws in the legend (the ``("made", artists)`` entries of
``_native.legend.new``).
"""

import functools

import numpy as np

from . import _handles, legend_handler as mhandlers
from ._native import legend as _legend
from .container import Container
from .offsetbox import DrawingArea

#: The parameters of the package's handlers that pltrs (Rust)'s handlers
#: take (`HandlerTuple`'s layout).
_NATIVE_PARAMETERS = {mhandlers.HandlerTuple: frozenset({"_ndivide", "_pad"})}


@functools.cache
def _default_state(cls):
    """The attributes of a handler of the package's class *cls* made with
    its default parameters."""
    return vars(cls())


def _draws_natively(handler):
    """Whether pltrs (Rust)'s handler of the class of *handler*, a handler
    of the package's class, draws as *handler* does: made with the class'
    default parameters (but those pltrs (Rust) takes), its update function
    none or the package's."""
    cls = type(handler)
    if cls.__module__ != mhandlers.__name__:
        return False
    state = dict(vars(handler))
    if state.get("_update_prop_func") is mhandlers.update_from_first_child:
        state["_update_prop_func"] = None
    default = _default_state(cls)
    native = _NATIVE_PARAMETERS.get(cls, frozenset())
    return state.keys() == default.keys() and all(
        np.array_equal(np.asarray(value, dtype=object),
                       np.asarray(default[key], dtype=object))
        for key, value in state.items() if key not in native)


def _has_native(handle):
    """Whether pltrs (Rust) can show *handle*: an artist, or a container."""
    return (isinstance(handle, Container)
            or getattr(handle, "_legend_entry", None) is not None
            or getattr(handle, "_native", None) is not None)


def runs_in_python(legend, handler_map, handler, handle):
    """Whether *handler* makes the artists of the entry of *handle* in
    Python (see the module documentation), *handler_map* giving the
    handlers of the handles of a tuple."""
    if isinstance(handle, tuple):
        return not _draws_natively(handler) or any(
            runs_in_python(legend, handler_map,
                           legend.get_legend_handler(handler_map, inner), inner)
            for inner in handle)
    return not (_has_native(handle) and _draws_natively(handler))


def _is_box_transform(transform, box):
    """Whether *transform* is the transform of the handle box *box*
    (``box.get_transform()``)."""
    return (getattr(transform, "_a", None) is box.dpi_transform
            and getattr(transform, "_b", None) is box.offset_transform)


def _follows(artist, box):
    """How *artist*, made in the handle box *box*, follows it: through its
    transform (``"box"``, also for an artist without a transform, which
    `.DrawingArea.add_artist` gives the box's), through its offsets
    (``"offsets"``, a collection whose offset transform is the box's), or
    not (``"own"``)."""
    if _is_box_transform(artist.__dict__.get("_given_transform"), box):
        return "box"
    if _is_box_transform(artist.__dict__.get("_offset_transform"), box):
        return "offsets"
    return "own" if artist.is_transform_set() else "box"


class MadeEntries:
    """The entries of a legend being made whose handlers run in Python:
    their handle boxes, their artists and the artist each handler returned
    (matplotlib's ``legend_handles``)."""

    def __init__(self):
        self._boxes = {}
        self.returned = {}

    def entry(self, legend, index, handler, handle):
        """Run *handler* for *handle*, the entry at *index* of *legend*, in
        a handle box of matplotlib's size; return the native entry."""
        fontsize = legend._fontsize
        descent = 0.35 * fontsize * (legend.handleheight - 0.7)  # heuristic.
        box = DrawingArea(width=legend.handlelength * fontsize,
                          height=fontsize * legend.handleheight - descent,
                          xdescent=0., ydescent=descent)
        self.returned[index] = handler.legend_artist(legend, handle, fontsize,
                                                     box)
        self._boxes[index] = box
        return ("made", [(artist._native, _follows(artist, box))
                         for artist in box._children])

    def adopt(self, legend):
        """Give the artists of the entries their handles in the legend made,
        now pltrs (Rust) draws them."""
        artists = _legend.entry_artists(legend._native)
        for index, box in self._boxes.items():
            for artist, native in zip(box._children, artists[index]):
                _handles.adopt(artist, native, legend.figure)
