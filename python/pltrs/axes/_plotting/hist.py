"""
`~.Axes.hist`: histograms.
"""

import datetime
import itertools
from numbers import Integral

import numpy as np

from ... import _api, _style_helpers, _values, cbook, colors as mcolors, patches as mpatches
from ... import _transform_trees
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ...container import BarContainer
from ... import _lazy_imports
from ..._program_classes import takes_any_from_program
from ._helpers import floats, record_container, wrap_all

_composed_reference = _lazy_imports.module(globals(), "._composed_reference")


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from ... import rcParams
    return rcParams


def _heights_converted(axes, tops, bottom, stacked, orientation):
    """matplotlib's ``bar`` (``barh``) of each dataset of a histogram takes
    the heights of its bars from their bottoms (``_convert_dx``), with
    numpy's warnings where the arithmetic meets infinities (which finite
    numbers never warn about); pltrs (Rust) makes the bars."""
    finite = np.isfinite(tops).all() and (
        bottom is None or np.isfinite(np.asarray(bottom, float)).all())
    if finite:
        return
    convert = axes.convert_yunits if orientation == 'vertical' else axes.convert_xunits
    for top in tops:
        if bottom is None:
            bottom = np.zeros(len(top))
        height = top - bottom if stacked else top
        axes._convert_dx(height, bottom, np.asarray(convert(bottom)), convert)
        if stacked:
            bottom = top


def _native_bins(bins):
    """*bins* as the native side takes them: a count, a name or the edges."""
    if isinstance(bins, str):
        # rcParams['hist.bins'] holds a count as its rc text.
        return int(bins) if bins.isdigit() else bins
    if isinstance(bins, Integral):
        return int(bins)
    return floats(bins)


def _narrow_edges(x, bins, bin_range, weights):
    """
    The edges of *bins* (a count or a strategy name) for the datasets *x*
    when their values are floats narrower than ``float64`` or complex
    numbers, which numpy's `~numpy.histogram` computes in their type
    (matplotlib then makes them floats, numpy warning of the imaginary
    parts it drops); None otherwise, for pltrs (Rust) to compute in
    ``float64``.
    """
    arrays = [np.asarray(xi) for xi in x]
    dtype = np.result_type(*[a.dtype for a in arrays]) if arrays else None
    if (dtype is None or dtype.kind not in "fc"
            or dtype.kind == "f" and dtype.itemsize >= 8
            or np.iterable(bins) and not isinstance(bins, str)):
        return None
    data = np.concatenate([a.ravel() for a in arrays])
    if bin_range is None and np.isfinite(data).any():
        bin_range = (np.nanmin(data), np.nanmax(data))
    if weights is not None:
        weights = np.concatenate([np.asarray(wi, float) for wi in weights])
    edges = np.histogram_bin_edges(data, _native_bins(bins), bin_range, weights)
    return np.array(edges, float) if dtype.kind == "c" else edges


#: The names of numpy's bin estimators (`numpy.histogram_bin_edges`).
_ESTIMATORS = ('auto', 'fd', 'doane', 'rice', 'scott', 'sqrt', 'stone', 'sturges')


def _plain_histogram(x, bins, bin_range, weights):
    """Whether pltrs (Rust) bins the datasets *x* (with their *weights*)
    as numpy would: numbers in all of them, a positive count of *bins*,
    an estimator's name or edges, and a *bin_range* of two numbers."""
    def numbers(values):
        return np.asarray(values).dtype.kind in "iuf"
    # (Complex values are binned as numpy bins them, see `_narrow_edges`.)
    return (all(np.asarray(xi).dtype.kind in "iufc" for xi in x)
            and all(wi is None or numbers(wi) for wi in weights)
            and (bins is None
                 or isinstance(bins, str) and bins in _ESTIMATORS
                 or isinstance(bins, Integral) and bins > 0
                 or np.ndim(bins) == 1 and numbers(bins))
            and (bin_range is None
                 or np.shape(bin_range) == (2,) and numbers(bin_range)))


def _data_range(x):
    """matplotlib's range of the datasets *x* when none is given: from
    the smallest to the largest value of all (NaN and masked values left
    out), None when there is none."""
    xmin, xmax = np.inf, -np.inf
    for xi in x:
        if len(xi):
            xmin = min(xmin, np.nanmin(xi))
            xmax = max(xmax, np.nanmax(xi))
    if xmin <= xmax:  # Only happens if we have seen a finite value.
        return (xmin, xmax)
    return None


def _numpy_histograms(x, bins, bin_range, weights, density):
    """
    matplotlib's binning of the datasets *x*, for those pltrs (Rust) does
    not bin as numpy would: ``np.histogram`` of each, with the bins of all
    of them first when there are several, raising and warning as numpy
    does.
    """
    if bin_range is None:
        bin_range = _data_range(x)
    hist_kwargs = {}
    if any(len(xi) for xi in x) and len(x) > 1:
        _w = None if weights[0] is None else np.concatenate(weights)
        bins = np.histogram_bin_edges(np.concatenate(x), bins, bin_range, _w)
    else:
        hist_kwargs['range'] = bin_range
    if density:
        hist_kwargs['density'] = density
    for xi, wi in zip(x, weights):
        _, bins = np.histogram(xi, bins, weights=wi, **hist_kwargs)


class HistMixin:
    """`hist`."""

    @_api.make_keyword_only("3.10", "range")
    @_preprocess_data(replace_names=["x", 'weights'], label_namer="x")
    def hist(self, x, bins=None, range=None, density=False, weights=None,
             cumulative=False, bottom=None, histtype='bar', align='mid',
             orientation='vertical', rwidth=None, log=False,
             color=None, label=None, stacked=False, **kwargs):
        """
        Compute and plot a histogram.

        This method uses `numpy.histogram` to bin the data in *x* and count the
        number of values in each bin, then draws the distribution either as a
        `.BarContainer` or `.Polygon`. The *bins*, *range*, *density*, and
        *weights* parameters are forwarded to `numpy.histogram`.

        If the data has already been binned and counted, use `~.bar` or
        `~.stairs` to plot the distribution::

            counts, bins = np.histogram(x)
            plt.stairs(counts, bins)

        Alternatively, plot pre-computed bins and counts using ``hist()`` by
        treating each bin as a single point with a weight equal to its count::

            plt.hist(bins[:-1], bins, weights=counts)

        The data input *x* can be a singular array, a list of datasets of
        potentially different lengths ([*x0*, *x1*, ...]), or a 2D ndarray in
        which each column is a dataset. Note that the ndarray form is
        transposed relative to the list form. If the input is an array, then
        the return value is a tuple (*n*, *bins*, *patches*); if the input is a
        sequence of arrays, then the return value is a tuple
        ([*n0*, *n1*, ...], *bins*, [*patches0*, *patches1*, ...]).

        Masked arrays are not supported.

        Parameters
        ----------
        x : (n,) array or sequence of (n,) arrays
            Input values, this takes either a single array or a sequence of
            arrays which are not required to be of the same length.

        bins : int or sequence or str, default: :rc:`hist.bins`
            If *bins* is an integer, it defines the number of equal-width bins
            in the range.

            If *bins* is a sequence, it defines the bin edges, including the
            left edge of the first bin and the right edge of the last bin;
            in this case, bins may be unequally spaced.  All but the last
            (righthand-most) bin is half-open.  In other words, if *bins* is::

                [1, 2, 3, 4]

            then the first bin is ``[1, 2)`` (including 1, but excluding 2) and
            the second ``[2, 3)``.  The last bin, however, is ``[3, 4]``, which
            *includes* 4.

            If *bins* is a string, it is one of the binning strategies
            supported by `numpy.histogram_bin_edges`: 'auto', 'fd', 'doane',
            'scott', 'stone', 'rice', 'sturges', or 'sqrt'.

        range : tuple or None, default: None
            The lower and upper range of the bins. Lower and upper outliers
            are ignored. If not provided, *range* is ``(x.min(), x.max())``.
            Range has no effect if *bins* is a sequence.

            If *bins* is a sequence or *range* is specified, autoscaling
            is based on the specified bin range instead of the
            range of x.

        density : bool, default: False
            If ``True``, draw and return a probability density: each bin
            will display the bin's raw count divided by the total number of
            counts *and the bin width*
            (``density = counts / (sum(counts) * np.diff(bins))``),
            so that the area under the histogram integrates to 1
            (``np.sum(density * np.diff(bins)) == 1``).

            If *stacked* is also ``True``, the sum of the histograms is
            normalized to 1.

        weights : (n,) array-like or None, default: None
            An array of weights, of the same shape as *x*.  Each value in
            *x* only contributes its associated weight towards the bin count
            (instead of 1).  If *density* is ``True``, the weights are
            normalized, so that the integral of the density over the range
            remains 1.

        cumulative : bool or -1, default: False
            If ``True``, then a histogram is computed where each bin gives the
            counts in that bin plus all bins for smaller values. The last bin
            gives the total number of datapoints.

            If *density* is also ``True`` then the histogram is normalized such
            that the last bin equals 1.

            If *cumulative* is a number less than 0 (e.g., -1), the direction
            of accumulation is reversed.  In this case, if *density* is also
            ``True``, then the histogram is normalized such that the first bin
            equals 1.

        bottom : array-like or float, default: 0
            Location of the bottom of each bin, i.e. bins are drawn from
            ``bottom`` to ``bottom + hist(x, bins)`` If a scalar, the bottom
            of each bin is shifted by the same amount. If an array, each bin
            is shifted independently and the length of bottom must match the
            number of bins. If None, defaults to 0.

        histtype : {'bar', 'barstacked', 'step', 'stepfilled'}, default: 'bar'
            The type of histogram to draw.

            - 'bar' is a traditional bar-type histogram.  If multiple data
              are given the bars are arranged side by side.
            - 'barstacked' is a bar-type histogram where multiple
              data are stacked on top of each other.
            - 'step' generates a lineplot that is by default unfilled.
            - 'stepfilled' generates a lineplot that is by default filled.

        align : {'left', 'mid', 'right'}, default: 'mid'
            The horizontal alignment of the histogram bars.

            - 'left': bars are centered on the left bin edges.
            - 'mid': bars are centered between the bin edges.
            - 'right': bars are centered on the right bin edges.

        orientation : {'vertical', 'horizontal'}, default: 'vertical'
            If 'horizontal', `~.Axes.barh` will be used for bar-type histograms
            and the *bottom* kwarg will be the left edges.

        rwidth : float or None, default: None
            The relative width of the bars as a fraction of the bin width.  If
            ``None``, automatically compute the width.

            Ignored if *histtype* is 'step' or 'stepfilled'.

        log : bool, default: False
            If ``True``, the histogram axis will be set to a log scale.

        color : :mpltype:`color` or list of :mpltype:`color` or None, default: None
            Color or sequence of colors, one per dataset.  Default (``None``)
            uses the standard line color sequence.

            .. versionadded:: 3.10
               It is now possible to use a single color with multiple datasets.

        label : str or list of str, optional
            String, or sequence of strings to match multiple datasets.  Bar
            charts yield multiple patches per dataset, but only the first gets
            the label, so that `~.Axes.legend` will work as expected.

        stacked : bool, default: False
            If ``True``, multiple data are stacked on top of each other If
            ``False`` multiple data are arranged side by side if histtype is
            'bar' or on top of each other if histtype is 'step'

        Returns
        -------
        n : array or list of arrays
            The values of the histogram bins. See *density* and *weights* for a
            description of the possible semantics.  If input *x* is an array,
            then this is an array of length *nbins*. If input is a sequence of
            arrays ``[data1, data2, ...]``, then this is a list of arrays with
            the values of the histograms for each of the arrays in the same
            order.  The dtype of the array *n* (or of its element arrays) will
            always be float even if no weighting or normalization is used.

        bins : array
            The edges of the bins. Length nbins + 1 (nbins left edges and right
            edge of last bin).  Always a single array even when multiple data
            sets are passed in.

        patches : `.BarContainer` or list of a single `.Polygon` or list of \
such objects
            Container of individual artists used to create the histogram
            or list of such containers if there are multiple input datasets.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            `~matplotlib.patches.Patch` properties. The following properties
            additionally accept a sequence of values corresponding to the
            datasets in *x*:
            *edgecolor*, *facecolor*, *linewidth*, *linestyle*, *hatch*.

            .. versionadded:: 3.10
               Allowing sequences of values in above listed Patch properties.

        See Also
        --------
        hist2d : 2D histogram with rectangular bins
        hexbin : 2D histogram with hexagonal bins
        stairs : Plot a pre-computed histogram
        bar : Plot a pre-computed histogram

        Notes
        -----
        For large numbers of bins (>1000), plotting can be significantly
        accelerated by using `~.Axes.stairs` to plot a pre-computed histogram
        (``plt.stairs(*np.histogram(data))``), or by setting *histtype* to
        'step' or 'stepfilled' rather than 'bar' or 'barstacked'.
        """
        if takes_any_from_program(type(self), ('bar', 'barh', 'fill')):
            # matplotlib's makes what it plots through these.
            return _composed_reference.hist(self, x, bins, range, density, weights, cumulative,
                bottom, histtype, align, orientation, rwidth, log, color, label, stacked,
                **kwargs)
        bin_range = range
        kwargs = cbook.normalize_kwargs(kwargs, mpatches.Patch)
        if np.isscalar(x):
            x = [x]
        bins = bins if bins is not None else _rcParams()['hist.bins']
        _api.check_in_list(['bar', 'barstacked', 'step', 'stepfilled'],
                           histtype=histtype)
        _api.check_in_list(['left', 'mid', 'right'], align=align)
        _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)
        if histtype == 'barstacked' and not stacked:
            stacked = True
        x = cbook._reshape_2D(x, 'x')
        nx = len(x)
        for arr in x:
            if len(arr) > 0 and isinstance(
                arr[0], (datetime.timedelta, np.timedelta64)
            ):
                raise TypeError(
                    "Axes.hist does not currently support timedelta inputs. "
                    "Convert to numeric values  (e.g., .total_seconds()) first."
                )
        # Process unit information.  _process_unit_info sets the unit and
        # converts the first dataset; then we convert each following dataset
        # one at a time.
        if orientation == "vertical":
            convert_units = self.convert_xunits
            x = [*self._process_unit_info([("x", x[0])], kwargs),
                 *map(convert_units, x[1:])]
        else:  # horizontal
            convert_units = self.convert_yunits
            x = [*self._process_unit_info([("y", x[0])], kwargs),
                 *map(convert_units, x[1:])]
        if bin_range is not None:
            bin_range = convert_units(bin_range)
        if not cbook.is_scalar_or_string(bins):
            bins = convert_units(bins)
        if weights is not None:
            w = cbook._reshape_2D(weights, 'weights')
        else:
            w = [None] * nx
        if len(w) != nx:
            raise ValueError('weights should have the same shape as x')
        for xi, wi in zip(x, w):
            if wi is not None and len(wi) != len(xi):
                raise ValueError('weights should have the same shape as x')
        if any(np.ma.is_masked(xi) for xi in x):
            # matplotlib bins the data of masked arrays, masked values
            # included (numpy's ``histogram`` reads the data), over the
            # range of the values not masked (``nanmin`` and ``nanmax``
            # skip the mask).
            if bin_range is None:
                bin_range = _data_range(x)
            x = [np.ma.getdata(xi) for xi in x]
        if not _plain_histogram(x, bins, bin_range, w):
            _numpy_histograms(x, bins, bin_range, w, bool(density) and not stacked)
        if histtype.startswith('bar') and rwidth is not None:
            # matplotlib clips the relative width of the bars with numpy.
            np.clip(rwidth, 0, 1)
        edges = _narrow_edges(x, bins, bin_range,
                              None if weights is None else w)
        if edges is not None:
            bins = edges
        x = [floats(xi) for xi in x]
        if color is None:
            colors = []
        else:
            if mcolors.is_color_like(color):
                color = [color]*nx
            colors = mcolors.to_rgba_array(color)
            if len(colors) != nx:
                raise ValueError(f"The 'color' keyword argument must have one "
                                 f"color per dataset, but {nx} datasets and "
                                 f"{len(colors)} colors were provided")
        if isinstance(cumulative, bool) or not isinstance(cumulative, (int, float)):
            cumulative = 1 if cumulative else 0
        else:
            cumulative = -1 if cumulative < 0 else (1 if cumulative else 0)
        tops, edges, bar_groups, steps = _plotting.hist(
            self._native, x,
            None if weights is None else [floats(wi) for wi in w],
            (_native_bins(bins),
             None if bin_range is None else tuple(map(float, bin_range)),
             bool(density), cumulative, bool(stacked)),
            (histtype, align, orientation,
             None if rwidth is None else float(rwidth),
             None if bottom is None else floats(bottom), bool(log)),
            [tuple(c) for c in colors])
        tops = np.array(tops, float).reshape(nx, -1)
        bins = np.array(edges, float)
        if histtype.startswith('bar'):
            _heights_converted(self, tops, bottom, stacked, orientation)
            patches = []
            for native, rectangles, values in bar_groups:
                bars = wrap_all(self, rectangles, mpatches.Rectangle)
                for bar in bars:
                    # matplotlib's bars are made of the float arrays of the
                    # histogram: their geometry is numpy floats.
                    _values.remember_all(bar, dict(zip(
                        ("x", "y", "width", "height"),
                        np.float64(bar._fields()[:4]))), ("x", "y", "width", "height"))
                container = BarContainer(
                    bars,
                    datavalues=np.asarray(values),
                    orientation=('vertical' if orientation == 'vertical'
                                 else 'horizontal'),
                    label='')
                _transform_trees.patches_added(self, bars)
                patches.append(record_container(self, container, native))
        else:
            patches = [[polygon] for polygon in
                       wrap_all(self, steps, mpatches.Polygon)]
            _transform_trees.patches_added(
                self, [polygon for (polygon,) in patches])
            for (polygon,) in patches:
                # matplotlib fills the steps with `fill`, whose polygons
                # take its label None and the cycle's colour as it holds it.
                polygon._label_none = True
                if color is None:
                    # An unfilled step's face is its edge's colour, clear.
                    side = "facecolor" if histtype == "stepfilled" else "edgecolor"
                    cycled = _values.cycle_color(self, polygon._get(side))
                    if cycled is not _values.NONE_GIVEN:
                        _values.remember(polygon, "facecolor", cycled)
        labels = [] if label is None else np.atleast_1d(np.asarray(label, str))
        if histtype == "step" and len(colors):
            # The steps of cycled colours have their edge colours already.
            kwargs.setdefault('edgecolor', colors)
        kwargs, style_gen = _style_helpers.style_generator(kwargs)
        for patch, lbl in itertools.zip_longest(patches, labels):
            if not patch:
                continue
            p = patch[0]
            kwargs.update(next(style_gen))
            # matplotlib's hist sets them on the patches its bar added.
            p._internal_update(kwargs)
            if lbl is not None:
                p.set_label(lbl)
            for p in patch[1:]:
                p._internal_update(kwargs)
                p.set_label('_nolegend_')
        if nx == 1:
            return tops[0], bins, patches[0]
        else:
            patch_type = ("BarContainer" if histtype.startswith("bar")
                          else "list[Polygon]")
            return tops, bins, cbook.silent_list(patch_type, patches)
