"""
Pseudocolour plots: `~.Axes.pcolor` and `~.Axes.pcolormesh`.
"""

import numpy as np

from ... import _api, cbook, collections as mcoll
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ._helpers import wrap
from .scatter import _mapping


def _mesh_args(fn, args):
    """
    ``(C, (X, Y))`` of the positional arguments ``[X, Y,] C`` of *fn*,
    the coordinates None when absent and C masked values as NaN.
    """
    if len(args) == 1:
        C, = args
        coords = (None, None)
    elif len(args) == 3:
        X, Y, C = args
        coords = tuple(np.asarray(np.ma.filled(np.ma.asarray(v, dtype=float), np.nan))
                       for v in (X, Y))
    else:
        raise _api.nargs_error(fn, takes="1 or 3", given=len(args))
    C = cbook.safe_masked_invalid(C, copy=True)
    if C.ndim != 2:
        raise TypeError(f"Dimensions of C must be 2D, not {C.ndim}D")
    return np.ma.filled(C.astype(float), np.nan), coords


class MeshMixin:
    """`pcolor` and `pcolormesh`."""

    def _pcolor(self, fn, polygons, args, shading, mapping, cls, kwargs):
        """The collection of `pcolor` (*polygons*) or `pcolormesh`."""
        if shading is not None:
            _api.check_in_list(['flat', 'nearest', 'gouraud', 'auto'],
                               shading=shading.lower())
            shading = shading.lower()
        C, coords = _mesh_args(fn, args)
        native = _plotting.pcolor(self._native, polygons, C, coords, shading, mapping)
        collection = wrap(self, native, cls)
        collection._internal_update(cbook.normalize_kwargs(kwargs, cls))
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
        """
        if colorizer is not None:
            raise NotImplementedError("pltrs: pcolor(colorizer=...)")
        if alpha is not None:
            kwargs["alpha"] = alpha
        return self._pcolor("pcolor", True, args, shading,
                            _mapping(cmap, norm, vmin, vmax), mcoll.PolyQuadMesh,
                            kwargs)

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
        """
        if colorizer is not None:
            raise NotImplementedError("pltrs: pcolormesh(colorizer=...)")
        if alpha is not None:
            kwargs["alpha"] = alpha
        kwargs.setdefault("antialiased", antialiased)
        return self._pcolor("pcolormesh", False, args, shading,
                            _mapping(cmap, norm, vmin, vmax), mcoll.QuadMesh,
                            kwargs)
