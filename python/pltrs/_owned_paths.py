"""
The paths a collection hands out (`.Collection.get_paths`,
`.ContourSet.get_paths`), and the path a `.PathPatch` is given, which
matplotlib's artists keep and draw: a program changing one of them
(``path.vertices = ...``, as astropy's ``transform_contour_set_inplace``
moves the lines of a contour set into another coordinate system, and its
frames move the outline of their patch as they draw) changes what the
artist draws. pltrs (Rust) draws them from paths of its own, so such a
path tells the artists holding it when its vertices, codes or
interpolation steps are set (`changed`), and they hand it to pltrs (Rust)
again.

matplotlib's collections also hand a non-affine transform of the
program's their own paths when they draw, and programs keep what it
computed by path (cartopy's ``InterProjectionTransform`` caches the
projected path of each source path). pltrs (Rust) maps the paths it
draws through such a transform by their vertices and codes, so a path
handed out is found again by its content (`held`), and is what the
transform gets.
"""

import weakref

import numpy as np

#: The attribute of a path handed out holding the bound methods it calls
#: once it changes, held weakly, one per artist holding it; copies and
#: pickles of the path, paths of their own, leave it out
#: (`.Path.__getstate__`).
OWNER = "_handed_out_by"


#: The paths handed out while they live, by a digest of their content.
_by_content = weakref.WeakValueDictionary()


def _digest(vertices, codes):
    """A digest of the content of a path of *vertices* and *codes*."""
    vertices = np.asarray(vertices, dtype=float)
    return (vertices.shape, hash(vertices.tobytes()),
            None if codes is None else hash(np.asarray(codes).tobytes()))


def hand_out(paths, push):
    """Have each of *paths* call *push*, a bound method of the artist
    holding them, once it changes, as well as the methods of the other
    artists holding it."""
    method = weakref.WeakMethod(push)
    for path in paths:
        owners = [owner for owner in path.__dict__.get(OWNER, ())
                  if owner() is not None and owner != method]
        setattr(path, OWNER, [*owners, method])
        _by_content[_digest(path.vertices, path.codes)] = path


def held(vertices, codes, steps):
    """The path handed out with *vertices*, *codes* and *steps*
    interpolation steps, if one lives, else None."""
    path = _by_content.get(_digest(vertices, codes))
    if (path is None or path._interpolation_steps != steps
            or not np.array_equal(path.vertices, vertices)
            or not np.array_equal(path.codes, codes)):
        return None
    return path


def changed(path):
    """Tell the artists holding *path*, if any, that it changed."""
    for method in path.__dict__.get(OWNER, ()):
        push = method()
        if push is not None:
            push()
