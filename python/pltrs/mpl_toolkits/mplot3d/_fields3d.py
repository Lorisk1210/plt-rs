"""
What pltrs (Rust) holds of the 3-D geometry of an artist, as the
attributes of matplotlib's `~mpl_toolkits.mplot3d.art3d` classes hold it
(`._properties.Field`): the containers matplotlib's classes and the
methods of `.Axes3D` build, made of the points pltrs (Rust) keeps for an
artist it made itself (``_native.mplot3d.kept``). Loaded when a program
first reads such an attribute.
"""

import numpy as np

from pltrs._native import mplot3d as _mplot3d

from ._properties import MISSING


def kept(artist):
    """What pltrs (Rust) keeps of the 3-D geometry of *artist*, as
    ``_native.mplot3d.kept`` gives it (the kind, the points of each part,
    the paths whose codes the parts keep, how polygons are sorted, whether
    they are closed, the least alpha of shaded markers), or None for an
    artist that is no 3-D artist of an Axes."""
    axes = artist.__dict__.get("_native") is not None and artist.axes
    if not axes or getattr(axes, "name", None) != "3d":
        return None
    return _mplot3d.kept(axes._native, artist._native)


def segments3d(collection):
    """
    The polylines pltrs (Rust) holds for a line collection a 3-D Axes
    made, in the container matplotlib's method builds for
    ``_segments3d``: a list of ``(N, 3)`` arrays (``quiver``,
    ``plot_wireframe``), one array of all (``errorbar``:
    ``_segments3d_as`` is ``"array"``) or lists of points as tuples
    (``stem``: ``"tuples"``).
    """
    held = kept(collection)
    if held is None or held[0] != "segments":
        return MISSING
    parts = held[1]
    container = collection.__dict__.get("_segments3d_as")
    if container == "array":
        return np.array(parts) if parts else np.zeros((0, 2, 3))
    if container == "tuples":
        return [[tuple(point) for point in part.tolist()] for part in parts]
    return list(parts)


def _codes(template):
    """The codes of the path *template* (its arrays), None without."""
    return template[1]


def verts_codes(collection):
    """The 3-D vertices and the codes of each path of a collection lifted
    into 3-D (matplotlib's ``_3dverts_codes``), as pltrs (Rust) holds
    them."""
    held = kept(collection)
    if held is None or held[0] != "paths":
        return MISSING
    return [(part, _codes(template)) for part, template in zip(held[1], held[2])]


def faces(collection):
    """
    matplotlib's array of the faces of a 3-D polygon collection and its
    mask of the vertices that ragged faces lack (``_faces``,
    ``_invalid_vertices``; ``_get_vector``), of the faces pltrs (Rust)
    holds: ``(faces, most vertices, 3)`` floats, and False or a bool array
    ``(faces, most vertices)``. A collection of polygons lifted from 2-D
    paths, and one made of a list, keep the mask as an array.
    """
    held = kept(collection)
    if held is None or held[0] != "polys":
        return MISSING
    parts = held[1]
    counts = np.fromiter(map(len, parts), dtype=np.intp, count=len(parts))
    most = counts.max(initial=0)
    padded = np.empty((len(parts), most, 3))
    for index, part in enumerate(parts):
        padded[index, :len(part)] = part
    invalid = np.arange(most) >= counts[:, None]
    if collection.__dict__.get("_faces_as") == "array" and not invalid.any():
        invalid = False
    return padded, invalid


def codes3d(collection):
    """The codes of the polygons of a 3-D polygon collection lifted from
    2-D paths (matplotlib's ``_codes3d``: a list of lists), None for
    polygons made of their vertices."""
    held = kept(collection)
    if held is None or held[0] != "polys":
        return MISSING
    return [_codes(template).tolist() for template in held[2]] or None


def zsort(collection):
    """The name of the depth sorting of a 3-D polygon collection pltrs
    (Rust) holds, None for another artist."""
    held = kept(collection)
    return None if held is None else held[3]


def direction(text):
    """The direction of a 3-D text pltrs (Rust) holds (matplotlib's
    ``_dir_vec``): an array of three, of ints where they are whole, as
    the directions of the axes are."""
    held = kept(text)
    if held is None or held[0] != "text":
        return MISSING
    vector = held[1][1][0]
    return vector.astype(int) if (vector == np.round(vector)).all() else vector
