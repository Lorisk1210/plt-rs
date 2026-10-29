"""
Lines: `~.Axes.plot`, the reference lines and spans (`~.Axes.axhline`,
`~.Axes.axvline`, `~.Axes.axline`, `~.Axes.axhspan`, `~.Axes.axvspan`)
and the segment families `~.Axes.hlines` and `~.Axes.vlines`.
"""

import inspect

import numpy as np

from ... import _api, cbook, collections as mcoll, lines as mlines
from ..._native import axes as _axes, plotting as _plotting
from ..._preprocess import _label_from_arg, _preprocess_data, _replacer
from ...patches import Rectangle
from ._helpers import axis_values, line_kwargs, wrap


def _check_no_units(vals, names):
    """Check that *vals*, named *names*, are not unitized."""
    from ... import units
    for val, name in zip(vals, names):
        if not units._is_natively_supported(val):
            raise ValueError(f"{name} must be a single scalar value, "
                             f"but got {val}")


def _format_parts(fmt, ambiguous_fmt_datakey=False):
    """
    ``(linestyle, marker, color)`` of the format string *fmt*, raising
    matplotlib's messages (worded for a possible data key when
    *ambiguous_fmt_datakey*).
    """
    try:
        return _plotting.format_parts(fmt)
    except ValueError as err:
        if ambiguous_fmt_datakey:
            raise ValueError(str(err).replace(
                "is not", "is neither a data key nor", 1)) from None
        raise


def _plot_datasets(args, data, kwargs):
    """
    Split ``plot``'s positional arguments into ``(x, y, fmt, label)``
    datasets, as matplotlib's ``_process_plot_var_args`` does: each group
    is ``[x], y, [fmt]``, and 2D *x* or *y* hold one dataset per column.
    """
    if data is None:
        args = [cbook.sanitize_sequence(a) for a in args]
    else:
        replaced = [_replacer(data, arg) for arg in args]
        if len(args) == 1:
            label_namer_idx = 0
        elif len(args) == 2:
            try:
                _plotting.format_parts(args[1])
            except (ValueError, TypeError):
                label_namer_idx = 1
            else:
                if replaced[1] is not args[1]:
                    _api.warn_external(
                        f"Second argument {args[1]!r} is ambiguous: could "
                        f"be a format string but is in 'data'; using as "
                        f"data.  If it was intended as data, set the "
                        f"format string to an empty string to suppress "
                        f"this warning.  If it was intended as a format "
                        f"string, explicitly pass the x-values as well.  "
                        f"Alternatively, rename the entry in 'data'.",
                        RuntimeWarning)
                    label_namer_idx = 1
                else:
                    label_namer_idx = 0
        elif len(args) == 3:
            label_namer_idx = 1
        else:
            raise ValueError(
                "Using arbitrary long args with data is not supported due "
                "to ambiguity of arguments; use multiple plotting calls "
                "instead")
        if kwargs.get("label") is None:
            kwargs["label"] = _label_from_arg(
                replaced[label_namer_idx], args[label_namer_idx])
        args = replaced
    ambiguous_fmt_datakey = data is not None and len(args) == 2

    if len(args) >= 4 and not cbook.is_scalar_or_string(kwargs.get("label")):
        raise ValueError("plot() with multiple groups of data (i.e., "
                         "pairs of x and y) does not support multiple "
                         "labels")
    while args:
        this, args = args[:2], args[2:]
        if args and isinstance(args[0], str):
            this += args[0],
            args = args[1:]
        yield from _group_datasets(this, kwargs, ambiguous_fmt_datakey)


def _group_datasets(tup, kwargs, ambiguous_fmt_datakey):
    """The datasets of one ``([x], y, [fmt])`` group (``_plot_args``)."""
    fmt = None
    if len(tup) > 1 and isinstance(tup[-1], str):
        *xy, fmt = tup
        parts = _format_parts(fmt, ambiguous_fmt_datakey)
    elif len(tup) == 3:
        raise ValueError('third arg must be a format string')
    else:
        xy = tup
        parts = (None, None, None)
    if any(v is None for v in tup):
        raise ValueError("x, y, and format string must not be None")
    if fmt is not None and fmt.lower() != 'none':
        for prop_name, val in zip(('linestyle', 'marker', 'color'), parts):
            if (val is not None and prop_name in kwargs and val != 'None'):
                _api.warn_external(
                    f"{prop_name} is redundantly defined by the "
                    f"'{prop_name}' keyword argument and the fmt string "
                    f'"{fmt}" (-> {prop_name}={val!r}). The keyword '
                    f"argument will take precedence.")
    if len(xy) == 2:
        x = cbook._check_1d(xy[0])
        y = cbook._check_1d(xy[1])
    else:
        x, y = cbook.index_of(xy[-1])
    if x.shape[0] != y.shape[0]:
        raise ValueError(f"x and y must have same first dimension, but "
                         f"have shapes {x.shape} and {y.shape}")
    if x.ndim > 2 or y.ndim > 2:
        raise ValueError(f"x and y can be no greater than 2D, but have "
                         f"shapes {x.shape} and {y.shape}")
    if x.ndim == 1:
        x = x[:, np.newaxis]
    if y.ndim == 1:
        y = y[:, np.newaxis]
    ncx, ncy = x.shape[1], y.shape[1]
    if ncx > 1 and ncy > 1 and ncx != ncy:
        raise ValueError(f"x has {ncx} columns but y has {ncy} columns")
    if ncx == 0 or ncy == 0:
        return
    label = kwargs.get('label')
    n_datasets = max(ncx, ncy)
    if cbook.is_scalar_or_string(label):
        labels = [label] * n_datasets
    elif len(label) == n_datasets:
        labels = label
    else:
        raise ValueError(
            f"label must be scalar or have the same length as the input "
            f"data, but found {len(label)} for {n_datasets} datasets.")
    for j, label in enumerate(labels):
        yield x[:, j % ncx], y[:, j % ncy], fmt, label


class LinesMixin:
    """`plot`, the reference lines and spans, `hlines` and `vlines`."""

    def plot(self, *args, scalex=True, scaley=True, data=None, **kwargs):
        """
        Plot y versus x as lines and/or markers.

        Call signatures::

            plot([x], y, [fmt], *, data=None, **kwargs)
            plot([x], y, [fmt], [x2], y2, [fmt2], ..., **kwargs)

        Returns
        -------
        list of `.Line2D`
            A list of lines representing the plotted data.
        """
        if not (scalex and scaley):
            raise NotImplementedError("pltrs: plot(scalex=False, scaley=False)")
        kwargs = cbook.normalize_kwargs(kwargs, mlines.Line2D)
        for pos_only in "xy":
            if pos_only in kwargs:
                raise _api.kwarg_error(inspect.stack()[1].function, pos_only)
        if not args:
            return []
        lines = []
        for x, y, fmt, label in list(_plot_datasets(list(args), data, kwargs)):
            props = {k: v for k, v in kwargs.items() if v is not None}
            if label is not None:
                props["label"] = label
            else:
                props.pop("label", None)
            native = _plotting.plot(self._native, axis_values(x), axis_values(y),
                                    fmt, mlines._native_kwargs(props))
            lines.append(wrap(self, native, mlines.Line2D))
        return lines

    def axhline(self, y=0, xmin=0, xmax=1, **kwargs):
        """
        Add a horizontal line spanning the whole or fraction of the Axes.

        Returns
        -------
        `~matplotlib.lines.Line2D`
        """
        _check_no_units([xmin, xmax], ['xmin', 'xmax'])
        if "transform" in kwargs:
            raise ValueError("'transform' is not allowed as a keyword "
                             "argument; axhline generates its own transform.")
        native = _axes.axline(self._native, "h", y, (xmin, xmax),
                              line_kwargs(kwargs))
        return wrap(self, native, mlines.Line2D)

    def axvline(self, x=0, ymin=0, ymax=1, **kwargs):
        """
        Add a vertical line spanning the whole or fraction of the Axes.

        Returns
        -------
        `~matplotlib.lines.Line2D`
        """
        _check_no_units([ymin, ymax], ['ymin', 'ymax'])
        if "transform" in kwargs:
            raise ValueError("'transform' is not allowed as a keyword "
                             "argument; axvline generates its own transform.")
        native = _axes.axline(self._native, "v", x, (ymin, ymax),
                              line_kwargs(kwargs))
        return wrap(self, native, mlines.Line2D)

    def axline(self, xy1, xy2=None, *, slope=None, **kwargs):
        """
        Add an infinitely long straight line through *xy1* and *xy2*, or
        through *xy1* with the given *slope*.

        Returns
        -------
        `.Line2D`
        """
        if slope is not None and (self.get_xscale() != 'linear' or
                                  self.get_yscale() != 'linear'):
            raise TypeError("'slope' cannot be used with non-linear scales")
        if (xy2 is None) == (slope is None):
            raise TypeError("Exactly one of 'xy2' and 'slope' must be given")
        if "transform" in kwargs:
            raise NotImplementedError("pltrs: axline(transform=...)")
        native = _plotting.axline(
            self._native, tuple(map(float, xy1)),
            None if xy2 is None else tuple(map(float, xy2)),
            None if slope is None else float(slope), line_kwargs(kwargs))
        return wrap(self, native, mlines.Line2D)

    def axhspan(self, ymin, ymax, xmin=0, xmax=1, **kwargs):
        """
        Add a horizontal span (rectangle) across the Axes.

        Returns
        -------
        `~matplotlib.patches.Rectangle`
            Horizontal span (rectangle) from (xmin, ymin) to (xmax, ymax).
        """
        _check_no_units([xmin, xmax], ['xmin', 'xmax'])
        native = _axes.axspan(self._native, "h", (ymin, ymax), (xmin, xmax))
        patch = wrap(self, native, Rectangle)
        patch._internal_update(cbook.normalize_kwargs(kwargs, Rectangle))
        return patch

    def axvspan(self, xmin, xmax, ymin=0, ymax=1, **kwargs):
        """
        Add a vertical span (rectangle) across the Axes.

        Returns
        -------
        `~matplotlib.patches.Rectangle`
            Vertical span (rectangle) from (xmin, ymin) to (xmax, ymax).
        """
        _check_no_units([ymin, ymax], ['ymin', 'ymax'])
        native = _axes.axspan(self._native, "v", (xmin, xmax), (ymin, ymax))
        patch = wrap(self, native, Rectangle)
        patch._internal_update(cbook.normalize_kwargs(kwargs, Rectangle))
        return patch

    def _segments(self, vertical, positions, low, high, colors, linestyles,
                  label, kwargs):
        """The segment collection of `hlines` (*vertical* false) or `vlines`."""
        if not np.iterable(positions):
            positions = [positions]
        if not np.iterable(low):
            low = [low]
        if not np.iterable(high):
            high = [high]
        positions, low, high = cbook._combine_masks(positions, low, high)
        native = _plotting.segments(
            self._native, vertical, axis_values(np.ravel(positions)),
            axis_values(np.ravel(low)), axis_values(np.ravel(high)))
        lines = wrap(self, native, mcoll.LineCollection)
        props = {"linestyle": linestyles, "label": label}
        if colors is not None:
            props["color"] = colors
        lines._internal_update(props)
        lines._internal_update(cbook.normalize_kwargs(kwargs, mcoll.LineCollection))
        return lines

    @_preprocess_data(replace_names=["y", "xmin", "xmax", "colors"],
                      label_namer="y")
    def hlines(self, y, xmin, xmax, colors=None, linestyles='solid',
               label='', **kwargs):
        """
        Plot horizontal lines at each *y* from *xmin* to *xmax*.

        Returns
        -------
        `~matplotlib.collections.LineCollection`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        return self._segments(False, y, xmin, xmax, colors, linestyles, label,
                              kwargs)

    @_preprocess_data(replace_names=["x", "ymin", "ymax", "colors"],
                      label_namer="x")
    def vlines(self, x, ymin, ymax, colors=None, linestyles='solid',
               label='', **kwargs):
        """
        Plot vertical lines at each *x* from *ymin* to *ymax*.

        Returns
        -------
        `~matplotlib.collections.LineCollection`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        return self._segments(True, x, ymin, ymax, colors, linestyles, label,
                              kwargs)
