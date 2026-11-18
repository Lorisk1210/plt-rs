"""
What the fifth round's merge changed where two agents' changes met (task
REGR5), and the corpus programs it changed: each expected value is
recorded from matplotlib 3.11.
"""

import io
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.collections as mcollections
import pltrs.pyplot as plt
import pltrs.ticker as mticker
import pltrs.units as munits
from pltrs.lines import Line2D
from build_facts import DEFAULT_INT


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


class _Length:
    """A value of the program's units, in metres."""

    def __init__(self, metres):
        self.metres = metres


class _LengthConverter(munits.ConversionInterface):
    """Counts the calls of `default_units`, as the gallery's evans_test
    callback records them."""

    defaults = 0

    @staticmethod
    def axisinfo(unit, axis):
        return munits.AxisInfo(majloc=mticker.AutoLocator(), label="m")

    @staticmethod
    def convert(obj, unit, axis):
        return [o.metres for o in obj] if np.iterable(obj) else obj.metres

    @classmethod
    def default_units(cls, x, axis):
        cls.defaults += 1
        return "m"


def test_plot_takes_the_units_of_its_data_once():
    munits.registry[_Length] = _LengthConverter()
    try:
        _LengthConverter.defaults = 0
        fig, ax = plt.subplots()
        ax.plot([_Length(1), _Length(2)], [1, 2])
        ax.fill([_Length(1), _Length(2), _Length(3)], [1, 2, 1])
        assert _LengthConverter.defaults == 2
    finally:
        del munits.registry[_Length]


@pytest.mark.parametrize("make", [mcollections.RegularPolyCollection,
                                  mcollections.StarPolygonCollection,
                                  mcollections.AsteriskPolygonCollection])
def test_regular_polygons_of_no_sizes_and_no_offsets_draw_nothing(make):
    fig, ax = plt.subplots()
    collection = ax.add_collection(make(4, sizes=[30]))
    collection.set(offsets=[[0, 0], [1, 1]])
    collection.set(offsets=np.empty((0, 2)))
    collection.set(sizes=[])
    for format in ("png", "pdf", "svg"):
        fig.savefig(io.BytesIO(), format=format)


def test_regular_polygons_of_no_sizes_fail_at_offsets():
    fig, ax = plt.subplots()
    ax.add_collection(mcollections.RegularPolyCollection(
        4, sizes=[], offsets=[[0, 0], [1, 1]], offset_transform=ax.transData))
    with pytest.raises(ValueError, match="^Expected 3-dimensional array, got 1$"):
        fig.savefig(io.BytesIO(), format="png")
    with pytest.raises(AttributeError, match="has no attribute 'translate'"):
        fig.savefig(io.BytesIO(), format="pdf")


def test_line_widths_of_text_fail_the_vector_renderers_and_no_offsets():
    fig, ax = plt.subplots()
    ax.scatter([1, 2], [1, 2]).set_linewidth("ab")
    for format in ("pdf", "svg", "ps"):
        with pytest.raises(ValueError, match="^could not convert string to float"):
            fig.savefig(io.BytesIO(), format=format)
    fig, ax = plt.subplots()
    ax.scatter([], []).set_linewidth("2")
    for format in ("png", "pdf", "svg"):
        fig.savefig(io.BytesIO(), format=format)


@pytest.mark.parametrize("projection", ["polar", None])
def test_integer_barbs_with_units_fail_on_affine_offsets_only(projection):
    fig = plt.figure()
    ax = fig.add_subplot(projection=projection)
    ax.bar(["a", "b"], [1, 2])
    ax.barbs([0, 1], [1, 2], [10, -20], [15, 5])
    if projection is None:
        with pytest.raises(TypeError, match=f"^Cannot convert fill_value nan to dtype {DEFAULT_INT.__name__}$"):
            fig.canvas.draw()
    else:
        fig.canvas.draw()


@pytest.mark.parametrize("scale", ["linear", "log"])
def test_an_animated_axis_measures_its_texts(scale):
    fig, ax = plt.subplots()
    ax.set_xscale(scale)
    axis = ax.xaxis
    axis.set_animated(True)
    axis.set_label_text("")
    fig.canvas.draw()
    assert type(axis.get_tightbbox()).__name__ == "Bbox"


def test_backgrounds_given_none_keep_matplotlibs_int_alpha():
    fig, ax = plt.subplots()
    ax.set_facecolor("0.9")
    ax.set_facecolor(None)
    fig.set_facecolor(None)
    for color in (ax.get_facecolor(), ax.patch.get_facecolor(), fig.get_facecolor()):
        assert color == (0.12156862745098039, 0.4666666666666667, 0.7058823529411765, 1)
        assert type(color[3]) is int
    assert fig.get_edgecolor() == (1.0, 1.0, 1.0, 1.0)


def test_a_figure_of_a_zorder_that_is_no_number_draws_as_its_root():
    fig = plt.figure()
    fig.add_subplot().plot([1, 2])
    fig.set_zorder("0.5")
    fig.canvas.draw()
    subfigure = fig.subfigures(1, 2)[0]
    subfigure.set_zorder("0.5")
    with pytest.raises(TypeError, match="'<' not supported between instances of 'str' and"):
        fig.canvas.draw()


def test_an_axis_sharing_categories_takes_the_category_converter():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    other = fig.add_subplot(2, 2, 4, projection="3d", sharex=ax)
    ax.xaxis.update_units(["a", "b"])
    assert other.xaxis.get_converter() is None
    assert list(other.xaxis.convert_units(["a", "c"])) == [0.0, 2.0]
    assert type(other.xaxis.get_converter()).__name__ == "StrCategoryConverter"
    fig, (left, right) = plt.subplots(1, 2, sharex=True)
    left.xaxis.update_units(["a", "b"])
    right.xaxis.convert_units(["a", "c"])
    assert type(right.xaxis.get_converter()).__name__ == "StrCategoryConverter"


class _WarningLine(Line2D):
    """A line whose `draw` warns, as seaborn's swarm points do when they
    cannot all be placed."""

    def draw(self, renderer):
        warnings.warn("drawn", UserWarning)
        super().draw(renderer)


@pytest.mark.parametrize("bbox_inches, draws", [(None, 1), ("tight", 2)])
def test_each_draw_of_a_print_gives_the_warnings_of_a_drawing_of_the_program(
        bbox_inches, draws):
    fig, ax = plt.subplots()
    ax.add_line(_WarningLine([0, 1], [0, 1]))
    with warnings.catch_warnings(record=True) as given:
        warnings.simplefilter("always")
        fig.savefig(io.BytesIO(), format="png", bbox_inches=bbox_inches)
    assert [str(warning.message) for warning in given] == ["drawn"] * draws
