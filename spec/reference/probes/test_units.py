"""Unit handling that matplotlib's tests leave unchecked: dates moved by
``timedelta`` extents (exactly: matplotlib adds them to the ``datetime``
before converting, so the numbers can differ in the last bit from adding
days to date numbers), error sizes that do not add to their values, and
geographic axes asked to share an axis.

Area ``units``. Dates are ``[year, month, day, hour, minute, second,
microsecond]`` lists and durations ``[days, seconds, microseconds]``
lists, as ``datetime`` and ``timedelta`` take them. Every probe draws on a
new ``Figure()`` with an Agg canvas.

Ops
---
``Rectangle.dates``  {start, duration} -> ``[x0, x1 - x0]`` of the bounds
    of ``Rectangle((datetime(*start), 0), timedelta(*duration), 1)`` added
    to new axes: the date numbers of the corner and of the width, exactly.
``Axes.bar.dates``  {starts, widths} -> the widths of the bars of
    ``bar([datetime(*s) ...], 1, width=[timedelta(*w) ...])``, exactly
    (each is the date number of the first start plus the width, less that
    of the first start).
``Axes.errorbar.dates``  {x, xerr} -> the ``[x0, x1]`` of each error bar of
    ``errorbar([datetime(*d) ...], [0, ...], xerr=timedelta(*xerr))``,
    exactly.
``Axes.errorbar.unaddable``  {kind} -> the error ``errorbar`` raises for
    error sizes that do not add to their values: a ``timedelta`` to numbers
    (``"number+timedelta"``) or a number to dates (``"date+number"``).
``Axes3D.errorbar.timedelta``  {} -> the error of ``errorbar([0, 1], [0,
    1], [0, 1], xerr=timedelta(days=1))`` on 3-D axes.
``share.geographic``  {projection, how} -> null when the axes are made and
    shared, the error otherwise: ``"subplots"`` makes ``fig.subplots(1, 2,
    sharex=True, subplot_kw={"projection": projection})``, ``"geo.sharex"``
    makes the geographic axes share the x axis of rectilinear ones,
    ``"rect.sharex"`` the other way round.
"""

import datetime

import matplotlib.patches as mpatches
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("units")


def _axes(projection=None):
    fig = Figure()
    FigureCanvasAgg(fig)
    return fig, fig.add_subplot(projection=projection)


def _rectangle(start, duration):
    fig, ax = _axes()
    patch = ax.add_patch(mpatches.Rectangle((datetime.datetime(*start), 0),
                                            datetime.timedelta(*duration), 1))
    fig.draw_without_rendering()
    bounds = patch.get_bbox()
    return [bounds.x0, bounds.x1 - bounds.x0]


def test_rectangle_dates():
    for start, duration in [
        ([2020, 5, 1, 0, 0, 1, 0], [0, 28800, 0]),
        ([2020, 5, 1, 0, 0, 3, 0], [0, 18007, 0]),
        ([2021, 3, 14, 15, 9, 26, 535897], [3, 1, 5]),
        ([1999, 12, 31, 23, 59, 59, 999999], [0, 0, 1]),
        ([2020, 1, 1, 0, 0, 0, 0], [2, 0, 0]),
    ]:
        probes.probe("Rectangle.dates", _rectangle, tol=0, start=start, duration=duration)


def _bars(starts, widths):
    fig, ax = _axes()
    bars = ax.bar([datetime.datetime(*s) for s in starts], 1,
                  width=[datetime.timedelta(*w) for w in widths])
    fig.draw_without_rendering()
    return [bar.get_width() for bar in bars]


def test_bar_dates():
    probes.probe("Axes.bar.dates", _bars, tol=0,
                 starts=[[2020, 5, 1, 0, 0, 1, 0], [2020, 5, 3, 0, 0, 7, 0]],
                 widths=[[0, 28800, 0], [0, 18007, 0]])


def _errorbar(x, xerr):
    fig, ax = _axes()
    bars = ax.errorbar([datetime.datetime(*d) for d in x], [0] * len(x),
                       xerr=datetime.timedelta(*xerr))
    fig.draw_without_rendering()
    return [[float(seg[0][0]), float(seg[1][0])] for seg in bars.lines[2][0].get_segments()]


def test_errorbar_dates():
    probes.probe("Axes.errorbar.dates", _errorbar, tol=0,
                 x=[[2020, 5, 1, 0, 0, 1, 0], [2020, 5, 2, 0, 0, 3, 0]], xerr=[0, 28800, 0])


def _unaddable(kind):
    fig, ax = _axes()
    day = datetime.timedelta(days=1)
    if kind == "number+timedelta":
        ax.errorbar([1.0, 2.0], [1.0, 2.0], xerr=day)
    else:
        ax.errorbar([datetime.datetime(2020, 5, 1), datetime.datetime(2020, 5, 2)],
                    [1.0, 2.0], xerr=0.5)
    fig.draw_without_rendering()


def test_errorbar_unaddable():
    for kind in ["number+timedelta", "date+number"]:
        probes.probe("Axes.errorbar.unaddable", _unaddable, kind=kind)


def _errorbar_3d():
    fig, ax = _axes("3d")
    ax.errorbar([0.0, 1.0], [0.0, 1.0], [0.0, 1.0], xerr=datetime.timedelta(days=1))
    fig.draw_without_rendering()


def test_errorbar_3d_timedelta():
    probes.probe("Axes3D.errorbar.timedelta", _errorbar_3d)


def _share(projection, how):
    fig = Figure()
    FigureCanvasAgg(fig)
    if how == "subplots":
        fig.subplots(1, 2, sharex=True, subplot_kw={"projection": projection})
    elif how == "geo.sharex":
        rect = fig.add_subplot(121)
        fig.add_subplot(122, projection=projection).sharex(rect)
    else:
        geo = fig.add_subplot(122, projection=projection)
        fig.add_subplot(121).sharex(geo)
    fig.draw_without_rendering()


def test_share_geographic():
    for projection in ["aitoff", "hammer", "lambert", "mollweide"]:
        for how in ["subplots", "geo.sharex", "rect.sharex"]:
            probes.probe("share.geographic", _share, projection=projection, how=how)
