"""
Artists after removal, calls in unusual orders and layout engines with
odd parameters, as the differential tester's ``orders`` and hostile
layout-engine programs (``spec/diff/generators/orders.py``,
``hostile.py``) compare them with matplotlib 3.11: a removed or cleared
artist keeps measuring in its old figure, the Axes' two removal methods
fail as matplotlib's do, objects that are no artists stay in the Axes
``add_artist`` was given them, and an engine keeps what it is set to and
fails where matplotlib's fails as it lays the figure out. Each expected
value is recorded from matplotlib.
"""

import io

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import patches
from pltrs.transforms import Affine2D, CompositeGenericTransform


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def points(bbox):
    """The corners of *bbox*, rounded."""
    return np.round(bbox.get_points(), 6).tolist()


# -- removed and cleared artists keep measuring in their old figure -----------

def test_a_removed_patch_keeps_its_transform_and_extent():
    fig, ax = plt.subplots()
    ellipse = ax.add_patch(patches.Ellipse((0.5, 0.5), 0.4, 0.2, angle=30))
    fig.canvas.draw()
    ellipse.remove()
    assert ellipse.axes is None and ellipse.figure is None
    assert ellipse.get_figure() is None and not ellipse.stale
    assert type(ellipse.get_transform()) is CompositeGenericTransform
    assert points(ellipse.get_window_extent()) == [
        [238.582441, 188.706434], [417.417559, 286.493566]]
    assert points(ellipse.get_tightbbox()) == [
        [238.582441, 188.706434], [417.417559, 286.493566]]
    with pytest.raises(ValueError, match=r"^list.remove\(x\): x not in list$"):
        ellipse.remove()
    ax.add_artist(ellipse)
    assert ellipse.axes is ax and ellipse.figure is fig


def test_artists_cleared_with_their_figure_cannot_be_removed():
    fig, ax = plt.subplots()
    ellipse = ax.add_patch(patches.Ellipse((0.5, 0.5), 0.4, 0.2, angle=30))
    fig.canvas.draw()
    fig.clear()
    assert ellipse.axes is None and ellipse.figure is None
    with pytest.raises(NotImplementedError, match="^cannot remove artist$"):
        ellipse.remove()
    assert points(ellipse.get_window_extent()) == [
        [238.582441, 188.706434], [417.417559, 286.493566]]


def test_removed_artists_keep_what_their_last_draw_computed():
    fig, ax = plt.subplots()
    note = ax.annotate("note", xy=(0.2, 0.2), xytext=(0.6, 0.7),
                       arrowprops=dict(arrowstyle="->"))
    fig.canvas.draw()
    note.remove()
    assert np.round(note.get_transform().get_matrix(), 6).tolist() == [
        [496.0, 0.0, 80.0], [0.0, 369.6, 52.8], [0.0, 0.0, 1.0]]
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute '_get_renderer'"):
        note.get_tightbbox()
    fig, ax = plt.subplots()
    arrows = ax.quiver([0, 1], [0, 1], [1, 1], [1, 2])
    rect = ax.add_patch(patches.Rectangle((0.2, 0.3), 0.4, 0.4))
    shadow = ax.add_patch(patches.Shadow(rect, 0.05, -0.05))
    arrow = ax.add_patch(patches.FancyArrowPatch(
        (0.1, 0.1), (0.8, 0.6), arrowstyle="-|>", mutation_scale=20))
    fig.canvas.draw()
    for artist in (arrows, shadow, arrow):
        artist.remove()
    assert type(arrows.get_transform()) is Affine2D
    assert np.round(shadow.get_transform().get_matrix(), 6).tolist() == [
        [180.363636, 0.0, 224.040404], [0.0, 134.4, 147.066667], [0.0, 0.0, 1.0]]
    assert points(arrow.get_window_extent()) == [
        [150.090232, 104.506091], [459.44807, 269.164295]]


def test_a_removed_contour_set_is_out_of_its_figure():
    fig, ax = plt.subplots()
    contours = ax.contour(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)))
    contours.remove()
    assert contours.figure is None and contours.axes is None
    with pytest.raises(ValueError, match=r"^list.remove\(x\): x not in list$"):
        contours.remove()


def test_a_removed_legend_measures_and_comes_back():
    fig, ax = plt.subplots()
    ax.plot([1, 2], label="a")
    legend = ax.legend()
    fig.canvas.draw()
    legend.remove()
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute '_get_renderer'"):
        legend.get_window_extent()
    legend.remove()  # An Axes' legend forgets it again without fail.
    ax.add_artist(legend)
    assert legend.axes is ax and legend.figure is fig
    legend.remove()


def test_the_parts_an_artist_made_of_itself_cannot_be_removed():
    fig, ax = plt.subplots()
    ax.plot([1, 2], label="a")
    legend = ax.legend()
    title = ax.set_title("title")
    fig.canvas.draw()
    for part in (title, ax.spines["left"], legend.get_texts()[0], legend.get_frame(),
                 legend.get_lines()[0]):
        with pytest.raises(NotImplementedError, match="^cannot remove artist$"):
            part.remove()
    assert title.axes is ax and title.figure is fig
    fig.canvas.draw()


def test_artists_cleared_with_a_3d_axes_cannot_be_removed():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    line, = ax.plot([0, 1, 2], [2, 0, 1], [0, 1, 2])
    fig.canvas.draw()
    fig.clear()
    assert line.figure is None and line.axes is None
    with pytest.raises(NotImplementedError, match="^cannot remove artist$"):
        line.remove()


def test_clearing_leaves_the_axes_and_subfigures_stale():
    fig, ax = plt.subplots()
    ax.plot([1, 2])
    fig.canvas.draw()
    ax.clear()
    assert ax.xaxis.stale and ax.yaxis.stale
    fig.canvas.draw()
    assert not ax.xaxis.stale
    fig.clear()
    assert ax.xaxis.stale and ax.stale
    other, other_ax = plt.subplots()
    with pytest.raises(ValueError, match="^Can not reset the Axes"):
        ax.xaxis.axes = other_ax
    fig = plt.figure()
    sub = fig.subfigures(1, 2)[0]
    fig.canvas.draw()
    fig.clear()
    assert sub.stale


# -- the Axes' two removal methods --------------------------------------------

def test_a_removed_axes_is_its_own_no_more():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    fig.canvas.draw()
    ax.remove()
    assert ax.axes is None and ax.figure is None and ax.stale
    assert repr(ax) == "<Axes: >"
    assert points(ax.get_window_extent()) == [[80.0, 52.8], [576.0, 422.4]]
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute '_get_renderer'"):
        ax.get_tightbbox()
    with pytest.raises(KeyError):
        ax.remove()
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'add_subplot'"):
        ax.twinx()


def test_axes_cleared_with_their_figure_stay_of_it():
    fig = plt.figure()
    polar = fig.add_subplot(projection="polar")
    fig.canvas.draw()
    fig.clear()
    assert polar.figure is fig and polar.axes is polar
    assert points(polar.get_window_extent()) == [[143.2, 52.8], [512.8, 422.4]]
    with pytest.raises(KeyError):
        polar.remove()


def test_child_axes_are_taken_out_of_their_hosts_list():
    fig, ax = plt.subplots()
    inset = ax.inset_axes([0.6, 0.6, 0.3, 0.3])
    fig.canvas.draw()
    inset.remove()
    assert inset.axes is None and not inset.stale
    assert points(inset.get_window_extent()) == [[377.6, 274.56], [526.4, 385.44]]
    with pytest.raises(ValueError, match=r"^list.remove\(x\): x not in list$"):
        inset.remove()
    fig, ax = plt.subplots()
    inset = ax.inset_axes([0.6, 0.6, 0.3, 0.3])
    fig.canvas.draw()
    fig.clear()
    assert inset.figure is fig
    assert points(inset.get_window_extent()) == [[377.6, 274.56], [526.4, 385.44]]
    inset.remove()  # From the list its host had before it was cleared.


def test_a_colorbar_is_removed_once():
    fig, ax = plt.subplots()
    colorbar = fig.colorbar(ax.imshow(np.eye(3)))
    colorbar.remove()
    assert colorbar.ax.figure is fig
    with pytest.raises(KeyError):
        colorbar.remove()


def test_the_parts_of_another_figure_cannot_be_added_to_a_figure():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    fig.canvas.draw()
    other = plt.figure()
    for part in (ax.xaxis, ax.xaxis.get_major_ticks()[0], ax.get_xticklabels()[0],
                 ax.table(cellText=[["a"]])[0, 0]):
        with pytest.raises(RuntimeError, match="^Can not put single artist in more than one figure$"):
            other.add_artist(part)


# -- objects that are no artists, added to an Axes ----------------------------

def test_a_container_stays_in_the_axes_it_was_added_to():
    fig, ax = plt.subplots()
    bars = ax.bar(["a", "b"], [1, 2])
    fig.canvas.draw()
    with pytest.raises(AttributeError, match="'BarContainer' object has no attribute 'set_figure'"):
        ax.add_artist(bars)
    assert bars.axes is ax
    assert any(child is bars for child in ax.get_children())
    assert ax.artists[-1] is bars
    with pytest.raises(AttributeError, match="'BarContainer' object has no attribute 'get_animated'"):
        fig.canvas.draw()
    with pytest.raises(AttributeError, match="'BarContainer' object has no attribute 'zorder'"):
        fig.savefig(io.BytesIO(), format="png")
    with pytest.raises(AttributeError, match="'BarContainer' object has no attribute 'set_figure'"):
        fig.add_artist(bars)
    ax.cla()
    assert bars.axes is None
    assert not any(child is bars for child in ax.get_children())
    fig.canvas.draw()


def test_a_collection_of_another_figure_is_drawn_where_it_was_added():
    fig, ax = plt.subplots()
    points = ax.scatter([0, 1, 2], [0, 1, 4], s=400)
    fig.canvas.draw()
    other, other_ax = plt.subplots()
    with pytest.raises(RuntimeError, match="^Can not put single artist in more than one figure$"):
        other_ax.add_collection(points)
    assert points.axes is ax and points.figure is fig
    assert other_ax.get_children()[0] is points
    assert (len(other_ax.artists), len(other_ax.collections)) == (0, 1)
    other.canvas.draw()
    # Drawn there through its own figure's transforms, as matplotlib draws
    # it: its markers' blue where they are in its figure.
    drawn = np.asarray(other.canvas.buffer_rgba())[..., :3]
    own = np.asarray(fig.canvas.buffer_rgba())[..., :3]
    blue = [31, 119, 180]
    assert np.array_equal((drawn == blue).all(axis=-1), (own == blue).all(axis=-1))
    assert (drawn == blue).all(axis=-1).any()
    points.remove()
    assert points.figure is None and ax.collections[0] is points
    with pytest.raises(ValueError, match=r"^list.remove\(x\): x not in list$"):
        points.remove()


def test_parts_of_another_figure_keep_it():
    fig, ax = plt.subplots()
    other = plt.figure()
    for part in (ax.xaxis, ax.xaxis.get_major_ticks()[0]):
        with pytest.raises(RuntimeError, match="^Can not put single artist in more than one figure$"):
            part.set_figure(other)


# -- layout engines keep what they are set to ---------------------------------

def test_a_constrained_layout_engine_fails_as_it_lays_out():
    fig = plt.figure(layout="constrained")
    ax = fig.add_subplot()
    ax.plot([1, 2])
    engine = fig.get_layout_engine()
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'get_size_inches'"):
        engine.execute(None)
    fig.canvas.draw()
    laid_out = ax.get_position().bounds
    engine.set(h_pad=[])
    fig.canvas.draw()  # matplotlib's draw passes over the layout's ValueError.
    assert ax.get_position().bounds == laid_out
    with pytest.raises(ValueError, match="The truth value of an empty array is ambiguous"):
        engine.execute(fig)
    engine.set(h_pad=0.04167, w_pad="0.1")
    with pytest.raises(TypeError, match="ufunc 'divide' not supported for the input types"):
        fig.canvas.draw()
    engine.set(w_pad=0.1, wspace="0.2")
    with pytest.raises(TypeError, match=r"unsupported operand type\(s\) for /: 'str' and 'int'"):
        fig.canvas.draw()
    engine.set(wspace=0.02, rect=(0,))
    with pytest.raises(IndexError, match="tuple index out of range"):
        fig.canvas.draw()
    engine.set(w_pad=3 / 72, rect=(0, 0, 1, 0.9, 0.9))
    fig.canvas.draw()  # In the first four.
    assert np.allclose(ax.get_position().bounds,
                       [0.0560765, 0.057871, 0.9374125, 0.833448], atol=1e-6)


def test_a_tight_layout_engine_fails_as_it_lays_out():
    fig = plt.figure(layout="tight")
    fig.add_subplot().plot([1, 2])
    engine = fig.get_layout_engine()
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute '_get_renderer'"):
        engine.execute(None)
    engine.set(pad="2")
    with pytest.raises(TypeError, match="can't multiply sequence by non-int of type 'float'"):
        fig.canvas.draw()
    engine.set(pad=2, rect=(0, 0, 1))
    with pytest.raises(ValueError, match=r"not enough values to unpack \(expected 4, got 3\)"):
        engine.execute(fig)
    fig.canvas.draw()  # matplotlib's draw passes over the layout's ValueError.


# -- the writers' own errors ---------------------------------------------------

def test_a_jpeg_quality_that_is_no_number_is_pillows_to_refuse():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="^Invalid quality setting$"):
        fig.canvas.print_jpeg(io.BytesIO(), pil_kwargs={"quality": "best"})
