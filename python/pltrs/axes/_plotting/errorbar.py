"""
`~.Axes.errorbar`: lines and markers with error bars.
"""

from numbers import Integral

import numpy as np

from ... import cbook, collections as mcoll, lines as mlines
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ...container import ErrorbarContainer
from ._helpers import axis_values, line_kwargs, record_container, wrap, wrap_all


def _errorevery_to_mask(x, errorevery):
    """
    Normalize `errorbar`'s *errorevery* to be a boolean mask for data *x*.

    This function is split out to be usable both by 2D and 3D errorbars.
    """
    if isinstance(errorevery, Integral):
        errorevery = (0, errorevery)
    if isinstance(errorevery, tuple):
        if (len(errorevery) == 2 and
                isinstance(errorevery[0], Integral) and
                isinstance(errorevery[1], Integral)):
            errorevery = slice(errorevery[0], None, errorevery[1])
        else:
            raise ValueError(
                f'{errorevery=!r} is a not a tuple of two integers')
    elif isinstance(errorevery, slice):
        pass
    elif not isinstance(errorevery, str) and np.iterable(errorevery):
        try:
            x[errorevery]  # fancy indexing
        except (ValueError, IndexError) as err:
            raise ValueError(
                f"{errorevery=!r} is iterable but not a valid NumPy fancy "
                "index to match 'xerr'/'yerr'") from err
    else:
        raise ValueError(f"{errorevery=!r} is not a recognized value")
    everymask = np.zeros(len(x), bool)
    everymask[errorevery] = True
    return everymask


def _upcast_err(err):
    """The error sizes *err* as an array (an object array keeps units)."""
    if (np.iterable(err) and len(err) > 0
            and isinstance(cbook._safe_first_finite(err), np.ndarray)):
        atype = type(cbook._safe_first_finite(err))
        if atype is np.ndarray:
            return np.asarray(err, dtype=object)
        return atype(err)
    return np.asarray(err, dtype=object)


def _error_bounds(dep_axis, dep, err):
    """
    The errors below and above each point of *dep*, broadcast to
    ``(2, n)`` float arrays, checked as matplotlib checks them.
    """
    try:
        np.broadcast_to(err, (2, len(dep)))
    except ValueError:
        raise ValueError(
            f"'{dep_axis}err' (shape: {np.shape(err)}) must be a "
            f"scalar or a 1D or (2, n) array-like whose shape matches "
            f"'{dep_axis}' (shape: {np.shape(dep)})") from None
    if err.dtype is np.dtype(object) and np.any(err == None):  # noqa: E711
        raise ValueError(
            f"'{dep_axis}err' must not contain None. "
            "Use NaN if you want to skip a value.")
    if np.any((check := err[err == err]) < -check):
        raise ValueError(
            f"'{dep_axis}err' must not contain negative values")
    lower, upper = np.broadcast_to(np.asarray(err, dtype=float), (2, len(dep)))
    return lower.copy(), upper.copy()


class ErrorbarMixin:
    """`errorbar`."""

    @_preprocess_data(replace_names=["x", "y", "xerr", "yerr"],
                      label_namer="y")
    def errorbar(self, x, y, yerr=None, xerr=None,
                 fmt='', ecolor=None, elinewidth=None, capsize=None,
                 barsabove=False, lolims=False, uplims=False,
                 xlolims=False, xuplims=False, errorevery=1,
                 capthick=None, elinestyle=None,
                 **kwargs):
        """
        Plot y versus x as lines and/or markers with attached errorbars.

        *x*, *y* define the data locations, *xerr*, *yerr* define the errorbar
        sizes. By default, this draws the data markers/lines as well as the
        errorbars. Use fmt='none' to draw errorbars without any data markers.

        Returns
        -------
        `.ErrorbarContainer`
            The container contains:

            - data_line : A `~matplotlib.lines.Line2D` instance of x, y plot markers
              and/or line.
            - caplines : A tuple of `~matplotlib.lines.Line2D` instances of the error
              bar caps.
            - barlinecols : A tuple of `.LineCollection` with the horizontal and
              vertical error ranges.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        kwargs = cbook.normalize_kwargs(kwargs, mlines.Line2D)
        # Drop anything that comes in as None to use the default instead.
        kwargs = {k: v for k, v in kwargs.items() if v is not None}
        if not isinstance(x, np.ndarray):
            x = np.asarray(x, dtype=object)
        if not isinstance(y, np.ndarray):
            y = np.asarray(y, dtype=object)
        if xerr is not None and not isinstance(xerr, np.ndarray):
            xerr = _upcast_err(xerr)
        if yerr is not None and not isinstance(yerr, np.ndarray):
            yerr = _upcast_err(yerr)
        x, y = np.atleast_1d(x, y)  # Make sure all the args are iterable.
        if len(x) != len(y):
            raise ValueError("'x' and 'y' must have the same size")
        everymask = _errorevery_to_mask(x, errorevery)
        n = len(x)
        bounds = {}
        for dep_axis, dep, err in [("x", x, xerr), ("y", y, yerr)]:
            if err is not None:
                bounds[dep_axis] = _error_bounds(dep_axis, dep, err)
        flags = [np.broadcast_to(lims, n).astype(bool).tolist()
                 for lims in (lolims, uplims, xlolims, xuplims)]
        if elinestyle is not None and isinstance(elinestyle, tuple):
            offset, seq = elinestyle
            elinestyle = (offset, None if seq is None else tuple(seq))
        native, data_line, caplines, barcols = _plotting.errorbar(
            self._native, (axis_values(x), axis_values(y)),
            (bounds.get("x"), bounds.get("y")),
            fmt, line_kwargs(kwargs),
            (ecolor, elinewidth, elinestyle, capsize, capthick, bool(barsabove)),
            tuple(flags), everymask.tolist())
        container = ErrorbarContainer(
            (wrap(self, data_line, mlines.Line2D),
             tuple(wrap_all(self, caplines, mlines.Line2D)),
             tuple(wrap_all(self, barcols, mcoll.LineCollection))),
            has_xerr=xerr is not None, has_yerr=yerr is not None,
            label=kwargs.get("label"))
        return record_container(self, container, native)
