"""
Pseudocolour plots: `~.Axes.pcolor` and `~.Axes.pcolormesh`.
"""

import numpy as np

from ... import _api, _live_transforms, cbook, collections as mcoll
from ..._native import collections as _collections, plotting as _plotting
from ..._preprocess import _preprocess_data
from ._helpers import convert_units, wrap
from .scatter import _mapping


def _mesh_args(axes, fn, args, shading):
    """
    ``(C, (X, Y), dtype)`` of the positional arguments ``[X, Y,] C`` of
    *fn* on *axes* drawn with *shading*: the coordinates None when absent
    (numbers of the axes' units otherwise), C masked values as NaN (the
    RGB or RGBA colours ``pcolormesh`` takes for C come as an ``(M, N,
    4)`` array), and the dtype of the coordinates matplotlib's mesh keeps
    (see `_coordinates_dtype`).
    """
    if len(args) == 1:
        C, = args
        coords = (None, None)
        given = None
    elif len(args) == 3:
        X, Y, C = args
        X, Y = convert_units(axes, "x", X), convert_units(axes, "y", Y)
        given = (X, Y)
        coords = tuple(np.asarray(np.ma.filled(np.ma.asarray(v, dtype=float), np.nan))
                       for v in (X, Y))
    else:
        raise _api.nargs_error(fn, takes="1 or 3", given=len(args))
    dtype = _coordinates_dtype(given, np.shape(C), shading)
    if type(C) is np.ndarray and C.ndim == 2 and C.dtype == np.float64 and C.dtype.isnative:
        # What the masking below makes of plain floats, without its copies:
        # pltrs (Rust) copies the values it keeps.
        finite = np.isfinite(C)
        return (C if finite.all() else np.where(finite, C, np.nan)), coords, dtype
    C = cbook.safe_masked_invalid(C, copy=True)
    if fn == "pcolormesh" and C.ndim == 3 and C.shape[2] in (3, 4):
        C = np.ma.filled(C.astype(float), np.nan)
        if C.shape[2] == 3:
            C = np.concatenate([C, np.ones(C.shape[:2] + (1,))], axis=2)
        return C, coords, dtype
    if C.ndim != 2:
        raise TypeError(f"Dimensions of C must be 2D, not {C.ndim}D")
    return np.ma.filled(C.astype(float), np.nan), coords, dtype


def _coordinates_dtype(given, shape, shading):
    """
    The dtype of the coordinates of matplotlib's mesh, which keeps the
    grids its ``_pcolorargs`` makes: those of the *given* ``(X, Y)`` (the
    integers of `numpy.arange` without them), made floats where the cells
    are centred on them (shading 'nearest', or 'auto' with grids of the
    size of C, of *shape*).
    """
    from ... import rcParams
    shading = (shading or rcParams['pcolor.shading']).lower()
    if given is None:
        return np.dtype(float) if shading == 'nearest' else np.arange(1).dtype
    X, Y = (np.asarray(v) for v in given)
    dtype = np.result_type(X, Y)
    centred = (np.shape(X)[-1], np.shape(Y)[0]) == tuple(shape[1::-1])
    if shading == 'nearest' or shading == 'auto' and centred:
        return np.result_type(dtype, float)
    return dtype



def _first_variate(values):
    """The first variate of *values*, a structured array of several (other
    arrays as they are, which the mesh then refuses as matplotlib does)."""
    if values.dtype.names:
        return values[values.dtype.names[0]].astype(float)
    return values

class MeshMixin:
    """`pcolor` and `pcolormesh`."""

    def _pcolor(self, fn, polygons, args, shading, mapping, cls, kwargs,
                antialiased=None):
        """The collection of `pcolor` (*polygons*) or `pcolormesh`, mapping
        its values as *mapping* (`_mapping`'s pair) says; the mesh of
        `pcolormesh` is drawn *antialiased* or not."""
        if shading is not None:
            _api.check_in_list(['flat', 'nearest', 'gouraud', 'auto'],
                               shading=shading.lower())
            shading = shading.lower()
        made, native_mapping = mapping
        variates = None
        if made.norm.n_components > 1 and args and not polygons:
            # Values of several variates (a structured array, as matplotlib
            # 3.11's ``pcolormesh`` takes them): the mesh is made of the
            # first variate, then maps them all through the bivariate or
            # multivariate colorizer.
            variates = np.asarray(args[-1])
            args = (*args[:-1], _first_variate(variates))
            native_mapping = (None, None, None, None)
        C, coords, dtype = _mesh_args(self, fn, args, shading)
        kwargs = cbook.normalize_kwargs(kwargs, cls)
        if (polygons and "antialiased" not in kwargs
                and not cbook._str_lower_equal(kwargs.get("edgecolor", "none"), "none")):
            # matplotlib's pcolor turns antialiasing off only for unstroked
            # cells; stroked ones take the collections' default.
            from ... import rcParams
            kwargs["antialiased"] = rcParams["patch.antialiased"]
        # The data limits cover the mesh in the coordinates it is drawn in.
        placement = (shading, _live_transforms.given_to_native(
            kwargs.get("transform")))
        if C.ndim == 3:
            native = _plotting.pcolormesh_rgba(self._native, C, coords, placement)
        else:
            native = _plotting.pcolor(self._native, polygons, C, coords, placement,
                                      native_mapping)
        collection = wrap(self, native, cls)
        collection._coordinates_dtype = dtype
        if antialiased is not None:
            # matplotlib's mesh draws with the antialiasing it is made with,
            # apart from the collection's own (`.QuadMesh.get_antialiased`).
            _collections.set_antialiaseds(collection._native, [bool(antialiased)])
        if variates is None:
            collection._attach_native_mapping(colorizer=made)
        else:
            collection._map_variates_through(made, variates)
        if C.ndim == 3:
            # The RGB(A) colours are the mesh's array, as matplotlib's mesh
            # is made with them (which autoscales its norm to them).
            collection.set_array(args[-1])
        collection._internal_update(kwargs)
        return collection

    @_preprocess_data()
    def pcolor(self, *args, shading=None, alpha=None, norm=None, cmap=None,
               vmin=None, vmax=None, colorizer=None, **kwargs):
        r"""
        Create a pseudocolor plot with a non-regular rectangular grid.

        Call signature::

            pcolor([X, Y,] C, /, **kwargs)

        Returns
        -------
        `matplotlib.collections.PolyQuadMesh`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        if alpha is not None:
            kwargs["alpha"] = alpha
        return self._pcolor("pcolor", True, args, shading,
                            _mapping(cmap, norm, vmin, vmax, colorizer, variates=True),
                            mcoll.PolyQuadMesh, kwargs)

    @_preprocess_data()
    def pcolormesh(self, *args, alpha=None, norm=None, cmap=None, vmin=None,
                   vmax=None, colorizer=None, shading=None, antialiased=False,
                   **kwargs):
        """
        Create a pseudocolor plot with a non-regular rectangular grid.

        Call signature::

            pcolormesh([X, Y,] C, /, **kwargs)

        Returns
        -------
        `matplotlib.collections.QuadMesh`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        if alpha is not None:
            kwargs["alpha"] = alpha
        return self._pcolor("pcolormesh", False, args, shading,
                            _mapping(cmap, norm, vmin, vmax, colorizer, variates=True),
                            mcoll.QuadMesh, kwargs, antialiased)
