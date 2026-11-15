"""
`~.Axes.errorbar`: lines and markers with error bars.
"""

from numbers import Integral, Number, Real
from types import NoneType

import numpy as np

from ... import _docstring
from ... import _api, _values, cbook, collections as mcoll, colors as mcolors, lines as mlines
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ..._program_classes import takes_any_from_program
from ...container import ErrorbarContainer
from ._helpers import (keep_data, line_data, line_kwargs, record_container, wrap,
                       wrap_all)
from ... import _lazy_imports

_errorbar_reference = _lazy_imports.module(globals(), "._errorbar_reference")


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


def _refuses(ecolor):
    """The error matplotlib's error bars raise for the colour *ecolor*, if
    they refuse it."""
    if ecolor is None:
        return None
    try:
        mcolors.to_rgba_array(ecolor)
    except (TypeError, ValueError) as err:
        return err
    return None


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

    @_api.make_keyword_only("3.10", "ecolor")
    @_preprocess_data(replace_names=["x", "y", "xerr", "yerr"],
                      label_namer="y")
    @_docstring.interpd
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

        .. versionadded:: 3.7
           Caps and error lines are drawn in polar coordinates on polar plots.


        Parameters
        ----------
        x, y : float or array-like
            The data positions.

        xerr, yerr : float or array-like, shape(N,) or shape(2, N), optional
            The errorbar sizes:

            - scalar: Symmetric +/- values for all data points.
            - shape(N,): Symmetric +/-values for each data point.
            - shape(2, N): Separate - and + values for each bar. First row
              contains the lower errors, the second row contains the upper
              errors.
            - *None*: No errorbar.

            All values must be >= 0.

            See :doc:`/gallery/statistics/errorbar_features`
            for an example on the usage of ``xerr`` and ``yerr``.

        fmt : str, default: ''
            The format for the data points / data lines. See `.plot` for
            details.

            Use 'none' (case-insensitive) to plot errorbars without any data
            markers.

        ecolor : :mpltype:`color`, default: None
            The color of the errorbar lines.  If None, use the color of the
            line connecting the markers.

        elinewidth : float, default: None
            The linewidth of the errorbar lines. If None, the linewidth of
            the current style is used.

        elinestyle : str or tuple, default: 'solid'
           The linestyle of the errorbar lines.
           Valid values for linestyles include {'-', '--', '-.',
            ':', '', (offset, on-off-seq)}. See `.Line2D.set_linestyle` for a
            complete description.

        capsize : float, default: :rc:`errorbar.capsize`
            The length of the error bar caps in points.

        capthick : float, default: None
            An alias to the keyword argument *markeredgewidth* (a.k.a. *mew*).
            This setting is a more sensible name for the property that
            controls the thickness of the error bar cap in points. For
            backwards compatibility, if *mew* or *markeredgewidth* are given,
            then they will over-ride *capthick*. This may change in future
            releases.

        barsabove : bool, default: False
            If True, will plot the errorbars above the plot
            symbols. Default is below.

        lolims, uplims, xlolims, xuplims : bool or array-like, default: False
            These arguments can be used to indicate that a value gives only
            upper/lower limits.  In that case a caret symbol is used to
            indicate this. *lims*-arguments may be scalars, or array-likes of
            the same length as *xerr* and *yerr*.  To use limits with inverted
            axes, `~.Axes.set_xlim` or `~.Axes.set_ylim` must be called before
            :meth:`errorbar`.  Note the tricky parameter names: setting e.g.
            *lolims* to True means that the y-value is a *lower* limit of the
            True value, so, only an *upward*-pointing arrow will be drawn!

        errorevery : int or (int, int), default: 1
            draws error bars on a subset of the data. *errorevery* =N draws
            error bars on the points (x[::N], y[::N]).
            *errorevery* =(start, N) draws error bars on the points
            (x[start::N], y[start::N]). e.g. errorevery=(6, 3)
            adds error bars to the data at (x[6], x[9], x[12], x[15], ...).
            Used to avoid overlapping error bars when two series share x-axis
            values.

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

        **kwargs
            All other keyword arguments are passed on to the `~.Axes.plot` call
            drawing the markers. For example, this code makes big red squares
            with thick green edges::

                x, y, yerr = rand(3, 10)
                errorbar(x, y, yerr, marker='s', mfc='red',
                         mec='green', ms=20, mew=4)

            where *mfc*, *mec*, *ms* and *mew* are aliases for the longer
            property names, *markerfacecolor*, *markeredgecolor*, *markersize*
            and *markeredgewidth*.

            Valid kwargs for the marker properties are:

            - *dashes*
            - *dash_capstyle*
            - *dash_joinstyle*
            - *drawstyle*
            - *fillstyle*
            - *linestyle*
            - *marker*
            - *markeredgecolor*
            - *markeredgewidth*
            - *markerfacecolor*
            - *markerfacecoloralt*
            - *markersize*
            - *markevery*
            - *solid_capstyle*
            - *solid_joinstyle*

            Refer to the corresponding `.Line2D` property for more details:

            %(Line2D:kwdoc)s
        """
        if takes_any_from_program(type(self), ("vlines", "hlines")):
            # matplotlib's draws the bars through these.
            return _errorbar_reference.errorbar(
                self, x, y, yerr, xerr, fmt, ecolor, elinewidth, capsize, barsabove,
                lolims, uplims, xlolims, xuplims, errorevery, capthick, elinestyle, **kwargs)
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
        if fmt != '' and not isinstance(fmt, str):
            # matplotlib makes the data line with ``_plot_args(x, y, fmt)``.
            raise ValueError('third arg must be a format string')
        refused = _refuses(ecolor) if xerr is not None or yerr is not None else None
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
        if refused is not None:
            # matplotlib's ``hlines`` (or ``vlines``) adds the first bars,
            # after the data line, then fails to give them the colour:
            # those are made, in the default colour meanwhile.
            first = "x" if "x" in bounds else "y"
            bounds = {first: bounds[first]}
            flags = [[False] * n] * 4
            capsize = 0
        # matplotlib styles the caps by their size (compared with zero),
        # then makes the bars in their line style.
        _ = (mlines._rcParams()["errorbar.capsize"] if capsize is None else capsize) > 0
        if elinestyle is not None:
            mcoll._checked_linestyles(elinestyle)
        if elinestyle is not None and isinstance(elinestyle, tuple):
            offset, seq = elinestyle
            elinestyle = (offset, None if seq is None else tuple(seq))
        # Several widths, or one that is no number, go to the bar
        # collections as matplotlib's ``eb_lines_style`` gives them (which
        # they keep), and a cap thickness that is no number to the caps.
        widths = (None if np.ndim(elinewidth) == 0 and isinstance(elinewidth, (Real, NoneType))
                  else elinewidth)
        thickness = None if isinstance(capthick, (Real, NoneType)) else capthick
        # Several colours (one per bar, holoviews' colour ops) go to the bar
        # collections, as matplotlib's ``eb_lines_style`` gives them.
        colors = (None if ecolor is None or refused is not None
                  or mcolors.is_color_like(ecolor) else ecolor)
        native, data_line, caplines, barcols = _plotting.errorbar(
            self._native, (xconv, yconv),
            (bounds.get("x"), bounds.get("y")),
            fmt, line_kwargs(kwargs),
            (None if refused is not None or colors is not None else ecolor,
             None if widths is not None else elinewidth, elinestyle, capsize,
             None if thickness is not None else capthick, bool(barsabove)),
            tuple(flags), everymask.tolist())
        barcols = wrap_all(self, barcols, mcoll.LineCollection)
        if "clip_box" in kwargs:
            # matplotlib's ``vlines`` sets it on the bars it added (the
            # data line and the caps, made with it, are clipped as added).
            for barcol in barcols:
                barcol.set_clip_box(kwargs["clip_box"])
        if widths is not None:
            for barcol in barcols:
                barcol.set_linewidth(widths)
        if colors is not None:
            for barcol in barcols:
                barcol.set_color(colors)
        caplines = wrap_all(self, caplines, mlines.Line2D)
        data_line = wrap(self, data_line, mlines.Line2D)
        if data_line is not None:
            keep_data(data_line, x, y)
        if refused is not None:
            # The bars keep the colour, failing their first draw with it.
            barcols[0].set_color(ecolor)
            raise refused
        for capline, (cap_x, cap_y) in zip(
                caplines, _cap_data(x, y, (xerr, yerr), flags, everymask, capsize)):
            keep_data(capline, cap_x, cap_y)
            # The caps' markers take the colour of the lines, as given
            # (without a data line, the one given or matplotlib's 'C0').
            cap_color = (ecolor if ecolor is not None
                         else data_line.get_color() if data_line is not None
                         else kwargs.get("color", "C0"))
            if colors is not None:
                # Kept, and refused where the caps are drawn.
                capline.set_markeredgecolor(colors)
            else:
                _values.remember(capline, "markeredgecolor", cap_color)
            if thickness is not None:
                capline.set_markeredgewidth(thickness)
        container = ErrorbarContainer(
            (data_line,
             tuple(caplines),
             tuple(barcols)),
            has_xerr=xerr is not None, has_yerr=yerr is not None,
            label=kwargs.get("label"))
        return record_container(self, container, native)
