"""
`~.Axes.errorbar`: lines and markers with error bars.
"""

from numbers import Integral, Number

import numpy as np

from ... import _values, cbook, collections as mcoll, lines as mlines
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ...container import ErrorbarContainer
from ._helpers import (keep_data, line_data, line_kwargs, record_container, wrap,
                       wrap_all)


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


def _in_units(err):
    """Whether the errors *err* are values in units rather than numbers."""
    flat = np.ravel(err)
    return flat.size > 0 and not isinstance(cbook._safe_first_finite(flat), Number)


def _error_bounds(dep_axis, dep, err, convert=None, depconv=None):
    """
    The errors below and above each point of *dep*, broadcast to
    ``(2, n)`` float arrays, checked as matplotlib checks them.

    Errors in units (``timedelta`` errors of dates, say) take *convert*,
    which converts values along the axis of *dep*, and *depconv*, *dep*
    converted: they become the distances, in axis numbers, from the
    points to the ends of their error bars.
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
    if convert is not None and _in_units(err):
        lower, upper = err if np.ndim(err) == 2 else (err, err)
        depconv = np.asarray(depconv, dtype=float)
        return (depconv - np.asarray(convert(dep - lower), dtype=float),
                np.asarray(convert(dep + upper), dtype=float) - depconv)
    # Masked errors are skipped as NaN ones are.
    err = np.ma.filled(np.ma.asarray(err, dtype=float), np.nan)
    lower, upper = np.broadcast_to(err, (2, len(dep)))
    return lower.copy(), upper.copy()


def _cap_data(x, y, errors, flags, everymask, capsize):
    """
    The data of each cap line, ``(xdata, ydata)`` as matplotlib's
    `~.Axes.errorbar` makes its caps of the arrays it is given (object
    arrays for lists), in its order: the *x* caps, then the *y* caps.
    """
    if capsize is None:
        capsize = mlines._rcParams()["errorbar.capsize"]
    data = []
    lolims, uplims, xlolims, xuplims = flags
    for dep_axis, dep, err, low_lims, up_lims, indep in [
            ("x", x, errors[0], xlolims, xuplims, y),
            ("y", y, errors[1], lolims, uplims, x)]:
        if err is None:
            continue
        low_lims = np.asarray(low_lims, bool)
        up_lims = np.asarray(up_lims, bool)
        low, high = dep + np.vstack([-(1 - low_lims), 1 - up_lims]) * err

        def placed(indep_values, dep_values):
            # Both coordinates start as the independent data, then the
            # dependent one is set (matplotlib's ``line.set(xdata=...)``).
            return ((dep_values, indep_values) if dep_axis == "x"
                    else (indep_values, dep_values))
        nolims = ~(low_lims | up_lims)
        if nolims.any() and capsize > 0:
            mask = nolims & everymask
            data += [placed(indep[mask], ends[mask]) for ends in (low, high)]
        for lims, ends in [(low_lims, high), (up_lims, low)]:
            if not lims.any():
                continue
            mask = lims & everymask
            x_masked, y_masked = x[mask], y[mask]
            data.append((ends[mask], y_masked) if dep_axis == "x"
                        else (x_masked, ends[mask]))
            if capsize > 0:
                data.append((x_masked, y_masked))
    return data


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
        xconv, yconv = line_data(self, x, y)
        bounds = {}
        for dep_axis, dep, depconv, err, convert in [
                ("x", x, xconv, xerr, self.convert_xunits),
                ("y", y, yconv, yerr, self.convert_yunits)]:
            if err is not None:
                bounds[dep_axis] = _error_bounds(dep_axis, dep, err, convert, depconv)
        flags = [np.broadcast_to(lims, n).astype(bool).tolist()
                 for lims in (lolims, uplims, xlolims, xuplims)]
        if elinestyle is not None and isinstance(elinestyle, tuple):
            offset, seq = elinestyle
            elinestyle = (offset, None if seq is None else tuple(seq))
        # Several widths go to the bar collections, as matplotlib's
        # ``eb_lines_style`` gives them.
        widths = None if np.ndim(elinewidth) == 0 else elinewidth
        native, data_line, caplines, barcols = _plotting.errorbar(
            self._native, (xconv, yconv),
            (bounds.get("x"), bounds.get("y")),
            fmt, line_kwargs(kwargs),
            (ecolor, None if widths is not None else elinewidth, elinestyle, capsize,
             capthick, bool(barsabove)),
            tuple(flags), everymask.tolist())
        barcols = wrap_all(self, barcols, mcoll.LineCollection)
        if widths is not None:
            for barcol in barcols:
                barcol.set_linewidth(widths)
        caplines = wrap_all(self, caplines, mlines.Line2D)
        data_line = wrap(self, data_line, mlines.Line2D)
        if data_line is not None:
            keep_data(data_line, x, y)
        for capline, (cap_x, cap_y) in zip(
                caplines, _cap_data(x, y, (xerr, yerr), flags, everymask, capsize)):
            keep_data(capline, cap_x, cap_y)
            # The caps' markers take the colour of the lines, as given
            # (without a data line, the one given or matplotlib's 'C0').
            cap_color = (ecolor if ecolor is not None
                         else data_line.get_color() if data_line is not None
                         else kwargs.get("color", "C0"))
            _values.remember(capline, "markeredgecolor", cap_color)
        container = ErrorbarContainer(
            (data_line,
             tuple(caplines),
             tuple(barcols)),
            has_xerr=xerr is not None, has_yerr=yerr is not None,
            label=kwargs.get("label"))
        return record_container(self, container, native)
