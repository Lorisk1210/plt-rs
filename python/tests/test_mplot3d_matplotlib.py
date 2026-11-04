"""
3-D axes as matplotlib 3.11's behave: what the differential tester
(``cargo xtask diff``) found pltrs' ``mpl_toolkits.mplot3d`` answering
otherwise. Every expected value was recorded from matplotlib 3.11.2.
"""

import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections as mcoll
from pltrs import patches as mpatches
from pltrs import path as mpath
from pltrs.backend_bases import MouseButton, MouseEvent
from pltrs.lines import Line2D
from pltrs.mpl_toolkits.mplot3d import art3d, proj3d


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _axes3d():
    fig = plt.figure()
    return fig, fig.add_subplot(projection="3d")


def test_the_trees_follow_the_square_box():
    fig, ax = _axes3d()
    assert ax.bbox.bounds == pytest.approx((80, 52.8, 496, 369.6))
    ax.get_position()
    assert ax.bbox.bounds == pytest.approx((143.2, 52.8, 369.6, 369.6))
    fig.canvas.draw()
    assert ax.transAxes.transform((1, 1)) == pytest.approx((512.8, 422.4))
    assert ax.zaxis.get_tick_space() == 8


def test_the_projection_of_the_last_draw():
    fig, ax = _axes3d()
    ax.plot([0, 1, 2], [0, 1, 0], [1, 2, 3])
    assert ax.M is None and ax.invM is None
    assert ax.format_coord(0.01, 0.01) == ""
    for read in (ax.get_axis_position, ax.xaxis.active_pane):
        with pytest.raises(TypeError, match=r"\*: 'NoneType' and 'float'"):
            read()
    fig.canvas.draw()
    assert np.allclose(ax.M, ax.get_proj())
    position = ax.get_axis_position()
    assert position == (False, True, False)
    assert all(type(high) is np.bool_ for high in position)
    assert ax.format_coord(0.01, 0.01) == "x=0.5754, y pane=1.0729, z=1.4442"
    loc = ax.xaxis.active_pane()[1]
    assert type(loc) is np.float64 and loc == pytest.approx(-0.14583333333333334)
    point, pane = ax._calc_coord(0.01, 0.01)
    assert type(pane) is np.int64 and pane == 1
    ax.view_init(10, 20)
    assert ax.get_axis_position() == (False, True, False), "the view of the draw"
    assert not np.allclose(ax.M, ax.get_proj())


def test_axes_off_and_on():
    fig, ax = _axes3d()
    assert (ax.axison, ax._axis3don) == (False, True)
    ax.set_axis_off()
    assert (ax.axison, ax._axis3don) == (False, False)
    ax.axis("on")
    assert (ax.axison, ax._axis3don) == (False, True)


def test_settings_kept_as_given():
    fig, ax = _axes3d()
    assert ax.get_subplotspec() is ax.get_subplotspec()
    ax.set_adjustable("datalim")
    assert ax.get_adjustable() == "datalim"
    ax.plot([0, 1, 2], [0, 1, 0], [1, 2, 3])
    with pytest.warns(UserWarning, match="set_ticklabels"):
        ax.set_xticklabels(["a", "b"])
    ax.set_xticks([0, 1, 2])
    assert ax.get_xticks().dtype == np.int64
    with pytest.raises(ValueError, match="FixedLocator locations"):
        ax.set_xticklabels(["a", "b"])
    other = plt.figure().add_subplot(projection="3d", elev=10, azim=20, roll=30)
    assert (other.elev, other.azim, other.roll) == (10, 20, 30)
    assert type(other.elev) is int and type(other.initial_roll) is int


def test_lines_scatters_and_texts_keep_their_data():
    fig, ax = _axes3d()
    line = ax.plot([0, 1, 2], [0, 1, 0], [1, 2, 3])[0]
    xs, ys, zs = line.get_data_3d()
    assert xs.dtype == np.int64 and zs.dtype == np.float64
    loose = art3d.Line3D([0, 1], [0, 1], [0, 1])
    assert loose.get_data_3d() == ([0, 1], [0, 1], [0, 1])
    sc = ax.scatter([0, 1], [0, 1], [0, 1], c=[1, 2])
    assert sc.get_sizes().dtype == np.int64
    x3, y3, z3 = sc._offsets3d
    assert np.ma.isMA(x3) and z3.dtype == np.int64
    text = ax.text(1, 2, 3, "three")
    fig.canvas.draw()
    assert np.ma.isMA(line.get_xdata()) and not np.ma.isMA(line.get_xdata(orig=False))
    assert text.get_position() == (1, 2) and type(text.get_position()[0]) is int
    assert text.get_rotation() == 0.0
    text.set_x(0.25)
    assert text.get_position_3d() == (0.25, 2, 3)


def test_polygons_answer_with_their_projection():
    fig, ax = _axes3d()
    poly = art3d.Poly3DCollection(
        [[(0, 0, 0), (1, 0, 0), (1, 1, 0)], [(0, 0, 1), (1, 0, 1), (1, 1, 1)]],
        facecolors=["r", "g"])
    ax.add_collection3d(poly)
    assert poly.get_facecolor()[:, 1].tolist() == [0.0, 0.5], "deepest first"
    assert len(poly.get_paths()) == 2
    poly.set_facecolor("b")
    assert poly.get_facecolor()[:, 1].tolist() == [0.0, 0.5], "until projected again"
    with pytest.raises(ValueError, match=r"from shape \(2,2\) into shape \(2,3\)"):
        art3d.Poly3DCollection([[(0, 0), (1, 1)]])
    with pytest.raises(AttributeError, match="get_proj"):
        art3d.Poly3DCollection([[(0, 0, 0), (1, 1, 1), (0, 1, 1)]]).get_facecolor()


def test_2d_plotting_leaves_the_data_limits_of_3d_axes():
    fig, ax = _axes3d()
    ax.scatter([1, 2], [3, 4], [5, 6])
    ax.bar([1, 2], [3, 4])
    assert ax.ignore_existing_data_limits
    ax.errorbar([1, 2], [3, 4], [5, 6], zerr=0.5)
    assert ax.dataLim.bounds == (1, 3, 1, 1)


def test_the_axes_of_3d_axes():
    fig, ax = _axes3d()
    ax.plot([0, 1, 2], [0, 2, 4], [1, 2, 3])
    assert type(ax.zaxis.line) is Line2D
    assert ax.zaxis.line.get_data() == ((0, 0), (0, 0))
    assert type(ax.zaxis.gridlines) is art3d.Line3DCollection
    fig.canvas.draw()
    assert len(ax.zaxis.line.get_xdata()) == 2
    ax.zaxis.line.set_color("r")
    assert ax.zaxis.line.get_color() == "r"


def test_logit_ticks_fail_as_matplotlibs():
    fig, ax = _axes3d()
    ax.plot(np.linspace(0, 2 * np.pi, 7), np.sin(np.linspace(0, 2 * np.pi, 7)),
            [3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])
    ax.set_ylim((-1, 1))
    ax.set_yscale("logit")
    ax.set_yticks([0.5])
    assert ax.get_yticks().tolist() == [0.5]
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", RuntimeWarning)
        for read in (ax.get_yticklabels, ax.get_tightbbox):
            with pytest.raises(ValueError, match="cannot convert float NaN to integer"):
                read()


def test_autoscaling_waits_for_the_draw():
    fig, ax = _axes3d()
    ax.scatter([0, 1, 2], [-1, 0, 1], [10, 20, 30])
    ax.margins(0.1, 0.2, 0.3)
    assert ax.get_zlim() == pytest.approx((3.3333333333333335, 36.666666666666664))
    ax.set_zmargin(0.5)
    assert ax.get_zlim() == pytest.approx((3.3333333333333335, 36.666666666666664))
    fig.canvas.draw()
    assert ax.get_zlim() == pytest.approx((-0.8333333333333333, 40.833333333333336))
    ax.autoscale(axis="w")
    with pytest.warns(UserWarning, match="identical low and high zlims"):
        ax.set_zlim(1, 1)
    with pytest.raises(ValueError, match="Axis limits 0, inf cannot be infinite"):
        ax.set_zlim((0, np.inf))


def test_mouse_and_toolbar_navigation():
    fig, ax = _axes3d()
    ax.scatter(0, 0, 0)
    fig.canvas.draw()
    MouseEvent._from_ax_coords("button_press_event", ax, (0, 0), MouseButton.LEFT)._process()
    MouseEvent._from_ax_coords("motion_notify_event", ax, (0.01, 0.01),
                               MouseButton.LEFT)._process()
    assert type(ax.elev) is np.float64 and type(ax.roll) is np.float64
    assert ax.format_coord(0.01, 0.01).startswith("elevation=")
    MouseEvent._from_ax_coords("button_release_event", ax, (0, 0), MouseButton.LEFT)._process()
    ax.disable_mouse_rotation()
    assert (ax._rotate_btn, ax._pan_btn, ax._zoom_btn) == ([], [], [])
    xlim = ax.get_xlim()
    ax._zoom_data_limits(0.5, 0.5, 0.5)
    assert np.ptp(ax.get_xlim()) == pytest.approx(np.ptp(xlim) / 2)
    assert ax._dist == 10 and ax._focal_length == 1 and ax._draw_grid
    position = ax.get_position(original=True).bounds
    ax._set_position([0.1, 0.1, 0.5, 0.5], "active")
    assert ax.get_position(original=True).bounds == position
    ax.set_position([0.2, 0.2, 0.5, 0.6])
    assert ax.get_position().bounds == pytest.approx((0.225, 0.2, 0.45, 0.6))
    assert not ax.get_in_layout()


def test_shared_views_take_the_angles_given():
    fig, ax = _axes3d()
    ax.view_init(5, 6, 7)
    third = fig.add_subplot(2, 2, 1, projection="3d")
    third.shareview(ax)
    assert (third.elev, third.azim) == (5, 6) and type(third.elev) is int
    with pytest.raises(ValueError, match="view angles are already shared"):
        third.shareview(fig.add_subplot(2, 2, 2, projection="3d"))


def test_proj3d_without_a_drawn_matrix():
    fig, ax = _axes3d()
    with pytest.raises(TypeError, match="'NoneType' and 'int'"):
        proj3d.proj_transform(1, 1, 1, ax.M)
    M = proj3d.world_transformation(0, 1, 0, 2, 0, 3)
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        clipped = proj3d.proj_transform_clip(1, 1, 1, M)
    assert [np.shape(c) for c in clipped] == [()] * 4
    assert type(fig.add_subplot(2, 2, 2, projection="3d").get_children()) is list


def test_lifted_collections_have_3d_classes():
    fig, ax = _axes3d()
    X, Y = np.meshgrid(np.linspace(-1, 1, 6), np.linspace(-2, 2, 5))
    ax.contour(X, Y, np.cos(X) * np.sin(Y), extend3d=True, stride=2)
    assert type(ax.get_children()[0]).__name__ == "Poly3DCollection"
    pp = mpatches.PathPatch(mpath.Path([(0, 0), (1, 0), (1, 1)], [1, 2, 2]))
    ax.add_patch(pp)
    art3d.pathpatch_2d_to_3d(pp, z=0.5, zdir="x")
    assert pp.get_zorder() == 1, "computed z-orders wait for a draw"
    assert isinstance(mcoll.LineCollection([]), mcoll.Collection)
