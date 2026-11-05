"""
Pseudocolour plots of triangular grids (matplotlib's ``tri._tripcolor``):
the arguments parse and check as matplotlib's do (from matplotlib 3.11.2,
under the Matplotlib License); pltrs (Rust) colours the triangles.
"""

import numpy as np

from .. import _api, _handles, cbook, collections as mcoll
from .._native import plotting as _plotting
from ._triangulation import Triangulation


def tripcolor(ax, *args, alpha=1.0, norm=None, cmap=None, vmin=None,
              vmax=None, shading='flat', facecolors=None, **kwargs):
    """
    Create a pseudocolor plot of an unstructured triangular grid.

    Call signatures::

      tripcolor(triangulation, c, *, ...)
      tripcolor(x, y, c, *, [triangles=triangles], [mask=mask], ...)

    Returns
    -------
    `~matplotlib.collections.PolyCollection` or `~matplotlib.collections.TriMesh`
        The result depends on *shading*: For ``shading='flat'`` the result is a
        `.PolyCollection`, for ``shading='gouraud'`` the result is a `.TriMesh`.
    """
    from ..axes._plotting.scatter import _mapping
    _api.check_in_list(['flat', 'gouraud'], shading=shading)

    tri, args, kwargs = Triangulation.get_from_args_and_kwargs(*args, **kwargs)

    if facecolors is not None:
        if args:
            _api.warn_external(
                "Positional parameter c has no effect when the keyword "
                "facecolors is given")
        if len(facecolors) != len(tri.triangles):
            raise ValueError("The length of facecolors must match the number "
                             "of triangles")
        values, faces = np.asarray(facecolors), True
    else:
        if not args:
            raise TypeError(
                "tripcolor() missing 1 required positional argument: 'c'; or "
                "1 required keyword-only argument: 'facecolors'")
        elif len(args) > 1:
            raise TypeError(f"Unexpected positional parameters: {args[1:]!r}")
        values = np.asarray(args[0])
        if len(values) == len(tri.x):
            faces = False
        elif len(values) == len(tri.triangles):
            faces = True
        else:
            raise ValueError('The length of c must match either the number '
                             'of points or the number of triangles')
    if shading == 'gouraud' and faces:
        raise ValueError(
            "shading='gouraud' can only be used when the colors "
            "are specified at the points, not at the faces.")

    if shading == 'flat':
        # Handling of linewidths, shading, edgecolors and antialiased as
        # in Axes.pcolor (a Gouraud-shaded mesh draws no edges).
        linewidths = (0.25,)
        if 'linewidth' in kwargs:
            kwargs['linewidths'] = kwargs.pop('linewidth')
        kwargs.setdefault('linewidths', linewidths)

        edgecolors = 'none'
        if 'edgecolor' in kwargs:
            kwargs['edgecolors'] = kwargs.pop('edgecolor')
        ec = kwargs.setdefault('edgecolors', edgecolors)

        if 'antialiased' in kwargs:
            kwargs['antialiaseds'] = kwargs.pop('antialiased')
        if 'antialiaseds' not in kwargs:
            # pltrs: the antialiasing pltrs (Rust) gave the edges it made
            # without colour is reset when they are given one.
            kwargs['antialiaseds'] = False if ec.lower() == "none" else None

    made, mapping = _mapping(cmap, norm, vmin, vmax, kwargs.pop('colorizer', None))
    polygons, mesh = _plotting.tripcolor(
        ax._native, tri._native,
        (np.ma.filled(np.ma.asarray(values, dtype=float), np.nan).ravel(), faces),
        mapping, shading == 'gouraud')
    if mesh is not None:
        made._scale_norm(norm, vmin, vmax, values)
        collection = _handles.wrap(mesh, mcoll.TriMesh, ax.figure)
        collection._hold_triangulation(tri)
    else:
        collection = _handles.wrap(polygons, mcoll.PolyCollection, ax.figure)
    collection._attach_native_mapping(colorizer=made)
    collection.set_alpha(alpha)
    collection._internal_update(cbook.normalize_kwargs(kwargs, mcoll.PolyCollection))
    return collection
