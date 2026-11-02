"""
Values with matplotlib's types, objects with matplotlib's classes: what the
differential tester (``cargo xtask diff``) found pltrs answering otherwise
than matplotlib 3.11. Every expected value was recorded from matplotlib.
"""

import copy
import os
import pickle

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors as mcolors
from pltrs import patches as mpatches
from pltrs.axes import Axes
from pltrs.font_manager import FontProperties
from pltrs.lines import Line2D
from pltrs.text import Text


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_parts_of_artists_are_of_matplotlibs_classes():
    fig, ax = plt.subplots()
    ax.plot([1, 2])
    label = ax.set_xlabel("x")
    assert type(label) is Text and label is ax.xaxis.label
    label.set_color("r")
    assert ax.xaxis.label.get_color() == "r"
    tick = ax.xaxis.get_major_ticks()[0]
    assert type(tick.label1) is Text and type(tick.tick1line) is Line2D
    tick.label1.set_fontsize(20)
    assert ax.xaxis.get_major_ticks()[0].label1.get_fontsize() == 20.0
    text = ax.text(0.5, 0.5, "box", bbox=dict(fc="y"))
    assert type(text.get_bbox_patch()) is mpatches.FancyBboxPatch
    note = ax.annotate("a", (0.2, 0.2), xytext=(0.5, 0.5),
                       arrowprops=dict(arrowstyle="->"))
    assert type(note.arrow_patch) is mpatches.FancyArrowPatch
    cell_text = ax.table([["a"]])[0, 0].get_text()
    assert type(cell_text) is Text
    cell_text.set_color("b")
    assert ax.tables[0][0, 0].get_text().get_color() == "b"
    fig.canvas.draw()
    assert label.get_figure() is fig
    pickle.loads(pickle.dumps(fig))


def test_limits_are_numpy_floats():
    fig, ax = plt.subplots()
    ax.plot([1, 2])
    assert all(type(v) is np.float64 for v in ax.get_xlim() + ax.get_ybound())
    assert all(type(v) is np.float64 for v in ax.axis())
    assert ax.get_xlim() == (np.float64(-0.05), np.float64(1.05))
    ax3 = plt.figure().add_subplot(projection="3d")
    assert all(type(v) is np.float64 for v in ax3.get_zlim() + ax3.get_xbound())


def test_zorder_reads_back_as_given():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2])
    assert (line.zorder, Line2D.zorder, ax.zorder) == (2, 2, 0)
    assert type(line.get_zorder()) is int
    line.set_zorder(2.5)
    assert line.zorder == line.get_zorder() == 2.5
    line.set_zorder(7)
    assert type(line.zorder) is int
    assert ax.spines["left"].zorder == 2.5


def test_set_ticks_returns_the_ticks_and_set_ticklabels_the_labels():
    fig, ax = plt.subplots()
    ticks = ax.set_xticks([1, 2, 3])
    assert [type(t).__name__ for t in ticks] == ["XTick"] * 3
    labels = ax.set_xticklabels(["x", "y", "z"])
    assert [t.get_text() for t in labels] == ["x", "y", "z"]
    ax3 = plt.figure().add_subplot(projection="3d")
    assert len(ax3.set_zticks([0, 1], ["p", "q"])) == 2
    ax3.set_xticks([0, 1])
    assert [t.get_text() for t in ax3.set_xticklabels(["a", "b"])] == ["a", "b"]
    polar = plt.figure().add_subplot(projection="polar")
    assert len(polar.set_rticks([1, 2])) == 2


def test_rgba_alpha_is_typed_as_matplotlib_types_it():
    assert mcolors.to_rgba("r") == (1.0, 0.0, 0.0, 1)
    assert type(mcolors.to_rgba("r")[3]) is int
    assert type(mcolors.to_rgba("red")[3]) is float
    assert type(mcolors.to_rgba((1, 0, 0))[3]) is int
    assert type(mcolors.to_rgba("C0")[3]) is int
    assert mcolors.to_rgba("r", 0.5) == (1.0, 0.0, 0.0, 0.5)
    assert mcolors.to_rgba("none") == (0.0, 0.0, 0.0, 0.0)


def test_line_colours_read_back_as_given():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2], marker="o")
    tab_blue = (0.12156862745098039, 0.4666666666666667, 0.7058823529411765)
    assert line.get_color() == tab_blue and len(line.get_color()) == 3
    assert line.get_markerfacecolor() == tab_blue
    line.set_color([0.5, 0.5, 0.5])
    assert line.get_color() == [0.5, 0.5, 0.5]
    assert ax.plot([1, 2], "r")[0].get_color() == (1.0, 0.0, 0.0, 1)
    assert ax.plot([1, 2], "g--")[0].get_color() == "g"
    assert ax.plot([1], color=(0.1, 0.2, 0.3))[0].get_color() == (0.1, 0.2, 0.3)
    ax.set_prop_cycle(color=["r", (0, 1, 0)])
    assert [ax.plot([1])[0].get_color() for _ in range(2)] == ["r", (0, 1, 0)]
    text = ax.text(0, 0, "x")
    text.set_color((1, 0, 0))
    assert text.get_color() == (1, 0, 0)


def test_patch_values_read_back_as_given():
    rect = mpatches.Rectangle((0, 1), 2, 3)
    assert (rect.get_width(), rect.get_xy(), rect.get_x()) == (2, (0, 1), 0)
    assert type(rect.get_width()) is int and type(rect.get_angle()) is float
    assert rect.get_facecolor()[3] == 1 and type(rect.get_facecolor()[3]) is int
    rect.set_hatch_linewidth(2)
    assert type(rect.get_hatch_linewidth()) is int
    ellipse = mpatches.Ellipse((0, 1), 2, 3)
    assert (ellipse.get_angle(), ellipse.width, ellipse.get_center()) == (0, 2, (0, 1))
    assert type(ellipse.angle) is int
    line = plt.subplots()[1].plot([1, 2])[0]
    assert type(line.get_pickradius()) is int
    line.set_sketch_params(1, 100, 2)
    assert line.get_sketch_params() == (1, 100, 2)


def test_children_are_labelled_as_matplotlib_labels_them():
    fig, ax = plt.subplots()
    scatter = ax.scatter([1, 2], [3, 4])
    image = ax.imshow([[1, 2]])
    line = ax.axhline(0.5)
    bars = ax.bar([1], [2])
    assert [scatter.get_label(), image.get_label(), line.get_label()] == [
        "_child0", "_child1", "_child2"]
    assert bars.get_label() == "_container0"
    assert bars.patches[0].get_label() == "_nolegend_"


def test_collection_defaults():
    fig, ax = plt.subplots()
    scatter = ax.scatter([1], [1])
    assert scatter.get_path_effects() is None
    assert scatter.get_urls() == [None]
    scatter.set_urls([])
    assert scatter.get_urls() == []


def test_named_and_moved_classes():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2])
    edges = line.sticky_edges
    assert isinstance(edges, tuple) and repr(edges) == "_XYPair(x=[], y=[])"
    edges.x.append(1.5)
    assert line.sticky_edges.x == [1.5]
    assert type(ax.lines).__qualname__ == "_AxesBase.ArtistList"
    assert type(ax.xaxis.get_major_ticks()[0]).__module__.endswith(".axis")
    assert mpatches.BoxStyle.Round.__module__.endswith(".patches")
    assert pltrs.dates.AutoDateLocator.__module__.endswith(".dates")
    path = line.get_path()
    assert type(path) is pltrs.path.Path and path is line.get_path()
    assert copy.deepcopy(path)._interpolation_steps == 1
    pickle.loads(pickle.dumps(path))


def test_spines_are_patches():
    fig = plt.figure()
    with plt.rc_context({"axes.edgecolor": "r", "axes.linewidth": 5}):
        ax = fig.add_axes((0.1, 0.1, 0.8, 0.8))
    spine = ax.spines["left"]
    assert spine.get_linewidth() == 5.0
    assert spine.get_edgecolor() == (1.0, 0.0, 0.0, 1.0)
    assert spine.get_facecolor() == (0.0, 0.0, 0.0, 0.0)
    spine.set_hatch("/")
    assert spine.get_hatch() == "/"


def test_text_font_properties_are_kept_and_change_the_text():
    fig, ax = plt.subplots()
    text = ax.text(0, 0, "x")
    font = text.get_fontproperties()
    assert font is text.get_fontproperties()
    font.set_size(30)
    assert text.get_fontsize() == 30.0
    text.set_fontsize(12)
    assert font.get_size() == 12.0
    assert font == FontProperties(size=12) or font.get_size() == 12.0


def test_colormap_changed_in_place_reaches_the_drawing():
    fig, ax = plt.subplots()
    image = ax.imshow([[0.0, 1.0]], vmin=0.5, vmax=1.0, cmap="gray")
    with pytest.warns(PendingDeprecationWarning):
        image.cmap.set_under("red")
    fig.canvas.draw()
    pixel = np.asarray(fig.canvas.buffer_rgba())[240, 200]
    assert tuple(pixel) == (255, 0, 0, 255)


def test_axes_made_apart_share_their_ticks():
    fig = plt.figure()
    a = Axes(fig, (0.1, 0.1, 0.3, 0.3))
    b = Axes(fig, (0.5, 0.1, 0.3, 0.3), sharex=a, sharey=a)
    fig.add_axes(a)
    fig.add_axes(b)
    b.set(xticks=[-2, 0, 2])
    assert list(a.get_xticks()) == [-2, 0, 2]


def test_programs_run_as_under_matplotlib():
    fig, ax = plt.subplots()
    image = ax.imshow([[1, 2]])
    image.remove()
    colorbar = fig.colorbar(image, ax=ax)
    assert (colorbar.vmin, colorbar.vmax) == (1.0, 2.0)
    bars = ax.grouped_bar([[1, 2], [3, 4]], bottom=[5, 6])
    assert bars.bar_containers[0].patches[0].get_y() == 5.0
    barbs = ax.barbs([0, 1], [0, 1], [10, -20], [15, 5], barbcolor=["b", "g"])
    assert barbs.get_edgecolor().shape == (2, 4)
    filled = ax.contourf([[0, 1], [1, 2]], extend="lower")
    assert filled.extend == "lower"
    ax.pcolormesh([[1, 2]], norm=mcolors.SymLogNorm(linthresh=1))
    fig.colorbar(ax.collections[-1])
    step = mpatches.StepPatch(values=[1, 2], edges=[0, 1, 2], label="steps")
    assert step.get_label() == "steps"
    arrow = mpatches.FancyArrow(0.1, 0.1, 0.5, 0.3, width=0.05)
    assert arrow.xy.shape == (8, 2)
    table = ax.table([["a"]])
    table.set_fontsize("large")
    assert table[0, 0].get_fontsize() == 12.0
    assert os.path.isfile(os.path.join(pltrs.get_data_path(), "sample_data",
                                       "grace_hopper.jpg"))
    pltrs._enums.CapStyle.demo()


class _Length:
    """A length in centimetres, a value in units for the tests below."""

    def __init__(self, cm):
        self.cm = cm


class _LengthConverter(pltrs.units.ConversionInterface):
    """Converts `_Length`s to centimetres."""

    @staticmethod
    def convert(value, unit, axis):
        if np.iterable(value):
            return np.array([v.cm for v in value])
        return value.cm

    @staticmethod
    def axisinfo(unit, axis):
        return pltrs.units.AxisInfo(label="centimetres")

    @staticmethod
    def default_units(x, axis):
        return "cm"


@pytest.fixture
def lengths():
    pltrs.units.registry[_Length] = _LengthConverter()
    yield _Length
    del pltrs.units.registry[_Length]


def test_values_in_units_convert_where_matplotlib_converts_them(lengths):
    fig, ax = plt.subplots()
    note = ax.annotate("note", [lengths(0.5), lengths(1.5)])
    assert ax.xaxis.get_converter() is None
    ax.set_xlim(lengths(0), lengths(4))
    assert ax.xaxis.label.get_text() == "centimetres"
    line = Line2D([lengths(0), lengths(2)], [lengths(1), lengths(3)], axes=ax)
    assert line.axes is ax
    ax.add_line(line)
    assert list(line.get_xydata()[:, 0]) == [0.0, 2.0]
    text = Text(lengths(3), lengths(2.5), "label", axes=ax)
    ax.add_artist(text)
    fig.canvas.draw()
    assert note.get_window_extent().width > 0


def test_voxels_read_the_colours_of_filled_cells_only():
    ax = plt.figure().add_subplot(projection="3d")
    filled = np.zeros((2, 2, 2), dtype=bool)
    filled[0, 0, 0] = True
    colors = np.empty(filled.shape, dtype=object)
    colors[0, 0, 0] = "red"
    faces = ax.voxels(filled, facecolors=colors)
    assert list(faces) == [(0, 0, 0)]
