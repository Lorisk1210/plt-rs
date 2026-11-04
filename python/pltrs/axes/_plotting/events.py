"""
Events and stems: `~.Axes.eventplot` and `~.Axes.stem`.
"""

import numpy as np

from ... import _api, _values, cbook, collections as mcoll, colors as mcolors, lines as mlines
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ...container import StemContainer
from ._helpers import floats, keep_unit_data, record_container, wrap, wrap_all
from .lines import _given_format_color


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


class EventsMixin:
    """`eventplot` and `stem`."""

    @_preprocess_data(replace_names=["positions", "lineoffsets",
                                     "linelengths", "linewidths",
                                     "colors", "linestyles"])
    def eventplot(self, positions, orientation='horizontal', lineoffsets=1,
                  linelengths=1, linewidths=None, colors=None, alpha=None,
                  linestyles='solid', **kwargs):
        """
        Plot identical parallel lines at the given positions.

        Returns
        -------
        list of `.EventCollection`
            The `.EventCollection` that were added.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)
        lineoffsets, linelengths = self._process_unit_info(
            [("y", lineoffsets), ("y", linelengths)], kwargs)
        if not np.iterable(positions):
            positions = [positions]
        elif any(np.iterable(position) for position in positions):
            positions = [np.asanyarray(position) for position in positions]
        else:
            positions = [np.asanyarray(positions)]
        positions = [self._process_unit_info([("x", position)], kwargs)[0]
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
        linestyles = _per_row("linestyles", list(linestyles), n)
        native_colors = ([] if all(c is None for c in colors) else
                         [tuple(mcolors.to_rgba(_rcParams()['lines.color']
                                                if c is None else c))
                          for c in colors])
        native = _plotting.eventplot(
            self._native, positions,
            "horizontal" if orientation == "horizontal" else "vertical",
            ([float(v) for v in lineoffsets], linelengths,
             None if all(w is None for w in linewidths) else
             [float(_rcParams()['lines.linewidth'] if w is None else w)
              for w in linewidths],
             native_colors,
             None if all(a is None for a in alpha) else
             [1.0 if a is None else float(a) for a in alpha],
             [mcoll._native_style(ls) for ls in linestyles]))
        collections = wrap_all(self, native, mcoll.EventCollection)
        _remember_rows(collections, *given)
        for collection in collections:
            collection._internal_update(kwargs)
        return collections

    @_preprocess_data()
    def stem(self, *args, linefmt=None, markerfmt=None, basefmt=None, bottom=0,
             label=None, orientation='vertical'):
        """
        Create a stem plot.

        Returns
        -------
        `.StemContainer`
            The container may be treated like a tuple
            (*markerline*, *stemlines*, *baseline*)

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
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
            bottom = self.convert_yunits(bottom)
        else:
            heads, locs = self._process_unit_info([("x", heads), ("y", locs)])
            bottom = self.convert_xunits(bottom)
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
            keep_unit_data(markerline, locs, heads)
            keep_unit_data(baseline, ends, base)
        else:
            keep_unit_data(markerline, heads, locs)
            keep_unit_data(baseline, base, ends)
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
