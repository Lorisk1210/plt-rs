"""
Two-dimensional binning: `~.Axes.hexbin` and `~.Axes.hist2d`.
"""

import numpy as np

from ... import _api, cbook, collections as mcoll, colors as mcolors
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ._helpers import wrap
from .scatter import _mapping

#: The reductions pltrs (Rust) has, by the functions matplotlib's callers
#: pass as ``reduce_C_function``.
_REDUCTIONS = {
    np.mean: "mean", np.sum: "sum", np.median: "median",
    np.max: "max", np.amax: "max", max: "max",
    np.min: "min", np.amin: "min", min: "min",
}


def _reduction(function):
    """The reduction *function* as the native side takes it: the name of
    one pltrs (Rust) has, else the function itself."""
    return _REDUCTIONS.get(function, function)


def _range(limits):
    """A histogram range in native form."""
    return None if limits is None else tuple(map(float, limits))


def _bins(bins):
    """The 2-D histogram bins of one axis in native form."""
    return int(bins) if np.ndim(bins) == 0 else np.asarray(bins, dtype=float)


class Bins2dMixin:
    """`hexbin` and `hist2d`."""

    @_preprocess_data(replace_names=["x", "y", "C"], label_namer="y")
    def hexbin(self, x, y, C=None, gridsize=100, bins=None,
               xscale='linear', yscale='linear', extent=None,
               cmap=None, norm=None, vmin=None, vmax=None,
               alpha=None, linewidths=None, edgecolors='face',
               reduce_C_function=np.mean, mincnt=None, marginals=False,
               colorizer=None, **kwargs):
        """
        Make a 2D hexagonal binning plot of points *x*, *y*.

        Returns
        -------
        `~matplotlib.collections.PolyCollection`
            A `.PolyCollection` defining the hexagonal bins.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        if cbook._str_equal(bins, 'log'):
            if norm is not None:
                _api.warn_external("Only one of 'bins' and 'norm' arguments "
                                   f"can be supplied, ignoring {bins=}")
            else:
                norm = mcolors.LogNorm(vmin=vmin, vmax=vmax)
                vmin = vmax = None
            bins = None
        made, mapping = _mapping(cmap, norm, vmin, vmax, colorizer)
        x, y, C = cbook.delete_masked_points(x, y, C)
        if np.iterable(gridsize):
            nx, ny = gridsize
            gridsize = (int(nx), int(ny))
        else:
            gridsize = (int(gridsize), None)
        x = np.asarray(x, float)
        y = np.asarray(y, float)
        for name, values, scale in (("x", x, xscale), ("y", y, yscale)):
            if scale == 'log' and np.any(values <= 0.0):
                raise ValueError(
                    f"{name} contains non-positive values, so cannot be log-scaled")
        if extent is not None:
            xmin, xmax, ymin, ymax = extent
            if xmin > xmax:
                raise ValueError("In extent, xmax must be greater than xmin")
            if ymin > ymax:
                raise ValueError("In extent, ymax must be greater than ymin")
            extent = tuple(map(float, extent))
        native = _plotting.hexbin(
            self._native, (x, y),
            (None if C is None else np.asarray(C, float), _reduction(reduce_C_function)),
            (gridsize, bins, (xscale, yscale), extent,
             None if mincnt is None else int(mincnt)),
            mapping, bool(marginals))
        collection = wrap(self, native, mcoll.PolyCollection)
        collection._attach_native_mapping(colorizer=made)
        collection._internal_update(cbook.normalize_kwargs(
            {'alpha': alpha, 'linewidths': linewidths, 'edgecolors': edgecolors,
             **kwargs}, mcoll.PolyCollection))
        return collection

    @_preprocess_data(replace_names=["x", "y", "weights"])
    def hist2d(self, x, y, bins=10, range=None, density=False, weights=None,
               cmin=None, cmax=None, **kwargs):
        """
        Make a 2D histogram plot.

        Returns
        -------
        h : 2D array
            The bi-dimensional histogram of samples x and y.
        xedges : 1D array
            The bin edges along the x-axis.
        yedges : 1D array
            The bin edges along the y-axis.
        image : `~.matplotlib.collections.QuadMesh`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        if np.ndim(bins) == 0 or (np.ndim(bins) == 1 and len(bins) != 2):
            bins = (bins, bins)
        xrange, yrange = (None, None) if range is None else range
        made, mapping = _mapping(kwargs.pop('cmap', None), kwargs.pop('norm', None),
                                 kwargs.pop('vmin', None), kwargs.pop('vmax', None),
                                 kwargs.pop('colorizer', None))
        h, xedges, yedges, native = _plotting.hist2d(
            self._native,
            (np.ravel(np.asarray(x, float)), np.ravel(np.asarray(y, float))),
            ((_bins(bins[0]), _bins(bins[1])), (_range(xrange), _range(yrange)),
             bool(density), None if weights is None else np.ravel(np.asarray(weights, float))),
            (None if cmin is None else float(cmin), None if cmax is None else float(cmax)),
            mapping)
        mesh = wrap(self, native, mcoll.QuadMesh)
        mesh._attach_native_mapping(colorizer=made)
        mesh._internal_update(cbook.normalize_kwargs(kwargs, mcoll.QuadMesh))
        return h, xedges, yedges, mesh
