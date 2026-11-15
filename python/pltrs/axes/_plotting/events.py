"""
Events and stems: `~.Axes.eventplot` and `~.Axes.stem`.
"""

from numbers import Real

import numpy as np

from ... import _docstring
from ... import _api, _values, artist as martist, cbook, collections as mcoll, colors as mcolors
from ... import lines as mlines
from ..._program_classes import takes_any_from_program
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ...container import StemContainer
from ... import _lazy_imports
from ._helpers import check_format, floats, keep_data, record_container, wrap, wrap_all
from .lines import _given_format_color

_events_reference = _lazy_imports.module(globals(), "._events_reference")


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from ... import rcParams
    return rcParams


def _per_row(name, values, n):
    """*values* (one or one per row) for each of the *n* rows, checked as
    matplotlib checks them."""
    if len(values) == 1:
        values = list(values) * n
    if len(values) != n:
        raise ValueError(f'{name} and positions are unequal sized sequences')
    return list(values)


def _remember_rows(collections, lineoffsets, linelengths, linewidths):
    """Keep the offsets, lengths and widths of the event *collections* as
    matplotlib types them: the numpy scalars of the arguments as arrays,
    one offset spreading to the cumulative sums."""
    n = len(collections)
    lineoffsets, linelengths, linewidths = (
        np.asarray(lineoffsets), np.asarray(linelengths), np.asarray(linewidths))
    if len(lineoffsets) == 1 and n != 1:
        lineoffsets = np.tile(lineoffsets, n)
        lineoffsets[0] = 0
        lineoffsets = np.cumsum(lineoffsets)
    rows = zip(collections, np.broadcast_to(lineoffsets, n),
               np.broadcast_to(linelengths, n), np.broadcast_to(linewidths, n))
    for collection, offset, length, width in rows:
        _values.remember_all(collection, {"lineoffset": offset, "linelength": length},
                             ("lineoffset", "linelength"))
        if width is not None:
            _values.remember(collection, "linewidth", np.atleast_1d(width))


def _native_rows(axes, positions, orientation, lineoffsets, linelengths, linewidths,
                 colors, alpha, linestyles, kwargs):
    """
    The rows of an `~.Axes.eventplot` of *axes* as pltrs (Rust) takes them
    (the arguments of ``_native.plotting.eventplot`` after the Axes), and
    the offsets, lengths and widths as given. Raises for arguments it does
    not take as they are, which matplotlib's code then plots (see
    `._events_reference`).
    """
    _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)
    lineoffsets, linelengths = axes._process_unit_info(
        [("y", lineoffsets), ("y", linelengths)], kwargs)
    if not np.iterable(positions):
        positions = [positions]
    elif any(np.iterable(position) for position in positions):
        positions = [np.asanyarray(position) for position in positions]
    else:
        positions = [np.asanyarray(positions)]
    positions = [axes._process_unit_info([("x", position)], kwargs)[0]
                 for position in positions]
    # Each row makes an `EventCollection`, which takes 1D positions.
    if any(np.ndim(position) != 1 for position in positions):
        raise ValueError('positions must be one-dimensional')
    positions = [floats(position) for position in positions]
    colors = cbook._local_over_kwdict(colors, kwargs, 'color')
    linewidths = cbook._local_over_kwdict(linewidths, kwargs, 'linewidth')
    linestyles = cbook._local_over_kwdict(linestyles, kwargs, 'linestyle')
    if not np.iterable(lineoffsets):
        lineoffsets = [lineoffsets]
    if not np.iterable(linelengths):
        linelengths = [linelengths]
    if not np.iterable(linewidths):
        linewidths = [linewidths]
    if not np.iterable(colors):
        colors = [colors]
    if not np.iterable(alpha):
        alpha = [alpha]
    if hasattr(linestyles, 'lower') or not np.iterable(linestyles):
        linestyles = [linestyles]
    for name, values in (("lineoffsets", lineoffsets),
                         ("linelengths", linelengths),
                         ("linestyles", linestyles),
                         ("linewidths", linewidths), ("alpha", alpha)):
        if len(values) == 0:
            raise ValueError(f'{name} cannot be empty')
    if len(colors) == 0:
        colors = [None]
    try:
        colors = mcolors.to_rgba_array(colors)
    except ValueError:
        pass
    n = len(positions)
    given = (lineoffsets, linelengths, linewidths)
    # pltrs (Rust) takes numbers, and the styles matplotlib's collections
    # take (each row's given as a list, as matplotlib wraps one style).
    for value in (*lineoffsets, *linelengths):
        if not isinstance(value, Real):
            raise TypeError(f"{value!r} is no number")
    for style in ([linestyles] if len(linestyles) == 1 else linestyles):
        mcoll._checked_linestyles(style)
    lineoffsets = np.asarray(lineoffsets, dtype=float)
    if len(lineoffsets) == 1 and n != 1:
        lineoffsets = np.tile(lineoffsets, n)
        lineoffsets[0] = 0
        lineoffsets = np.cumsum(lineoffsets)
    lineoffsets = _per_row("lineoffsets", list(lineoffsets), n)
    linelengths = _per_row("linelengths", [float(v) for v in linelengths], n)
    linewidths = _per_row("linewidths", list(linewidths), n)
    colors = _per_row("colors", list(colors), n)
    alpha = _per_row("alpha", list(alpha), n)
    for row_alpha in alpha:
        martist._check_array_alpha(row_alpha)
    linestyles = _per_row("linestyles", list(linestyles), n)
    native_colors = ([] if all(c is None for c in colors) else
                     [tuple(mcolors.to_rgba(_rcParams()['lines.color']
                                            if c is None else c))
                      for c in colors])
    rows = (positions, "horizontal" if orientation == "horizontal" else "vertical",
            ([float(v) for v in lineoffsets], linelengths,
             None if all(w is None for w in linewidths) else
             [float(_rcParams()['lines.linewidth'] if w is None else w)
              for w in linewidths],
             native_colors,
             None if all(a is None for a in alpha) else
             [1.0 if a is None else float(a) for a in alpha],
             [mcoll._native_style(ls) for ls in linestyles]))
    return rows, given


class EventsMixin:
    """`eventplot` and `stem`."""

    @_api.make_keyword_only("3.10", "orientation")
    @_preprocess_data(replace_names=["positions", "lineoffsets",
                                     "linelengths", "linewidths",
                                     "colors", "linestyles"])
    @_docstring.interpd
    def eventplot(self, positions, orientation='horizontal', lineoffsets=1,
                  linelengths=1, linewidths=None, colors=None, alpha=None,
                  linestyles='solid', **kwargs):
        """
        Plot identical parallel lines at the given positions.

        This type of plot is commonly used in neuroscience for representing
        neural events, where it is usually called a spike raster, dot raster,
        or raster plot.

        However, it is useful in any situation where you wish to show the
        timing or position of multiple sets of discrete events, such as the
        arrival times of people to a business on each day of the month or the
        date of hurricanes each year of the last century.

        Parameters
        ----------
        positions : array-like or list of array-like
            A 1D array-like defines the positions of one sequence of events.

            Multiple groups of events may be passed as a list of array-likes.
            Each group can be styled independently by passing lists of values
            to *lineoffsets*, *linelengths*, *linewidths*, *colors* and
            *linestyles*.

            Note that *positions* can be a 2D array, but in practice different
            event groups usually have different counts so that one will use a
            list of different-length arrays rather than a 2D array.

        orientation : {'horizontal', 'vertical'}, default: 'horizontal'
            The direction of the event sequence:

            - 'horizontal': the events are arranged horizontally.
              The indicator lines are vertical.
            - 'vertical': the events are arranged vertically.
              The indicator lines are horizontal.

        lineoffsets : float or array-like, default: 1
            The offset of the center of the lines from the origin, in the
            direction orthogonal to *orientation*.

            If *positions* is 2D, this can be a sequence with length matching
            the length of *positions*.

        linelengths : float or array-like, default: 1
            The total height of the lines (i.e. the lines stretches from
            ``lineoffset - linelength/2`` to ``lineoffset + linelength/2``).

            If *positions* is 2D, this can be a sequence with length matching
            the length of *positions*.

        linewidths : float or array-like, default: :rc:`lines.linewidth`
            The line width(s) of the event lines, in points.

            If *positions* is 2D, this can be a sequence with length matching
            the length of *positions*.

        colors : :mpltype:`color` or list of color, default: :rc:`lines.color`
            The color(s) of the event lines.

            If *positions* is 2D, this can be a sequence with length matching
            the length of *positions*.

        alpha : float or array-like, default: 1
            The alpha blending value(s), between 0 (transparent) and 1
            (opaque).

            If *positions* is 2D, this can be a sequence with length matching
            the length of *positions*.

        linestyles : str or tuple or list of such values, default: 'solid'
            Default is 'solid'. Valid strings are ['solid', 'dashed',
            'dashdot', 'dotted', '-', '--', '-.', ':']. Dash tuples
            should be of the form::

                (offset, onoffseq),

            where *onoffseq* is an even length tuple of on and off ink
            in points.

            If *positions* is 2D, this can be a sequence with length matching
            the length of *positions*.

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            Other keyword arguments are line collection properties.  See
            `.LineCollection` for a list of the valid properties.

        Returns
        -------
        list of `.EventCollection`
            The `.EventCollection` that were added.

        Notes
        -----
        For *linelengths*, *linewidths*, *colors*, *alpha* and *linestyles*, if
        only a single value is given, that value is applied to all lines. If an
        array-like is given, it must have the same length as *positions*, and
        each value will be applied to the corresponding row of the array.

        Examples
        --------
        .. plot:: gallery/lines_bars_and_markers/eventplot_demo.py
        """
        given_kwargs = dict(kwargs)
        try:
            rows, given = _native_rows(self, positions, orientation, lineoffsets,
                                       linelengths, linewidths, colors, alpha,
                                       linestyles, kwargs)
        except Exception:
            # matplotlib's own code, failing where and as it fails (after
            # adding the rows made before), or plotting what pltrs (Rust)
            # does not take as it does.
            return _events_reference.eventplot(
                self, positions, orientation, lineoffsets, linelengths, linewidths,
                colors, alpha, linestyles, **given_kwargs)
        native = _plotting.eventplot(self._native, *rows)
        collections = wrap_all(self, native, mcoll.EventCollection)
        _remember_rows(collections, *given)
        for collection in collections:
            collection._made_with(kwargs)
        return collections

    @_preprocess_data()
    def stem(self, *args, linefmt=None, markerfmt=None, basefmt=None, bottom=0,
             label=None, orientation='vertical'):
        """
        Create a stem plot.

        A stem plot draws lines perpendicular to a baseline at each location
        *locs* from the baseline to *heads*, and places a marker there. For
        vertical stem plots (the default), the *locs* are *x* positions, and
        the *heads* are *y* values. For horizontal stem plots, the *locs* are
        *y* positions, and the *heads* are *x* values.

        Call signature::

          stem([locs,] heads, linefmt=None, markerfmt=None, basefmt=None)

        The *locs*-positions are optional. *linefmt* may be provided as
        positional, but all other formats must be provided as keyword
        arguments.

        Parameters
        ----------
        locs : array-like, default: (0, 1, ..., len(heads) - 1)
            For vertical stem plots, the x-positions of the stems.
            For horizontal stem plots, the y-positions of the stems.

        heads : array-like
            For vertical stem plots, the y-values of the stem heads.
            For horizontal stem plots, the x-values of the stem heads.

        linefmt : str, optional
            A string defining the color and/or linestyle of the vertical lines:

            =========  =============
            Character  Line Style
            =========  =============
            ``'-'``    solid line
            ``'--'``   dashed line
            ``'-.'``   dash-dot line
            ``':'``    dotted line
            =========  =============

            Default: 'C0-', i.e. solid line with the first color of the color
            cycle.

            Note: Markers specified through this parameter (e.g. 'x') will be
            silently ignored. Instead, markers should be specified using
            *markerfmt*.

        markerfmt : str, optional
            A string defining the color and/or shape of the markers at the stem
            heads. If the marker is not given, use the marker 'o', i.e. filled
            circles. If the color is not given, use the color from *linefmt*.

        basefmt : str, default: 'C3-' ('C2-' in classic mode)
            A format string defining the properties of the baseline.

        orientation : {'vertical', 'horizontal'}, default: 'vertical'
            The orientation of the stems.

        bottom : float, default: 0
            The y/x-position of the baseline (depending on *orientation*).

        label : str, optional
            The label to use for the stems in legends.

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        Returns
        -------
        `.StemContainer`
            The container may be treated like a tuple
            (*markerline*, *stemlines*, *baseline*)

        Notes
        -----
        .. seealso::
            The MATLAB function
            `stem <https://www.mathworks.com/help/matlab/ref/stem.html>`_
            which inspired this method.
        """
        if not 1 <= len(args) <= 3:
            raise _api.nargs_error('stem', '1-3', len(args))
        _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)
        if len(args) == 1:
            heads, = args
            locs = np.arange(len(heads))
            args = ()
        elif isinstance(args[1], str):
            heads, *args = args
            locs = np.arange(len(heads))
        else:
            locs, heads, *args = args
        if orientation == 'vertical':
            locs, heads = self._process_unit_info([("x", locs), ("y", heads)])
        else:
            heads, locs = self._process_unit_info([("x", heads), ("y", locs)])
        if (not isinstance(bottom, Real)
                or takes_any_from_program(type(self), ("plot", "vlines", "hlines"))):
            # A bottom in units, or none: as matplotlib's `vlines` takes it;
            # or a class overriding what matplotlib's draws the stems with.
            return _events_reference.stem(self, locs, heads, args, linefmt, markerfmt,
                                          basefmt, bottom, label, orientation)
        heads = cbook._check_1d(heads)
        locs = cbook._check_1d(locs)
        if np.ndim(heads) != 1 or np.shape(heads) != np.shape(locs):
            raise ValueError(f"could not broadcast the heads of shape "
                             f"{np.shape(heads)} to the locations of shape "
                             f"{np.shape(locs)}")
        if linefmt is None:
            linefmt = args[0] if len(args) > 0 else "C0-"
        if markerfmt is None:
            markerfmt = "o"
        if markerfmt == '':
            markerfmt = ' '  # = empty line style; '' would resolve rcParams
        if basefmt is None:
            basefmt = ("C2-" if _rcParams()["_internal.classic_mode"] else
                       "C3-")
        for fmt in (linefmt, markerfmt, basefmt):
            check_format(fmt)
        native, markerline, stemlines, baseline = _plotting.stem(
            self._native, (floats(locs), floats(heads)),
            (linefmt, markerfmt, basefmt), float(bottom), orientation,
            None if label is None else str(label))
        # matplotlib plots the markers and the baseline with `plot`, given
        # the data and the colours of the formats.
        markerline = wrap(self, markerline, mlines.Line2D)
        baseline = wrap(self, baseline, mlines.Line2D)
        ends = cbook._check_1d([np.min(locs), np.max(locs)])
        base = cbook._check_1d([bottom, bottom])
        if orientation == 'vertical':
            keep_data(markerline, locs, heads)
            keep_data(baseline, ends, base)
        else:
            keep_data(markerline, heads, locs)
            keep_data(baseline, base, ends)
        linecolor = _given_format_color(linefmt)
        markercolor = _given_format_color(markerfmt)
        for line, color in [(markerline, linecolor if markercolor is None else markercolor),
                            (baseline, _given_format_color(basefmt))]:
            if color is not None:
                mlines._remember_given(line, {"color": color})
        container = StemContainer(
            (markerline, wrap(self, stemlines, mcoll.LineCollection), baseline),
            label=label)
        return record_container(self, container, native)
