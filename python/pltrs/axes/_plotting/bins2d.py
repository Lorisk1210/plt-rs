"""
Two-dimensional binning: `~.Axes.hexbin` and `~.Axes.hist2d`.
"""

import math
import numbers

import numpy as np

from ... import _docstring
from ... import _api, cbook, collections as mcoll, colorizer as mcolorizer, colors as mcolors
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ..._program_classes import takes_from_program
from ... import _lazy_imports
from ._helpers import wrap
from .scatter import _mapping

_hexbin_reference = _lazy_imports.module(globals(), "._hexbin_reference")

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


def _hexbin_plain(gridsize, bins, xscale, yscale, extent, mincnt, colorizer):
    """Whether pltrs (Rust) bins the arguments of `~.Axes.hexbin` as
    matplotlib's code does: a grid of integer sizes, an *extent* of finite
    numbers, a *bins* of None, ``'log'``, an integer or numbers, integer
    counts, a scale named, and a colorizer or none (the data are checked as
    pltrs (Rust) bins them)."""
    sizes = gridsize if np.iterable(gridsize) else [gridsize]
    return (all(isinstance(size, numbers.Integral) for size in sizes)
            and (extent is None or (np.iterable(extent) and len(extent) == 4 and all(
                isinstance(end, numbers.Real) and math.isfinite(end) for end in extent)))
            and (bins is None or cbook._str_equal(bins, 'log')
                 or isinstance(bins, numbers.Integral)
                 or (np.iterable(bins) and not isinstance(bins, str)
                     and np.asarray(bins).dtype.kind in "iuf"))
            and (mincnt is None or isinstance(mincnt, numbers.Integral))
            and all(isinstance(scale, str) and scale in ('linear', 'log')
                    for scale in (xscale, yscale))
            and (colorizer is None or isinstance(colorizer, mcolorizer.Colorizer)))


def _hist2d_plain(x, y, bins, range, weights, cmin, cmax):
    """Whether pltrs (Rust) bins the arguments of `~.Axes.hist2d` as numpy's
    ``histogram2d`` does: numbers to bin and weigh, bins that are counts or
    edges of numbers (one or two of them), ranges of numbers and bounds of
    the counts that are numbers."""
    def numeric(values):
        try:
            return np.asarray(values).dtype.kind in "biuf"
        except ValueError:  # Ragged.
            return False

    def bins_of(spec):
        return (isinstance(spec, numbers.Integral) and spec > 0
                or (np.ndim(spec) == 1 and numeric(spec)))
    if not (numeric(x) and numeric(y) and (numeric(bins) or np.iterable(bins))):
        return False
    pairs = (bins if np.iterable(bins) and len(bins) == 2
             and all(np.ndim(b) <= 1 for b in bins) else [bins])
    return (all(bins_of(spec) for spec in pairs)
            and (range is None or (numeric(range) and np.shape(range) == (2, 2)))
            and (weights is None or (np.ndim(weights) == 1 and len(weights) == len(x)
                                     and numeric(weights)))
            and all(bound is None or isinstance(bound, numbers.Real)
                    for bound in (cmin, cmax)))


def _hist2d_through_pcolormesh(axes, x, y, bins, range, density, weights, cmin, cmax,
                               kwargs):
    """matplotlib 3.11.2's `~.Axes.hist2d` (under the Matplotlib License),
    for an Axes class whose ``pcolormesh`` is the program's."""
    h, xedges, yedges = np.histogram2d(x, y, bins=bins, range=range,
                                       density=density, weights=weights)
    if cmin is not None:
        h[h < cmin] = None
    if cmax is not None:
        h[h > cmax] = None
    pc = axes.pcolormesh(xedges, yedges, h.T, **kwargs)
    return h, xedges, yedges, pc


def _native_hexbin(axes, x, y, C, gridsize, bins, xscale, yscale, extent, cmap,
                   norm, vmin, vmax, alpha, linewidths, edgecolors,
                   reduce_C_function, mincnt, marginals, colorizer, kwargs):
    """`~.Axes.hexbin` of *axes* binned and drawn by pltrs (Rust)."""
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
    # matplotlib reduces values only where there are values to reduce.
    reduction = (_reduction(reduce_C_function) if C is not None or marginals
                 else "mean")
    native = _plotting.hexbin(
        axes._native, (x, y),
        (None if C is None else np.asarray(C, float), reduction),
        (gridsize, bins, (xscale, yscale), extent,
         None if mincnt is None else int(mincnt)),
        mapping, bool(marginals))
    collection = wrap(axes, native, mcoll.PolyCollection)
    collection._attach_native_mapping(colorizer=made)
    # matplotlib makes the collection with these, then sets the other
    # properties, which may set them again (``linewidth=``).
    collection._internal_update(
        {'alpha': alpha, 'linewidths': linewidths, 'edgecolors': edgecolors})
    collection._made_with(kwargs)
    return collection


class Bins2dMixin:
    """`hexbin` and `hist2d`."""

    @_api.make_keyword_only("3.10", "gridsize")
    @_preprocess_data(replace_names=["x", "y", "C"], label_namer="y")
    @_docstring.interpd
    def hexbin(self, x, y, C=None, gridsize=100, bins=None,
               xscale='linear', yscale='linear', extent=None,
               cmap=None, norm=None, vmin=None, vmax=None,
               alpha=None, linewidths=None, edgecolors='face',
               reduce_C_function=np.mean, mincnt=None, marginals=False,
               colorizer=None, **kwargs):
        """
        Make a 2D hexagonal binning plot of points *x*, *y*.

        If *C* is *None*, the value of the hexagon is determined by the number
        of points in the hexagon. Otherwise, *C* specifies values at the
        coordinate (x[i], y[i]). For each hexagon, these values are reduced
        using *reduce_C_function*.

        Parameters
        ----------
        x, y : array-like
            The data positions. *x* and *y* must be of the same length.

        C : array-like, optional
            If given, these values are accumulated in the bins. Otherwise,
            every point has a value of 1. Must be of the same length as *x*
            and *y*.

        gridsize : int or (int, int), default: 100
            If a single int, the number of hexagons in the *x*-direction.
            The number of hexagons in the *y*-direction is chosen such that
            the hexagons are approximately regular.

            Alternatively, if a tuple (*nx*, *ny*), the number of hexagons
            in the *x*-direction and the *y*-direction. In the
            *y*-direction, counting is done along vertically aligned
            hexagons, not along the zig-zag chains of hexagons; see the
            following illustration.

            .. plot::

               import numpy
               import matplotlib.pyplot as plt

               np.random.seed(19680801)
               n= 300
               x = np.random.standard_normal(n)
               y = np.random.standard_normal(n)

               fig, ax = plt.subplots(figsize=(4, 4))
               h = ax.hexbin(x, y, gridsize=(5, 3))
               hx, hy = h.get_offsets().T
               ax.plot(hx[24::3], hy[24::3], 'ro-')
               ax.plot(hx[-3:], hy[-3:], 'ro-')
               ax.set_title('gridsize=(5, 3)')
               ax.axis('off')

            To get approximately regular hexagons, choose
            :math:`n_x = \\sqrt{3}\\,n_y`.

        bins : 'log' or int or sequence, default: None
            Discretization of the hexagon values.

            - If *None*, no binning is applied; the color of each hexagon
              directly corresponds to its count value.
            - If 'log', use a logarithmic scale for the colormap.
              Internally, :math:`log_{10}(i)` is used to determine the
              hexagon color. This is equivalent to ``norm=LogNorm()``.
              Note that 0 counts are thus marked with the "bad" color.
            - If an integer, divide the counts in the specified number
              of bins, and color the hexagons accordingly.
            - If a sequence of values, the values of the lower bound of
              the bins to be used.

        xscale : {'linear', 'log'}, default: 'linear'
            Use a linear or log10 scale on the horizontal axis.

        yscale : {'linear', 'log'}, default: 'linear'
            Use a linear or log10 scale on the vertical axis.

        mincnt : int >= 0, default: *None*
            If not *None*, only display cells with at least *mincnt*
            number of points in the cell.

        marginals : bool, default: *False*
            If marginals is *True*, plot the marginal density as
            colormapped rectangles along the bottom of the x-axis and
            left of the y-axis.

        extent : 4-tuple of float, default: *None*
            The limits of the bins (xmin, xmax, ymin, ymax).
            The default assigns the limits based on
            *gridsize*, *x*, *y*, *xscale* and *yscale*.

            If *xscale* or *yscale* is set to 'log', the limits are
            expected to be the exponent for a power of 10. E.g. for
            x-limits of 1 and 50 in 'linear' scale and y-limits
            of 10 and 1000 in 'log' scale, enter (1, 50, 1, 3).

        Returns
        -------
        `~matplotlib.collections.PolyCollection`
            A `.PolyCollection` defining the hexagonal bins.

            - `.PolyCollection.get_offsets` contains a Mx2 array containing
              the x, y positions of the M hexagon centers in data coordinates.
            - `.PolyCollection.get_array` contains the values of the M
              hexagons.

            If *marginals* is *True*, horizontal
            bar and vertical bar (both PolyCollections) will be attached
            to the return collection as attributes *hbar* and *vbar*.

        Other Parameters
        ----------------
        %(cmap_doc)s

        %(norm_doc)s

        %(vmin_vmax_doc)s

        alpha : float between 0 and 1, optional
            The alpha blending value, between 0 (transparent) and 1 (opaque).

        linewidths : float, default: *None*
            If *None*, defaults to :rc:`patch.linewidth`.

        edgecolors : {'face', 'none', *None*} or color, default: 'face'
            The color of the hexagon edges. Possible values are:

            - 'face': Draw the edges in the same color as the fill color.
            - 'none': No edges are drawn. This can sometimes lead to unsightly
              unpainted pixels between the hexagons.
            - *None*: Draw outlines in the default color.
            - An explicit color.

        reduce_C_function : callable, default: `numpy.mean`
            The function to aggregate *C* within the bins. It is ignored if
            *C* is not given. This must have the signature::

                def reduce_C_function(C: array) -> float

            Commonly used functions are:

            - `numpy.mean`: average of the points
            - `numpy.sum`: integral of the point values
            - `numpy.amax`: value taken from the largest point

            By default will only reduce cells with at least 1 point because some
            reduction functions (such as `numpy.amax`) will error/warn with empty
            input. Changing *mincnt* will adjust the cutoff, and if set to 0 will
            pass empty input to the reduction function.

        %(colorizer_doc)s

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs : `~matplotlib.collections.PolyCollection` properties
            All other keyword arguments are passed on to `.PolyCollection`:

            %(PolyCollection:kwdoc)s

        See Also
        --------
        hist2d : 2D histogram rectangular bins
        """
        self._process_unit_info([("x", x), ("y", y)], kwargs, convert=False)
        arguments = (x, y, C, gridsize, bins, xscale, yscale, extent, cmap, norm,
                     vmin, vmax, alpha, linewidths, edgecolors, reduce_C_function,
                     mincnt, marginals, colorizer, kwargs)
        if not _hexbin_plain(gridsize, bins, xscale, yscale, extent, mincnt, colorizer):
            return _hexbin_reference.hexbin(self, *arguments)
        try:
            return _native_hexbin(self, *arguments)
        except (TypeError, ValueError, IndexError, KeyError, OverflowError,
                ZeroDivisionError):
            # Refused by pltrs (Rust): matplotlib's code raises its own
            # error, or bins what pltrs (Rust) does not take.
            return _hexbin_reference.hexbin(self, *arguments)

    @_api.make_keyword_only("3.10", "range")
    @_preprocess_data(replace_names=["x", "y", "weights"])
    @_docstring.interpd
    def hist2d(self, x, y, bins=10, range=None, density=False, weights=None,
               cmin=None, cmax=None, **kwargs):
        """
        Make a 2D histogram plot.

        Parameters
        ----------
        x, y : array-like, shape (n, )
            Input values

        bins : None or int or [int, int] or array-like or [array, array]

            The bin specification:

            - If int, the number of bins for the two dimensions
              (``nx = ny = bins``).
            - If ``[int, int]``, the number of bins in each dimension
              (``nx, ny = bins``).
            - If array-like, the bin edges for the two dimensions
              (``x_edges = y_edges = bins``).
            - If ``[array, array]``, the bin edges in each dimension
              (``x_edges, y_edges = bins``).

            The default value is 10.

        range : array-like shape(2, 2), optional
            The leftmost and rightmost edges of the bins along each dimension
            (if not specified explicitly in the bins parameters): ``[[xmin,
            xmax], [ymin, ymax]]``. All values outside of this range will be
            considered outliers and not tallied in the histogram.

        density : bool, default: False
            Normalize histogram.  See the documentation for the *density*
            parameter of `~.Axes.hist` for more details.

        weights : array-like, shape (n, ), optional
            An array of values w_i weighing each sample (x_i, y_i).

        cmin, cmax : float, default: None
            All bins that has count less than *cmin* or more than *cmax* will not be
            displayed (set to NaN before passing to `~.Axes.pcolormesh`) and these count
            values in the return value count histogram will also be set to nan upon
            return.

        Returns
        -------
        h : 2D array
            The bi-dimensional histogram of samples x and y. Values in x are
            histogrammed along the first dimension and values in y are
            histogrammed along the second dimension.
        xedges : 1D array
            The bin edges along the x-axis.
        yedges : 1D array
            The bin edges along the y-axis.
        image : `~.matplotlib.collections.QuadMesh`

        Other Parameters
        ----------------
        %(cmap_doc)s

        %(norm_doc)s

        %(vmin_vmax_doc)s

        %(colorizer_doc)s

        alpha : ``0 <= scalar <= 1`` or ``None``, optional
            The alpha blending value.

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            Additional parameters are passed along to the
            `~.Axes.pcolormesh` method and `~matplotlib.collections.QuadMesh`
            constructor.

        See Also
        --------
        hist : 1D histogram plotting
        hexbin : 2D histogram with hexagonal bins

        Notes
        -----
        Rendering the histogram with a logarithmic color scale is accomplished
        by passing a `.colors.LogNorm` instance to the *norm* keyword
        argument. Likewise, power-law normalization (similar in effect to gamma
        correction) can be accomplished with `.colors.PowerNorm`.

        .. versionchanged:: 3.11
           Previously, `~.Axes.hist2d` would force the axes limits to match the
           extents of the histogram; now, autoscaling also takes other plot
           elements into account.
        """
        if takes_from_program(type(self), "pcolormesh"):
            # matplotlib's draws the counts with the class' own pcolormesh
            # (ultraplot's, which takes keywords of its own).
            return _hist2d_through_pcolormesh(self, x, y, bins, range, density, weights,
                                              cmin, cmax, kwargs)
        # numpy's histogram2d, which matplotlib's computes with, compares
        # the lengths first.
        if len(x) != len(y):
            raise ValueError('x and y must have the same length.')
        if not _hist2d_plain(x, y, bins, range, weights, cmin, cmax):
            # numpy's binning and matplotlib's comparisons with the counts,
            # which fail as theirs do for what pltrs (Rust) would take.
            h, _, _ = np.histogram2d(x, y, bins=bins, range=range, density=density,
                                     weights=weights)
            for bound, compare in ((cmin, np.less), (cmax, np.greater)):
                if bound is not None:
                    compare(h, bound)
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
        mesh._made_with(cbook.normalize_kwargs(kwargs, mcoll.QuadMesh))
        # As matplotlib's QuadMesh constructor ends.
        mesh.set_mouseover(False)
        return h, xedges, yedges, mesh
