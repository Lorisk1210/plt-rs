"""
What the sixth round of corpora (``cargo xtask diff --corpus``) found
pltrs doing otherwise than matplotlib 3.11, reduced to matplotlib's API:
3-D axes sharing axes with 2-D ones, the arrays a program's norm is
handed for images, the offset text read without autoscaling, curves'
data limits and mathtext's dummy symbols. Every expected value was
recorded from matplotlib.
"""

import io
import logging

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.colors import Normalize
from pltrs.patches import FancyArrowPatch, PathPatch
from pltrs.path import Path


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _limits(ax):
    """The x and y limits of *ax* as plain floats."""
    return tuple(map(float, (*ax.get_xlim(), *ax.get_ylim())))


IMAGE = np.add.outer(np.arange(64.), np.arange(80.)) / 50


def test_3d_axes_share_x_and_y_with_2d_axes_both_ways():
    fig = plt.figure(figsize=(12, 4))
    ax1 = fig.add_subplot(1, 3, 1)
    ax1.imshow(IMAGE)
    ax1.scatter([3, 70], [5, 60])
    ax2 = fig.add_subplot(1, 3, 2, projection='3d', sharex=ax1, sharey=ax1)
    # The 3-D axes autoscales its unit box, which the 2-D axes follows.
    unit = (-0.02083333333333334, 1.0208333333333333)
    assert _limits(ax1) == (*unit, *unit[::-1])
    assert _limits(ax2) == _limits(ax1)
    assert ax1.get_shared_x_axes().get_siblings(ax1) == [ax2, ax1]
    assert ax2._sharex is ax1 and ax2.xaxis.major is ax1.xaxis.major
    x, y = np.meshgrid(np.arange(0, 64), np.arange(0, 80))
    ax2.plot_surface(x, y, IMAGE.T, linewidth=0, alpha=0.5)
    assert _limits(ax1) == (-4.59375, 67.59375, 84.76041666666667, -5.760416666666667)
    ax3 = fig.add_subplot(1, 3, 3, sharex=ax1, sharey=ax1)
    ax3.imshow(IMAGE)
    assert _limits(ax2) == _limits(ax3) == (-0.5, 79.5, 63.5, -0.5)
    fig.tight_layout()
    assert list(ax2.get_xticks()) == list(range(-10, 90, 10))
    ax2.set_xlim(10, 20)
    assert ax1.get_xlim() == ax3.get_xlim() == (10, 20)
    ax1.set_ylim(5, 30)
    assert ax2.get_ylim() == (5, 30)


def test_2d_axes_share_with_a_3d_one():
    fig = plt.figure()
    deep = fig.add_subplot(1, 2, 1, projection='3d')
    deep.set_ylim(-1, 2)
    flat = fig.add_subplot(1, 2, 2, sharey=deep)
    assert flat.get_ylim() == (-1, 2)
    assert flat._sharey is deep
    assert deep.get_shared_y_axes().get_siblings(deep) == [flat, deep]
    flat.set_ylim(0, 4)
    assert deep.get_ylim() == (0, 4)


class Recording(Normalize):
    """A norm noting what it is called with."""

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.calls = []

    def __call__(self, value, clip=None):
        a = np.asanyarray(value)
        self.calls.append((type(value).__name__, a.shape))
        return super().__call__(value, clip)


def test_an_image_s_norm_gets_the_resampled_array():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    norm = Recording(0.2, 0.9)
    ax.imshow(np.arange(12.).reshape(3, 4) / 11, norm=norm)
    fig.canvas.draw()
    assert norm.calls == [('MaskedArray', (116, 154))]


def test_an_image_s_norm_is_called_before_its_colorbar_s():
    fig = plt.figure(figsize=(8, 6))
    norm = Recording(vmin=0.2, vmax=0.97)
    im = plt.imshow(np.linspace(0.3, 0.97, 64).reshape(8, 8),
                    interpolation="nearest", norm=norm)
    plt.colorbar(im)
    norm.calls.clear()
    fig.savefig(io.BytesIO(), bbox_inches="tight")
    images = [i for i, call in enumerate(norm.calls) if call[1] == (462, 462)]
    assert len(norm.calls) == 199
    assert images == [0, 124], "first in each of the two draws"
    assert norm.calls[1] == ('MaskedArray', (256, 1))


def test_reading_the_offset_text_autoscales_nothing():
    fig, axes = plt.subplots(4, 4, figsize=(10, 10))
    ax = axes.flat[1]
    ax.scatter([-1.0, 1.0, 0.0], [-1 / 3, -1 / 3, 2 / 3], s=200)
    ax.tick_params(axis="x", labelbottom=False)
    arrow = FancyArrowPatch((-1.0, -1 / 3), (1.0, -1 / 3), arrowstyle="-|>",
                            shrinkA=np.sqrt(200) / 2, shrinkB=np.sqrt(200) / 2,
                            mutation_scale=20, linewidth=2)
    ax.add_patch(arrow)
    # The arrow was placed with the view of 0 to 1, not yet autoscaled.
    assert ax.dataLim.y0 == -0.35722943722943734


def test_curves_bound_the_data_as_matplotlib_evaluates_them():
    fig, ax = plt.subplots()
    loop = Path([(0.4999999105930326, -0.6928203403949738),
                 (0.5866024643182748, -0.6928203403949738),
                 (0.5866024643182748, -0.866025447845459),
                 (0.4999999105930326, -0.866025447845459)],
                [Path.MOVETO, Path.CURVE4, Path.CURVE4, Path.CURVE4])
    ax.add_patch(PathPatch(loop))
    assert ax.dataLim.x1 == 0.5649518258869642


def test_mathtext_logs_the_symbols_no_font_has(caplog):
    with caplog.at_level(logging.WARNING):
        plt.scatter(0, 0, marker='$\N{SMILING FACE WITH SUNGLASSES}$', s=500)
        plt.gcf().canvas.draw()
    assert [r.getMessage() for r in caplog.records] == [
        "Font 'rm' does not have a glyph for '\\U0001f60e' [U+1f60e], "
        "substituting with a dummy symbol."]


def test_figure_fractions_of_annotations_ignore_the_saved_window():
    # matplotlib places an annotation's "figure fraction" in the whole
    # figure (`figbbox`), which a tight save leaves unmoved: the text
    # ending at the figure's right edge runs off the window saved.
    fig = plt.figure()
    ax = fig.add_subplot(projection='polar')
    ax.plot([0, 1], [0, 1])
    ax.annotate('a polar annotation', xy=(2, 0.5), xytext=(1.0, 0.75),
                textcoords='figure fraction', ha='right', va='bottom',
                arrowprops=dict(facecolor='red', shrink=0.05))
    buffer = io.BytesIO()
    fig.savefig(buffer, format='png', bbox_inches='tight')
    buffer.seek(0)
    image = plt.imread(buffer)
    assert (image[:, -5:, :3] < 0.5).any(), "the text runs to the edge"


def test_the_program_s_box_styles_shape_each_draw_of_a_print():
    calls = []

    def style(x0, y0, width, height, size, mutation_aspect=1):
        calls.append(size)
        return Path([(x0, y0), (x0 + width, y0), (x0 + width, y0 + height),
                      (x0, y0)], closed=True)

    fig = plt.figure()
    fig.text(0.5, 0.5, "boxed", bbox=dict(boxstyle=style))
    counts = []
    for kwargs in ({}, {"bbox_inches": "tight"}):
        calls.clear()
        fig.savefig(io.BytesIO(), **kwargs)
        counts.append(len(calls))
    fig.set_layout_engine("tight")
    calls.clear()
    fig.savefig(io.BytesIO())
    counts.append(len(calls))
    # matplotlib draws a figure once more before a tight or laid-out print.
    assert counts == [1, 2, 2]
