"""
`~.Axes.hist`: histograms.
"""

import datetime
import itertools
from numbers import Integral

import numpy as np

from ... import _api, _style_helpers, cbook, colors as mcolors, patches as mpatches
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ...container import BarContainer
from ._helpers import floats, record_container, wrap_all


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from ... import rcParams
    return rcParams


def _native_bins(bins):
    """*bins* as the native side takes them: a count, a name or the edges."""
    if isinstance(bins, str):
        # rcParams['hist.bins'] holds a count as its rc text.
        return int(bins) if bins.isdigit() else bins
    if isinstance(bins, Integral):
        return int(bins)
    return floats(bins)


class HistMixin:
    """`hist`."""

    @_preprocess_data(replace_names=["x", 'weights'], label_namer="x")
    def hist(self, x, bins=None, range=None, density=False, weights=None,
             cumulative=False, bottom=None, histtype='bar', align='mid',
             orientation='vertical', rwidth=None, log=False,
             color=None, label=None, stacked=False, **kwargs):
        """
        Compute and plot a histogram.

        Returns
        -------
        n : array or list of arrays
            The values of the histogram bins.
        bins : array
            The edges of the bins.
        patches : `.BarContainer` or list of a single `.Polygon` or list of such objects
            Container of individual artists used to create the histogram
            or list of such containers if there are multiple input datasets.
        """
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
        x = [floats(xi) for xi in x]
        if weights is not None:
            w = cbook._reshape_2D(weights, 'weights')
        else:
            w = [None] * nx
        if len(w) != nx:
            raise ValueError('weights should have the same shape as x')
        for xi, wi in zip(x, w):
            if wi is not None and len(wi) != len(xi):
                raise ValueError('weights should have the same shape as x')
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
            self._native, [list(xi) for xi in x],
            None if weights is None else [list(floats(wi)) for wi in w],
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
            patches = []
            for native, rectangles, values in bar_groups:
                container = BarContainer(
                    wrap_all(self, rectangles, mpatches.Rectangle),
                    datavalues=np.asarray(values),
                    orientation=('vertical' if orientation == 'vertical'
                                 else 'horizontal'),
                    label='')
                patches.append(record_container(self, container, native))
        else:
            patches = [[polygon] for polygon in
                       wrap_all(self, steps, mpatches.Polygon)]
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
