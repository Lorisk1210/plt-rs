"""
Lines: `~.Axes.plot`, the reference lines and spans (`~.Axes.axhline`,
`~.Axes.axvline`, `~.Axes.axline`, `~.Axes.axhspan`, `~.Axes.axvspan`)
and the segment families `~.Axes.hlines` and `~.Axes.vlines`.
"""

import inspect
import re

import numpy as np

from ... import _api, _live_transforms, _values, cbook, collections as mcoll, lines as mlines
from ..._artist_kwargs import keep_program_picker
from ..._native import axes as _axes, patches as _patches, plotting as _plotting
from ..._preprocess import _label_from_arg, _preprocess_data, _replacer
from ...patches import Rectangle
from ._helpers import floats, keep_unit_data, line_data, line_kwargs, wrap


def _span_geometry(patch, x, y, width, height):
    """Keep the geometry of the span *patch* as matplotlib's rectangle keeps
    it: its corner, width and height computed from the arguments as given."""
    _values.remember_all(patch, {"x": x, "y": y, "width": width, "height": height},
                         Rectangle._FIELDS)


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


def _format_color(fmt):
    """
    The colour of a format string as matplotlib's ``_process_plot_format``
    keeps it: ``to_rgba`` of a format that is a colour as a whole, or of the
    ``"Cn"`` in it; None for the others (``'g--'`` is ``'g'``, as pltrs
    (Rust) keeps it) and for none.
    """
    if not fmt:
        return None
    from ... import colors as mcolors
    try:
        color = mcolors.to_rgba(fmt)
    except ValueError:
        pass
    else:
        try:
            int(fmt)
        except ValueError:
            return color
        return None  # '1' is the tri_down marker, not a grey level.
    cycle = re.search(r"C\d+", fmt)
    return None if cycle is None else mcolors.to_rgba(cycle[0])


def _given_format_color(fmt):
    """The colour of the format string *fmt* as `plot` is given it from
    matplotlib's ``_process_plot_format`` (see `_format_color`), None for a
    format without one."""
    color = _format_parts(fmt)[2]
    return None if color is None else _format_color(fmt) or color


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


def _share_span_steps():
    """matplotlib's ``p.get_path()._interpolation_steps =
    GRIDLINE_INTERPOLATION_STEPS`` for a span, through the path all
    rectangles share."""
    from ...axis import GRIDLINE_INTERPOLATION_STEPS
    _patches.set_shared_rectangle_steps(GRIDLINE_INTERPOLATION_STEPS)


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
        kwargs = cbook.normalize_kwargs(kwargs, mlines.Line2D)
        for pos_only in "xy":
            if pos_only in kwargs:
                raise _api.kwarg_error(inspect.stack()[1].function, pos_only)
        self._process_unit_info(kwargs=kwargs)
        if not args:
            return []
        lines = []
        for x, y, fmt, label in list(_plot_datasets(list(args), data, kwargs)):
            props = {k: v for k, v in kwargs.items() if v is not None}
            if "transform" in props:
                props["transform"] = _live_transforms.for_axes(
                    props["transform"], self)
            if label is not None:
                props["label"] = label
            else:
                props.pop("label", None)
            native = _plotting.plot(self._native, *line_data(self, x, y), fmt,
                                    mlines._native_kwargs(props),
                                    (bool(scalex), bool(scaley)))
            line = keep_program_picker(wrap(self, native, mlines.Line2D), props)
            whole = _format_color(fmt)
            mlines._remember_given(
                line, props if whole is None else {"color": whole, **props})
            lines.append(keep_unit_data(line, x, y))
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
        yy, = self._process_unit_info([("y", y)], kwargs)
        native = _axes.axline(self._native, "h", float(yy), (xmin, xmax),
                              line_kwargs(kwargs))
        line = keep_program_picker(wrap(self, native, mlines.Line2D), kwargs)
        # matplotlib's line keeps the data it was made with.
        line._xorig, line._yorig = [xmin, xmax], [y, y]
        return line

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
        xx, = self._process_unit_info([("x", x)], kwargs)
        native = _axes.axline(self._native, "v", float(xx), (ymin, ymax),
                              line_kwargs(kwargs))
        line = keep_program_picker(wrap(self, native, mlines.Line2D), kwargs)
        line._xorig, line._yorig = [x, x], [ymin, ymax]
        return line

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
        native = _plotting.axline(
            self._native, tuple(map(float, xy1)),
            None if xy2 is None else tuple(map(float, xy2)),
            None if slope is None else float(slope), line_kwargs(kwargs))
        line = wrap(self, native, mlines.AxLine)
        line._remember_geometry(xy1, xy2, slope)
        return keep_program_picker(line, kwargs)

    def axhspan(self, ymin, ymax, xmin=0, xmax=1, **kwargs):
        """
        Add a horizontal span (rectangle) across the Axes.

        Returns
        -------
        `~matplotlib.patches.Rectangle`
            Horizontal span (rectangle) from (xmin, ymin) to (xmax, ymax).
        """
        _check_no_units([xmin, xmax], ['xmin', 'xmax'])
        (ymin, ymax), = self._process_unit_info([("y", [ymin, ymax])], kwargs)
        native = _axes.axspan(self._native, "h", (ymin, ymax), (xmin, xmax))
        patch = wrap(self, native, Rectangle)
        # Made without a colour, its face is :rc:`patch.facecolor`.
        _values.remember(patch, "facecolor", None)
        _span_geometry(patch, xmin, ymin, xmax - xmin, ymax - ymin)
        patch._internal_update(cbook.normalize_kwargs(kwargs, Rectangle))
        _share_span_steps()
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
        (xmin, xmax), = self._process_unit_info([("x", [xmin, xmax])], kwargs)
        native = _axes.axspan(self._native, "v", (xmin, xmax), (ymin, ymax))
        patch = wrap(self, native, Rectangle)
        # Made without a colour, its face is :rc:`patch.facecolor`.
        _values.remember(patch, "facecolor", None)
        _span_geometry(patch, xmin, ymin, xmax - xmin, ymax - ymin)
        patch._internal_update(cbook.normalize_kwargs(kwargs, Rectangle))
        _share_span_steps()
        return patch

    def _segments(self, vertical, positions, low, high, colors, linestyles,
                  label, kwargs):
        """The segment collection of `hlines` (*vertical* false) or `vlines`."""
        # We do the conversion first since not all unitized data is uniform.
        if vertical:
            positions, low, high = self._process_unit_info(
                [("x", positions), ("y", low), ("y", high)], kwargs)
        else:
            low, high, positions = self._process_unit_info(
                [("x", low), ("x", high), ("y", positions)], kwargs)
        if not np.iterable(positions):
            positions = [positions]
        if not np.iterable(low):
            low = [low]
        if not np.iterable(high):
            high = [high]
        positions, low, high = cbook._combine_masks(positions, low, high)
        kwargs = cbook.normalize_kwargs(kwargs, mcoll.LineCollection)
        transform = kwargs.pop("transform", None)
        native = _plotting.segments(
            self._native, vertical,
            tuple(floats(np.ravel(v)) for v in (positions, low, high)),
            _live_transforms.given_to_native(transform))
        lines = wrap(self, native, mcoll.LineCollection)
        if transform is not None:
            lines.set_transform(transform)
        props = {"linestyle": linestyles, "label": label}
        if colors is not None:
            props["color"] = colors
        lines._internal_update(props)
        lines._internal_update(kwargs)
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
