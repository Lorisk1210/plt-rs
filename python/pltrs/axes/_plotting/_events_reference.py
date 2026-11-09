"""
matplotlib's `~.Axes.eventplot` and `~.Axes.stem`, for the arguments
pltrs (Rust) does not take as they are (see `.events`): the event rows
made one by one as `.EventCollection` objects, the stems drawn with
`~.Axes.vlines` and `~.Axes.plot`, failing where and as matplotlib's fail
(matplotlib 3.11's code, nearly as it is).
"""

import numpy as np

from ... import _val_or_rc, axis as maxis, cbook, collections as mcoll, colors as mcolors
from ...container import StemContainer
from .lines import _format_parts, _given_format_color


def eventplot(axes, positions, orientation='horizontal', lineoffsets=1,
              linelengths=1, linewidths=None, colors=None, alpha=None,
              linestyles='solid', **kwargs):
    """`~.Axes.eventplot` of *axes* as matplotlib makes it: an
    `.EventCollection` per row, made and added one by one."""
    lineoffsets, linelengths = axes._process_unit_info(
        [("y", lineoffsets), ("y", linelengths)], kwargs)

    # fix positions, noting that it can be a list of lists:
    if not np.iterable(positions):
        positions = [positions]
    elif any(np.iterable(position) for position in positions):
        positions = [np.asanyarray(position) for position in positions]
    else:
        positions = [np.asanyarray(positions)]

    poss = []
    for position in positions:
        poss += axes._process_unit_info([("x", position)], kwargs)
    positions = poss

    # prevent 'singular' keys from **kwargs dict from overriding the effect
    # of 'plural' keyword arguments (e.g. 'color' overriding 'colors')
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

    lineoffsets = np.asarray(lineoffsets)
    linelengths = np.asarray(linelengths)
    linewidths = np.asarray(linewidths)

    if len(lineoffsets) == 0:
        raise ValueError('lineoffsets cannot be empty')
    if len(linelengths) == 0:
        raise ValueError('linelengths cannot be empty')
    if len(linestyles) == 0:
        raise ValueError('linestyles cannot be empty')
    if len(linewidths) == 0:
        raise ValueError('linewidths cannot be empty')
    if len(alpha) == 0:
        raise ValueError('alpha cannot be empty')
    if len(colors) == 0:
        colors = [None]
    try:
        # Early conversion of the colors into RGBA values to take care
        # of cases like colors='0.5' or colors='C1'.  (Issue #8193)
        colors = mcolors.to_rgba_array(colors)
    except ValueError:
        # Will fail if any element of *colors* is None. But as long
        # as len(colors) == 1 or len(positions), the rest of the
        # code should process *colors* properly.
        pass

    if len(lineoffsets) == 1 and len(positions) != 1:
        lineoffsets = np.tile(lineoffsets, len(positions))
        lineoffsets[0] = 0
        lineoffsets = np.cumsum(lineoffsets)
    if len(linelengths) == 1:
        linelengths = np.tile(linelengths, len(positions))
    if len(linewidths) == 1:
        linewidths = np.tile(linewidths, len(positions))
    if len(colors) == 1:
        colors = list(colors) * len(positions)
    if len(alpha) == 1:
        alpha = list(alpha) * len(positions)
    if len(linestyles) == 1:
        linestyles = [linestyles] * len(positions)

    if len(lineoffsets) != len(positions):
        raise ValueError('lineoffsets and positions are unequal sized '
                         'sequences')
    if len(linelengths) != len(positions):
        raise ValueError('linelengths and positions are unequal sized '
                         'sequences')
    if len(linewidths) != len(positions):
        raise ValueError('linewidths and positions are unequal sized '
                         'sequences')
    if len(colors) != len(positions):
        raise ValueError('colors and positions are unequal sized '
                         'sequences')
    if len(alpha) != len(positions):
        raise ValueError('alpha and positions are unequal sized '
                         'sequences')
    if len(linestyles) != len(positions):
        raise ValueError('linestyles and positions are unequal sized '
                         'sequences')

    colls = []
    for position, lineoffset, linelength, linewidth, color, alpha_, \
        linestyle in \
            zip(positions, lineoffsets, linelengths, linewidths,
                colors, alpha, linestyles):
        coll = mcoll.EventCollection(position,
                                     orientation=orientation,
                                     lineoffset=lineoffset,
                                     linelength=linelength,
                                     linewidth=linewidth,
                                     color=color,
                                     alpha=alpha_,
                                     linestyle=linestyle)
        axes.add_collection(coll, autolim=False)
        coll._internal_update(kwargs)
        colls.append(coll)

    if len(positions) > 0:
        # try to get min/max
        min_max = [(np.min(_p), np.max(_p)) for _p in positions
                   if len(_p) > 0]
        # if we have any non-empty positions, try to autoscale
        if len(min_max) > 0:
            mins, maxes = zip(*min_max)
            minpos = np.min(mins)
            maxpos = np.max(maxes)

            minline = (lineoffsets - linelengths).min()
            maxline = (lineoffsets + linelengths).max()

            if orientation == "vertical":
                corners = (minline, minpos), (maxline, maxpos)
            else:  # "horizontal"
                corners = (minpos, minline), (maxpos, maxline)
            axes.update_datalim(corners)
            axes._request_autoscale_view()

    return colls


def _process_plot_format(fmt):
    """``(linestyle, marker, color)`` of the format string *fmt*, as
    matplotlib's ``_process_plot_format`` gives them: no line style for a
    format that is a colour as a whole."""
    linestyle, marker, _ = _format_parts(fmt)
    if marker is None:
        linestyle = None
    return linestyle, marker, _given_format_color(fmt)


def stem(axes, locs, heads, args, linefmt, markerfmt, basefmt, bottom, label,
         orientation):
    """`~.Axes.stem` of *axes* as matplotlib makes it once it has its
    locations *locs* and *heads* (in the Axes' units) and the formats
    *args* left: the stems from *bottom* drawn with `~.Axes.vlines`
    (`~.Axes.hlines`), which convert it, the markers and the baseline with
    `~.Axes.plot`."""
    heads = cbook._check_1d(heads)
    locs = cbook._check_1d(locs)

    # resolve line format
    if linefmt is None:
        linefmt = args[0] if len(args) > 0 else "C0-"
    linestyle, linemarker, linecolor = _process_plot_format(linefmt)

    # resolve marker format
    if markerfmt is None:
        # if not given as kwarg, fall back to 'o'
        markerfmt = "o"
    if markerfmt == '':
        markerfmt = ' '  # = empty line style; '' would resolve rcParams
    markerstyle, markermarker, markercolor = _process_plot_format(markerfmt)
    if markermarker is None:
        markermarker = 'o'
    if markerstyle is None:
        markerstyle = 'None'
    if markercolor is None:
        markercolor = linecolor

    # resolve baseline format
    if basefmt is None:
        from ... import rcParams
        basefmt = ("C2-" if rcParams["_internal.classic_mode"] else
                   "C3-")
    basestyle, basemarker, basecolor = _process_plot_format(basefmt)

    # New behaviour in 3.1 is to use a LineCollection for the stemlines
    linestyle = _val_or_rc(linestyle, 'lines.linestyle')
    xlines = axes.vlines if orientation == "vertical" else axes.hlines
    stemlines = xlines(
        locs, bottom, heads,
        colors=linecolor, linestyles=linestyle, label="_nolegend_")

    if orientation == 'horizontal':
        marker_x = heads
        marker_y = locs
        baseline_x = [bottom, bottom]
        baseline_y = [np.min(locs), np.max(locs)]
    else:
        marker_x = locs
        marker_y = heads
        baseline_x = [np.min(locs), np.max(locs)]
        baseline_y = [bottom, bottom]

    markerline, = axes.plot(marker_x, marker_y,
                            color=markercolor, linestyle=markerstyle,
                            marker=markermarker, label="_nolegend_")

    baseline, = axes.plot(baseline_x, baseline_y,
                          color=basecolor, linestyle=basestyle,
                          marker=basemarker, label="_nolegend_")
    baseline.get_path()._interpolation_steps = \
        maxis.GRIDLINE_INTERPOLATION_STEPS

    stem_container = StemContainer((markerline, stemlines, baseline),
                                   label=label)
    axes.add_container(stem_container)
    return stem_container
