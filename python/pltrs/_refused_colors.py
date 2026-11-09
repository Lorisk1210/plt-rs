"""
The colours a collection refused, which matplotlib's sets again as it
first draws.

matplotlib's collection keeps the face or edge colour it is given as the
colour set (``_original_facecolor``, ``_original_edgecolor``) before it
converts it, so a colour it refuses stays the one set. Its first draw
maps its values (``update_scalarmappable``, whose mapping flags then
change from None) and sets both colours again from those: a refused one
fails that draw as it failed to be set; later draws, whose flags no
longer change, do not set them again. A usable colour of the same side
replaces it.
"""

import functools

from . import _unusable, colors as mcolors
from ._native import artist as _artist


def take(collection, side, c, give):
    """
    Give *collection* the colour *c* of *side* (``"facecolor"`` or
    ``"edgecolor"``) through *give*, which raises for a colour it refuses:
    then the collection keeps *c* (see `._unusable`) to fail its first
    draw with, unless it was drawn before, and the error propagates.
    """
    key = f"refused {side}"
    try:
        give()
    except (TypeError, ValueError):
        if not _flags_set(collection):
            _unusable.keep(collection, key, c, functools.partial(_set_again, collection),
                           measured=False)
        raise
    _unusable.drop(collection, key)


def _flags_set(collection):
    """Whether matplotlib's *collection* set its mapping flags: whether it
    was drawn."""
    return (collection.__dict__.get("_mappable_flags_set", False)
            or _artist.draw_count(collection._native) > 0)


def _set_again(collection, c):
    """Set the colour *c* again as matplotlib's first draw of *collection*
    does (the draw checking it first: a collection keeping a colour is
    drawn through Python), failing as it does; nothing at later draws.
    The colour stays kept (the draw that fails is recorded before it is
    made, see `._python_drawing`)."""
    first = not collection.__dict__.get("_mappable_flags_set", False)
    collection._mappable_flags_set = True
    if first:
        mcolors.to_rgba_array(c)
