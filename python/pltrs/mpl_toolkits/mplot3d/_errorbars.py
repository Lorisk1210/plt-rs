"""
matplotlib's arithmetic on the arguments of `Axes3D.errorbar`, which pltrs
(Rust) draws: run on them first, it raises what matplotlib's raises and
warns as its numpy arithmetic warns (NaN and infinite sizes, all-NaN
bounds), in its order.
"""

import numpy as np

import pltrs as mpl
from pltrs import _pybind, colors as mcolors
from pltrs.axes._plotting.lines import _format_parts

from . import art3d


def _extract_errs(err, data, lomask, himask, everymask):
    """The low and high ends of the bars of *data* for sizes *err*
    (matplotlib's ``_extract_errs``)."""
    if len(err.shape) == 2:
        low_err, high_err = err
    else:
        low_err, high_err = err, err
    lows = np.where(lomask | ~everymask, data, data - low_err)
    highs = np.where(himask | ~everymask, data, data + high_err)
    return lows, highs


def check_format(fmt):
    """Raise as matplotlib's ``_plot_args`` of ``(x, y, fmt)`` does for a
    format *fmt* that is no string, before errorbar makes its data line."""
    if not isinstance(fmt, str):
        raise ValueError('third arg must be a format string')


def leave_data_line(ax, coords, fmt, kwargs, options):
    """
    Add to *ax* the data line through *coords* matplotlib's errorbar has
    added when its arithmetic on the bars fails: styled by the format
    *fmt* (none for ``'none'``) and *kwargs*, of the next colour of the
    cycle unless they give one, and just above the bars (below them for
    ``barsabove``), from ``options = (barsabove, axlim_clip)``; the view
    does not take it in.
    """
    if fmt.lower() == 'none':
        return
    barsabove, axlim_clip = options
    style = dict(zip(('linestyle', 'marker', 'color'), _format_parts(fmt)))
    style = {key: value for key, value in style.items() if value is not None}
    style.update(kwargs)
    if style.get('color') is None:
        style['color'] = ax._get_lines.get_next_color()
    style['label'] = '_nolegend_'
    zorder = style.pop('zorder', 2)
    line = art3d.Line3D(*coords, axlim_clip=axlim_clip, **style)
    line.set_zorder(zorder - .1 if barsabove else zorder + .1)
    ax.add_line(line)


def check(coords, errors, lims, everymask, style):
    """
    Raise and warn as matplotlib's errorbar does on its arguments once it
    has made its data line: *coords* ``(x, y, z)`` as given (each made
    iterable), *errors* ``(xerr, yerr, zerr)``, *lims* ``((xlolims,
    ylolims, zlolims), (xuplims, yuplims, zuplims))``, *everymask* the
    points with bars and *style* ``(ecolor, capsize)`` (no *ecolor*: the
    line's, a colour).
    """
    ecolor, capsize = style
    if capsize is None:
        capsize = mpl.rcParams["errorbar.capsize"]
    capped = capsize > 0
    coorderrs = []
    for zdir, data, err, lolims, uplims in zip('xyz', coords, errors, *lims):
        if err is None:
            continue
        dir_vector = art3d.get_dir_vector(zdir)
        if not np.iterable(err):
            err = [err] * len(data)
        err = np.atleast_1d(err)
        lolims = np.broadcast_to(lolims, len(data)).astype(bool)
        uplims = np.broadcast_to(uplims, len(data)).astype(bool)
        coorderr = [
            _extract_errs(err * dir_vector[i], coord, lolims, uplims, everymask)
            for i, coord in enumerate(coords)]
        if ecolor is not None:
            if (~(lolims | uplims)).any() and capped:
                # The caps are lines of the colour of the bars.
                mcolors._check_color_like(color=ecolor)
            # The bars, a line collection of their colour.
            mcolors.to_rgba_array(ecolor)
        coorderrs.append(coorderr)
    coorderrs = np.array(coorderrs)
    for i in range(3):
        # matplotlib's ``_digout_minmax`` of the bounds it autoscales to.
        np.nanmin(coorderrs[:, i, :, :])
        np.nanmax(coorderrs[:, i, :, :])


def drawn_widths(elinewidth, capthick):
    """
    The widths ``(elinewidth, capthick)`` pltrs (Rust) draws the bars and
    caps with, and the exceptions ``(bars, caps)`` matplotlib's draw of
    them raises (None: none): matplotlib keeps a string *elinewidth*
    (ignoring an empty one), and the dash pattern of its bars scaled by
    it casts to no C++ value; the caps' edges are as thick as ``float``
    reads *capthick*.
    """
    bars = caps = None
    if isinstance(elinewidth, str):
        if elinewidth:
            bars = _pybind.cast_error(())
        elinewidth = None
    if isinstance(capthick, str):
        try:
            capthick = float(capthick)
        except ValueError as err:
            caps = err
            capthick = None
    return (elinewidth, capthick), (bars, caps)
