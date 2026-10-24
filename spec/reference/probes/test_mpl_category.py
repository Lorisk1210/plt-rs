"""matplotlib's own tests of categorical axes (``tests/test_category.py``):
category mappings and their updates, the string converter, the category
locator and formatter, the axes that ``scatter``, ``plot`` and ``bar`` make
categorical (alone and in turn), histograms of categories and category
limits.

Area ``mpl_category``.

Ops
---
``category.UnitData``  {data, update?} -> ``[keys, values]`` of the
    mapping of ``UnitData(data)``, after ``update(update)`` when given.
``category.StrCategoryConverter.convert``  {value} -> ``convert(value,
    UnitData(), axis)``: a position per string of a list, or one position
    for a string.
``category.StrCategoryLocator.tick_values``  {data} -> the ticks of a
    locator over ``UnitData(data)``.
``category.StrCategoryFormatter.__call__``  {data} -> ``[[labels(i, i)],
    [labels(i, None)]]`` for each position ``i`` of ``UnitData(data)``.
``axes.Axes.plot_units``  {calls} -> ``{"x", "y"}``: each axis of a
    ``plt.figure().subplots()`` axes after the calls, ``null`` unless it is
    categorical, else ``{"locs", "labels", "after", "units"}``: its major
    tick locations, the major formatter's ``formatter(i, i)`` at each, its
    ``formatter(n)`` one past them, and ``[keys, values]`` of the axis'
    ``UnitData``. A call is ``[method, x, y]`` with ``method`` one of
    ``scatter``, ``plot`` and ``bar`` (heights given as category names
    are converted by the y axis), or ``["xaxis.update_units", values]``.
``axes.Axes.hist``  {x} -> ``{"n", "bins"}`` of ``ax.hist(x)`` for category
    names ``x``.
``axes.Axes.set_xlim``  {x, y, left, right} -> ``get_xlim()`` after
    ``plot(x, y)`` and ``set_xlim(left, right)`` with category names.

Cases
-----
Named after the upstream tests.

Already covered
---------------
``test_overriding_units_in_plot`` by the ``mpl_rest`` figure group.

Left out
--------
Tests of values Rust's types rule out: ``test_non_string_fails``,
``test_non_string_update_fails``, ``test_convert_fail``,
``test_mixed_type_exception`` and ``test_mixed_type_update_exception``
(numbers and strings mixed), the ``bytes`` and numpy string array variants
of ``TestPlotBytes`` and ``TestPlotNumlike``; ``test_axisinfo`` and
``test_default_units`` (the classes of matplotlib's units protocol; the
locator and formatter it installs are probed through the axes).
"""

import numpy as np
import matplotlib.category as cat
from matplotlib.figure import Figure

from spec_support import ProbeSet

probes = ProbeSet("mpl_category")


def _mapping(unit):
    return [list(unit._mapping.keys()), list(unit._mapping.values())]


def _unit_data(data, update=None):
    unit = cat.UnitData(data)
    if update is not None:
        unit.update(update)
    return _mapping(unit)


class FakeAxis:
    def __init__(self, units):
        self.units = units


def _convert(value):
    unit = cat.UnitData()
    return cat.StrCategoryConverter().convert(value, unit, FakeAxis(unit))


def _locator(data):
    return cat.StrCategoryLocator(cat.UnitData(data)._mapping).tick_values(None, None)


def _formatter(data):
    labels = cat.StrCategoryFormatter(cat.UnitData(data)._mapping)
    return [[labels(i, i) for i in range(len(data))],
            [labels(i, None) for i in range(len(data))]]


def _axis_summary(axis):
    if not isinstance(axis.units, cat.UnitData):
        return None
    locs = axis.get_majorticklocs()
    return {"locs": locs, "labels": [axis.major.formatter(i, i) for i in locs],
            "after": axis.major.formatter(len(locs)), "units": _mapping(axis.units)}


def _plot_units(calls):
    ax = Figure().subplots()
    for method, *values in calls:
        if method == "xaxis.update_units":
            ax.xaxis.update_units(*values)
        else:
            getattr(ax, method)(*values)
    return {"x": _axis_summary(ax.xaxis), "y": _axis_summary(ax.yaxis)}


def _probe_plots(*calls):
    return probes.probe("axes.Axes.plot_units", _plot_units, calls=list(calls))


PLOTTERS = ["scatter", "plot", "bar"]


class TestUnitData:
    def test_unit(self):
        for data, locs in [(["hello world"], [0]), (["Здравствуйте мир"], [0]),
                           (['A', "np.nan", 'B', "3.14", "мир"], [0, 1, 2, 3, 4])]:
            r = probes.probe("category.UnitData", _unit_data, data=data)
            assert r == [data, locs]

    def test_update(self):
        r = probes.probe("category.UnitData", _unit_data, data=['a', 'd'], update=['b', 'd', 'e'])
        assert r == [['a', 'd', 'b', 'e'], [0, 1, 2, 3]]


class TestStrCategoryConverter:
    def test_convert(self):
        for vals in [["Здравствуйте мир"], ["hello world"], ['a', 'b', 'c'], ["1", "2"],
                     [chr(c) for c in range(ord("A"), ord("Z") + 1)]]:
            r = probes.probe("category.StrCategoryConverter.convert", _convert, value=vals)
            np.testing.assert_allclose(r, range(len(vals)))

    def test_convert_one_string(self):
        for value in ["hi", "мир"]:
            assert probes.probe("category.StrCategoryConverter.convert", _convert,
                                value=value) == 0


class TestStrCategoryLocator:
    def test_StrCategoryLocator(self):
        locs = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        r = probes.probe("category.StrCategoryLocator.tick_values", _locator,
                         data=[str(j) for j in locs])
        np.testing.assert_array_equal(r, locs)

    def test_StrCategoryLocatorPlot(self):
        for plotter in PLOTTERS:
            r = _probe_plots([plotter, [1, 2, 3], ["a", "b", "c"]])
            np.testing.assert_array_equal(r["y"]["locs"], range(3))


class TestStrCategoryFormatter:
    CASES = [["hello", "world", "hi"], ["Здравствуйте", "привет"]]

    def test_StrCategoryFormatter(self):
        for ydata in self.CASES:
            r = probes.probe("category.StrCategoryFormatter.__call__", _formatter, data=ydata)
            assert r == [ydata, ydata]

    def test_StrCategoryFormatterPlot(self):
        for ydata in self.CASES:
            for plotter in PLOTTERS:
                r = _probe_plots([plotter, list(range(len(ydata))), ydata])
                assert r["y"]["labels"] == ydata and r["y"]["after"] == ""


class TestPlotBytes:
    def test_plot_bytes(self):
        for plotter in PLOTTERS:
            r = _probe_plots([plotter, ['a', 'b', 'c'], [4, 6, 5]])
            assert r["x"]["labels"] == ['a', 'b', 'c']


class TestPlotNumlike:
    def test_plot_numlike(self):
        for plotter in PLOTTERS:
            r = _probe_plots([plotter, ['1', '11', '3'], [4, 6, 5]])
            assert r["x"]["units"] == [['1', '11', '3'], [0, 1, 2]]


class TestPlotTypes:
    X = ["hello", "happy", "world"]
    XY = [2, 6, 3]
    Y = ["Python", "is", "fun"]
    YX = [3, 4, 5]

    def test_plot_unicode(self):
        for plotter in PLOTTERS:
            _probe_plots([plotter, ['Здравствуйте', 'привет'], [0, 1]])

    def test_plot_xaxis(self):
        for plotter in PLOTTERS:
            _probe_plots([plotter, self.X, self.XY])

    def test_plot_yaxis(self):
        for plotter in PLOTTERS:
            _probe_plots([plotter, self.YX, self.Y])

    def test_plot_xyaxis(self):
        for plotter in PLOTTERS:
            _probe_plots([plotter, self.X, self.Y])

    def test_update_plot(self):
        for plotter in PLOTTERS:
            r = _probe_plots([plotter, ['a', 'b'], ['e', 'g']],
                             [plotter, ['a', 'b', 'd'], ['f', 'a', 'b']],
                             [plotter, ['b', 'c', 'd'], ['g', 'e', 'd']])
            assert r["x"]["labels"] == ['a', 'b', 'd', 'c']
            assert r["y"]["labels"] == ['e', 'g', 'f', 'a', 'b', 'd']

    def test_update_plot_heterogenous_plotter(self):
        _probe_plots(["scatter", ['a', 'b'], ['e', 'g']],
                     ["plot", ['a', 'b', 'd'], ['f', 'a', 'b']],
                     ["bar", ['b', 'c', 'd'], ['g', 'e', 'd']])


def test_no_deprecation_on_empty_data():
    _probe_plots(["xaxis.update_units", ["a", "b"]], ["plot", [], []])


def _hist(x):
    n, bins, _ = Figure().subplots().hist(x)
    return {"n": n, "bins": bins}


def test_hist():
    r = probes.probe("axes.Axes.hist", _hist, x=['a', 'b', 'a', 'c', 'ff'])
    np.testing.assert_allclose(r["n"], [2., 0., 0., 1., 0., 0., 1., 0., 0., 1.])


def _set_xlim(x, y, left, right):
    ax = Figure().subplots()
    ax.plot(x, y)
    ax.set_xlim(left, right)
    return ax.get_xlim()


def test_set_lim():
    probes.probe("axes.Axes.set_xlim", _set_xlim, x=["a", "b", "c", "d"], y=[1, 2, 3, 4],
                 left="b", right="c")
