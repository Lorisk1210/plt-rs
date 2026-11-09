"""
What matplotlib's collections do with the arguments they are made with,
for those pltrs (Rust) does not hold.

matplotlib's collection constructors set their arguments through the
setters after the keywords (``Collection.__init__``), keeping what is
no number or no path as given, and fail where they compute with them:
the paths of patches, the points of a triangulation, the colours of the
patches a `.PatchCollection` matches. The collections make their native
collection of what pltrs (Rust) holds of them (`held_paths`,
`held_patches`) and run matplotlib's computations where it runs them.
Ported from matplotlib 3.11.2's ``collections.py`` under the Matplotlib
License.
"""

import numpy as np

from .path import Path


def set_made_offsets(collection, offsets):
    """Give *collection* the *offsets* it is made with as matplotlib's
    `.Collection` constructor takes them: as floats, unconverted; none at
    all, of any shape, as no items (which its draw does not draw)."""
    offsets = np.asanyarray(offsets, float)
    collection.set_offsets(offsets.reshape(0, 2) if offsets.size == 0 else offsets)


def held_paths(paths):
    """The paths pltrs (Rust) makes a `.PathCollection` of: *paths* (None
    as none), else None when they are no sequence of paths, which
    matplotlib's collection keeps as given (``set_paths``)."""
    if paths is None:
        return []
    if np.iterable(paths) and all(isinstance(path, Path) for path in paths):
        return list(paths)
    return None


def held_patches(patches, match_original):
    """The native patches pltrs (Rust) makes a `.PatchCollection` of
    *patches* (a list), else None when one is no `.Patch`: the collection
    then sets its paths as matplotlib's does (``set_paths``), which fails
    on what is no patch. Matching the originals' colours (*match_original*)
    reads them first, failing as matplotlib's constructor fails."""
    from .patches import Patch
    if all(isinstance(patch, Patch) for patch in patches):
        return [patch._native for patch in patches]
    if match_original:
        for patch in patches:
            if patch.get_fill():
                patch.get_facecolor()
        for getter in ("get_edgecolor", "get_linewidth", "get_linestyle",
                       "get_antialiased"):
            for patch in patches:
                getattr(patch, getter)()
    return None
