"""
What the differential tester's hostile programs (``cargo xtask diff
--generated``, ``spec/diff/generators/hostile.py``) found pltrs'
``mpl_toolkits.mplot3d`` doing otherwise than matplotlib 3.11: values its
3-D artists and axes keep unchecked and fail on when they are drawn, the
order of its checks, and the exceptions it raises. Every expected value
was recorded from matplotlib 3.11.2.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections as mcoll
from pltrs import patches as mpatches
from pltrs import path as mpath
from pltrs.mpl_toolkits.mplot3d import art3d

ISFINITE = "ufunc 'isfinite' not supported for the input types"
INHOMOGENEOUS = "setting an array element with a sequence"
X = np.linspace(0, 2 * np.pi, 7)
Y = np.sin(X)
VALUES = np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])
Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))


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


# -- data matplotlib's draws fail on ------------------------------------------

def test_a_patch_without_an_outline_fails_to_draw():
    fig, ax = _axes3d()
    circle = mpatches.Circle((0.5, 0.5), 0.2)
    ax.add_patch(circle)
    art3d.patch_2d_to_3d(circle, z=0.5, zdir="y")
    fig.canvas.draw()
    circle.set_3d_properties([], 1)
    for call in (fig.canvas.draw, circle.do_3d_projection):
        with pytest.raises(ValueError, match=r"^not enough values to unpack "
                           r"\(expected 3, got 0\)$"):
            call()
    circle.set_3d_properties([(0.5, 0.5), (1, 1), (0, 2)], 1)
    fig.canvas.draw()


def test_a_path_patch_out_of_its_scales_fails_to_draw():
    fig, ax = _axes3d()
    square = mpath.Path([(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)], closed=True)
    patch = mpatches.PathPatch(mpath.Path([(0, 0), (1, 1), (1, 0)], [1, 2, 2]))
    ax.add_patch(patch)
    art3d.pathpatch_2d_to_3d(patch, z=1, zdir="x")
    patch.set_3d_properties(square, float("nan"))
    with pytest.raises(TypeError, match="not 'zip'$"):
        fig.canvas.draw()
    patch.set_3d_properties(square, "ab")
    with pytest.raises(TypeError, match=ISFINITE):
        fig.canvas.draw()
    patch.set_3d_properties(square, 1)
    fig.canvas.draw()


def test_an_infinite_limit_makes_the_projection_singular():
    fig, ax = _axes3d()
    ax.plot(X, Y, VALUES)
    ax.xaxis.set_view_interval(float("nan"), 4.0)
    assert np.isnan(ax.xaxis.get_view_interval()).all()
    fig.canvas.draw()
    ax.xaxis.set_view_interval(0.0, 4.0)
    assert list(ax.xaxis.get_view_interval()) == [4.0, 0.0]
    ax.xaxis.set_view_interval(float("inf"), 4.0)
    with pytest.raises(np.linalg.LinAlgError, match="^Singular matrix$"):
        fig.canvas.draw()
    with pytest.raises(ValueError, match="^Axis limits cannot be NaN or Inf$"):
        ax.xaxis.set_inverted(True)


def test_ticks_widen_the_view_as_pythons_min_and_max_do():
    fig, ax = _axes3d()
    ax.plot([0, 1], [0, 1], [0, 1])
    ax.xaxis.set_view_interval(-0.4, float("inf"))
    ax.xaxis.set(ticks=[float("nan"), 1, 2])
    assert np.isnan(ax.xaxis.get_view_interval()).all()
    ax.xaxis.set(ticks=[0, 1, 2])
    assert list(ax.xaxis.get_view_interval()) == [2.0, 0.0]
    fig.canvas.draw()


def test_a_nan_minimum_alpha_draws():
    fig, ax = _axes3d()
    ax.scatter3D(X, Y, VALUES, depthshade_minalpha=float("nan"))
    fig.canvas.draw()
    ax.cla()
    ax.bar([1, 2], [3, 1])


def test_texts_keep_their_positions_unchecked():
    fig, ax = _axes3d()
    with pytest.raises(AttributeError,
                       match="^'Text' object has no property 'nonexistent_kw'$"):
        ax.text(1, 1, 1, "txt", nonexistent_kw=1)
    text = ax.text(1, 1, None, "txt")
    assert text.get_position_3d() == (1, 1, None)
    with pytest.raises(TypeError, match=ISFINITE):
        fig.canvas.draw()
    text.set_z(2)
    fig.canvas.draw()
    with pytest.raises(IndexError, match="^tuple index out of range$"):
        text.set_position_3d((5,))
    assert text.get_position_3d() == (5, 1, 2)
    text.set_position_3d((1, 1, 1, 1))
    assert text.get_position_3d() == (1, 1, 1)
    with pytest.raises(IndexError, match="^string index out of range$"):
        text.set_position_3d("ab")
    assert text.get_position_3d() == ("a", "b", 1)
    with pytest.raises(TypeError, match="^'NoneType' object is not subscriptable$"):
        text.set_position_3d(None)
    text.set_position_3d((1, 2, 3))
    text.set_3d_properties(float("nan"))
    assert text.get_position_3d()[:2] == (1, 2)
    assert np.isnan(text.get_position_3d()[2])
    fig.canvas.draw()


def test_lines_and_scatters_of_dates_fail_to_draw():
    fig, ax = _axes3d()
    days = np.datetime64("2024-01-01") + np.arange(7)
    for make in (lambda: ax.plot(X, days, VALUES)[0],
                 lambda: ax.scatter3D(X, Y, days)):
        artist = make()
        with pytest.raises(TypeError, match="'float' and 'datetime.date'$"):
            fig.canvas.draw()
        artist.remove()
        fig.canvas.draw()


def test_wireframes_stems_and_line_collections_of_no_numbers_fail_to_draw():
    fig, ax = _axes3d()
    X2, Y2 = np.meshgrid(np.linspace(-1, 1, 6), np.linspace(-2, 2, 5))
    for make in (lambda: ax.plot_wireframe(X2, None, np.cos(X2)),
                 lambda: ax.add_collection3d(
                     mcoll.LineCollection([[(0, 0), (1, 1)]]), zs=None),
                 lambda: ax.stem(X, Y, VALUES, bottom=None)):
        artist = make()
        with pytest.raises(TypeError, match=ISFINITE):
            fig.canvas.draw()
        artist.remove()
        fig.canvas.draw()
    with pytest.raises(TypeError, match="ufunc 'minimum' did not contain a loop"):
        ax.add_collection3d(mcoll.LineCollection([[(0, 0), (1, 1)]]), zs="ab")


def test_patch_collections_with_more_positions_than_offsets_fail_to_draw():
    fig, ax = _axes3d()
    patches = [mpatches.Rectangle((0, 0), 1, 2), mpatches.Circle((1, 1), 0.5)]
    collection = art3d.Patch3DCollection(patches, zs=[0, 1], zdir="x")
    xs, ys, zs = collection._offsets3d
    assert (list(xs), list(ys), list(zs)) == ([0, 1], [0.0], [0.0])
    ax.add_collection3d(collection)
    for call in (fig.canvas.draw, collection.do_3d_projection):
        with pytest.raises(ValueError, match=INHOMOGENEOUS):
            call()
    with pytest.raises(TypeError, match="^'NoneType' object is not subscriptable$"):
        art3d.Patch3DCollection(patches[:1], zdir=None)


# -- what the 3-D axes keeps ---------------------------------------------------

def test_view_angles_are_kept_unchecked():
    fig, ax = _axes3d()
    ax.plot([0, 1], [0, 1], [0, 1])
    ax.view_init(30, 60, roll="x")
    assert ax.roll == "x"
    for call in (fig.canvas.draw, ax.get_proj):
        with pytest.raises(TypeError,
                           match=r"^can only concatenate str \(not \"int\"\) to str$"):
            call()
    ax.view_init("30", 60)
    with pytest.raises(TypeError, match="ufunc 'deg2rad' not supported"):
        fig.canvas.draw()
    ax.view_init(30, 60)
    fig.canvas.draw()
    ax.azim = np.array([1, 2])
    with pytest.raises(ValueError, match=INHOMOGENEOUS):
        fig.canvas.draw()
    ax.azim = 3
    fig.canvas.draw()


def test_margins_and_view_margins_as_matplotlib_takes_them():
    fig, ax = _axes3d()
    ax.plot([0, 1], [0, 1], [0, 1])
    fig.canvas.draw()
    assert ax.set_zlim3d(0, 3, view_margin=float("inf")) == (0.0, 3.0)
    with pytest.raises(ValueError, match="^Axis limits cannot be NaN or Inf$"):
        ax.set_zlim3d(0, 3, view_margin=float("nan"))
    with pytest.raises(ValueError, match="^Axis limits cannot be NaN or Inf$"):
        ax.set_xbound(float("nan"), 2)
    with pytest.raises(TypeError, match="'>' not supported"):
        ax.set_xbound(view_margin="0.1")
    with pytest.raises(TypeError, match="'<' not supported"):
        ax.set_zbound("0", 5)
    ax.set_xbound(0, 3)
    ax.set_zmargin(float("nan"))
    assert np.isnan(ax.get_zmargin())
    fig.canvas.draw()


# -- the order and the words of matplotlib's checks ---------------------------

def test_contour_sets_are_placed_as_matplotlib_places_them():
    fig, ax = _axes3d()
    ax.add_contour_set(ax.contour(np.arange(6), np.arange(5), Z), zdir="")
    with pytest.raises(TypeError, match=r"^can only concatenate str \(not "
                       r"\"NoneType\"\) to str$"):
        ax.add_contour_set(ax.contour(np.arange(6), np.arange(5), Z), zdir=None)
    with pytest.raises(AttributeError,
                       match="^'NoneType' object has no attribute 'levels'$"):
        ax.add_contour_set(None)
    ax.cla()
    cset = ax.contour(np.arange(6), np.arange(5), Z)
    ax.add_contour_set(cset, offset=list(range(len(cset.get_paths()))))
    with pytest.raises(ValueError, match="could not be broadcast"):
        ax.add_contour_set(cset, offset=())
    fig.canvas.draw()
    ax.add_contour_set(cset, offset="ab")
    with pytest.raises(TypeError, match=ISFINITE):
        fig.canvas.draw()
    ax.cla()
    levels = ax._add_contourf_set(ax.contourf(np.arange(6), np.arange(5), Z))
    assert levels.tolist() == [-1.75, -1.25, -0.75, -0.25, 0.25, 0.75, 1.25, 1.75]
    with pytest.raises(TypeError, match="not \"NoneType\""):
        ax.contour(np.arange(6), np.arange(5), Z, zdir=None)
    with pytest.raises(AttributeError, match="has no attribute 'do_3d_projection'$"):
        fig.canvas.draw()


def test_stems_check_their_arguments_as_matplotlib_does():
    fig, ax = _axes3d()
    with pytest.raises(ValueError, match="^zero-size array to reduction operation "
                       "minimum which has no identity$"):
        ax.stem(np.empty((0,)), Y, VALUES)
    with pytest.raises(TypeError, match="^'NoneType' object is not iterable$"):
        ax.stem(None, Y, VALUES)
    with pytest.raises(TypeError, match="^object of type 'NoneType' has no len\\(\\)$"):
        ax.stem(X, Y, VALUES, linefmt=None)
    with pytest.raises(TypeError, match="^plot\\(\\) for multiple values for "
                       "argument 'zs'$"):
        ax.stem(X, Y, VALUES, basefmt=None)
    with pytest.raises(ValueError, match="^x, y, and format string must not be None$"):
        ax.stem(X, Y, VALUES, markerfmt=None)
    ax.stem(np.array([0.0]), Y, VALUES)
    ax.stem(X, Y, VALUES, linefmt=(1, 0, 0))
    fig.canvas.draw()


def test_fills_bars_and_quivers_fail_where_matplotlib_does():
    fig, ax = _axes3d()
    with pytest.raises(TypeError, match="ufunc 'isnan' not supported"):
        ax.fill_between(None, Y, 0, X, Y, 1)
    with pytest.raises(ValueError, match=r"shapes \(6,1\) \(0,4\) $"):
        ax.fill_between(X, Y, 0, X, Y, 1, facecolors=[])
    ax.fill_between(X, Y, 0, X, Y, 1, facecolors=[], shade=True, mode="polygon")
    with pytest.raises(ValueError, match="could not convert string to float"):
        ax.fill_between(X, "data", 0, X, Y, 1, mode="polygon")
    with pytest.raises(TypeError, match="'<=' not supported"):
        ax.bar3d(None, [0, 1], [0, 0], 0.5, 0.5, [1, 2])
    with pytest.raises(KeyError):
        ax.bar3d([0, 1], [0, 1], [0, 0], 0.5, 0.5, [1, 2], zsort="")
    with pytest.raises(AttributeError, match="no attribute 'direction'$"):
        ax.bar3d([0, 1], [0, 1], [0, 0], 0.5, 0.5, [1, 2], lightsource=3)
    with pytest.raises(TypeError, match="unsupported operand type"):
        ax.quiver(None, [0], [0], [1], [1], [1])
    with pytest.raises(TypeError, match="did not contain a loop"):
        ax.quiver([0], [0], [0], [1], [1], [1], length="4")
    ax.quiver([], [], [], [], [], [], pivot="nope")
    fig.canvas.draw()


def test_plots_check_their_direction_as_juggle_axes_does():
    fig, ax = _axes3d()
    with pytest.raises(TypeError, match="^'NoneType' object is not subscriptable$"):
        ax.plot(X, Y, VALUES, zdir=None)
    with pytest.raises(TypeError, match="^'int' object is not subscriptable$"):
        ax.plot(X, Y, VALUES, zdir=3)
    ax.plot(X, Y, VALUES, zdir="Z")


def test_line_data_set_after_a_draw_reads_back_as_set():
    fig, ax = _axes3d()
    line, = ax.plot(X, Y, VALUES)
    fig.canvas.draw()
    line.set(xdata=[])
    assert list(line.get_xdata()) == []
    line.set(xdata=[3, 2, 1])
    assert list(line.get_xdata()) == [3, 2, 1]
    line.set(ydata="ab")
    fig.canvas.draw()
    assert np.ma.isMaskedArray(line.get_xdata())


def test_sort_positions_of_no_number_fail_the_draws_of_polygons():
    fig, ax = _axes3d()
    X2, Y2 = np.meshgrid(np.linspace(-1, 1, 6), np.linspace(-2, 2, 5))
    surface = ax.plot_surface(X2, Y2, np.cos(X2))
    surface.set_sort_zpos("1")
    with pytest.raises(ValueError, match="^dot not available for this type$"):
        fig.canvas.draw()
    surface.set_sort_zpos(np.float32(1))
    fig.canvas.draw()
    ax.plot_wireframe(X2, Y2, np.cos(X2)).set_sort_zpos("ab")
    fig.canvas.draw()


def test_projections_of_points_check_them_as_numpy_does():
    from pltrs.mpl_toolkits.mplot3d import proj3d
    with pytest.raises(ValueError, match=INHOMOGENEOUS):
        proj3d.proj_transform([0.5], [0.5, 1], [0.5, 1], np.eye(4))
    with pytest.raises(IndexError, match="^index 3 is out of bounds for axis 0 "
                       "with size 0$"):
        proj3d.inv_transform([0.5], [0.5], [0.5], np.empty((0, 4)))
    with pytest.raises(TypeError, match="ufunc 'multiply' did not contain a loop"):
        proj3d.proj_transform([0.5, 1], [0.5, 1], [0.5, 1], "data")
    masked = np.ma.masked_invalid(np.diag([np.nan, 1, 1, 1]))
    xs, ys, zs = proj3d.proj_transform([0.5, 1], [0.5, 1], [0.5, 1], masked)
    assert list(np.ma.getmaskarray(xs)) == [True, True]
    assert list(ys) == [0.5, 1.0]
    with pytest.raises(np.ma.MaskError, match="data size is 6, mask size is 8"):
        proj3d.proj_transform(np.ma.masked_invalid([0.5, np.nan]), [0.5, 1],
                              [0.5, 1], np.eye(4))
    xs, ys, zs = proj3d.inv_transform(0.5, 0.5, 0.5, np.eye(4))
    assert xs.shape == (1,)
    assert isinstance(proj3d.proj_transform(0.5, 0.5, 0.5, np.eye(4))[0], np.float64)


def test_errors_name_what_matplotlib_inherits():
    fig, ax = _axes3d()
    with pytest.raises(TypeError, match=r"^Axis\.get_scale\(\) got an unexpected "
                       "keyword argument 'nonexistent_kw'$"):
        ax.get_zscale(nonexistent_kw=1)
    circle = mpatches.Circle((0, 0), 1)
    art3d.patch_2d_to_3d(circle)
    with pytest.raises(TypeError, match=r"^Patch\.get_patch_transform\(\) got an "
                       "unexpected keyword argument"):
        circle.get_patch_transform(nonexistent_kw=1)
    from pltrs.mpl_toolkits.mplot3d import Axes3D
    for kwargs in ({"sharez": 0}, {"shareview": 0}):
        with pytest.raises(TypeError, match="^cannot create weak reference to "
                           "'int' object$"):
            Axes3D(fig, **kwargs)


def test_flags_read_back_as_given():
    fig, ax = _axes3d()
    for flag in (1, "False", np.True_):
        ax.set_autoscalez_on(flag)
        assert ax.get_autoscalez_on() is flag
    ax.set_autoscalez_on(None)
    assert ax.get_autoscalez_on() is np.True_
    ax.set_zlim(0, 1)
    assert ax.get_autoscalez_on() is False
    collection = art3d.Patch3DCollection([mpatches.Circle((0, 0), 1)])
    ax.add_collection3d(collection)
    for flag in (None, 1, "False", np.True_):
        collection.set_depthshade(flag)
        assert collection.get_depthshade() is flag


def test_z_data_is_formatted_by_the_axis_formatter():
    fig, ax = _axes3d()
    ax.plot([0, 1e-3], [0, 2], [0, 30000])
    fig.canvas.draw()
    assert ax.format_zdata(12345.678) == "12345.6780"
    assert ax.format_zdata(np.float32(2)) == "2.0000"
    ax.fmt_zdata = lambda z: "z!"
    assert ax.format_zdata(1) == "z!"
