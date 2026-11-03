"""
Pseudocolour plots: `~.Axes.pcolor` and `~.Axes.pcolormesh`.
"""

import numpy as np

from ... import _api, _live_transforms, cbook, collections as mcoll
from ..._native import collections as _collections, plotting as _plotting
from ..._preprocess import _preprocess_data
from ._helpers import convert_units, wrap
from .scatter import _mapping


def _mesh_args(axes, fn, args):
    """
    ``(C, (X, Y))`` of the positional arguments ``[X, Y,] C`` of *fn* on
    *axes*, the coordinates None when absent (numbers of the axes' units
    otherwise) and C masked values as NaN; the RGB or RGBA colours
    ``pcolormesh`` takes for C come as an ``(M, N, 4)`` array.
    """
    if len(args) == 1:
        C, = args
        coords = (None, None)
    elif len(args) == 3:
        X, Y, C = args
        X, Y = convert_units(axes, "x", X), convert_units(axes, "y", Y)
        coords = tuple(np.asarray(np.ma.filled(np.ma.asarray(v, dtype=float), np.nan))
                       for v in (X, Y))
    else:
        raise _api.nargs_error(fn, takes="1 or 3", given=len(args))
    if type(C) is np.ndarray and C.ndim == 2 and C.dtype == np.float64 and C.dtype.isnative:
        # What the masking below makes of plain floats, without its copies:
        # pltrs (Rust) copies the values it keeps.
        finite = np.isfinite(C)
        return (C if finite.all() else np.where(finite, C, np.nan)), coords
    C = cbook.safe_masked_invalid(C, copy=True)
    if fn == "pcolormesh" and C.ndim == 3 and C.shape[2] in (3, 4):
        C = np.ma.filled(C.astype(float), np.nan)
        if C.shape[2] == 3:
            C = np.concatenate([C, np.ones(C.shape[:2] + (1,))], axis=2)
        return C, coords
    if C.ndim != 2:
        raise TypeError(f"Dimensions of C must be 2D, not {C.ndim}D")
    return np.ma.filled(C.astype(float), np.nan), coords


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
        C, coords = _mesh_args(self, fn, args)
        made, native_mapping = mapping
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
        if antialiased is not None:
            # matplotlib's mesh draws with the antialiasing it is made with,
            # apart from the collection's own (`.QuadMesh.get_antialiased`).
            _collections.set_antialiaseds(collection._native, [bool(antialiased)])
        collection._attach_native_mapping(colorizer=made)
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
                            _mapping(cmap, norm, vmin, vmax, colorizer),
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
                            _mapping(cmap, norm, vmin, vmax, colorizer),
                            mcoll.QuadMesh, kwargs, antialiased)
