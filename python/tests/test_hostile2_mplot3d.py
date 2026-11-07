"""
What the second round of the differential tester's hostile programs
(``spec/diff/generators/hostile.py``) found pltrs'
``mpl_toolkits.mplot3d`` doing otherwise than matplotlib 3.11: 3-D
artists removed and added back, properties no artist takes leaving
nothing in the axes, values of the axes kept unchecked failing the draws
where matplotlib's fail, and the units of the axes converting what is
given. Every expected value was recorded from matplotlib 3.11.2.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.mpl_toolkits.mplot3d import art3d
from pltrs.units import ConversionError

Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
X, Y = np.meshgrid(np.arange(6), np.arange(5))


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def axes3d():
    """A figure with a 3-D axes."""
    fig = plt.figure()
    return fig, fig.add_subplot(projection="3d")


MAKERS = {
    "plot": lambda ax: ax.plot([0, 1], [0, 1], [0, 1])[0],
    "scatter": lambda ax: ax.scatter([0, 1], [0, 1], [0, 1]),
    "surface": lambda ax: ax.plot_surface(X, Y, Z),
    "wireframe": lambda ax: ax.plot_wireframe(X, Y, Z),
    "contour": lambda ax: ax.contour(X, Y, Z, levels=3),
    "contourf": lambda ax: ax.contourf(X, Y, Z, levels=3),
    "text": lambda ax: ax.text(0, 0, 0, "t"),
    "bar": lambda ax: ax.bar([1, 2], [1, 2])[0],
    "lines": lambda ax: ax.add_collection3d(
        art3d.Line3DCollection([[(0, 0, 0), (1, 1, 1)]])),
    "polygons": lambda ax: ax.add_collection3d(
        art3d.Poly3DCollection([[(0, 0, 0), (1, 0, 0), (1, 1, 1)]])),
}


@pytest.mark.parametrize("make", MAKERS.values(), ids=MAKERS)
def test_artists_removed_and_drawn_can_be_added_back(make):
    fig, ax = axes3d()
    artist = make(ax)
    fig.canvas.draw()
    artist.remove()
    fig.canvas.draw()
    assert ax.add_artist(artist) is artist
    assert artist.axes is ax
    fig.canvas.draw()
    artist.remove()
    fig.canvas.draw()


def test_a_removed_patch_measures_its_last_projection():
    fig, ax = axes3d()
    patch = ax.bar([1, 2], [3, 4], zs=1, zdir="y")[0]
    fig.canvas.draw()
    before = patch.get_window_extent().extents
    patch.remove()
    assert patch.get_window_extent().extents == pytest.approx(before)
    assert before == pytest.approx(
        [244.16164269530088, 172.99241352169838, 323.2413697364736, 304.8820515990363])


CALLS = {
    "scatter": (lambda ax: ax.scatter([0, 1], [0, 1], [0, 1], nonexistent_kw=1),
                "PathCollection"),
    "contour": (lambda ax: ax.contour(X, Y, Z, nonexistent_kw=1), "QuadContourSet"),
    "contourf": (lambda ax: ax.contourf(X, Y, Z, nonexistent_kw=1), "QuadContourSet"),
    "tricontour": (lambda ax: ax.tricontour([0, 1, 0, 1, 0.5], [0, 0, 1, 1, 0.5],
                                            [1, 2, 3, 4, 0], nonexistent_kw=1),
                   "TriContourSet"),
    "surface": (lambda ax: ax.plot_surface(X, Y, Z, nonexistent_kw=1), "Poly3DCollection"),
    "wireframe": (lambda ax: ax.plot_wireframe(X, Y, Z, nonexistent_kw=1),
                  "Line3DCollection"),
    "quiver": (lambda ax: ax.quiver([0], [0], [0], [1], [1], [1], nonexistent_kw=1),
               "Line3DCollection"),
    "bar3d": (lambda ax: ax.bar3d([0], [0], [0], 1, 1, 1, nonexistent_kw=1),
              "Poly3DCollection"),
    "voxels": (lambda ax: ax.voxels(np.ones((2, 2, 2), bool), nonexistent_kw=1),
               "Poly3DCollection"),
}


@pytest.mark.parametrize("call, cls", CALLS.values(), ids=CALLS)
def test_properties_no_artist_takes_add_nothing(call, cls):
    fig, ax = axes3d()
    with pytest.raises(AttributeError) as error:
        call(ax)
    assert str(error.value) == (
        f"{cls}.set() got an unexpected keyword argument 'nonexistent_kw'")
    assert error.value.name == "nonexistent_kw"
    assert len(ax.collections) == 0
    fig.canvas.draw()


def test_contours_convert_their_grids_with_the_units_of_the_axes():
    fig, ax = axes3d()
    with pytest.raises(TypeError, match=r"Number of dimensions of x \(0\) and y \(1\)"):
        ax.contour("data", np.arange(5), Z)
    fig.canvas.draw()


def test_contours_of_invalid_points_fail_to_draw():
    fig, ax = axes3d()
    ax.contour(np.array([np.nan, 1.0, 2.0, 3.0, 4.0, 5.0]), np.arange(5), Z)
    with pytest.raises(ValueError, match="operands could not be broadcast together"):
        fig.canvas.draw()


def test_tick_locations_convert_with_the_units_of_the_axis():
    fig, ax = axes3d()
    ax.plot([0, 1], [0, 1], [0, 1])
    with pytest.raises(ValueError, match=r"'locs' must be 1D with shape \(N,\)"):
        ax.set_xticks(None)
    with pytest.raises(ConversionError, match="Failed to convert value"):
        ax.set_xticks("ab")
    # The axis took the converter of the strings, which fails on None.
    with pytest.raises(ConversionError, match="Failed to convert value"):
        ax.set_xticks(None)


@pytest.mark.parametrize("name, value, error, message", [
    ("zorder", "3", TypeError, "'>' not supported between instances of 'float' and 'str'"),
    ("agg_filter", 3, TypeError, "'int' object is not callable"),
    ("agg_filter", "x", TypeError, "'str' object is not callable"),
])
def test_values_of_the_axes_fail_the_draws_of_the_axes(name, value, error, message):
    fig, ax = axes3d()
    ax.plot([0, 1], [0, 1], [0, 1])
    ax.xaxis.set(**{name: value})
    with pytest.raises(error) as raised:
        fig.canvas.draw()
    assert str(raised.value) == message
    ax.set_axis_off()
    if name != "zorder":
        fig.canvas.draw()
    ax.set_axis_on()
    ax.xaxis.set(**{name: {"zorder": 3, "agg_filter": None}[name]})
    fig.canvas.draw()


def test_label_pads_of_no_number_fail_the_draws_of_the_axes():
    fig, ax = axes3d()
    ax.set_zlabel("z", labelpad="5")
    assert ax.zaxis.labelpad == "5"
    with pytest.raises(TypeError, match=r'can only concatenate str \(not "float"\) to str'):
        fig.canvas.draw()
    ax.set_zlabel("z", labelpad=5)
    fig.canvas.draw()


def test_an_axis_without_a_zorder_takes_its_class_default():
    fig, ax = axes3d()
    ax.xaxis.set_zorder(None)
    assert ax.xaxis.get_zorder() == 0
    fig.canvas.draw()


@pytest.mark.parametrize("norm", [3, "x"])
def test_surfaces_without_a_colormap_ignore_their_norm(norm):
    fig, ax = axes3d()
    ax.plot_surface(X, Y, Z, norm=norm)
    ax.plot_trisurf([0, 1, 0, 1, 0.5], [0, 0, 1, 1, 0.5], [1, 2, 3, 4, 0], norm=norm)
    fig.canvas.draw()
