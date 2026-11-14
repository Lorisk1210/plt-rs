"""
Lines: `~.Axes.plot`, the reference lines and spans (`~.Axes.axhline`,
`~.Axes.axvline`, `~.Axes.axline`, `~.Axes.axhspan`, `~.Axes.axvspan`)
and the segment families `~.Axes.hlines` and `~.Axes.vlines`.
"""

import re

import numpy as np

from ... import _docstring
from ... import _api, _live_transforms, _values, cbook, collections as mcoll, colors as mcolors
from ... import lines as mlines
from ... import _python_drawing, _transform_trees
from .. import _failed_calls
from ..._artist_kwargs import check_known, keep_python_kwargs
from ..._native import axes as _axes, patches as _patches, plotting as _plotting
from ..._preprocess import _label_from_arg, _preprocess_data, _replacer
from ...patches import Rectangle
from ._helpers import converted_line_data, floats, keep_data, made_line_kwargs, wrap


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


def _plot_datasets(args, data, kwargs, axes=None):
    """
    Split ``plot``'s positional arguments into ``(x, y, fmt, label)``
    datasets, as matplotlib's ``_process_plot_var_args`` does: each group
    is ``[x], y, [fmt]``, and 2D *x* or *y* hold one dataset per column.
    The axis of *axes* take the units of each group's data before its
    shapes are checked, as matplotlib's do.
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
        yield from _group_datasets(this, kwargs, ambiguous_fmt_datakey, axes)


def _group_datasets(tup, kwargs, ambiguous_fmt_datakey, axes):
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
    if axes is not None:
        axes.xaxis.update_units(x)
        axes.yaxis.update_units(y)
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

    @_docstring.interpd
    def plot(self, *args, scalex=True, scaley=True, data=None, **kwargs):
        """
        Plot y versus x as lines and/or markers.

        Call signatures::

            plot([x], y, [fmt], *, data=None, **kwargs)
            plot([x], y, [fmt], [x2], y2, [fmt2], ..., **kwargs)

        The coordinates of the points or line nodes are given by *x*, *y*.

        The optional parameter *fmt* is a convenient way for defining basic
        formatting like color, marker and linestyle. It's a shortcut string
        notation described in the *Notes* section below.

        >>> plot(x, y)        # plot x and y using default line style and color
        >>> plot(x, y, 'bo')  # plot x and y using blue circle markers
        >>> plot(y)           # plot y using x as index array 0..N-1
        >>> plot(y, 'r+')     # ditto, but with red plusses

        You can use `.Line2D` properties as keyword arguments for more
        control on the appearance. Line properties and *fmt* can be mixed.
        The following two calls yield identical results:

        >>> plot(x, y, 'go--', linewidth=2, markersize=12)
        >>> plot(x, y, color='green', marker='o', linestyle='dashed',
        ...      linewidth=2, markersize=12)

        When conflicting with *fmt*, keyword arguments take precedence.


        **Plotting labelled data**

        There's a convenient way for plotting objects with labelled data (i.e.
        data that can be accessed by index ``obj['y']``). Instead of giving
        the data in *x* and *y*, you can provide the object in the *data*
        parameter and just give the labels for *x* and *y*::

        >>> plot('xlabel', 'ylabel', data=obj)

        All indexable objects are supported. This could e.g. be a `dict`, a
        `pandas.DataFrame` or a structured numpy array.


        **Plotting multiple sets of data**

        There are various ways to plot multiple sets of data.

        - The most straight forward way is just to call `plot` multiple times.
          Example:

          >>> plot(x1, y1, 'bo')
          >>> plot(x2, y2, 'go')

        - If *x* and/or *y* are 2D arrays, a separate data set will be drawn
          for every column. If both *x* and *y* are 2D, they must have the
          same shape. If only one of them is 2D with shape (N, m) the other
          must have length N and will be used for every data set m.

          Example:

          >>> x = [1, 2, 3]
          >>> y = np.array([[1, 2], [3, 4], [5, 6]])
          >>> plot(x, y)

          is equivalent to:

          >>> for col in range(y.shape[1]):
          ...     plot(x, y[:, col])

        - The third way is to specify multiple sets of *[x]*, *y*, *[fmt]*
          groups::

          >>> plot(x1, y1, 'g^', x2, y2, 'g-')

          In this case, any additional keyword argument applies to all
          datasets. Also, this syntax cannot be combined with the *data*
          parameter.

        By default, each line is assigned a different style specified by a
        'style cycle'. The *fmt* and line property parameters are only
        necessary if you want explicit deviations from these defaults.
        Alternatively, you can also change the style cycle using
        :rc:`axes.prop_cycle`.


        Parameters
        ----------
        x, y : array-like or float
            The horizontal / vertical coordinates of the data points.
            *x* values are optional and default to ``range(len(y))``.

            Commonly, these parameters are 1D arrays.

            They can also be scalars, or two-dimensional (in that case, the
            columns represent separate data sets).

            These arguments cannot be passed as keywords.

        fmt : str, optional
            A format string, e.g. 'ro' for red circles. See the *Notes*
            section for a full description of the format strings.

            Format strings are just an abbreviation for quickly setting
            basic line properties. All of these and more can also be
            controlled by keyword arguments.

            This argument cannot be passed as keyword.

        data : indexable object, optional
            An object with labelled data. If given, provide the label names to
            plot in *x* and *y*.

            .. note::
                Technically there's a slight ambiguity in calls where the
                second label is a valid *fmt*. ``plot('n', 'o', data=obj)``
                could be ``plt(x, y)`` or ``plt(y, fmt)``. In such cases,
                the former interpretation is chosen, but a warning is issued.
                You may suppress the warning by adding an empty format string
                ``plot('n', 'o', '', data=obj)``.

        Returns
        -------
        list of `.Line2D`
            A list of lines representing the plotted data.

        Other Parameters
        ----------------
        scalex, scaley : bool, default: True
            These parameters determine if the view limits are adapted to the
            data limits. The values are passed on to
            `~.axes.Axes.autoscale_view`.

        **kwargs : `~matplotlib.lines.Line2D` properties, optional
            *kwargs* are used to specify properties like a line label (for
            auto legends), linewidth, antialiasing, marker face color.
            Example::

            >>> plot([1, 2, 3], [1, 2, 3], 'go-', label='line 1', linewidth=2)
            >>> plot([1, 2, 3], [1, 4, 9], 'rs', label='line 2')

            If you specify multiple lines with one plot call, the kwargs apply
            to all those lines. In case the label object is iterable, each
            element is used as labels for each set of data.

            Here is a list of available `.Line2D` properties:

            %(Line2D:kwdoc)s

        See Also
        --------
        scatter : XY scatter plot with markers of varying size and/or color (
            sometimes also called bubble chart).

        Notes
        -----
        **Format Strings**

        A format string consists of a part for color, marker and line::

            fmt = '[marker][line][color]'

        Each of them is optional. If not provided, the value from the style
        cycle is used. Exception: If ``line`` is given, but no ``marker``,
        the data will be a line without markers.

        Other combinations such as ``[color][marker][line]`` are also
        supported, but note that their parsing may be ambiguous.

        **Markers**

        =============   ===============================
        character       description
        =============   ===============================
        ``'.'``         point marker
        ``','``         pixel marker
        ``'o'``         circle marker
        ``'v'``         triangle_down marker
        ``'^'``         triangle_up marker
        ``'<'``         triangle_left marker
        ``'>'``         triangle_right marker
        ``'1'``         tri_down marker
        ``'2'``         tri_up marker
        ``'3'``         tri_left marker
        ``'4'``         tri_right marker
        ``'8'``         octagon marker
        ``'s'``         square marker
        ``'p'``         pentagon marker
        ``'P'``         plus (filled) marker
        ``'*'``         star marker
        ``'h'``         hexagon1 marker
        ``'H'``         hexagon2 marker
        ``'+'``         plus marker
        ``'x'``         x marker
        ``'X'``         x (filled) marker
        ``'D'``         diamond marker
        ``'d'``         thin_diamond marker
        ``'|'``         vline marker
        ``'_'``         hline marker
        =============   ===============================

        **Line Styles**

        =============    ===============================
        character        description
        =============    ===============================
        ``'-'``          solid line style
        ``'--'``         dashed line style
        ``'-.'``         dash-dot line style
        ``':'``          dotted line style
        =============    ===============================

        Example format strings::

            'b'    # blue markers with default shape
            'or'   # red circles
            '-g'   # green solid line
            '--'   # dashed line with default color
            '^k:'  # black triangle_up markers connected by a dotted line

        **Colors**

        The supported color abbreviations are the single letter codes

        =============    ===============================
        character        color
        =============    ===============================
        ``'b'``          blue
        ``'g'``          green
        ``'r'``          red
        ``'c'``          cyan
        ``'m'``          magenta
        ``'y'``          yellow
        ``'k'``          black
        ``'w'``          white
        =============    ===============================

        and the ``'CN'`` colors that index into the default property cycle.

        If the color is the only part of the format string, you can
        additionally use any  `matplotlib.colors` spec, e.g. full names
        (``'green'``) or hex strings (``'#008000'``).
        """
        kwargs = cbook.normalize_kwargs(kwargs, mlines.Line2D)
        for pos_only in "xy":
            if pos_only in kwargs:
                # matplotlib's names the method calling its line maker.
                raise _api.kwarg_error("plot", pos_only)
        self._process_unit_info(kwargs=kwargs)
        if not args:
            return []
        lines = []
        for x, y, fmt, label in list(_plot_datasets(list(args), data, kwargs, self)):
            props = {k: v for k, v in kwargs.items() if v is not None}
            if "transform" in props:
                props["transform"] = _live_transforms.for_axes(
                    props["transform"], self)
            if label is not None:
                props["label"] = label
            else:
                props.pop("label", None)
            data = converted_line_data(self, x, y)
            if not fmt and "marker" not in props:
                mlines._check_rc_marker("lines.marker")
            try:
                native = _plotting.plot(self._native, *data, fmt, mlines._native_kwargs(props),
                                        (bool(scalex), bool(scaley)))
            except AttributeError:
                if props.get("color") is None and (not fmt or _format_parts(fmt)[2] is None):
                    # matplotlib's line takes its colour from the cycle
                    # before it refuses a property.
                    self._get_lines.get_next_color()
                raise
            line = wrap(self, native, mlines.Line2D)
            if "transform" in props:
                # The line keeps the transform given, as matplotlib's
                # constructor sets it, and follows it where pltrs (Rust)
                # took parts of it by their matrix.
                line._given_transform = props["transform"]
                _live_transforms.watch_given(line, props["transform"])
            whole = _format_color(fmt)
            mlines._remember_given(
                line, props if whole is None else {"color": whole, **props})
            lines.append(keep_data(keep_python_kwargs(line, props), x, y))
        return lines

    @_docstring.interpd
    def axhline(self, y=0, xmin=0, xmax=1, **kwargs):
        """
        Add a horizontal line spanning the whole or fraction of the Axes.

        Note: If you want to set x-limits in data coordinates, use
        `~.Axes.hlines` instead.

        Parameters
        ----------
        y : float, default: 0
            y position in :ref:`data coordinates <coordinate-systems>`.

        xmin : float, default: 0
            The start x-position in :ref:`axes coordinates <coordinate-systems>`.
            Should be between 0 and 1, 0 being the far left of the plot,
            1 the far right of the plot.

        xmax : float, default: 1
            The end x-position in :ref:`axes coordinates <coordinate-systems>`.
            Should be between 0 and 1, 0 being the far left of the plot,
            1 the far right of the plot.

        Returns
        -------
        `~matplotlib.lines.Line2D`
            A `.Line2D` specified via two points ``(xmin, y)``, ``(xmax, y)``.
            Its transform is set such that *x* is in
            :ref:`axes coordinates <coordinate-systems>` and *y* is in
            :ref:`data coordinates <coordinate-systems>`.

            This is still a generic line and the horizontal character is only
            realized through using identical *y* values for both points. Thus,
            if you want to change the *y* value later, you have to provide two
            values ``line.set_ydata([3, 3])``.

        Other Parameters
        ----------------
        **kwargs
            Valid keyword arguments are `.Line2D` properties, except for
            'transform':

            %(Line2D:kwdoc)s

        See Also
        --------
        hlines : Add horizontal lines in data coordinates.
        axhspan : Add a horizontal span (rectangle) across the axis.
        axline : Add a line with an arbitrary slope.

        Examples
        --------
        * draw a thick red hline at 'y' = 0 that spans the xrange::

            >>> axhline(linewidth=4, color='r')

        * draw a default hline at 'y' = 1 that spans the xrange::

            >>> axhline(y=1)

        * draw a default hline at 'y' = .5 that spans the middle half of
          the xrange::

            >>> axhline(y=.5, xmin=0.25, xmax=0.75)
        """
        _check_no_units([xmin, xmax], ['xmin', 'xmax'])
        if "transform" in kwargs:
            raise ValueError("'transform' is not allowed as a keyword "
                             "argument; axhline generates its own transform.")
        yy, = self._process_unit_info([("y", y)], kwargs)
        # matplotlib compares the position with the bounds (to autoscale),
        # which is where positions that are no numbers fail.
        ymin, ymax = self.get_ybound()
        _ = (yy < ymin) or (yy > ymax)
        native = _axes.axline(self._native, "h", float(yy), (xmin, xmax),
                              made_line_kwargs(kwargs))
        line = keep_python_kwargs(wrap(self, native, mlines.Line2D), kwargs)
        # matplotlib's line keeps the data it was made with.
        line._xorig, line._yorig = [xmin, xmax], [y, y]
        return line

    @_docstring.interpd
    def axvline(self, x=0, ymin=0, ymax=1, **kwargs):
        """
        Add a vertical line spanning the whole or fraction of the Axes.

        Note: If you want to set y-limits in data coordinates, use
        `~.Axes.vlines` instead.

        Parameters
        ----------
        x : float, default: 0
            x position in :ref:`data coordinates <coordinate-systems>`.

        ymin : float, default: 0
            The start y-position in :ref:`axes coordinates <coordinate-systems>`.
            Should be between 0 and 1, 0 being the bottom of the plot, 1 the
            top of the plot.

        ymax : float, default: 1
            The end y-position in :ref:`axes coordinates <coordinate-systems>`.
            Should be between 0 and 1, 0 being the bottom of the plot, 1 the
            top of the plot.

        Returns
        -------
        `~matplotlib.lines.Line2D`
            A `.Line2D` specified via two points ``(x, ymin)``, ``(x, ymax)``.
            Its transform is set such that *x* is in
            :ref:`data coordinates <coordinate-systems>` and *y* is in
            :ref:`axes coordinates <coordinate-systems>`.

            This is still a generic line and the vertical character is only
            realized through using identical *x* values for both points. Thus,
            if you want to change the *x* value later, you have to provide two
            values ``line.set_xdata([3, 3])``.

        Other Parameters
        ----------------
        **kwargs
            Valid keyword arguments are `.Line2D` properties, except for
            'transform':

            %(Line2D:kwdoc)s

        See Also
        --------
        vlines : Add vertical lines in data coordinates.
        axvspan : Add a vertical span (rectangle) across the axis.
        axline : Add a line with an arbitrary slope.

        Examples
        --------
        * draw a thick red vline at *x* = 0 that spans the yrange::

            >>> axvline(linewidth=4, color='r')

        * draw a default vline at *x* = 1 that spans the yrange::

            >>> axvline(x=1)

        * draw a default vline at *x* = .5 that spans the middle half of
          the yrange::

            >>> axvline(x=.5, ymin=0.25, ymax=0.75)
        """
        _check_no_units([ymin, ymax], ['ymin', 'ymax'])
        if "transform" in kwargs:
            raise ValueError("'transform' is not allowed as a keyword "
                             "argument; axvline generates its own transform.")
        xx, = self._process_unit_info([("x", x)], kwargs)
        # matplotlib compares the position with the bounds (to autoscale),
        # which is where positions that are no numbers fail.
        xmin, xmax = self.get_xbound()
        _ = (xx < xmin) or (xx > xmax)
        native = _axes.axline(self._native, "v", float(xx), (ymin, ymax),
                              made_line_kwargs(kwargs))
        line = keep_python_kwargs(wrap(self, native, mlines.Line2D), kwargs)
        line._xorig, line._yorig = [x, x], [ymin, ymax]
        return line

    @_docstring.interpd
    def axline(self, xy1, xy2=None, *, slope=None, **kwargs):
        """
        Add an infinitely long straight line.

        The line can be defined either by two points *xy1* and *xy2*, or
        by one point *xy1* and a *slope*.

        This draws a straight line "on the screen", regardless of the x and y
        scales, and is thus also suitable for drawing exponential decays in
        semilog plots, power laws in loglog plots, etc. However, *slope*
        should only be used with linear scales; It has no clear meaning for
        all other scales, and thus the behavior is undefined. Please specify
        the line using the points *xy1*, *xy2* for non-linear scales.

        The *transform* keyword argument only applies to the points *xy1*,
        *xy2*. The *slope* (if given) is always in data coordinates. This can
        be used e.g. with ``ax.transAxes`` for drawing grid lines with a fixed
        slope.

        Parameters
        ----------
        xy1, xy2 : (float, float)
            Points for the line to pass through.
            Either *xy2* or *slope* has to be given.
        slope : float, optional
            The slope of the line. Either *xy2* or *slope* has to be given.

        Returns
        -------
        `.AxLine`

        Other Parameters
        ----------------
        **kwargs
            Valid kwargs are `.Line2D` properties

            %(Line2D:kwdoc)s

        See Also
        --------
        axhline : for horizontal lines
        axvline : for vertical lines

        Examples
        --------
        Draw a thick red line passing through (0, 0) and (1, 1)::

            >>> axline((0, 0), (1, 1), linewidth=4, color='r')
        """
        if slope is not None and (self.get_xscale() != 'linear' or
                                  self.get_yscale() != 'linear'):
            raise TypeError("'slope' cannot be used with non-linear scales")
        datalim = [xy1] if xy2 is None else [xy1, xy2]
        if "transform" in kwargs:
            # if a transform is passed (i.e. line points not in data space),
            # data limits should not be adjusted.
            datalim = []
        # matplotlib's `AxLine` sets its properties, then checks its
        # geometry.
        check_known(mlines.AxLine, kwargs)
        if (xy2 is None) == (slope is None):
            raise TypeError("Exactly one of 'xy2' and 'slope' must be given")
        if mlines._axline_geometry(xy1, xy2, slope) is None:
            # matplotlib adds the line, then fails taking its points into
            # the data limits; the line fails where its draws place it.
            line = mlines.AxLine(xy1, xy2, slope, **kwargs)._add_to_axes(self, "line")
            _failed_calls.keep_added(self)
            self.update_datalim(datalim)
            self._request_autoscale_view()
            return line
        native = _plotting.axline(
            self._native, tuple(map(float, xy1)),
            None if xy2 is None else tuple(map(float, xy2)),
            None if slope is None else float(slope), made_line_kwargs(kwargs))
        line = wrap(self, native, mlines.AxLine)
        line._remember_geometry(xy1, xy2, slope)
        if not hasattr(self, "transLimits"):
            # matplotlib's line maps the view through the Axes'
            # ``transLimits``, which Axes of other projections lack (polar
            # ones): its draw fails there.
            _python_drawing.draw_through_python(line, False)
        return keep_python_kwargs(line, kwargs)

    @_docstring.interpd
    def axhspan(self, ymin, ymax, xmin=0, xmax=1, **kwargs):
        """
        Add a horizontal span (rectangle) across the Axes.

        The rectangle spans from *ymin* to *ymax* vertically, and, by default,
        the whole x-axis horizontally.  The x-span can be set using *xmin*
        (default: 0) and *xmax* (default: 1) which are in axis units; e.g.
        ``xmin = 0.5`` always refers to the middle of the x-axis regardless of
        the limits set by `~.Axes.set_xlim`.

        Parameters
        ----------
        ymin : float
            Lower y-coordinate of the span, in data units.
        ymax : float
            Upper y-coordinate of the span, in data units.
        xmin : float, default: 0
            Lower x-coordinate of the span, in x-axis (0-1) units.
        xmax : float, default: 1
            Upper x-coordinate of the span, in x-axis (0-1) units.

        Returns
        -------
        `~matplotlib.patches.Rectangle`
            Horizontal span (rectangle) from (xmin, ymin) to (xmax, ymax).

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.patches.Rectangle` properties

        %(Rectangle:kwdoc)s

        See Also
        --------
        axvspan : Add a vertical span across the Axes.
        """
        _check_no_units([xmin, xmax], ['xmin', 'xmax'])
        (ymin, ymax), = self._process_unit_info([("y", [ymin, ymax])], kwargs)
        # matplotlib's rectangle is made with the extents of the span.
        width, height = xmax - xmin, ymax - ymin
        native = _axes.axspan(self._native, "h", (ymin, ymax), (xmin, xmax))
        patch = wrap(self, native, Rectangle)
        # Made without a colour, its face is :rc:`patch.facecolor`.
        _values.remember(patch, "facecolor", None)
        _span_geometry(patch, xmin, ymin, width, height)
        patch._internal_update(cbook.normalize_kwargs(kwargs, Rectangle))
        _share_span_steps()
        _transform_trees.patches_added(self, [patch])
        return patch

    @_docstring.interpd
    def axvspan(self, xmin, xmax, ymin=0, ymax=1, **kwargs):
        """
        Add a vertical span (rectangle) across the Axes.

        The rectangle spans from *xmin* to *xmax* horizontally, and, by
        default, the whole y-axis vertically.  The y-span can be set using
        *ymin* (default: 0) and *ymax* (default: 1) which are in axis units;
        e.g. ``ymin = 0.5`` always refers to the middle of the y-axis
        regardless of the limits set by `~.Axes.set_ylim`.

        Parameters
        ----------
        xmin : float
            Lower x-coordinate of the span, in data units.
        xmax : float
            Upper x-coordinate of the span, in data units.
        ymin : float, default: 0
            Lower y-coordinate of the span, in y-axis units (0-1).
        ymax : float, default: 1
            Upper y-coordinate of the span, in y-axis units (0-1).

        Returns
        -------
        `~matplotlib.patches.Rectangle`
            Vertical span (rectangle) from (xmin, ymin) to (xmax, ymax).

        Other Parameters
        ----------------
        **kwargs : `~matplotlib.patches.Rectangle` properties

        %(Rectangle:kwdoc)s

        See Also
        --------
        axhspan : Add a horizontal span across the Axes.

        Examples
        --------
        Draw a vertical, green, translucent rectangle from x = 1.25 to
        x = 1.55 that spans the yrange of the Axes.

        >>> axvspan(1.25, 1.55, facecolor='g', alpha=0.5)

        """
        _check_no_units([ymin, ymax], ['ymin', 'ymax'])
        (xmin, xmax), = self._process_unit_info([("x", [xmin, xmax])], kwargs)
        # matplotlib's rectangle is made with the extents of the span.
        width, height = xmax - xmin, ymax - ymin
        native = _axes.axspan(self._native, "v", (xmin, xmax), (ymin, ymax))
        patch = wrap(self, native, Rectangle)
        # Made without a colour, its face is :rc:`patch.facecolor`.
        _values.remember(patch, "facecolor", None)
        _span_geometry(patch, xmin, ymin, width, height)
        patch._internal_update(cbook.normalize_kwargs(kwargs, Rectangle))
        _share_span_steps()
        _transform_trees.patches_added(self, [patch])
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
        # matplotlib makes the collection with the line styles and colours
        # given, which it refuses before it adds the collection.
        mcoll._checked_linestyles(linestyles)
        if colors is not None:
            mcolors.to_rgba_array(colors)
        transform = kwargs.pop("transform", None)
        native = _plotting.segments(
            self._native, vertical,
            tuple(floats(np.ravel(v)) for v in (positions, low, high)),
            _live_transforms.given_to_native(transform, self))
        lines = wrap(self, native, mcoll.LineCollection)
        if transform is not None:
            lines.set_transform(transform)
        props = {"linestyle": linestyles}
        if label is not None and str(label):
            # (matplotlib's `add_collection` labels one without a label
            # `_child<n>`, as pltrs (Rust) did.)
            props["label"] = label
        if colors is not None:
            props["color"] = colors
        lines._internal_update(props)
        lines._internal_update(kwargs)
        return lines

    @_api.make_keyword_only("3.10", "label")
    @_preprocess_data(replace_names=["y", "xmin", "xmax", "colors"],
                      label_namer="y")
    def hlines(self, y, xmin, xmax, colors=None, linestyles='solid',
               label='', **kwargs):
        """
        Plot horizontal lines at each *y* from *xmin* to *xmax*.

        Parameters
        ----------
        y : float or array-like
            y-indexes where to plot the lines.

        xmin, xmax : float or array-like
            Respective beginning and end of each line. If scalars are
            provided, all lines will have the same length.

        colors : :mpltype:`color` or list of color , default: :rc:`lines.color`

        linestyles : {'solid', 'dashed', 'dashdot', 'dotted'}, default: 'solid'

        label : str, default: ''

        Returns
        -------
        `~matplotlib.collections.LineCollection`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        **kwargs :  `~matplotlib.collections.LineCollection` properties.

        See Also
        --------
        vlines : vertical lines
        axhline : horizontal line across the Axes
        """
        return self._segments(False, y, xmin, xmax, colors, linestyles, label,
                              kwargs)

    @_api.make_keyword_only("3.10", "label")
    @_preprocess_data(replace_names=["x", "ymin", "ymax", "colors"],
                      label_namer="x")
    def vlines(self, x, ymin, ymax, colors=None, linestyles='solid',
               label='', **kwargs):
        """
        Plot vertical lines at each *x* from *ymin* to *ymax*.

        Parameters
        ----------
        x : float or array-like
            x-indexes where to plot the lines.

        ymin, ymax : float or array-like
            Respective beginning and end of each line. If scalars are
            provided, all lines will have the same length.

        colors : :mpltype:`color` or list of color, default: :rc:`lines.color`

        linestyles : {'solid', 'dashed', 'dashdot', 'dotted'}, default: 'solid'

        label : str, default: ''

        Returns
        -------
        `~matplotlib.collections.LineCollection`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        **kwargs : `~matplotlib.collections.LineCollection` properties.

        See Also
        --------
        hlines : horizontal lines
        axvline : vertical line across the Axes
        """
        return self._segments(True, x, ymin, ymax, colors, linestyles, label,
                              kwargs)
