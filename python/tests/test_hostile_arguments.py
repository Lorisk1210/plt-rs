"""
What the differential tester's hostile and unusual-order programs
(``spec/diff/generators/hostile.py``, ``orders.py``) found pltrs doing
otherwise than matplotlib 3.11: values matplotlib's setters keep
unchecked, the numbers it takes as they are, and Axes and artists
removed, deleted and moved. Each expected value is recorded from
matplotlib.
"""

import io

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.offsetbox import AnchoredOffsetbox, TextArea


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


# -- values matplotlib keeps unchecked ----------------------------------------

def test_any_picker_is_kept():
    fig, ax = plt.subplots()
    text = ax.text(0.5, 0.5, "t")
    ax.set(picker="False")
    text.set_picker("x")
    assert (ax.get_picker(), text.get_picker()) == ("False", "x")
    assert ax.pickable() and text.pickable()
    line, = ax.plot([1, 2])
    with pytest.raises(ValueError, match="^pick radius should be a distance$"):
        line.set_picker("False")


def test_pick_radii_take_numpy_numbers():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2])
    points = ax.scatter([1, 2], [3, 4])
    line.set_pickradius(np.int64(8))
    points.set_pickradius(np.float32(2.5))
    assert line.get_pickradius() == 8 and isinstance(line.get_pickradius(), np.int64)
    assert points.get_pickradius() == np.float32(2.5)
    with pytest.raises(ValueError, match="^pickradius must be a real-valued number, not '8'$"):
        points.set_pickradius("8")


def test_a_zorder_that_is_no_number_is_kept():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2])
    line.set_zorder("3")
    assert line.get_zorder() == "3" and line.zorder == "3"
    line.set_zorder(None)
    assert line.get_zorder() == 2
    ax.set_zorder(float("nan"))
    assert np.isnan(ax.get_zorder())
    fig.canvas.draw()


def test_agg_filters_are_called_as_given_when_drawn():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2, 3])
    line.set_agg_filter(3)
    assert line.get_agg_filter() == 3
    with pytest.raises(TypeError, match="^'int' object is not callable$"):
        fig.canvas.draw()
    fig.savefig(io.BytesIO(), format="pdf")
    line.set_agg_filter(0)
    with pytest.raises(TypeError, match="^'int' object is not callable$"):
        fig.canvas.draw()
    line.set_agg_filter(None)
    fig.canvas.draw()


def test_offset_boxes_draw_no_agg_filter():
    fig, ax = plt.subplots()
    box = AnchoredOffsetbox("upper left", child=TextArea("t"))
    ax.add_artist(box)
    box.set_agg_filter(3)
    assert box.get_agg_filter() == 3
    fig.canvas.draw()


def test_path_effects_are_drawn_as_given():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2, 3])
    text = ax.text(0.5, 0.5, "x")
    text.set_path_effects("x")
    assert text.get_path_effects() == "x"
    with pytest.raises(AttributeError, match="^'str' object has no attribute 'draw_path'$"):
        fig.canvas.draw()
    text.set_path_effects([])
    line.set_path_effects(3)
    assert line.get_path_effects() == 3
    with pytest.raises(TypeError, match="^'int' object is not iterable$"):
        fig.canvas.draw()
    with pytest.raises(TypeError, match="^'int' object is not iterable$"):
        fig.savefig(io.BytesIO(), format="pdf")
    line.set_path_effects(0)
    fig.canvas.draw()


def test_a_transform_that_is_no_transform_is_kept():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2])
    line.set_transform(3)
    assert line.get_transform() == 3 and line.is_transform_set()
    ax.set(transform="x")
    assert ax.get_transform() == "x"


def test_margins_that_are_not_finite_add_nothing():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    ax.set_xmargin(float("nan"))
    ax.set_ymargin(float("inf"))
    assert np.isnan(ax.get_xmargin()) and ax.get_ymargin() == float("inf")
    fig.canvas.draw()
    assert ax.get_xlim() == (0.0, 2.0)
    assert ax.get_ylim() == (1.0, 3.0)
    with pytest.raises(ValueError, match="^margin must be greater than -0.5$"):
        ax.set_xmargin(-0.5)


def test_urls_and_hatch_widths_are_kept_as_given():
    fig, ax = plt.subplots()
    points = ax.scatter([1, 2], [3, 4])
    points.set_urls([0])
    assert points.get_urls() == [0]
    fig.savefig(io.BytesIO(), format="svg")
    points.set_urls(None)
    assert points.get_urls() == [None]
    bar = ax.bar([1, 2], [3, 4])[0]
    bar.set_hatch_linewidth(None)
    points.set_hatch_linewidth("2")
    assert bar.get_hatch_linewidth() is None
    assert points.get_hatch_linewidth() == "2"


def test_font_features_are_kept_as_a_tuple():
    fig, ax = plt.subplots()
    text = ax.text(0.5, 0.5, "t")
    text.set_fontfeatures([0, 1])
    assert text.get_fontfeatures() == (0, 1)
    text.set_fontfeatures("ab")
    assert text.get_fontfeatures() == ("a", "b")
    text.set_fontfeatures(None)
    assert text.get_fontfeatures() is None


def test_alphas_take_numpy_numbers():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2])
    points = ax.scatter([1, 2], [3, 4])
    points.set_alpha(np.float32(0.5))
    line.set_alpha(np.int64(1))
    assert points.get_alpha() == np.float32(0.5) and isinstance(points.get_alpha(), np.float32)
    assert line.get_alpha() == 1 and isinstance(line.get_alpha(), np.int64)
    with pytest.raises(TypeError, match="^alpha must be numeric or None, not a string$"):
        points.set_alpha("0.5")
    with pytest.raises(TypeError, match="^alpha must be numeric or None, not <class 'str'>$"):
        line.set_alpha("0.5")
    with pytest.raises(ValueError, match=r"^alpha must be between 0 and 1, inclusive, but min is 0\.0, max is 2\.0$"):
        points.set_alpha([0.0, 2.0])


def test_geographic_grids_refuse_what_numpy_refuses():
    fig = plt.figure()
    ax = fig.add_subplot(projection="aitoff")
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for \+: 'int' and 'str'$"):
        ax.set_latitude_grid("15")
    with pytest.raises(ValueError, match="^arange: cannot compute length$"):
        ax.set_longitude_grid(float("nan"))
    with pytest.raises(ZeroDivisionError, match="^division by zero$"):
        ax.set_latitude_grid(0)
    with pytest.raises(TypeError, match="^loop of ufunc does not support argument 0 of type NoneType"):
        ax.set_longitude_grid_ends(None)
    ax.set_latitude_grid(-15)
    assert len(ax.yaxis.get_majorticklocs()) == 0
    ax.set_longitude_grid_ends(float("nan"))


def test_an_anchored_box_gives_its_child_its_axes():
    fig, ax = plt.subplots()
    box = AnchoredOffsetbox("upper left", child=TextArea("t"))
    ax.add_artist(box)
    child = TextArea("u")
    box.set_child(child)
    assert child.axes is ax and box.get_child() is child
    with pytest.raises(AttributeError, match="^'int' object has no attribute 'axes'"):
        box.set_child(3)
    assert box.get_child() == 3


# -- unusual orders ------------------------------------------------------------

def test_deleted_axes_stay_of_their_figure():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2, 3])
    fig.canvas.draw()
    fig.delaxes(ax)
    assert line.axes is ax and ax.figure is fig and fig.axes == []
    assert len(ax.plot([1, 2])) == 1
    fig.canvas.draw()
    assert fig.add_axes(ax) is ax and fig.axes == [ax]
    fig.delaxes(ax)
    with pytest.raises(KeyError):
        fig.delaxes(ax)


def test_a_second_remove_fails_as_matplotlib_s():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2, 3])
    line.remove()
    assert line.axes is None and line.figure is None
    with pytest.raises(ValueError, match=r"^list\.remove\(x\): x not in list$"):
        line.remove()
    ax.add_artist(line)
    assert line.axes is ax
    line.remove()
    assert line.axes is None
    other = fig.add_subplot(2, 1, 2)
    other.remove()
    with pytest.raises(KeyError):
        other.remove()
    with pytest.raises(NotImplementedError, match="^cannot remove artist$"):
        plt.Line2D([0], [1]).remove()


def test_artists_moved_across_axes_and_figures():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2, 3])
    fig2, ax2 = plt.subplots()
    with pytest.raises(RuntimeError, match="^Can not put single artist in more than one figure$"):
        fig2.add_artist(line)
    with pytest.raises(ValueError, match="^Can not reset the Axes. You are probably trying to reuse an "
                                         "artist in more than one Axes which is not supported$"):
        ax2.add_artist(line)
    with pytest.raises(RuntimeError, match="^Can not put single artist in more than one figure$"):
        ax2.add_line(line)
    assert line.axes is ax and line.figure is fig
