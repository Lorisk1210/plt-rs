"""matplotlib's own tests of unit conversion (``matplotlib/tests/test_units.py``)
for the units plt-rs models: dates (``datetime64`` arrays, ``datetime``
values and ``timedelta`` extents), category names and the converters of an
axis. The tests of Python unit classes (a ``Quantity`` facade with a mocked
``ConversionInterface``, ``matplotlib.testing.jpl_units``, array-likes and
pandas) are left out.

Area ``mpl_units``. Every probe runs in the style matplotlib's test fixture
applies (``classic`` plus ``_classic_test_patch``), given as ``style``, on
``Figure()`` with an Agg canvas, drawn (``draw_without_rendering``) before
anything is recorded. Converters are recorded by kind: ``"dates"``,
``"categories"`` or null.

Ops
---
``Axes.scatter``  {start, stop, unit, nan_first} -> ``{converter, xlim,
    ylim, offsets}`` of ``scatter(times, y)`` with ``times =
    np.arange(start, stop, dtype=f"datetime64[{unit}]")`` and ``y`` counting
    from 0, its first value NaN when ``nan_first``; empty when ``start ==
    stop``; the offsets are those of the points drawn (``test_empty_arrays``,
    ``test_scatter_element0_masked``).
``Axes.errorbar``  {days, yerr_days} -> ``{converter, xlim, ylim,
    segments}`` of ``errorbar(np.arange(len(days)), [datetime(2020, 5, d)
    for d in days], timedelta(days=yerr_days))``: the ``[y0, y1]`` of each
    error bar (``test_errorbar_mixed_units``).
``Axis.set_units``  {tz_hours} -> after ``ax1.plot(y1)`` and ``ax2.plot(y2)``
    of the first days of the months of 2020 and 2021 on ``fig.subplots(1, 2,
    sharey=True)``, ``ax1.yaxis.set_units(timezone(timedelta(hours=
    tz_hours)))``: the major tick locations of ``ax2``'s y axis and what its
    formatter makes of them (``test_shared_axis_datetime``; plt-rs places
    dates in a time zone with ``axis_date``).
``Axes.plot.shared_categories``  {first, second} -> ``{names, xdata,
    labels}`` after ``ax1.plot(first, ...)`` and ``ax2.plot(second, ...)``
    (left out when ``second`` is empty) on ``fig.subplots(1, 2, sharex=True,
    sharey=True)``: the category names of both x axes, in position order,
    the x data of the lines and what the x axes' major formatters make of
    their ticks
    (``test_shared_axis_categorical``, up to the ``set_units(UnitData(...))``
    that makes matplotlib's lines convert their names again).
``Axis.set_converter``  {steps} -> one entry per step on the x axis of a new
    axes: ``["set", kind]`` calls ``set_converter`` with a
    ``StrCategoryConverter`` (one instance for the whole op) or a
    ``DateConverter`` and records null, or ``"error"`` when it raises;
    ``["plot"]`` plots ``{"a": 1, "b": 2}`` and records null; ``["get"]``
    records the converter's kind (``test_explicit_converter``; plt-rs
    converters are kinds, so the test's second ``StrCategoryConverter``
    instance has no counterpart).

``test_empty_arrays``, ``test_scatter_element0_masked`` and
``test_errorbar_mixed_units`` are also figure cases of ``mpl_tests_units``;
``test_plot_masked_units`` and ``test_subclass`` are figure cases of
``mpl_misc``.

Left out: the tests of Python unit protocols: ``test_numpy_facade``,
``test_empty_set_limits_with_units``, ``test_shared_axis_quantity`` and
``test_empty_default_limits`` (a ``Quantity`` class with a mocked
``ConversionInterface``), ``test_jpl_bar_units``, ``test_jpl_barh_units`` and
``test_jpl_datetime_units_consistent`` (``matplotlib.testing.jpl_units``),
``test_plot_kernel`` (an array-like class) and
``test_connection_patch_units`` (pandas).
"""

import datetime

import matplotlib as mpl
import matplotlib.style
import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.category import StrCategoryConverter
from matplotlib.dates import DateConverter, _SwitchableDateConverter
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("mpl_units")

CLASSIC = ["classic", "_classic_test_patch"]


def probe(op, fn, **args):
    def styled(style, **kw):
        with matplotlib.style.context(style):
            return fn(**kw)

    return probes.probe(op, styled, style=CLASSIC, **args)


def _figure():
    fig = Figure()
    FigureCanvasAgg(fig)
    return fig


def _kind(converter):
    if isinstance(converter, StrCategoryConverter):
        return "categories"
    if isinstance(converter, (DateConverter, _SwitchableDateConverter)):
        return "dates"
    return None


def _scatter(start, stop, unit, nan_first):
    fig = _figure()
    ax = fig.subplots()
    times = np.arange(start, stop, dtype=f"datetime64[{unit}]")
    y = np.arange(len(times), dtype=float)
    if nan_first and len(y):
        y[0] = np.nan
    points = ax.scatter(times, y)
    fig.draw_without_rendering()
    return {"converter": _kind(ax.xaxis.get_converter()), "xlim": list(ax.get_xlim()),
            "ylim": list(ax.get_ylim()),
            "offsets": np.ma.compress_rows(np.ma.masked_invalid(points.get_offsets())).tolist()}


def test_empty_arrays():
    probe("Axes.scatter", _scatter, start="2005-02-01", stop="2005-02-01", unit="ns",
          nan_first=False)


def test_scatter_element0_masked():
    probe("Axes.scatter", _scatter, start="2005-02", stop="2005-03", unit="D", nan_first=True)


def _errorbar(days, yerr_days):
    fig = _figure()
    ax = fig.subplots()
    bars = ax.errorbar(np.arange(len(days)), [datetime.datetime(2020, 5, d) for d in days],
                       datetime.timedelta(days=yerr_days))
    fig.draw_without_rendering()
    segments = [[float(segment[0][1]), float(segment[1][1])]
                for segment in bars.lines[2][0].get_segments()]
    return {"converter": _kind(ax.yaxis.get_converter()), "xlim": list(ax.get_xlim()),
            "ylim": list(ax.get_ylim()), "segments": segments}


def test_errorbar_mixed_units():
    probe("Axes.errorbar", _errorbar, days=[2 * i + 1 for i in range(10)], yerr_days=0.5)


def _shared_datetime(tz_hours):
    fig = _figure()
    ax1, ax2 = fig.subplots(1, 2, sharey=True)
    utc = datetime.timezone.utc
    ax1.plot([datetime.datetime(2020, i, 1, tzinfo=utc) for i in range(1, 13)])
    ax2.plot([datetime.datetime(2021, i, 1, tzinfo=utc) for i in range(1, 13)])
    ax1.yaxis.set_units(datetime.timezone(datetime.timedelta(hours=tz_hours)))
    fig.draw_without_rendering()
    ticks = ax2.get_yticks()
    return {"ticks": ticks.tolist(),
            "labels": ax2.yaxis.get_major_formatter().format_ticks(ticks)}


def test_shared_axis_datetime():
    probe("Axis.set_units", _shared_datetime, tz_hours=5)


def _shared_categories(first, second):
    fig = _figure()
    axs = fig.subplots(1, 2, sharex=True, sharey=True)
    lines = [ax.plot(names, range(len(names)))[0]
             for ax, names in zip(axs, (first, second)) if names]
    fig.draw_without_rendering()
    return {"names": [list(ax.xaxis.get_units()._mapping) for ax in axs],
            "xdata": [line.get_xydata()[:, 0].tolist() for line in lines],
            "labels": [ax.xaxis.get_major_formatter().format_ticks(ax.get_xticks())
                       for ax in axs]}


def test_shared_axis_categorical():
    for first, second in [(["a", "b"], ["a", "b"]), (["a", "b"], ["c", "a"]), (["a", "b"], [])]:
        probe("Axes.plot.shared_categories", _shared_categories, first=first, second=second)


def _set_converter(steps):
    ax = _figure().subplots()
    converters = {"categories": StrCategoryConverter(), "dates": DateConverter()}
    recorded = []
    for kind, *rest in steps:
        if kind == "set":
            try:
                ax.xaxis.set_converter(converters[rest[0]])
            except RuntimeError:
                recorded.append("error")
            else:
                recorded.append(None)
        elif kind == "plot":
            data = {"a": 1, "b": 2}
            ax.plot(data.keys(), data.values())
            recorded.append(None)
        else:
            recorded.append(_kind(ax.xaxis.get_converter()))
    return recorded


def test_explicit_converter():
    probe("Axis.set_converter", _set_converter, steps=[
        ["set", "categories"], ["get"], ["plot"], ["get"], ["set", "categories"],
        ["set", "dates"], ["get"]])
    probe("Axis.set_converter", _set_converter, steps=[
        ["plot"], ["get"], ["set", "categories"], ["get"]])
    probe("Axis.set_converter", _set_converter, steps=[
        ["plot"], ["set", "dates"], ["get"]])
