"""
Empirical cumulative distribution functions: `~.Axes.ecdf`.
"""

import numpy as np

from ... import _api, cbook, lines as mlines
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ._helpers import wrap


class EcdfMixin:
    """`ecdf`."""

    @_preprocess_data(replace_names=["x", "weights"], label_namer="x")
    def ecdf(self, x, weights=None, *, complementary=False,
             orientation="vertical", compress=False, **kwargs):
        """
        Compute and plot the empirical cumulative distribution function of *x*.

        Returns
        -------
        `.Line2D`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        _api.check_in_list(["horizontal", "vertical"], orientation=orientation)
        if "drawstyle" in kwargs or "ds" in kwargs:
            raise TypeError("Cannot pass 'drawstyle' or 'ds' to ecdf()")
        if np.ma.getmask(x).any():
            raise ValueError("ecdf() does not support masked entries")
        native = _plotting.ecdf(
            self._native, np.asarray(x, dtype=float),
            None if weights is None else np.asarray(weights, dtype=float),
            (bool(complementary), orientation == "horizontal", bool(compress)),
            mlines._native_kwargs(cbook.normalize_kwargs(kwargs, mlines.Line2D)))
        return wrap(self, native, mlines.Line2D)
