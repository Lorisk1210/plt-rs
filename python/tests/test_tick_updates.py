"""
The program's tick helpers called where matplotlib calls them, as often,
in the same order and with the same arguments, with the figure free to
read and change meanwhile (`pltrs._tick_updates`, pltrs (Rust)'s tick
sites and lent figures). The expected values are real matplotlib 3.11's
for the same programs.
"""

import collections
import io

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors, ticker, units


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """Every test starts and ends with matplotlib's default rcParams and
    closes its figures."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _counted(layout=None, save=False, bbox=None, minor=False, top=False):
    """How often a draw (or a save) calls a locator and formatters of the
    program's."""
    calls = collections.Counter()
    fig, ax = plt.subplots(layout=layout)
    ax.plot([1, 2, 3])

    class Located(ticker.MaxNLocator):
        def __call__(self):
            calls["locator"] += 1
            return super().__call__()

    def label(name):
        def write(x, pos=None):
            calls[name] += 1
            return f"{x:.2f}"
        return ticker.FuncFormatter(write)

    ax.xaxis.set_major_locator(Located(4))
    ax.xaxis.set_major_formatter(label("x"))
    ax.yaxis.set_major_formatter(label("y"))
    if minor:
        ax.minorticks_on()
        ax.xaxis.set_minor_formatter(label("x minor"))
    if top:
        ax.xaxis.tick_top()
        ax.set_title("title")
    if save:
        fig.savefig(io.BytesIO(), format="png", bbox_inches=bbox)
    else:
        fig.canvas.draw()
    return dict(calls)


@pytest.mark.parametrize("kwargs, expected", [
    ({}, {"locator": 3, "x": 18, "y": 33}),
    ({"minor": True}, {"locator": 9, "x": 18, "x minor": 36, "y": 33}),
    ({"top": True}, {"locator": 6, "x": 36, "y": 66}),
    ({"layout": "tight", "save": True}, {"locator": 16, "x": 96, "y": 176}),
    ({"layout": "constrained", "save": True},
     {"locator": 16, "x": 96, "y": 176}),
    ({"save": True, "bbox": "tight"}, {"locator": 11, "x": 66, "y": 121}),
])
def test_tick_helpers_are_called_where_matplotlib_calls_them(kwargs, expected):
    # A plain draw updates each axis' ticks three times; a title over top
    # ticks, a layout engine, a tight box and the draw before a print
    # update them again.
    assert _counted(**kwargs) == expected


def test_a_formatter_reads_and_changes_its_figure_while_it_is_laid_out():
    fig, ax = plt.subplots(layout="constrained")
    ax.plot([0, 10], [0, 3])
    seen = []

    def label(x, pos):
        seen.append(tuple(float(v) for v in ax.get_xlim()))
        title = ax.set_title(f"{len(seen)}")
        seen.append(type(title).__name__)
        return f"{x:g}/{ax.get_xlim()[1]:g}"

    ax.xaxis.set_major_formatter(ticker.FuncFormatter(label))
    fig.canvas.draw()
    assert (len(seen), seen[:2]) == (208, [(-0.5, 10.5), "Text"])
    assert ax.get_title() == "207"
    assert [t.get_text() for t in ax.get_xticklabels()] == [
        "-2/10.5", "0/10.5", "2/10.5", "4/10.5", "6/10.5", "8/10.5",
        "10/10.5", "12/10.5"]
    assert fig.stale


def test_a_function_scale_is_called_by_the_tick_updates_as_in_matplotlib():
    fig, ax = plt.subplots()
    ax.plot([0, 4], [0, 1])
    shapes = collections.Counter()

    def forward(values):
        shapes[np.shape(values)] += 1
        return np.asarray(values) ** 0.5

    ax.set_xscale("function", functions=(forward, lambda v: np.asarray(v) ** 2))
    shapes.clear()
    fig.canvas.draw()
    # Each tick through the scale's transform, as `_update_ticks` asks it
    # (the paths drawn are counted apart).
    assert shapes[(1, 1)] == 30


def test_contour_labels_are_written_as_matplotlib_writes_them():
    x = np.linspace(-1, 1, 30)
    X, Y = np.meshgrid(x, x)
    fig, ax = plt.subplots()
    cs = ax.contour(X, Y, X**2 + Y**2, levels=[0.25, 0.5, 0.75])
    written = []

    def label(value):
        written.append(round(float(value), 2))
        return f"{value:.2f}"

    texts = ax.clabel(cs, fmt=label)
    # Each level measured, then each of its labels written as it is added.
    assert written == [0.25, 0.25, 0.5, 0.5, 0.75, 0.75]
    assert [t.get_text() for t in texts] == ["0.25", "0.50", "0.75"]


def test_a_converter_is_asked_for_default_limits_as_in_matplotlib():
    asked = []

    class Foo:
        def __init__(self, value):
            self.value = value

    class FooConverter(units.ConversionInterface):
        @staticmethod
        def axisinfo(unit, axis):
            asked.append(unit)
            return units.AxisInfo(majloc=ticker.IndexLocator(8, 0), label="foo")

        @staticmethod
        def convert(obj, unit, axis):
            return [o.value / unit for o in obj] if np.iterable(obj) else obj.value / unit

        @staticmethod
        def default_units(x, axis):
            return 1.0

    units.registry[Foo] = FooConverter()
    try:
        fig, (ax1, ax2) = plt.subplots(1, 2)
        values = [Foo(v) for v in range(0, 50, 2)]
        ax2.plot(values, range(25), "o", xunits=2.0)
        ax1.plot(values, range(25))
        fig.canvas.draw()
    finally:
        del units.registry[Foo]
    # The second axes is asked again for the default limits of the
    # limits it has not changed yet (`set_default_intervals`).
    assert asked == [2.0, 1.0, 1.0]


def test_the_figure_is_lent_to_the_program_only_while_it_ticks():
    fig, ax = plt.subplots()
    ax.plot([0, 1])
    events = []

    def label(x, pos):
        # Drawing again from the formatter lays the figure out inside.
        if not events:
            events.append("nested")
            fig.canvas.draw()
        return f"{x:.1f}"

    ax.xaxis.set_major_formatter(ticker.FuncFormatter(label))
    fig.canvas.draw()
    assert events == ["nested"]
    assert [t.get_text() for t in ax.get_xticklabels()][:2] == ["-0.2", "0.0"]
