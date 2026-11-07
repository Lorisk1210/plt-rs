"""
The paths of the program's that a collection draws.

matplotlib's collections keep the `.Path` objects they are given and read
their vertices and codes whenever they use them, so that a program
changing the arrays of a path in place after making a collection of it
(``vertices[...] = ...`` on the array the path was made from, as the
marker collections of *From Python to Numpy* move their markers) changes
what the collection draws. pltrs (Rust) draws a collection from paths of
its own, copied when it was given them. So a collection given paths that
can change (any but read-only ones) keeps them with copies of their
arrays as it handed them over (`keep`), and hands them over again where
matplotlib reads them (`sync`) if one of them changed: when an Axes adds
the collection (its data limits), and at each draw of its figure
(`._computed_transforms`, whose draw hook runs before the figure's
artists are drawn). Collections of paths pltrs made (a scatter's markers)
keep nothing and cost nothing. A path whose vertices or codes are set
anew tells the artists it was handed to at once (`._owned_paths`).
"""

import numpy as np

from ._native import collections as _collections
from .path import Path

#: The attribute of a collection holding the paths of the program's it
#: was given and their arrays as it handed them to pltrs (Rust).
KEPT = "_program_paths"


def _arrays(paths):
    """Copies of the vertices and codes of each of *paths*."""
    return [(np.array(path.vertices),
             None if path.codes is None else np.array(path.codes))
            for path in paths]


def _unchanged(path, arrays):
    """Whether *path* has the vertices and codes *arrays*."""
    vertices, codes = arrays
    if not np.array_equal(path.vertices, vertices):
        return False
    if codes is None or path.codes is None:
        return codes is None and path.codes is None
    return np.array_equal(path.codes, codes)


def keep(collection, paths):
    """
    Have *collection*, just handed *paths* (a list of `.Path` objects, or
    None), keep them if any of them can change in place.
    """
    paths = [] if paths is None else list(paths)
    if not (all(isinstance(path, Path) for path in paths)
            and any(not path._readonly for path in paths)):
        collection.__dict__.pop(KEPT, None)
        return
    setattr(collection, KEPT, (paths, _arrays(paths)))
    figure = collection._figure
    if figure is not None:
        from ._computed_transforms import track
        track(collection, figure)


def kept(collection):
    """Whether *collection* keeps paths of the program's."""
    return KEPT in collection.__dict__


def sync(collection):
    """Hand pltrs (Rust) the paths *collection* keeps again if one of them
    changed since it handed them over."""
    paths, arrays = collection.__dict__.get(KEPT, ((), ()))
    if all(map(_unchanged, paths, arrays)):
        return
    _collections.set_paths(collection._native, paths)
    collection.__dict__.pop("_kept_paths", None)
    setattr(collection, KEPT, (paths, _arrays(paths)))
