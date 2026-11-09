"""
Pseudocolour plots of triangular grids (matplotlib's ``tri._tripcolor``):
the arguments parse and check as matplotlib's do (from matplotlib 3.11.2,
under the Matplotlib License); pltrs (Rust) colours the triangles.
"""

import numpy as np

from .. import _docstring
from .. import _api, _handles, cbook, collections as mcoll
from .._native import plotting as _plotting
from ._triangulation import Triangulation


@_docstring.interpd
def tripcolor(ax, *args, alpha=1.0, norm=None, cmap=None, vmin=None,
              vmax=None, shading='flat', facecolors=None, **kwargs):
    """
    Create a pseudocolor plot of an unstructured triangular grid.

    Call signatures::

      tripcolor(triangulation, c, *, ...)
      tripcolor(x, y, c, *, [triangles=triangles], [mask=mask], ...)

    The triangular grid can be specified either by passing a `.Triangulation`
    object as the first parameter, or by passing the points *x*, *y* and
    optionally the *triangles* and a *mask*. See `.Triangulation` for an
    explanation of these parameters.

    It is possible to pass the triangles positionally, i.e.
    ``tripcolor(x, y, triangles, c, ...)``. However, this is discouraged.
    For more clarity, pass *triangles* via keyword argument.

    If neither of *triangulation* or *triangles* are given, the triangulation
    is calculated on the fly. In this case, it does not make sense to provide
    colors at the triangle faces via *c* or *facecolors* because there are
    multiple possible triangulations for a group of points and you don't know
    which triangles will be constructed.

    Parameters
    ----------
    triangulation : `.Triangulation`
        An already created triangular grid.
    x, y, triangles, mask
        Parameters defining the triangular grid. See `.Triangulation`.
        This is mutually exclusive with specifying *triangulation*.
    c : array-like
        The color values, either for the points or for the triangles. Which one
        is automatically inferred from the length of *c*, i.e. does it match
        the number of points or the number of triangles. If there are the same
        number of points and triangles in the triangulation it is assumed that
        color values are defined at points; to force the use of color values at
        triangles use the keyword argument ``facecolors=c`` instead of just
        ``c``.
        This parameter is position-only.
    facecolors : array-like, optional
        Can be used alternatively to *c* to specify colors at the triangle
        faces. This parameter takes precedence over *c*.
    shading : {'flat', 'gouraud'}, default: 'flat'
        If  'flat' and the color values *c* are defined at points, the color
        values used for each triangle are from the mean c of the triangle's
        three points. If *shading* is 'gouraud' then color values must be
        defined at points.
    %(cmap_doc)s

    %(norm_doc)s

    %(vmin_vmax_doc)s

    %(colorizer_doc)s

    Returns
    -------
    `~matplotlib.collections.PolyCollection` or `~matplotlib.collections.TriMesh`
        The result depends on *shading*: For ``shading='flat'`` the result is a
        `.PolyCollection`, for ``shading='gouraud'`` the result is a `.TriMesh`.

    Other Parameters
    ----------------
    **kwargs : `~matplotlib.collections.Collection` properties

        %(Collection:kwdoc)s
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
    if not np.can_cast(values.dtype, float, "same_kind"):
        # matplotlib's collection is made with the values as its array.
        raise TypeError(f"Image data of dtype {values.dtype} cannot be "
                        f"converted to float")
    from ..axes import Axes
    if not isinstance(ax, Axes):
        # matplotlib's tripcolor turns the grid of *ax* off first.
        ax.grid(False)
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
    if faces:
        # matplotlib's collection is made with the values of the faces as
        # given (those of the triangles not masked), in their dtype.
        collection._A = cbook.safe_masked_invalid(
            values if tri.mask is None else values[~tri.mask], copy=True)
    collection.set_alpha(alpha)
    collection._internal_update(cbook.normalize_kwargs(kwargs, mcoll.PolyCollection))
    return collection
