"""
Matrices drawn as images: `~.Axes.pcolorfast`, `~.Axes.spy` and
`~.Axes.matshow`.
"""

import numpy as np

from ... import _api, _values, cbook, collections as mcoll, colors as mcolors
from ... import image as mimage, lines as mlines
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ._helpers import wrap
from .scatter import _mapping


def _fast_coords(args, nr, nc):
    """The ``(X, Y)`` of `~.Axes.pcolorfast`'s positional arguments (None
    without), checked as matplotlib checks them."""
    if len(args) == 1:
        return None
    if len(args) != 3:
        raise _api.nargs_error('pcolorfast', '1 or 3', len(args))
    x = np.asarray(args[0])
    y = np.asarray(args[1])
    if x.ndim == 1 and y.ndim == 1:
        if not (x.size == 2 and y.size == 2):
            if x.size != nc + 1:
                raise ValueError(
                    f"Length of X ({x.size}) must be one larger than the "
                    f"number of columns in C ({nc})")
            if y.size != nr + 1:
                raise ValueError(
                    f"Length of Y ({y.size}) must be one larger than the "
                    f"number of rows in C ({nr})"
                )
    elif not (x.ndim == 2 and y.ndim == 2):
        raise TypeError(
            f"When 3 positional parameters are passed to pcolorfast, the first "
            f"two (X and Y) must be both 1D or both 2D; the given X was "
            f"{x.ndim}D and the given Y was {y.ndim}D")
    return x.astype(float), y.astype(float)


class ImagesMixin:
    """`pcolorfast`, `spy` and `matshow`."""

    @_preprocess_data()
    def pcolorfast(self, *args, alpha=None, norm=None, cmap=None, vmin=None,
                   vmax=None, colorizer=None, **kwargs):
        """
        Create a pseudocolor plot with a non-regular rectangular grid.

        Call signature::

            ax.pcolorfast([X, Y], C, /, **kwargs)

        Returns
        -------
        `.AxesImage` or `.PcolorImage` or `.QuadMesh`
            The return type depends on the type of grid.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        C = args[-1]
        nr, nc = np.shape(C)[:2]
        coords = _fast_coords(args, nr, nc)
        if coords is not None and coords[0].ndim == 2 and np.ndim(C) not in {2, 3}:
            raise ValueError("C must be 2D or 3D")
        A = mimage._ImageBase._normalize_image_array(C)
        made, mapping = _mapping(cmap, norm, vmin, vmax, colorizer, variates=True)
        if A.ndim == 2:
            made._scale_norm(norm, vmin, vmax, A)
        image, pcolor_image, mesh = _plotting.pcolorfast(
            self._native, coords, mimage._native_data(A), mapping,
            None if alpha is None else float(alpha))
        if mesh is not None:
            ret = wrap(self, mesh, mcoll.QuadMesh)
            ret._attach_native_mapping(colorizer=made)
        elif image is not None:
            ret = wrap(self, image, mimage.AxesImage)
            ret._set_mapped_data(made, A)
        else:
            ret = wrap(self, pcolor_image, mimage.PcolorImage)
            ret._set_mapped_data(made, A)
        ret._internal_update(kwargs)
        return ret

    @_preprocess_data()
    def spy(self, Z, precision=0, marker=None, markersize=None,
            aspect='equal', origin="upper", **kwargs):
        """
        Plot the sparsity pattern of a 2D array.

        Returns
        -------
        `~matplotlib.image.AxesImage` or `.Line2D`
            The return type depends on the plotting style (see above).

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        if marker is None and markersize is None and hasattr(Z, 'tocoo'):
            marker = 's'
        _api.check_in_list(["upper", "lower"], origin=origin)
        image = marker is None and markersize is None
        if image:
            if 'interpolation' in kwargs:
                raise _api.kwarg_error("spy", "interpolation")
            cmap, norm = kwargs.pop('cmap', None), kwargs.pop('norm', None)
            vmin, vmax = kwargs.pop('vmin', None), kwargs.pop('vmax', None)
            made, mapping = _mapping(cmap, norm, vmin, vmax, kwargs.pop('colorizer', None))
            if cmap is None:
                made.cmap = mcolors.ListedColormap(['w', 'k'], name='binary')
            if norm is None and vmin is None and vmax is None:
                made.norm = mcolors.NoNorm()
            line = mlines._native_kwargs({})
        else:
            if 'linestyle' in kwargs:
                raise _api.kwarg_error("spy", "linestyle")
            mapping = (None, None, None, None)
            line = mlines._native_kwargs(cbook.normalize_kwargs(kwargs, mlines.Line2D))
        if hasattr(Z, 'toarray'):
            Z = Z.toarray()
        image, line = _plotting.spy(
            self._native, np.asarray(Z, dtype=float), float(precision), mapping,
            (None if marker is None else mlines._marker_spec(marker),
             None if markersize is None else float(markersize), aspect, origin, line))
        if image is not None:
            ret = wrap(self, image, mimage.AxesImage)
            # matplotlib's spy draws through imshow, which keeps the extent
            # it sets as a list.
            _values.remember(ret, "extent", list(ret.get_extent()))
            ret._set_mapped_data(made, np.abs(np.asarray(Z)) > precision)
            ret._internal_update(kwargs)
            return ret
        return wrap(self, line, mlines.Line2D)

    def matshow(self, Z, **kwargs):
        """
        Plot the values of a 2D matrix or array as color-coded image.

        Returns
        -------
        `~matplotlib.image.AxesImage`
        """
        Z = np.asanyarray(Z)
        kw = {'origin': 'upper',
              'interpolation': 'nearest',
              'aspect': 'equal',          # (already the imshow default)
              **kwargs}
        im = self.imshow(Z, **kw)
        _plotting.style_matrix_axes(self._native)
        return im
