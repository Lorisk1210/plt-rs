"""
What the corpora of real-world programs (``cargo xtask diff --corpus``,
``spec/diff/corpora.toml``) still found pltrs doing otherwise than
matplotlib 3.11 after the first round (``test_corpus_programs.py``). Every
expected value was recorded from matplotlib.
"""

import io
import logging
import os
import re
import types

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import patches as mpatches, patheffects, ticker
from pltrs.artist import Artist
from pltrs.backend_bases import GraphicsContextBase, RendererBase
from pltrs.collections import LineCollection, PolyCollection
from pltrs.font_manager import FontProperties
from pltrs.mpl_toolkits.mplot3d import proj3d


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


class _Messages(logging.Handler):
    """Keeps the messages logged meanwhile."""

    def __init__(self):
        super().__init__(logging.WARNING)
        self.messages = []

    def emit(self, record):
        self.messages.append(record.getMessage())

    def count(self, part):
        """How many messages hold *part*; forgets them all."""
        counted = sum(part in message for message in self.messages)
        self.messages.clear()
        return counted


@pytest.fixture
def logged():
    """The warnings logged during the test."""
    handler = _Messages()
    root = logging.getLogger()
    level = root.level
    root.addHandler(handler)
    root.setLevel(logging.WARNING)
    yield handler
    root.removeHandler(handler)
    root.setLevel(level)


def _red(fig):
    """The red channel of *fig* drawn on its canvas."""
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba())[:, :, 0].astype(int)


def test_the_two_marks_of_a_tick_keep_their_own_sizes():
    # Rougier's ruler lengthens every ninth mark of the minor tick lines.
    fig = plt.figure(figsize=(4.37, 2.18), dpi=100)
    ax = fig.add_axes([0, 0, 1, 1])
    ax.xaxis.set_minor_locator(ticker.MultipleLocator(0.1))
    ax.xaxis.tick_top()
    ax.set_xlim(0, 11.1)
    ax.set_xticks(np.arange(12))
    for line in ax.xaxis.get_ticklines(True)[2::9]:
        line.set_markersize(3)
    fig.canvas.draw()
    sizes = [(round(float(tick.get_loc()), 2), tick.tick1line.get_markersize(),
              tick.tick2line.get_markersize())
             for tick in ax.xaxis.get_minor_ticks()[:7]]
    assert sizes == [(-0.1, 2.0, 2.0), (0.1, 3.0, 2.0), (0.2, 2.0, 2.0),
                     (0.3, 2.0, 2.0), (0.4, 2.0, 2.0), (0.5, 2.0, 3.0),
                     (0.6, 2.0, 2.0)]
    assert _red(fig)[:6, 4].tolist() == [0, 241, 255, 255, 255, 255]


def test_collections_without_offsets_stay_out_of_tight_boxes():
    # A mesh drawn unclipped beyond its axes (Rougier's bunnies).
    fig = plt.figure(figsize=(4, 4), dpi=100)
    ax = fig.add_subplot(xlim=(-1, 1), ylim=(-1, 1), xticks=[], yticks=[],
                         frameon=False)
    collection = PolyCollection([[(-3, -3), (3, -3), (0, 3)]], clip_on=False)
    ax.add_collection(collection)
    renderer = fig.canvas.get_renderer()
    assert ax.get_tightbbox(renderer).bounds == pytest.approx((50.0, 44.0, 310.0, 308.0))
    assert not np.isfinite(collection.get_window_extent(renderer).x0)
    fig.tight_layout()
    assert ax.get_position().bounds == pytest.approx((0.0375, 0.0375, 0.925, 0.925))


def test_the_alignment_set_on_a_legend_box_aligns_its_title():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    ax.plot([0, 1], label="a long label")
    legend = ax.legend(title="T")
    legend.get_children()[0].align = "left"
    fig.canvas.draw()
    assert legend.get_alignment() == "left"
    offset = (legend.get_title().get_window_extent().x0
              - legend.get_window_extent().x0)
    assert offset == pytest.approx(5.56, abs=0.005)


def test_collections_take_no_path_effects_from_the_rc():
    # xkcd's white stroke goes around lines, not around scatter markers.
    with plt.xkcd():
        fig = plt.figure(figsize=(1, 1), dpi=100)
        ax = fig.add_axes([0, 0, 1, 1])
        ax.set_axis_off()
        ax.set(xlim=(0, 1), ylim=(0, 1))
        ax.plot([0, 1], [0.5, 0.5], color="k", lw=2, zorder=1)
        markers = ax.scatter([0.5], [0.5], s=50, c="k", zorder=2)
        assert markers.get_path_effects() is None
        assert _red(fig)[50, 40:61].tolist() == [0] * 21


class _Arrow3D(mpatches.FancyArrowPatch):
    """The arrow of many 3-D programs: it projects its own ends."""

    projections = 0

    def __init__(self, xs, ys, zs, *args, **kwargs):
        super().__init__((0, 0), (0, 0), *args, **kwargs)
        self._verts3d = xs, ys, zs

    def do_3d_projection(self, renderer=None):
        type(self).projections += 1
        xs3d, ys3d, zs3d = self._verts3d
        xs, ys, zs = proj3d.proj_transform(xs3d, ys3d, zs3d, self.axes.M)
        self.set_positions((xs[0], ys[0]), (xs[1], ys[1]))
        return np.min(zs)


def test_a_patch_projecting_itself_is_projected_at_every_draw():
    fig = plt.figure(figsize=(4, 4), dpi=100)
    ax = fig.add_subplot(projection="3d")
    arrow = _Arrow3D([0, 1], [0, 1], [0, 1], mutation_scale=10, arrowstyle="-|>")
    ax.add_artist(arrow)
    _Arrow3D.projections = 0
    fig.savefig(io.BytesIO(), format="png")
    assert _Arrow3D.projections == 1
    ends = [value for point in arrow._posA_posB for value in point]
    assert ends == pytest.approx([-0.0778, -0.04742, 0.07832, 0.04773], abs=1e-5)
    assert arrow.zorder == 2.5
    # A print laid out by an engine is drawn twice, as matplotlib's is.
    fig.tight_layout()
    fig.savefig(io.BytesIO(), format="png")
    assert _Arrow3D.projections == 3


class _RoundCaps(GraphicsContextBase):
    """windmap's graphics contexts: round caps."""

    made = 0

    def __init__(self):
        super().__init__()
        type(self).made += 1
        self._capstyle = "round"


def test_graphics_contexts_of_the_program_start_every_artist():
    own = RendererBase.new_gc
    RendererBase.new_gc = types.MethodType(lambda self: _RoundCaps(), RendererBase)
    try:
        fig = plt.figure(figsize=(1, 1), dpi=100)
        ax = fig.add_axes([0, 0, 1, 1])
        ax.set_axis_off()
        ax.set(xlim=(0, 1), ylim=(0, 1))
        ax.add_collection(LineCollection([[(0.3, 0.5), (0.7, 0.5)]],
                                         linewidths=10, colors="k"))
        _RoundCaps.made = 0
        pixels = _red(fig)
    finally:
        RendererBase.new_gc = own
    assert _RoundCaps.made == 2
    assert pixels[50, 22:30].tolist() == [255, 40, 0, 0, 0, 0, 0, 0]
    assert pixels[44, 22:30].tolist() == [255, 255, 255, 189, 36, 0, 0, 0]


def test_markers_inside_a_pixel_stamp_their_fill_again_for_a_zero_width_edge():
    # Agg keeps the fill's scanlines when the stroke covers no cell.
    fig = plt.figure(figsize=(0.5, 0.5), dpi=100)
    ax = fig.add_axes([0, 0, 1, 1])
    ax.set_axis_off()
    ax.set(xlim=(0, 1), ylim=(0, 1))
    ax.scatter([0.5], [0.5], s=0.5, linewidths=0, facecolor="0.5", edgecolor="red")
    ax.scatter([0.25], [0.25], s=0.5, linewidths=0, color="0.5")
    fig.canvas.draw()
    pixels = np.asarray(fig.canvas.buffer_rgba())[:, :, :3].astype(int)
    ys, xs = np.nonzero((pixels != 255).any(axis=2))
    drawn = sorted((int(y), int(x), tuple(pixels[y, x].tolist())) for y, x in zip(ys, xs))
    assert drawn == [(25, 25, (231, 39, 39)), (38, 13, (135, 135, 135))]


def test_a_missing_font_file_fails_when_the_text_is_drawn():
    fig = plt.figure()
    fig.text(0.5, 0.5, "x", fontproperties=FontProperties(fname="/nonexistent/x.ttf"))
    # matplotlib opens the file at its real path (on Windows with a drive).
    missing = os.path.realpath("/nonexistent/x.ttf")
    with pytest.raises(FileNotFoundError,
                       match=f"^{re.escape(f'[Errno 2] No such file or directory: {missing!r}')}$"):
        fig.canvas.draw()


def test_reading_the_offset_text_looks_no_font_up(logged):
    fig, ax = plt.subplots()
    ax.set_xticks([0, 1])
    ax.set_xticklabels(["a", "b"], fontfamily="No Such Family")
    fig.canvas.draw()
    logged.count("")
    ax.xaxis.offsetText.get_position()
    assert logged.count("not found") == 0


def test_the_custom_math_fontset_looks_its_fonts_up(logged):
    # A cursive family installed nowhere (macOS has Apple Chancery).
    pltrs.rcParams["font.cursive"] = ["No Such Cursive Family"]
    pltrs.rcParams["mathtext.fontset"] = "custom"
    # A weight no other test asks for, so that findfont's cache misses.
    pltrs.rcParams["mathtext.cal"] = "cursive:light"
    fig = plt.figure()
    fig.text(0.5, 0.5, "$x$")
    fig.canvas.draw()
    falling_back = [message for message in logged.messages
                    if "cursive" in message and "Falling back" in message]
    assert falling_back == [
        "findfont: Font family ['cursive'] not found. Falling back to DejaVu Sans."]


def test_titles_look_their_fonts_up_as_often_as_matplotlib_s(logged):
    fig = plt.figure(figsize=(4, 3))
    fig.add_subplot().set_title("t", fontname="No Such Title Family")
    logged.count("")
    fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
    assert logged.count("No Such Title Family") == 9
    fig.savefig(io.BytesIO(), format="png")
    assert logged.count("No Such Title Family") == 4


def test_texts_with_path_effects_look_up_in_both_draws_of_a_print(logged):
    fig = plt.figure(figsize=(4, 3))
    ax = fig.add_subplot()
    text = ax.text(0.5, 0.5, "100", family="No Such Effect Family")
    text.set_path_effects([patheffects.Stroke(linewidth=2, foreground="white"),
                           patheffects.Normal()])
    fig.tight_layout()
    logged.count("")
    fig.savefig(io.BytesIO(), format="png")
    assert logged.count("No Such Effect Family") == 4


class _Counted(Artist):
    """An artist of the program's counting its draws."""

    drawn = 0

    def draw(self, renderer):
        type(self).drawn += 1


def test_the_program_s_drawing_runs_in_both_draws_of_a_print():
    fig = plt.figure()
    fig.add_artist(_Counted())
    _Counted.drawn = 0
    fig.savefig(io.BytesIO(), format="png")
    assert _Counted.drawn == 1
    fig.tight_layout()
    fig.savefig(io.BytesIO(), format="png")
    assert _Counted.drawn == 3
