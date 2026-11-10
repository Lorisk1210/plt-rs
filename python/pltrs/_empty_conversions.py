"""
The empty paths and offsets of collections, which matplotlib converts to
the units of their Axes whenever it draws them.

matplotlib's ``Collection._prepare_points`` converts the vertices of each
path and the offsets of a collection whose Axes have units. Numbers pass
through untouched (`.units._is_natively_supported`), but an empty array
holds no number to tell, so the axis' converter converts it, and a
converter refusing it fails the draw: the category converter an axis
took from a failed conversion, which has no categories to convert with.
(The draw of a collection of offsets, none of them, returns before.)
pltrs (Rust) draws collections with their numbers as they are, so once
an axis of an Axes takes a converter, the collections with an empty path
keep that conversion (see `._unusable`), which fails where matplotlib's
draw does.
"""

import numpy as np

from . import _unusable
from ._collection_draw_checks import _draws_nothing
from .path import Path

#: The name the conversion is kept under.
_KEY = "empty arrays in units"


def refresh(axes):
    """Have the collections of *axes* (an Axes of no projection's own,
    whose axis took a converter) that hold an empty path keep their
    conversion."""
    for collection in getattr(axes, "collections", ()):
        if _empty_arrays(collection):
            _unusable.keep(collection, _KEY, collection, _convert, measured=False)


def _empty_arrays(collection):
    """The empty coordinate arrays of *collection* that matplotlib's draw
    converts: the x and y of each empty path (paths that are none are left
    to the checks of what the collection keeps), unless the draw returns
    first. Its offsets, converted too, are not empty then."""
    if _draws_nothing(collection):
        return []
    paths = collection.get_paths() or ()
    arrays = [np.asarray(path.vertices).reshape(-1, 2).T
              for path in paths if isinstance(path, Path)]
    return [array for array in arrays if array.shape[1] == 0]


def _convert(collection):
    """Convert the empty arrays of *collection* as matplotlib's draw does,
    failing as its converters fail."""
    if collection.have_units():
        for xs, ys in _empty_arrays(collection):
            collection.convert_xunits(xs)
            collection.convert_yunits(ys)
