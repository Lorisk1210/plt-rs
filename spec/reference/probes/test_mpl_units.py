"""matplotlib's own tests of unit conversion (``matplotlib/tests/test_units.py``):
dates (``datetime64`` arrays, ``datetime`` values and ``timedelta``
extents), category names, the converters of an axis, and user unit classes:
the test module's ``Quantity`` facade with its ``quantity_converter``
fixture (``spec_support.units``), ``matplotlib.testing.jpl_units`` and an
array-like class. Converters are registered only while a probe runs, as the
test fixture restores the registry after each test.

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
    formatter makes of them (``test_shared_axis_datetime``).
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

``Axes.set_lim.quantity``  {x, y} -> ``{xlim, ylim, units, labels}`` after
    ``set_xlim(Quantity(x[0], x[2]), Quantity(x[1], x[2]))`` and the same
    for y on a new axes, drawn: the limits, the units of the x and y axis
    and their labels (``test_empty_set_limits_with_units``).
``Axis.set_units.shared_quantity``  {} -> ``test_shared_axis_quantity`` on
    ``fig.subplots(2, 1, sharex="all", sharey="all")``: ``units`` and
    ``labels`` (``[[ax1 x, ax1 y], [ax2 x, ax2 y]]``) after both plots and
    again after ``ax1.xaxis.set_units("seconds")`` and
    ``ax2.yaxis.set_units("inches")``, then drawn, the lines' converted
    ``xy`` data and both axes' ``xlim`` and ``ylim``.
``Axes.default_limits.quantity``  {steps} -> one entry per step on a new
    axes (``test_empty_default_limits``): ``["update_units", axis,
    magnitude, unit]`` calls ``update_units(Quantity([magnitude], unit))``
    on the x or y axis, ``["axhline", y]`` and ``["axvline", x]`` draw a
    line at a number, ``["plot"]`` plots ``Quantity(np.arange(0, 3),
    "miles")`` against ``Quantity(np.arange(0, 6, 2), "feet")``, each
    recording null; ``["draw"]`` draws the figure and records ``[xlim,
    ylim]``.
``jpl.EpochConverter.convert``  {date} -> ``{datetime, epoch}``: what the
    registry's converters of ``datetime(*date)`` and of ``jpl_units.Epoch("ET",
    dt=datetime(*date))`` make of them
    (``test_jpl_datetime_units_consistent``).
``Axes.plot.array_like``  {values} -> ``{xdata, ydata}`` of ``plot(Kernel(values))``,
    ``Kernel`` the test's class with only ``__array__`` and ``shape``
    (``test_plot_kernel``).

``test_empty_arrays``, ``test_scatter_element0_masked``,
``test_errorbar_mixed_units``, ``test_numpy_facade``, ``test_plot_masked_units``,
``test_jpl_bar_units`` and ``test_jpl_barh_units`` are figure cases of
``mpl_tests_units``; ``test_subclass`` is a figure case of ``mpl_misc``.

Left out: ``test_connection_patch_units`` (pandas).
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
from spec_support.units import Quantity, jpl_registered, quantity_registered

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


def _lim_quantity(x, y):
    with quantity_registered():
        fig = _figure()
        ax = fig.subplots()
        ax.set_xlim(Quantity(x[0], x[2]), Quantity(x[1], x[2]))
        ax.set_ylim(Quantity(y[0], y[2]), Quantity(y[1], y[2]))
        fig.draw_without_rendering()
        return {"xlim": list(ax.get_xlim()), "ylim": list(ax.get_ylim()),
                "units": [ax.xaxis.get_units(), ax.yaxis.get_units()],
                "labels": [ax.get_xlabel(), ax.get_ylabel()]}


def test_empty_set_limits_with_units():
    probe("Axes.set_lim.quantity", _lim_quantity, x=[-1, 6, "meters"], y=[-1, 16, "hours"])


def _axes_units(axs):
    return [[ax.xaxis.get_units(), ax.yaxis.get_units()] for ax in axs]


def _axes_labels(axs):
    return [[ax.get_xlabel(), ax.get_ylabel()] for ax in axs]


def _shared_quantity():
    with quantity_registered():
        fig = _figure()
        axs = fig.subplots(2, 1, sharex="all", sharey="all")
        x = Quantity(np.linspace(0, 1, 10), "hours")
        lines = [axs[0].plot(x, Quantity(np.linspace(1, 2, 10), "feet"))[0],
                 axs[1].plot(x, Quantity(np.linspace(3, 4, 10), "feet"))[0]]
        plotted = {"units": _axes_units(axs), "labels": _axes_labels(axs)}
        axs[0].xaxis.set_units("seconds")
        axs[1].yaxis.set_units("inches")
        fig.draw_without_rendering()
        return {"plotted": plotted, "units": _axes_units(axs), "labels": _axes_labels(axs),
                "xy": [line.get_xydata().tolist() for line in lines],
                "xlim": [list(ax.get_xlim()) for ax in axs],
                "ylim": [list(ax.get_ylim()) for ax in axs]}


def test_shared_axis_quantity():
    probe("Axis.set_units.shared_quantity", _shared_quantity)


def _default_limits(steps):
    with quantity_registered():
        fig = _figure()
        ax = fig.subplots()
        recorded = []
        for kind, *rest in steps:
            if kind == "update_units":
                name, magnitude, unit = rest
                getattr(ax, f"{name}axis").update_units(Quantity([magnitude], unit))
            elif kind == "axhline":
                ax.axhline(rest[0])
            elif kind == "axvline":
                ax.axvline(rest[0])
            elif kind == "plot":
                ax.plot(Quantity(np.arange(0, 3), "miles"), Quantity(np.arange(0, 6, 2), "feet"))
            if kind == "draw":
                fig.draw_without_rendering()
                recorded.append([list(ax.get_xlim()), list(ax.get_ylim())])
            else:
                recorded.append(None)
        return recorded


def test_empty_default_limits():
    probe("Axes.default_limits.quantity", _default_limits, steps=[
        ["update_units", "x", 10, "miles"], ["draw"], ["update_units", "y", 10, "miles"],
        ["draw"]])
    probe("Axes.default_limits.quantity", _default_limits, steps=[
        ["axhline", 30], ["plot"], ["draw"]])
    probe("Axes.default_limits.quantity", _default_limits, steps=[
        ["axvline", 30], ["plot"], ["draw"]])
    probe("Axes.default_limits.quantity", _default_limits, steps=[
        ["update_units", "x", 10, "miles"], ["axhline", 30], ["draw"]])
    probe("Axes.default_limits.quantity", _default_limits, steps=[
        ["update_units", "y", 10, "miles"], ["axvline", 30], ["draw"]])


def _jpl_consistent(date):
    import matplotlib.testing.jpl_units as units

    with jpl_registered():
        dt = datetime.datetime(*date)
        jpl = units.Epoch("ET", dt=dt)
        return {"datetime": mpl.units.registry.get_converter(dt).convert(dt, None, None),
                "epoch": mpl.units.registry.get_converter(jpl).convert(jpl, None, None)}


def test_jpl_datetime_units_consistent():
    probe("jpl.EpochConverter.convert", _jpl_consistent, date=[2009, 4, 26])


class Kernel:
    """``test_units.Kernel``: an array-like with ``__array__`` and ``shape``."""

    def __init__(self, array):
        self._array = np.asanyarray(array)

    def __array__(self, dtype=None, copy=None):
        if dtype is not None and dtype != self._array.dtype:
            if copy is not None and not copy:
                raise ValueError(
                    f"Converting array from {self._array.dtype} to "
                    f"{dtype} requires a copy"
                )

        arr = np.asarray(self._array, dtype=dtype)
        return (arr if not copy else np.copy(arr))

    @property
    def shape(self):
        return self._array.shape


def _array_like(values):
    line, = _figure().subplots().plot(Kernel(values))
    return {"xdata": line.get_xdata().tolist(), "ydata": line.get_ydata().tolist()}


def test_plot_kernel():
    probe("Axes.plot.array_like", _array_like, values=[1, 2, 3, 4, 5])
