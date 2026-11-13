"""
What the libraries of ``cargo xtask ecosystem suite`` need of matplotlib,
beyond `test_eco6`: legends their own class removes (ultraplot's),
containers forwarding the names they lack to an Axes (ultraplot's
external axes), sizes of the program's reading their divider while the
figure is laid out (mplhep's ``RemainderFixed``), axis labels moved along
their axis (mplhep's ``xlabel_sci_adjust``), colormaps named in a
registry the program put in place of matplotlib's (ultraplot's), copies
of artists added back to their Axes and alignments of a ``str`` class of
the program's (mplcursors'), images freed once removed, error bars of
several colours and bars of dates (holoviews'), and data limits a
program stretches itself, then recomputes (mpl-interactions'), and child
Axes placed by locators of the program's, the parents child Axes name and
colorbars given a formatter (ultraplot's colorbar panels). Each test
does what a library does, without the library. The expected values
were recorded from matplotlib 3.11.2 running the same code.
"""

import copy
import gc
import io
import weakref

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import cm, colors as mcolors, ticker
from pltrs.axes import Axes
from pltrs.collections import LineCollection
from pltrs.legend import Legend
from pltrs.mpl_toolkits.axes_grid1 import axes_size, make_axes_locatable
from pltrs.transforms import Bbox, TransformedBbox

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


class _SelfForgettingLegend(Legend):
    """A legend whose removal makes its Axes forget it, as ultraplot's
    does (``self.axes.legend_ = None`` before the base removal)."""

    def remove(self):
        axes = self.axes
        if axes is not None and axes.legend_ is self:
            axes.legend_ = None
        super().remove()


def test_a_legend_forgetting_itself_in_its_own_remove_is_removed_once():
    fig, ax = plt.subplots()
    ax.plot([0, 1], label="line")
    handles, labels = ax.get_legend_handles_labels()
    legend = _SelfForgettingLegend(ax, handles, labels)
    # As `Axes.legend` places a legend.
    ax.legend_ = legend
    legend._remove_method = lambda legend: setattr(ax, "legend_", None)
    legend.remove()
    assert ax.legend_ is None
    assert ax.get_legend() is None
    assert legend not in ax.get_children()
    ax.legend_ = None
    assert ax.legend_ is None


class _Forwarding(Axes):
    """An Axes forwarding the names it lacks to another Axes (ultraplot's
    external axes containers)."""

    def __getattr__(self, name):
        other = self.__dict__.get("_other")
        if other is None:
            raise AttributeError(name)
        return getattr(other, name)


def test_an_axes_is_its_own_axes_whatever_it_forwards():
    fig = plt.figure()
    other = fig.add_subplot(1, 2, 1)
    ax = fig.add_subplot(1, 2, 2, axes_class=_Forwarding)
    ax.__dict__["_other"] = other
    assert ax._axes is ax
    assert other._axes is other


class _Square(axes_size.Scaled):
    """The width and height of a square Axes in a divider, read from the
    divider's position while the figure is laid out (mplhep's
    ``RemainderFixed``)."""

    def __init__(self, divider, margin):
        self.divider = divider
        self.margin = margin

    def get_size(self, renderer):
        fig = self.divider._fig
        box = Bbox.from_bounds(*self.divider.get_position()).transformed(
            fig.transFigure)
        return 0, min(box.width, box.height) / fig.dpi - self.margin


def test_a_size_of_the_program_reads_its_divider_while_laid_out():
    fig, ax = plt.subplots(figsize=(6, 4))
    divider = make_axes_locatable(ax)
    cax = divider.append_axes(
        "right", size=axes_size.Fixed(0.4), pad=axes_size.Fixed(0.1))
    sizes = [axes_size.Fixed(0.1), axes_size.Fixed(0.4)]
    divider.set_horizontal([_Square(divider, 0.5), *sizes])
    divider.set_vertical([_Square(divider, 0.5), *sizes])
    fig.savefig(io.BytesIO(), format="png")
    assert ax.get_position().bounds == pytest.approx(
        (0.125, 0.11, 0.43, 0.645))
    assert cax.get_position().bounds == pytest.approx(
        (0.5716666666666668, 0.11, 0.06666666666666665, 0.645))


def test_an_axis_label_moves_along_its_axis_after_a_draw():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    ax.set_xlabel("time", loc="right")
    fig.canvas.draw()
    ax.xaxis.label.set_x(0.7)
    fig.canvas.draw()
    label = ax.xaxis.label
    assert (label.get_position()[0], label.get_ha()) == (0.7, "right")
    assert label.get_window_extent().x1 == pytest.approx(267.0)
    ax.xaxis.label.set_x(1.0)
    fig.canvas.draw()
    assert ax.xaxis.label.get_window_extent().x1 == pytest.approx(360.0)


class _Registry(cm.ColormapRegistry):
    """A registry of colormaps of the program's, knowing names matplotlib's
    does not (ultraplot's ``ColormapDatabase``)."""

    def __init__(self, base):
        super().__init__(dict(base.items()))
        self._cmaps["Ends"] = mcolors.ListedColormap(["k", "w"], name="Ends")


@pytest.fixture
def program_registry(monkeypatch):
    """The program's registry in place of matplotlib's, and its colormap
    the default one."""
    registry = _Registry(pltrs.colormaps)
    monkeypatch.setattr(cm, "_colormaps", registry)
    monkeypatch.setattr(pltrs, "colormaps", registry)
    pltrs.rcParams["image.cmap"] = "Ends"
    return registry


def test_values_map_through_a_colormap_of_the_programs_registry(
        program_registry):
    lines = LineCollection([[(0, 0), (1, 1)], [(1, 1), (2, 0)]])
    lines.set_array(np.array([0.0, 1.0]))
    assert lines.get_cmap().name == "Ends"
    assert lines.to_rgba(np.array([0.0, 1.0])).tolist() == [
        [0.0, 0.0, 0.0, 1.0], [1.0, 1.0, 1.0, 1.0]]


def test_a_copy_of_an_artist_of_an_axes_is_added_to_it():
    fig, ax = plt.subplots()
    for make in (lambda: ax.plot([0, 1])[0],
                 lambda: ax.scatter([0, 1], [0, 1]),
                 lambda: ax.eventplot([0, 1])[0]):
        artist = make()
        highlight = copy.copy(artist)
        highlight.set(color="yellow")
        ax.add_artist(highlight)
        children = ax.get_children()
        assert highlight in children and artist in children
        assert highlight.axes is ax
        highlight.remove()
        assert highlight not in ax.get_children()
        assert artist in ax.get_children()


class _Marked(str):
    """A default alignment the program tells apart from one set (mplcursors'
    ``_MarkedStr``)."""


def test_text_alignments_read_back_as_the_objects_given():
    fig, ax = plt.subplots()
    text = ax.annotate("", (0, 0), horizontalalignment=_Marked("center"),
                       verticalalignment=_Marked("center"))
    assert isinstance(text.get_horizontalalignment(), _Marked)
    assert isinstance(text.get_verticalalignment(), _Marked)
    text.set(ha="right", va="bottom")
    assert type(text.get_ha()) is str and text.get_ha() == "right"
    assert text.get_va() == "bottom"


def test_images_of_every_kind_are_the_images_of_their_axes():
    from pltrs.image import NonUniformImage, PcolorImage
    fig, ax = plt.subplots()
    pcolor = ax.pcolorfast(np.arange(3) ** 2, np.arange(3) ** 2, np.zeros((2, 2)))
    assert isinstance(pcolor, PcolorImage)
    image = NonUniformImage(ax)
    image.set_data([0, 1], [0, 1], np.zeros((2, 2)))
    ax.add_image(image)
    assert list(ax.images) == [pcolor, image]
    assert list(ax.artists) == []


def test_an_image_removed_from_its_axes_is_freed():
    fig, ax = plt.subplots()

    def made_and_removed():
        image = ax.imshow([[0, 1], [2, 3]])
        assert ax._gci() is None, "only pyplot makes it the current image"
        alive = weakref.finalize(image, lambda: None)
        image.remove()
        return alive

    alive = made_and_removed()
    gc.collect()
    assert not alive.alive
    image = plt.imshow([[0, 1]])
    assert ax._gci() is image


def test_error_bars_take_one_colour_per_bar():
    fig, ax = plt.subplots()
    colors = np.array(["#000000", "#FF0000", "#00FF00"], dtype=object)
    bars = ax.errorbar([0, 1, 2], [1, 2, 3], yerr=[0.1, 0.2, 0.3],
                       ecolor=colors, fmt="o")
    assert bars[2][0].get_colors().tolist() == [
        [0.0, 0.0, 0.0, 1.0], [1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]]
    fig.canvas.draw()
    capped = ax.errorbar([0, 1, 2], [1, 2, 3], yerr=[0.1, 0.2, 0.3],
                         ecolor=colors, capsize=3, fmt="o")
    assert [list(cap.get_markeredgecolor()) for cap in capped[1]] == [
        list(colors)] * 2
    with pytest.raises(ValueError, match="Invalid RGBA argument"):
        fig.canvas.draw()


def test_bars_of_dates_keep_their_width():
    fig, ax = plt.subplots()
    days = np.array(["2000-01-01", "2000-01-02"], dtype="datetime64[ns]")
    bars = ax.bar(days, [1, 2], width=0.8)
    assert bars[0].get_width() == 0.8
    assert bars[0].get_x() == pytest.approx(10956.6)
    numbers = ax.bar(np.array([10957.0, 10958.0]), [1, 2], width=0.8)
    assert numbers[0].get_width() == 0.7999999999992724


def test_data_limits_a_program_stretched_are_recomputed():
    fig, ax = plt.subplots()
    ax.plot(np.arange(10) / 10)
    ax.scatter([0], [0])
    ax.dataLim.update_from_data_xy(np.asarray([[0, 0], [2, 10]]), ignore=False)
    assert ax.dataLim.bounds == (0, 0, 9, 10)
    ax.relim()
    assert ax.dataLim.bounds == pytest.approx((0, 0, 9, 0.9))
    ax.dataLim.update_from_data_xy(np.asarray([[0, 0], [2, 4]]), ignore=False)
    assert ax.dataLim.bounds == pytest.approx((0, 0, 9, 4))


class _FigureBoundsLocator:
    """A locator of the program's giving its Axes a box in figure fractions
    (ultraplot's own ``_TransformedBoundsLocator``)."""

    def __init__(self, bounds, transform):
        self._bounds = bounds
        self._transform = transform

    def __call__(self, ax, renderer):
        box = TransformedBbox(Bbox.from_bounds(*self._bounds), self._transform)
        return TransformedBbox(box, ax.figure.transSubfigure.inverted())


@pytest.mark.parametrize("bounds, expected", [
    ((0, 0, 1, 1), (0.125, 0.11, 0.775, 0.35)),
    ((0.1, 0.25, 0.8, 0.5), (0.2025, 0.1975, 0.62, 0.175)),
])
def test_child_axes_go_where_a_locator_of_the_program_puts_them(bounds, expected):
    fig = plt.figure()
    panel = fig.add_subplot(2, 1, 2)
    locator = _FigureBoundsLocator(bounds, panel.transAxes)
    child = Axes(fig, locator(panel, None).bounds, zorder=5)
    child.set_axes_locator(locator)
    panel.add_child_axes(child)
    fig.canvas.draw()
    assert child.get_position().bounds == pytest.approx(expected)


def test_child_axes_name_their_parent_as_their_axes():
    fig, ax = plt.subplots()
    twin = ax.twinx()
    inset = ax.inset_axes([0.5, 0.5, 0.4, 0.4])
    assert ax._axes is ax and ax.axes is ax
    assert inset._axes is ax and inset.axes is ax
    assert twin._axes is twin


def test_colorbars_label_their_ticks_with_a_formatter_given():
    fig, ax = plt.subplots()
    mesh = ax.pcolor(np.arange(6).reshape(2, 3) - 2.5)
    cax = fig.add_axes([0.1, 0.05, 0.8, 0.05])
    formatter = ticker.ScalarFormatter()
    bar = fig.colorbar(mesh, cax=cax, orientation="horizontal", format=formatter)
    fig.canvas.draw()
    assert bar.formatter is formatter
    assert [label.get_text() for label in cax.xaxis.get_majorticklabels()] == [
        "\N{MINUS SIGN}3", "\N{MINUS SIGN}2", "\N{MINUS SIGN}1", "0", "1", "2", "3"]


def test_axes_are_measured_where_a_locator_of_the_program_puts_them():
    fig = plt.figure(figsize=(4, 3), dpi=100)
    ax = fig.add_axes([0.1, 0.1, 0.3, 0.3])
    ax.set_axis_off()
    ax.set_axes_locator(
        lambda ax, renderer: Bbox.from_bounds(0.5, 0.6, 0.25, 0.2))
    box = ax.get_tightbbox(fig.canvas.get_renderer())
    assert box.bounds == pytest.approx((200.0, 180.0, 100.0, 60.0))
    # Not called, it leaves the Axes at its original position until drawn.
    assert ax.get_tightbbox(fig.canvas.get_renderer(), call_axes_locator=False
                            ).bounds == pytest.approx((40.0, 30.0, 120.0, 90.0))
    assert ax.get_position().bounds == pytest.approx((0.1, 0.1, 0.3, 0.3))
