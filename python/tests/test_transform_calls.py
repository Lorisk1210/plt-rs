"""
Where a draw calls the program's scales, transforms and curvilinear
grids, as often and with the arrays matplotlib calls them with: the tick
labels measured and drawn, the tick lines and gridlines drawn (each path,
then its points), the spines, the view limits mapped once while they
stay, a line's path kept while its data and transform stay, the grid of
a curvilinear axes computed when its floating axes are made. The
expected values are real matplotlib 3.11's for the same programs.
"""

import io

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import scale as mscale, ticker as mticker, transforms as mtransforms
from pltrs.mpl_toolkits.axisartist import Axes as ArtistAxes
from pltrs.mpl_toolkits.axisartist.grid_helper_curvelinear import GridHelperCurveLinear


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """Every test starts and ends with matplotlib's default rcParams and
    closes its figures."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _call(x):
    """What a call is recorded as: the shape of the array given and its
    first value."""
    return np.shape(x), round(float(np.ravel(x)[0]), 4) if np.size(x) else None


def _function_scale_calls(layout, saves):
    """The calls of a function scale's forward function in each of
    *saves* saves of a figure laid out with *layout*."""
    calls = []

    def forward(x):
        calls.append(_call(x))
        return np.sqrt(x)

    fig, ax = plt.subplots(figsize=(3, 2), layout=layout)
    ax.plot([1, 2, 3], [1, 4, 9])
    ax.set_yscale("function", functions=(forward, lambda x: x ** 2))
    ax.set_yticks([1, 4, 9])
    ax.set_xticks([1, 2, 3])
    saved = []
    for _ in range(saves):
        calls.clear()
        fig.savefig(io.BytesIO(), format="png")
        saved.append(list(calls))
    return saved


#: matplotlib's calls in a save of the figure without a layout engine: the
#: x tick labels measured (their y through the scale), the view limits
#: mapped once, the bottom spine, the tick updates of the y axis, its
#: labels measured, its ticks drawn, the line, the spines.
PLAIN_SAVE = [
    ((1,), 0.0), ((4,), 0.81), ((1,), 0.0), ((1,), 0.0),
    ((1,), 0.0), ((1,), 0.0), ((1,), 0.0), ((1,), 0.0),
    ((1,), 0.0), ((1,), 0.0), ((1,), 0.0), ((1,), 0.0),
    ((1,), 0.0), ((1,), 0.0), ((1,), 0.0), ((1,), 0.0),
    ((2,), 0.0), ((2, 1), 0.81), ((1, 1), 1.0), ((1, 1), 4.0),
    ((1, 1), 9.0), ((1,), 1.0), ((1,), 4.0), ((1,), 9.0),
    ((1,), 1.0), ((1,), 1.0), ((1,), 1.0), ((1,), 4.0),
    ((1,), 4.0), ((1,), 4.0), ((1,), 9.0), ((1,), 9.0),
    ((1,), 9.0), ((2, 1), 0.81), ((1, 1), 1.0), ((1, 1), 4.0),
    ((1, 1), 9.0), ((1,), 1.0), ((1,), 4.0), ((1,), 9.0),
    ((2,), 0.81), ((2, 1), 0.81), ((1, 1), 1.0), ((1, 1), 4.0),
    ((1, 1), 9.0), ((3,), 1.0), ((3,), 1.0), ((2,), 0.81),
    ((2,), 0.81), ((2,), 0.0), ((2,), 1.0),
]

#: The first calls of a save with constrained layout, which measures the
#: axes for the layout first.
CONSTRAINED_FIRST = [
    ((1,), 0.0), ((4,), 0.81), ((1,), 0.0), ((1,), 0.0),
    ((2,), 0.0), ((1,), 0.0), ((1,), 0.0), ((1,), 0.0),
    ((2, 1), 0.81), ((1, 1), 1.0), ((1, 1), 4.0), ((1, 1), 9.0),
    ((2, 1), 0.81), ((1, 1), 1.0), ((1, 1), 4.0), ((1, 1), 9.0),
    ((1,), 1.0), ((1,), 4.0), ((1,), 9.0), ((2,), 0.81),
    ((2, 1), 0.81), ((1, 1), 1.0), ((1, 1), 4.0), ((1, 1), 9.0),
]


def test_a_function_scale_is_called_at_each_part_drawn_as_in_matplotlib():
    assert _function_scale_calls(None, 1) == [PLAIN_SAVE]


def test_a_layout_and_a_second_save_call_a_function_scale_as_in_matplotlib():
    first, second = _function_scale_calls("constrained", 2)
    assert first[:len(CONSTRAINED_FIRST)] == CONSTRAINED_FIRST
    # The second save maps neither the view limits nor the line again.
    assert (len(first), len(second)) == (175, 172)


@pytest.mark.parametrize("points, expected", [(500, [2, 0]), (1500, [2, 2])])
def test_a_line_keeps_its_mapped_path_unless_it_draws_a_slice(points, expected):
    # A line of more than 1000 sorted points draws the slice in view,
    # mapped anew at every draw.
    calls = []

    def forward(x):
        calls.append(np.shape(x))
        return x

    fig, ax = plt.subplots()
    x = np.linspace(0, 1, points)
    ax.plot(x, x)
    ax.set_yscale("function", functions=(forward, lambda x: x))
    counts = []
    for _ in range(2):
        calls.clear()
        fig.canvas.draw()
        counts.append(calls.count((points,)))
    assert counts == expected


def test_a_scale_class_of_the_program_maps_rows_where_matplotlib_draws():
    calls = []

    class Doubling(mtransforms.Transform):
        input_dims = output_dims = 1

        def transform_non_affine(self, a):
            calls.append(np.shape(a))
            return np.asarray(a) * 2

        def inverted(self):
            return Halving()

    class Halving(mtransforms.Transform):
        input_dims = output_dims = 1

        def transform_non_affine(self, a):
            return np.asarray(a) / 2

        def inverted(self):
            return Doubling()

    class DoubledScale(mscale.ScaleBase):
        name = "pltrs_test_doubled"

        def __init__(self):
            pass

        def get_transform(self):
            return Doubling()

        def set_default_locators_and_formatters(self, axis):
            axis.set_major_locator(mticker.FixedLocator([0, 1, 2]))

    mscale.register_scale(DoubledScale)
    fig, ax = plt.subplots()
    ax.plot([0, 1, 2], [0, 1, 2])
    ax.set_yscale("pltrs_test_doubled")
    calls.clear()
    fig.canvas.draw()
    # Rows where parts are drawn and measured, columns where the ticks
    # are updated.
    assert (sorted(set(calls)), len(calls)) == ([(1,), (1, 1), (2,), (2, 1), (3,), (4,)], 82)


def test_a_curvilinear_grid_is_computed_where_matplotlib_computes_it():
    calls = []

    def tr(x, y):
        calls.append(("tr", np.shape(x)))
        return x, y - x

    def inv_tr(x, y):
        calls.append(("inv", np.shape(x)))
        return x, y + x

    fig = plt.figure()
    ax = fig.add_subplot(axes_class=ArtistAxes, grid_helper=GridHelperCurveLinear((tr, inv_tr)))
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    assert calls == []
    ax.axis["t"] = ax.new_floating_axis(0, 3)
    # The grid over the limits, then the floating axis' line.
    assert calls == [("inv", (400,))] + [("tr", (100,))] * 20 + [("inv", (400,)), ("tr", (100,))]
    calls.clear()
    fig.canvas.draw()
    # The grid kept; the axis' line, its ticks, its line again and its
    # label (without text).
    assert calls == ([("inv", (400,)), ("tr", (100,))] + [("tr", (11,))] * 5
                     + [("inv", (400,)), ("tr", (100,))] + [("tr", (1,))] * 5)
