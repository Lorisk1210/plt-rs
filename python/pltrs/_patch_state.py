"""
The state of patches that matplotlib keeps as an object of the patch's
own, which programs change in place (see `._own_state`): the path the
patch draws (``_path``), of whose vertices a polygon's ``xy`` are. The
patches whose path matplotlib makes at each call (a rectangle's shared
unit square, a box's, an arrow patch's) keep none. A shadow keeps the
patch it shadows (``patch``), whose outline it reads where it draws.
"""

import numpy as np

from ._native import patches as _patches
from ._own_state import RECORDS, State, sync
from .path import Path


def _read_path(patch):
    # The outline drawn in place of the shape's, if a path handed out was
    # changed in place (`.Patch._take_path`: a pickle carries it), else the
    # shape's.
    changed = None if patch._makes_own_path else _patches.get_program_path(patch._native)
    path = Path(*(_patches.get_path(patch._native) if changed is None else changed))
    if patch._path_read_only:
        # matplotlib's shared unit path, which nothing may change.
        path._vertices.flags.writeable = False
        if path._codes is not None:
            path._codes.flags.writeable = False
        path._readonly = True
    path._steps_of = (
        lambda: _patches.get_interpolation_steps(patch._native),
        lambda steps: _patches.set_interpolation_steps(patch._native, steps))
    return path


def _path_key(path):
    """The vertices and codes of *path* (its interpolation steps are pltrs
    (Rust)'s already)."""
    vertices, codes = path._vertices, path._codes
    return (np.shape(vertices), np.asarray(vertices).tobytes(),
            None if codes is None else np.asarray(codes).tobytes())


#: The path drawn (matplotlib's ``_path``), handed to pltrs (Rust) as the
#: patch's class takes it (`.Patch._take_path`).
PATH = State("path", _read_path, lambda patch, path: patch._take_path(path), _path_key)


def _shadowed_outline(patch):
    """The outline *patch* draws, placed by its patch transform, as bytes:
    what its shadow shows. What the program changed in place of the patch
    itself is the patch's first, as matplotlib's shadow reads the patch's
    path as it is."""
    if RECORDS in patch.__dict__:
        sync(patch)
    vertices, codes = _patches.placed_outline(patch._native)
    return (vertices.shape, vertices.tobytes(), None if codes is None else codes.tobytes())


#: The patch a `.Shadow` shadows (matplotlib's ``patch``), whose path and
#: patch transform matplotlib's shadow reads each time it draws: its
#: outline is handed to pltrs (Rust) again whenever it changed (a setter
#: of the patch's geometry, its path changed in place).
SHADOWED = State("shadowed", lambda shadow: shadow.patch,
                 lambda shadow, patch: _patches.set_shadow_source(shadow._native, patch._native),
                 _shadowed_outline)
