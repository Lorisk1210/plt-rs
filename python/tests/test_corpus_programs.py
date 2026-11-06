"""
What the corpora of real-world programs (``cargo xtask diff --corpus``,
``spec/diff/corpora.toml``) found pltrs doing otherwise than matplotlib
3.11. Every expected value was recorded from matplotlib.
"""

import io
import logging
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import ticker
from pltrs.collections import PolyCollection
from pltrs.legend import Legend
from pltrs.transforms import IdentityTransform


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


class _Lookups(logging.Handler):
    """Counts the font manager's warnings about families no font has."""

    def __init__(self):
        super().__init__(logging.WARNING)
        self.count = 0

    def emit(self, record):
        if "not found" in record.getMessage():
            self.count += 1


@pytest.fixture
def lookups():
    """The warnings of a family no font has, counted."""
    handler = _Lookups()
    logging.getLogger().addHandler(handler)
    pltrs.rcParams["font.family"] = "Source Code Pro"
    yield handler
    logging.getLogger().removeHandler(handler)


def _counted(lookups, action):
    lookups.count = 0
    action()
    return lookups.count


def test_hexbin_values_of_another_number_than_the_points():
    x = np.array([1., 2, np.nan, 4, 5])
    y = np.array([1., 2, 3, 4, 5])
    _, ax = plt.subplots()
    assert ax.hexbin(x, y, np.arange(10.), gridsize=3).get_array().tolist() == [0, 1, 2, 3]
    assert ax.hexbin(x, y, [1., np.nan, 3, 4, 5, 6],
                     gridsize=3).get_array().tolist() == [1, 3, 4]
    with pytest.raises(IndexError, match="index 3 is out of bounds for axis 0 with size 3"):
        ax.hexbin(x, y, np.arange(3.), gridsize=3)
    # matplotlib sets the keywords after the collection's own arguments.
    assert ax.hexbin([1, 2], [1, 2], gridsize=4, linewidth=0.25).get_linewidth() == [0.25]


def test_line_widths_and_sizes_given_as_strings():
    _, ax = plt.subplots()
    line, = ax.plot([1, 2], [1, 2], mew="1", marker=".")
    assert line.get_markeredgewidth() == "1"
    line, = ax.plot([1, 2], [1, 2], lw="2", ms="3")
    assert (line.get_linewidth(), line.get_markersize()) == (2.0, 3.0)
    plt.gcf().canvas.draw()


def test_hatch_color_takes_colormap_colors():
    color = plt.get_cmap("Oranges")(0.3)
    pltrs.rcParams["hatch.color"] = color
    assert pltrs.rcParams["hatch.color"] == color


def test_text_box_naming_the_boxes_own_arguments():
    _, ax = plt.subplots()
    with pytest.raises(TypeError, match=r"^FancyBboxPatch.__init__\(\) got multiple "
                                        r"values for argument 'width'$"):
        ax.text(0, 0, "x", bbox={"width": 3})
    with pytest.raises(TypeError, match=r"FancyBboxPatch\(\) got multiple values for "
                                        r"keyword argument 'transform'$"):
        ax.text(0, 0, "x", bbox={"transform": IdentityTransform()})
    with pytest.raises(AttributeError, match="unexpected keyword argument 'foo'"):
        ax.text(0, 0, "x", bbox={"foo": 1})
    assert len(ax.texts) == 0


def test_axes_failing_their_keywords_are_not_added():
    fig = plt.figure()
    with pytest.raises(AttributeError, match="unexpected keyword argument 'axisbg'"):
        fig.add_axes([0.1, 0.1, .8, .8], axisbg='.75')
    with pytest.raises(AttributeError, match="unexpected keyword argument 'bar'"):
        plt.subplot(2, 1, 2, bar=2)
    assert fig.axes == []


def test_pylab_and_pyplot_names():
    import pltrs.pylab as pylab
    for name in ("cm", "subplot", "axhline", "colorbar", "flatten", "silent_list", "mpl",
                 "Figure", "GridSpec", "cycler"):
        assert hasattr(pylab, name), name
    namespace = {}
    exec("from pltrs.pylab import *", namespace)
    assert "subplot" in namespace and "cm" in namespace
    public = {name for name in dir(plt) if not name.startswith("_")}
    assert {"AbstractContextManager", "Enum", "IO", "cast", "overload"} <= public
    assert not {"image", "is_interactive", "rc_file_defaults"} & public


def test_fixed_formatter_warns_without_a_fixed_locator():
    _, ax = plt.subplots()
    ax.xaxis.set_major_locator(ticker.MultipleLocator(1.0))
    with pytest.warns(UserWarning, match="FixedFormatter should only be used together"):
        ax.xaxis.set_major_formatter(ticker.FixedFormatter(["a", "b"]))
    ax.yaxis.set_major_locator(ticker.FixedLocator([0, 1]))
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        ax.yaxis.set_major_formatter(ticker.FixedFormatter(["a", "b"]))
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        plt.subplots()[1].set_xticklabels(["0", "1M"])
    assert [str(w.message)[:26] for w in caught] == ["set_ticklabels() should on"]


def test_function_scale_once_the_transforms_are_read():
    _, ax = plt.subplots()
    ax.text(0.5, 0.1, "x", transform=ax.transAxes)
    ax.set_xscale("function", functions=(lambda x: x ** 2, lambda x: x ** 0.5))
    assert ax.get_xscale() == "function"


def test_no_path_effects():
    fig, ax = plt.subplots()
    text = ax.text(0.5, 0.5, "x", path_effects=None)
    assert text.get_path_effects() is None
    fig.canvas.draw()


def test_a_colour_given_colours_the_hatches():
    _, ax = plt.subplots()
    filled = ax.fill_between([0, 1], [0, 1], color="red", hatch="///", alpha=0.4,
                             edgecolor="black")
    assert filled.get_hatchcolor().tolist() == [[1, 0, 0, 0.4]]
    polygons = PolyCollection([[(0, 0), (1, 0), (1, 1)]], color="red", hatch="/",
                              edgecolor="blue")
    assert polygons.get_hatchcolor().tolist() == [[1, 0, 0, 1]]


def test_a_legend_made_for_axes_is_not_their_legend():
    _, ax = plt.subplots()
    lines = [ax.plot([0, 1], [i, i + 1])[0] for i in range(4)]
    first = ax.legend(lines[:2], ["A", "B"], loc="upper right")
    second = Legend(ax, lines[2:], ["C", "D"], loc="lower right")
    assert ax.get_legend() is first and second not in ax.get_children()
    ax.add_artist(second)
    assert ax.get_legend() is first
    assert second in ax.get_children() and list(ax.artists) == [second]


def test_an_expanded_legend_is_as_wide_as_its_last_draw():
    pltrs.rcParams.update({"figure.constrained_layout.use": True, "font.size": 4,
                           "legend.fontsize": 3})
    x = np.linspace(0.1, 10 * np.pi, 1000)
    fig, ax = plt.subplots(figsize=(5.7 / 2.54, 1.0 / 2.54))
    ax.plot(x, np.sin(x), label="Sine")
    ax.plot(x, np.cos(x), label="Cosine")
    legend = ax.legend(bbox_to_anchor=(0.0, .9, 1.02, 0.1), frameon=False,
                       mode="expand", ncol=2, loc="lower left")
    ax.set_xticks([]), ax.set_yticks([])
    fig.canvas.draw()
    # The layout measured the legend as narrow as its entries.
    assert ax.get_position().width == pytest.approx(0.9628625263157895, rel=1e-6)
    assert legend.get_window_extent().width == pytest.approx(216.2302911286089, rel=1e-6)


def test_tick_labels_with_a_box():
    fig, ax = plt.subplots(figsize=(3, 2))
    for label in ax.get_xticklabels():
        label.set_bbox({"facecolor": "red", "edgecolor": "None", "alpha": 0.65})
    fig.canvas.draw()
    pixels = np.asarray(fig.canvas.buffer_rgba())
    assert ((pixels[..., 0] > 200) & (pixels[..., 1] < 150)).sum() == 3067


@pytest.mark.parametrize("layout, saves", [
    ({}, [("png", 10, 8), ("pdf", 10, 10), ("tight", 22, 22)]),
    ({"figure.constrained_layout.use": True},
     [("png", 24, 22), ("pdf", 28, 28), ("svg", 28, 28), ("tight", 30, 30)]),
])
def test_font_lookups_of_prints_drawn_twice(lookups, layout, saves):
    pltrs.rcParams.update(layout)
    for name, first, second in saves:
        fig, ax = plt.subplots(figsize=(2, 2))
        ax.set(xticks=[0, 1], yticks=[])
        kwargs = ({"format": "png", "bbox_inches": "tight"} if name == "tight"
                  else {"format": name})
        counts = [_counted(lookups, lambda: fig.savefig(io.BytesIO(), **kwargs))
                  for _ in range(2)]
        assert counts == [first, second], name
        plt.close(fig)


def test_font_lookups_after_tight_layout(lookups):
    pltrs.rcParams["font.sans-serif"] = ["Roboto Condensed"]
    pltrs.rcParams["font.family"] = "sans-serif"
    fig = plt.figure(figsize=(6, 2.5))
    ax = plt.subplot()
    ax.plot([0, 1], [0, 1])
    ax.set_yticks([0, 1]), ax.set_xticks([0, 1])
    assert _counted(lookups, plt.tight_layout) == 10
    assert _counted(lookups, lambda: fig.savefig(io.BytesIO(), format="pdf")) == 32
    assert _counted(lookups, lambda: fig.savefig(io.BytesIO(), format="png")) == 28


def test_font_lookups_of_measures(lookups):
    fig, ax = plt.subplots(figsize=(2, 2))
    ax.set(xticks=[0, 1], yticks=[])
    renderer = fig.canvas.get_renderer()
    assert _counted(lookups, lambda: fig.get_tightbbox(renderer)) == 6
    assert _counted(lookups, lambda: fig.get_tightbbox(renderer)) == 4
    assert _counted(lookups, fig.canvas.draw) == 8
    assert _counted(lookups, fig.draw_without_rendering) == 6


def test_font_lookups_of_a_title_over_tick_labels(lookups):
    pltrs.rcParams["font.sans-serif"] = ["Roboto Condensed"]
    pltrs.rcParams["font.family"] = "sans-serif"
    fig = plt.figure(figsize=(7, 7))
    ax = plt.subplot()
    ax.set_xticks(np.linspace(0, 1, 5)), ax.set_yticks(np.linspace(0, 1, 5))
    ax.set_title("Title", family="Roboto")
    # 55 for the families of the ticks, 4 for the title's.
    assert _counted(lookups, lambda: fig.savefig(io.BytesIO(), format="png")) == 59
