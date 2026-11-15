"""
What the eleventh round of corpora (``cargo xtask diff --corpus``: books,
courses, analyses, matplotlib's own documentation) showed pltrs doing
otherwise than matplotlib 3.11, reduced to matplotlib's API: colours given
as ``(color, alpha)`` pairs (the what's new of 3.8, the margins figure of
the documentation), the best place of a legend beside infinite lines (the
what's new of 3.3), y labels aligned in a figure laid out for the first
time (python-control's Bode plots), titles set in a style's context (the
default style changes of 2.0), collections added to 3-D axes as to
2-D ones (the what's new of 3.7), 3-D texts placed before they are added
(pytransform3d's frames), the warnings of contours of complex
data (Newton's basins of attraction) and an image of the figure printed
to its tight box at another resolution (the what's new of 1.5), and the
TeX preamble read where the texts are drawn (tueplots' font bundles).
Every expected value was recorded from matplotlib.
"""

import io
import shutil
import warnings

import numpy as np
import pytest
from PIL import Image

import pltrs
import pltrs.pyplot as plt
from pltrs.patches import Rectangle
from pltrs.mpl_toolkits.mplot3d.art3d import Poly3DCollection


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


ORANGE = (1.0, 0.6470588235294118, 0.0)
GREEN = (0.0, 0.5019607843137255, 0.0)


def test_colour_alpha_pairs_colour_lines_patches_texts_and_frames():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2], color=("blue", 0.4))
    assert line.get_color() == ("blue", 0.4)
    assert line.get_alpha() is None
    span = ax.axvspan(0, 1, color=("orange", 0.1))
    assert span.get_facecolor() == span.get_edgecolor() == (*ORANGE, 0.1)
    rect = Rectangle((.2, .2), .6, .6, facecolor=("green", 0.5), edgecolor=("red", .3))
    assert rect.get_facecolor() == (*GREEN, 0.5)
    assert rect.get_edgecolor() == (1.0, 0.0, 0.0, 0.3)
    # The artist's alpha replaces the pair's.
    faded = Rectangle((0, 0), 1, 1, facecolor=("green", 0.5), alpha=0.2)
    assert faded.get_facecolor() == (*GREEN, 0.2)
    assert ax.text(0, 0, "a", color=("C1", .4)).get_color() == ("C1", 0.4)
    ax.grid(color=("blue", .4))
    ax.spines["left"].set_color(("blue", .4))
    assert ax.spines["left"].get_edgecolor() == (0.0, 0.0, 1.0, 0.4)
    fig.set_facecolor(("blue", .4))
    assert fig.get_facecolor() == (0.0, 0.0, 1.0, 0.4)
    # The frame's alpha is the legend's framealpha.
    legend = ax.legend(["x"], facecolor=("red", .3))
    assert legend.get_frame().get_facecolor() == (1.0, 0.0, 0.0, 0.8)
    fig.canvas.draw()


def test_best_legend_avoids_the_part_of_infinite_lines_in_view():
    fig, ax = plt.subplots()
    ax.axline((.1, .1), slope=5, color='C0', label='by slope')
    ax.axline((.1, .2), (.8, .7), color='C3', label='by points')
    legend = ax.legend()
    fig.canvas.draw()
    # Lower right, away from the steep line crossing the upper left.
    assert np.round(legend.get_window_extent().bounds, 2).tolist() == [
        455.06, 59.74, 114.0, 46.73]


def test_aligned_labels_clear_the_siblings_tick_labels_in_a_first_layout():
    fig, (upper, lower) = plt.subplots(2, 1)
    fig.align_labels()
    upper.plot([0, 1], [-45, -3])
    lower.plot([0, 1], [-4.4, 0])
    lower.set_yticks([0, -0.785, -1.571, -2.356, -3.142, -3.927])
    upper.set_ylabel("Magnitude [dB]")
    fig.tight_layout()
    assert np.round(upper.get_position().bounds, 6).tolist() == [
        0.15096, 0.564815, 0.825603, 0.403935]


def test_add_collection_leaves_the_data_limits_of_3d_axes():
    block = np.array([[[1, 1, 0], [1, 0, 0], [0, 1, 0]],
                      [[1, 1, 0], [1, 1, 1], [1, 0, 0]]])
    ax = plt.figure().add_subplot(projection='3d')
    ax.add_collection(Poly3DCollection(block, facecolors='b'))
    assert np.round(ax.get_xlim(), 6).tolist() == [-0.020833, 1.020833]
    assert np.round(ax.get_zlim(), 6).tolist() == [-0.020833, 1.020833]
    ax.figure.canvas.draw()
    assert np.round(ax.get_xlim(), 6).tolist() == [-0.020833, 1.020833]
    # add_collection3d takes the collection's limits in.
    other = plt.figure().add_subplot(projection='3d')
    other.add_collection3d(Poly3DCollection(block))
    assert np.round(other.get_xlim(), 6).tolist() == [-0.072917, 1.072917]


def test_contours_of_complex_data_warn_at_each_cast():
    x = np.linspace(-2, 2, 5)
    X, Y = np.meshgrid(x, x)
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        plt.figure().add_subplot().contourf(X, Y, X + 1j * Y)
    # The data's maximum, its minimum, and the field contoured.
    assert [type(w.message).__name__ for w in caught] == ["ComplexWarning"] * 3


def test_figure_image_printed_to_its_tight_box_at_another_resolution():
    fig = plt.figure(dpi=100)
    fig.figimage(np.linspace(0, 1, 250000).reshape(500, 500), resize=True)
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png", dpi=80, bbox_inches="tight")
    pixels = np.asarray(Image.open(buffer).convert("RGB"))
    white = (pixels == 255).all(axis=2)
    assert pixels.shape == (416, 416, 3)
    # Clipped to 5.1 - -0.1 = 5.199999999999999 inches: 415 pixels.
    assert np.nonzero(~white.all(axis=0))[0][[0, -1]].tolist() == [0, 414]
    assert np.nonzero(~white.all(axis=1))[0][[0, -1]].tolist() == [1, 415]


def test_a_title_set_in_a_context_is_sized_by_its_font_size():
    fig, ax = plt.subplots()
    with pltrs.rc_context({"font.size": 12}):
        title = ax.set_title("classic")
        assert title.get_fontsize() == pytest.approx(14.4)
    assert title.get_fontsize() == pytest.approx(14.4)
    with pltrs.rc_context({"axes.titlesize": "small", "font.size": 20}):
        assert ax.set_title("y", loc="left").get_fontsize() == pytest.approx(16.66)


def test_a_loose_text3d_keeps_the_position_set_before_its_3d_properties():
    from pltrs.mpl_toolkits.mplot3d.art3d import Text3D
    ax = plt.figure().add_subplot(projection="3d")
    text = Text3D(0, 0, 0, text="", zdir="x")
    text.set_position([1.0, 0.5])
    text.set_3d_properties(-1.0, zdir="x")
    assert text.get_position() == (1.0, 0.5)
    assert text.get_position_3d() == (1.0, 0.5, -1.0)
    ax._add_text(text)
    assert text.get_position_3d() == (1.0, 0.5, -1.0)


@pytest.mark.skipif(shutil.which("latex") is None, reason="needs LaTeX")
def test_tex_reads_the_preamble_in_effect_where_it_draws(tmp_path):
    preamble = {"text.latex.preamble": r"\usepackage{nonexistentpackage}"}
    with pltrs.rc_context({"text.usetex": True, **preamble}):
        fig, ax = plt.subplots()
        ax.set_title("Title")
    # The texts keep their usetex; the preamble is the one in effect now.
    fig.savefig(tmp_path / "after.png")
    with pltrs.rc_context(preamble), pytest.raises(RuntimeError, match="nonexistentpackage"):
        fig.savefig(tmp_path / "inside.png")
