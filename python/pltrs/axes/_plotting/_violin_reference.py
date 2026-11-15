"""
matplotlib's `~.Axes.violin`, for an Axes class of the program's that
overrides a plotting method matplotlib's draws the violins with
(`~.Axes.fill_between`, `~.Axes.fill_betweenx`, `~.Axes.vlines`,
`~.Axes.hlines`: ultraplot styles them there), which pltrs (Rust) makes
itself otherwise (matplotlib 3.11's code, nearly as it is, under the
Matplotlib License).
"""

import datetime
import functools
import itertools

import numpy as np

from ... import _api, colors as mcolors, rcParams


def violin(axes, vpstats, positions=None, vert=None,
           orientation='vertical', widths=0.5, showmeans=False,
           showextrema=True, showmedians=False, side='both',
           facecolor=None, linecolor=None):
    """`~.Axes.violin` of *axes* as matplotlib makes it: the bodies through
its `~.Axes.fill_between` or `~.Axes.fill_betweenx`, the lines through
its `~.Axes.vlines` and `~.Axes.hlines`."""

    # Statistical quantities to be plotted on the violins
    means = []
    mins = []
    maxes = []
    medians = []
    quantiles = []

    qlens = []  # Number of quantiles in each dataset.

    artists = {}  # Collections to be returned

    N = len(vpstats)
    datashape_message = ("List of violinplot statistics and `{0}` "
                         "values must have the same length")

    # vert and orientation parameters are linked until vert's
    # deprecation period expires. If both are selected,
    # vert takes precedence.
    if vert is not None:
        _api.warn_deprecated(
            "3.11",
            name="vert: bool",
            alternative="orientation: {'vertical', 'horizontal'}",
        )
        orientation = 'vertical' if vert else 'horizontal'
    _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)

    # Validate positions
    if positions is None:
        positions = range(1, N + 1)
    elif len(positions) != N:
        raise ValueError(datashape_message.format("positions"))
    # Validate widths
    if np.isscalar(widths):
        widths = [widths] * N
    elif len(widths) != N:
        raise ValueError(datashape_message.format("widths"))

    # For usability / better error message:
    # Validate that datetime-like positions have timedelta-like widths.
    # Checking only the first element is good enough for standard misuse cases
    if N > 0:  # No need to validate if there is no data
        pos0 = positions[0]
        width0 = widths[0]
        if (isinstance(pos0, (datetime.datetime, datetime.date))
            and not isinstance(width0, datetime.timedelta)):
            raise TypeError(
                "datetime/date 'position' values require timedelta 'widths'. "
                "For example, use positions=[datetime.date(2024, 1, 1)] "
                "and widths=[datetime.timedelta(days=1)].")
        elif (isinstance(pos0, np.datetime64)
            and not isinstance(width0, np.timedelta64)):
            raise TypeError(
                "np.datetime64 'position' values require np.timedelta64 'widths'")
    _api.check_in_list(["both", "low", "high"], side=side)

    # Calculate ranges for statistics lines (shape (2, N)).
    line_ends = [[-0.25 if side in ['both', 'low'] else 0],
                 [0.25 if side in ['both', 'high'] else 0]] \
                      * np.array(widths) + positions

    # Make a cycle of color to iterate through, using 'none' as fallback
    def cycle_color(color, alpha=None):
        rgba = mcolors.to_rgba_array(color, alpha=alpha)
        color_cycler = itertools.chain(itertools.cycle(rgba),
                                       itertools.repeat('none'))
        color_list = []
        for _ in range(N):
            color_list.append(next(color_cycler))
        return color_list

    # Convert colors to chain (number of colors can be different from len(vpstats))
    if facecolor is None or linecolor is None:
        if not rcParams['_internal.classic_mode']:
            next_color = axes._get_lines.get_next_color()

    if facecolor is not None:
        facecolor = cycle_color(facecolor)
        body_artist_alpha = None
    else:
        body_artist_alpha = 0.3
        # Use default colors if user doesn't provide them
        if rcParams['_internal.classic_mode']:
            facecolor = cycle_color('y')
        else:
            facecolor = cycle_color(next_color)

    if rcParams['_internal.classic_mode']:
        # Classic mode uses patch.force_edgecolor=True, so we need to
        # set the edgecolor to make sure it has an alpha.
        body_edgecolor = ("k", 0.3)
    else:
        body_edgecolor = None

    if linecolor is not None:
        linecolor = cycle_color(linecolor)
    else:
        if rcParams['_internal.classic_mode']:
            linecolor = cycle_color('r')
        else:
            linecolor = cycle_color(next_color)

    # Check whether we are rendering vertically or horizontally
    if orientation == 'vertical':
        fill = axes.fill_betweenx
        if side in ['low', 'high']:
            perp_lines = functools.partial(axes.hlines, colors=linecolor,
                                            capstyle='projecting')
            par_lines = functools.partial(axes.vlines, colors=linecolor,
                                            capstyle='projecting')
        else:
            perp_lines = functools.partial(axes.hlines, colors=linecolor)
            par_lines = functools.partial(axes.vlines, colors=linecolor)
    else:
        fill = axes.fill_between
        if side in ['low', 'high']:
            perp_lines = functools.partial(axes.vlines, colors=linecolor,
                                            capstyle='projecting')
            par_lines = functools.partial(axes.hlines, colors=linecolor,
                                            capstyle='projecting')
        else:
            perp_lines = functools.partial(axes.vlines, colors=linecolor)
            par_lines = functools.partial(axes.hlines, colors=linecolor)

    # Render violins
    bodies = []
    bodies_zip = zip(vpstats, positions, widths, facecolor)
    for stats, pos, width, facecolor in bodies_zip:
        # The 0.5 factor reflects the fact that we plot from v-p to v+p.
        vals = np.array(stats['vals'])
        if len(vals) > 0:
            vals = 0.5 * width * vals / vals.max()
        bodies += [fill(stats['coords'],
                        -vals + pos if side in ['both', 'low'] else pos,
                        vals + pos if side in ['both', 'high'] else pos,
                        facecolor=facecolor, edgecolor=body_edgecolor,
                        alpha=body_artist_alpha)]
        means.append(stats['mean'])
        mins.append(stats['min'])
        maxes.append(stats['max'])
        medians.append(stats['median'])
        q = stats.get('quantiles')  # a list of floats, or None
        if q is None:
            q = []
        quantiles.extend(q)
        qlens.append(len(q))
    artists['bodies'] = bodies

    if showmeans:  # Render means
        artists['cmeans'] = perp_lines(means, *line_ends)
    if showextrema:  # Render extrema
        artists['cmaxes'] = perp_lines(maxes, *line_ends)
        artists['cmins'] = perp_lines(mins, *line_ends)
        artists['cbars'] = par_lines(positions, mins, maxes)
    if showmedians:  # Render medians
        artists['cmedians'] = perp_lines(medians, *line_ends)
    if quantiles:  # Render quantiles: each width is repeated qlen times.
        artists['cquantiles'] = perp_lines(
            quantiles, *np.repeat(line_ends, qlens, axis=1))

    return artists
