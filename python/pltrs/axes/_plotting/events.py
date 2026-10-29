"""
Events and stems: `~.Axes.eventplot` and `~.Axes.stem`.
"""

import numpy as np

from ... import _api, cbook, collections as mcoll, colors as mcolors, lines as mlines
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ...container import StemContainer
from ._helpers import axis_values, floats, record_container, wrap, wrap_all


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
        """
        _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)
        if not np.iterable(positions):
            positions = [positions]
        elif any(np.iterable(position) for position in positions):
            positions = [np.asanyarray(position) for position in positions]
        else:
            positions = [np.asanyarray(positions)]
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
            self._native, [axis_values(p) for p in positions],
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
        for collection in collections:
            collection._internal_update(kwargs)
        return collections

    def stem(self, *args, linefmt=None, markerfmt=None, basefmt=None, bottom=0,
             label=None, orientation='vertical'):
        """
        Create a stem plot.

        Returns
        -------
        `.StemContainer`
            The container may be treated like a tuple
            (*markerline*, *stemlines*, *baseline*)
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
        heads = cbook._check_1d(heads)
        locs = cbook._check_1d(locs)
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
            self._native, (axis_values(locs), axis_values(heads)),
            (linefmt, markerfmt, basefmt), float(bottom), orientation,
            None if label is None else str(label))
        container = StemContainer(
            (wrap(self, markerline, mlines.Line2D),
             wrap(self, stemlines, mcoll.LineCollection),
             wrap(self, baseline, mlines.Line2D)), label=label)
        return record_container(self, container, native)
