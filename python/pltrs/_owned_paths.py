"""
The paths a collection hands out (`.Collection.get_paths`,
`.ContourSet.get_paths`), which matplotlib's collections keep and draw:
a program changing one of them (``path.vertices = ...``, as astropy's
``transform_contour_set_inplace`` moves the lines of a contour set into
another coordinate system) changes what the collection draws. pltrs
(Rust) draws a collection from paths of its own, so a path handed out
tells its collection when its vertices or codes are set (`changed`), and
the collection hands its paths to pltrs (Rust) again.
"""

import weakref

#: The attribute of a path handed out holding the bound method it calls
#: once it changes, held weakly; copies and pickles of the path, paths of
#: their own, leave it out (`.Path.__getstate__`).
OWNER = "_handed_out_by"


def hand_out(paths, push):
    """Have each of *paths* call *push*, a bound method of the collection
    handing them out, once it changes."""
    method = weakref.WeakMethod(push)
    for path in paths:
        setattr(path, OWNER, method)


def changed(path):
    """Tell the collection that handed *path* out, if any, that it
    changed."""
    method = path.__dict__.get(OWNER)
    push = None if method is None else method()
    if push is not None:
        push()
