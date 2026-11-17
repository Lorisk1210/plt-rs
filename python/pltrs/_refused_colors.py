"""
The colours a collection refused, which matplotlib's sets again as it
draws.

matplotlib's collection keeps the face or edge colour it is given as the
colour set (``_original_facecolor``, ``_original_edgecolor``) before it
converts it, so a colour it refuses stays the one set. Its draws map its
values (``update_scalarmappable``) and set the colours of the sides the
mapping does not colour again from those: at the first draw (whose
mapping flags change from None), and at every draw while it maps an
array (a face colour mapped then, an edge colour unless it is ``'face'``
or None). A refused colour fails those draws as it failed to be set. A
usable colour of the same side replaces it. A draw of offsets, none of
them, returns before it maps anything, and counts as no draw.
"""

import functools

from . import (_lazy_imports, _python_drawing, _unusable, cbook, colorizer as mcolorizer,
               colors as mcolors)
from ._native import artist as _artist

_collection_draw_checks = _lazy_imports.module(globals(), "._collection_draw_checks")


def take(collection, side, c, give):
    """
    Give *collection* the colour *c* of *side* (``"facecolor"`` or
    ``"edgecolor"``) through *give*, which raises for a colour it refuses:
    then the collection keeps *c* (see `._unusable`) to fail the draws
    that set it again with, and the error propagates. Returns what *give*
    returns.
    """
    key = f"refused {side}"
    try:
        given = give()
    except (TypeError, ValueError):
        drawn = _flags_set(collection)
        if drawn:
            collection._mappable_flags_set = True
        if side == "edgecolor" or not drawn:
            _unusable.keep(collection, key, c, functools.partial(_set_again, collection, side),
                           measured=False)
        raise
    _unusable.drop(collection, key)
    return given


def _flags_set(collection):
    """Whether matplotlib's *collection* set its mapping flags: whether it
    was drawn."""
    return (collection.__dict__.get("_mappable_flags_set", False)
            or _artist.draw_count(collection._native) > 0)


def _set_again(collection, side, c):
    """Set the colour *c* of *side* again as matplotlib's draw of
    *collection* does, failing as it does: at its first draw unless the
    mapping colours that side, and at every draw mapping an array for
    edges (refused, they are neither ``'face'`` nor None, so they are not
    mapped). The colour stays kept (the draw that fails is recorded
    before it is made, see `._python_drawing`)."""
    if _collection_draw_checks._draws_nothing(collection):
        return
    first = not collection.__dict__.get("_mappable_flags_set", False)
    # matplotlib's flags are set where its figure's draw reaches the
    # collection, before it fails on the colour.
    _python_drawing.when_reached(collection, "_mappable_flags_set", True)
    mapping = mcolorizer._array_of(collection) is not None
    if side == "facecolor":
        if first and not mapping:
            mcolors.to_rgba_array(c)
    elif first or (mapping and not cbook._str_equal(
            collection.__dict__.get("_original_facecolor"), "none")):
        mcolors.to_rgba_array(c)
