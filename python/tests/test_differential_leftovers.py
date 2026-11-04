"""
The differential tester's (``cargo xtask diff``) 2-D leftovers: the data of
tick lines, norms a draw scales, the groups of shared axes and the locator
a clear binds, exceptions of the program's code in a layout pass, the font
lookups of legends and prints, unscaled `.FT2Font` layouts, the positions
of axes placed in others, the trees of a program's projection, and the
animated axes a draw leaves out. Each expected value is matplotlib 3.11's.
"""

import io
import logging

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors, ticker
from pltrs.axes import Axes
from pltrs.axis import XTick, YTick
from pltrs.font_manager import findfont
from pltrs.ft2font import FT2Font, LoadFlags
from pltrs.projections import register_projection


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _lines(tick):
    return tick.tick1line.get_data(), tick.tick2line.get_data(), tick.gridline.get_data()


def test_tick_lines_are_made_at_zero_then_placed():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    xtick, ytick = XTick(ax, 5), YTick(ax, 5)
    assert _lines(xtick) == (([0], [0]), ([0], [1]), ([0, 0], [0, 1]))
    assert _lines(ytick) == (([0], [0]), ([1], [0]), ([0, 1], [0, 0]))
    assert xtick.label1.get_position() == (0, 0)
    xtick.update_position(3)
    assert _lines(xtick) == (((3,), [0]), ((3,), [1]), ((3,), [0, 1]))
    assert [_lines(t) for t in ax.yaxis.get_major_ticks()[:2]] == [
        (([0], [0]), ([1], [0]), ([0, 1], [0, 0]))] * 2
    fig.canvas.draw()
    first, second = ax.xaxis.get_major_ticks()[:2]
    assert _lines(second) == (((0.0,), [0]), ((0.0,), [1]), ((0.0,), [0, 1]))
    assert type(first.tick1line.get_xdata()[0]) is np.float64
    assert [t.tick1line.get_data() for t in ax.yaxis.get_major_ticks()[:2]] == [
        ([0], (0.75,)), ([0], (1.0,))]
    ax.set_xlim(0, 10)
    assert first.tick1line.get_data() == ((-0.25,), [0]), "until the next draw"
    fig.canvas.draw()
    assert first.tick1line.get_data() == ((0.0,), [0])


@pytest.mark.parametrize("make", [
    lambda ax: ax.pcolormesh(np.arange(1, 13.0).reshape(3, 4)),
    lambda ax: ax.scatter([1, 2, 3], [1, 2, 3], c=[1, 5, 9]),
])
def test_a_draw_scales_a_norm_set_before_it(make):
    fig, ax = plt.subplots()
    mappable = make(ax)
    vmax = mappable.get_array().max()
    mappable.set_norm(colors.LogNorm())
    assert mappable.get_clim() == (None, None)
    fig.canvas.draw()
    assert mappable.get_clim() == (1.0, vmax)
    assert type(mappable.norm.vmin) is float
    mappable.set_visible(False)
    mappable.set_norm(colors.LogNorm())
    fig.canvas.draw()
    assert mappable.get_clim() == (None, None), "not drawn"


def test_shared_axes_are_viewed_as_a_grouper():
    fig, (a, b, c) = plt.subplots(1, 3, sharex=True)
    alone = fig.add_subplot(2, 2, 4)
    shared = a.get_shared_x_axes()
    assert type(shared).__name__ == "GrouperView"
    assert type(Axes.get_shared_y_axes()).__name__ == "GrouperView"
    assert shared.joined(a, b) and not shared.joined(a, alone)
    assert b in shared and alone not in shared
    assert shared.get_siblings(a) == [b, a, c], "in the order they joined"
    assert shared.get_siblings(a, include_self=False) == [b, c]
    assert shared.get_siblings(alone) == [alone]
    assert [b, a, c] in list(shared)


def test_a_clear_binds_the_shared_locator_to_the_last_other_axes():
    fig, (wide, narrow) = plt.subplots(
        1, 2, sharex=True, gridspec_kw=dict(width_ratios=[6, 1]))
    for ax in (wide, narrow):
        ax.plot([0, 100], [0, 1])
    fig.canvas.draw()
    assert list(wide.get_xticks()) == [-20, 0, 20, 40, 60, 80, 100, 120]
    wide.clear()
    wide.plot([0, 100], [0, 1])
    fig.canvas.draw()
    assert list(wide.get_xticks()) == [-100, 0, 100, 200], "as many as fit the narrow axes"
    assert wide.xaxis.get_major_locator().axis is narrow.xaxis
    narrow.clear()
    narrow.plot([0, 100], [0, 1])
    fig.canvas.draw()
    assert list(narrow.get_xticks()) == [-20, 0, 20, 40, 60, 80, 100, 120]


def _failing_once(kind):
    calls = []

    def label(x, pos):
        calls.append(pos)
        if len(calls) == 1:
            raise kind("first call fails")
        return f"{x:.1f}"
    return ticker.FuncFormatter(label), calls


@pytest.mark.parametrize("layout, kind, calls, left", [
    ("constrained", ValueError, 34, 0.125),
    ("tight", ValueError, 34, 0.125),
    ("constrained", TypeError, 144, 0.0701),
    ("tight", TypeError, 144, 0.0871),
])
def test_exceptions_of_a_layout_pass(layout, kind, calls, left):
    # A ValueError ends the layout pass, which the draw passes over; a
    # TypeError measuring an axis is taken for that of a get_tightbbox
    # without for_layout_only, the axis measured again.
    fig, ax = plt.subplots(layout=layout)
    ax.plot([1, 2, 3])
    formatter, made = _failing_once(kind)
    ax.xaxis.set_major_formatter(formatter)
    fig.canvas.draw()
    assert len(made) == calls
    assert round(ax.get_position().x0, 4) == left


def test_font_lookups_of_legends_and_prints():
    class Count(logging.Handler):
        n = 0

        def emit(self, record):
            Count.n += 1
    handler = Count(logging.WARNING)
    logging.getLogger().addHandler(handler)
    try:
        fig, ax = plt.subplots()
        ax.plot([1, 2], label="a")
        ax.plot([2, 1], label="b")
        missing = {"family": "NoSuchFont"}
        ax.legend(prop=missing, title="T", title_fontproperties=missing)
        counts = []
        for draw in [fig.canvas.draw, fig.canvas.draw,
                     *(lambda fmt=fmt: fig.savefig(io.BytesIO(), format=fmt)
                       for fmt in ["svg", "png", "pdf", "ps"])]:
            Count.n = 0
            draw()
            counts.append(Count.n)
    finally:
        logging.getLogger().removeHandler(handler)
    assert counts == [22, 18, 22, 18, 22, 23]


def test_unscaled_ft2font_lays_out_without_kerning():
    font = FT2Font(findfont("DejaVu Sans"))
    assert font.set_text("AVA").tolist() == [[0.0, 0.0], [1401.0, 0.0], [2802.0, 0.0]]
    assert font.get_bitmap_offset() == (16, 0)
    assert [g.prev_kern for g in font._layout("AV", LoadFlags.DEFAULT)] == [0.0, 21.890625]
    font.set_text("VA V")
    assert font.get_bitmap_offset() == (0, 0), "the space's box at the origin"
    assert font.load_char(ord("A")).linearHoriAdvance == 0
    font.set_size(12, 72)
    assert font.load_char(ord("A")).linearHoriAdvance == 537984


def test_axes_placed_in_others_have_their_locators_bits():
    fig, ax = plt.subplots()
    top = ax.secondary_xaxis("top")
    fig.canvas.draw()
    assert str(top) == "SecondaryAxis(0.125,0.88;0.775x7.7e-11)"
    assert top.get_position().bounds == (
        0.125, 0.8799999999999999, 0.775, 7.699996196208758e-11)
    assert top._position.get_points().tolist() == [
        [0.125, 0.8799999999999999], [0.9, 0.8800000000769999]]


def test_a_program_projection_makes_its_trees_spanning_the_figure():
    seen = []

    class Spanning(Axes):
        name = "test_differential_spanning"

        def _set_lim_and_transforms(self):
            super()._set_lim_and_transforms()
            seen.append(self.transAxes.get_matrix()[:2].tolist())
    register_projection(Spanning)
    ax = plt.figure(figsize=(6.4, 4.8)).add_subplot(projection=Spanning.name)
    assert seen == [[[640.0, 0.0, 0.0], [0.0, 480.0, 0.0]]]
    assert ax.transAxes.get_matrix()[:2].tolist() == [
        [496.0, 0.0, 80.0], [0.0, 369.59999999999997, 52.8]]


def test_a_draw_leaves_animated_axes_to_the_program():
    calls = []
    fig, ax = plt.subplots()
    secondary = ax.secondary_xaxis("top")
    secondary.xaxis.set_major_formatter(
        ticker.FuncFormatter(lambda x, pos: calls.append(pos) or f"{x}"))
    fig.canvas.draw()
    assert len(calls) == 18
    secondary.set_animated(True)
    calls.clear()
    fig.canvas.draw()
    assert calls == []
    fig.savefig(io.BytesIO(), format="png")
    assert len(calls) == 18, "a save draws the animated artists of axes"


@pytest.mark.parametrize("fill, expected", [
    (dict(step="mid"), [
        [[[0, 1], [0, 0], [0.5, 0], [0.5, 0], [1.5, 0], [1.5, 0], [2.5, 0], [2.5, 0],
          [3, 0], [3, 3], [3, 3], [2.5, 3], [2.5, 1], [1.5, 1], [1.5, 2], [0.5, 2],
          [0.5, 1], [0, 1], [0, 1]]],
        [[[0, 3], [0, 1], [0.5, 1], [0.5, 2], [1.5, 2], [1.5, 1], [2.5, 1], [2.5, 3],
          [3, 3], [3, 4], [3, 4], [2.5, 4], [2.5, 3], [1.5, 3], [1.5, 3], [0.5, 3],
          [0.5, 3], [0, 3], [0, 3]]]]),
    (dict(where=[True, True, False, True], interpolate=True), [
        [[[0, 0], [0, 0], [1, 0], [2, 0], [1, 2], [0, 1], [0, 0]],
         [[2, 0], [3, 0], [3, 0], [3, 3], [2, 0]]],
        [[[0, 1], [0, 1], [1, 2], [1, 2], [1, 3], [0, 3], [0, 1]],
         [[3, 3], [3, 3], [3, 3], [3, 4], [3, 3]]]]),
])
def test_stackplot_passes_fill_between_keywords_on(fill, expected):
    fig, ax = plt.subplots()
    layers = ax.stackplot(np.arange(4.0), [1, 2, 1, 3], [2, 1, 2, 1], **fill)
    got = [[p.vertices.round(3).tolist() for p in layer.get_paths()] for layer in layers]
    assert got == expected
    assert ax.get_ylim() == (0.0, 4.2)
