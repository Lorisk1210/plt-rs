"""
What the differential tester's hostile and interaction programs (eighth
round) found pltrs doing otherwise than matplotlib 3.11: the trapezoid map
of the triangle finder, the norms of BboxImage, the polar transform's
inverse and its radial frame, host Axes whose agg filter fails, the order
in which a figure draw fails, a zero mutation aspect, the cell block limit
of Agg and the arguments of Agg's ``draw_path_collection``. Each expected
value is recorded from matplotlib.
"""

import math

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import image, patches, transforms
from pltrs import tri as mtri
from pltrs.mpl_toolkits.axes_grid1 import host_subplot


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


X = np.linspace(0, 2 * np.pi, 7)
VALUES = [3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5]

GRID_X = [0.0, 1.0, 2.0, 0.0, 1.0, 2.0, 0.0, 1.0, 2.0]
GRID_Y = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 2.0, 2.0, 2.0]
GRID_TRIANGLES = [[0, 1, 4], [0, 4, 3], [1, 2, 5], [1, 5, 4],
                  [3, 4, 7], [3, 7, 6], [4, 5, 8], [4, 8, 7]]


def test_the_trapezoid_map_finds_matplotlibs_triangles():
    finder = mtri.TrapezoidMapTriFinder(
        mtri.Triangulation(GRID_X, GRID_Y, GRID_TRIANGLES))
    points = [(0.3, 0.1), (0.2, 0.6), (0.5, 0.5), (0.5, 1.0), (0.5, 0.0),
              (1.0, 1.0), (0.0, 0.0), (2.0, 2.0), (1.0, 2.0), (2.5, 1.0)]
    found = finder([x for x, _ in points], [y for _, y in points])
    assert found.tolist() == [0, 1, 1, 4, 0, 0, 0, 6, 4, -1]
    # Points that are not finite end where the search through the map ends.
    found = finder([np.nan, 0.5, np.nan, np.inf, 1.0],
                   [1.0, np.nan, np.nan, 1.0, -np.inf])
    assert found.tolist() == [-1, 1, -1, -1, -1]


def test_masked_triangles_are_not_found():
    finder = mtri.TrapezoidMapTriFinder(mtri.Triangulation(
        GRID_X, GRID_Y, GRID_TRIANGLES,
        mask=[False, False, False, True, False, True, False, False]))
    found = finder([1.6, 1.3, 0.0, 0.2], [0.3, 0.8, 2.0, 1.9])
    assert found.tolist() == [2, -1, -1, -1]


def test_an_invalid_triangulation_fails_when_the_finder_is_made():
    triangulation = mtri.Triangulation([0.0, 1.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0],
                                       [[0, 1, 2], [0, 1, 3]])
    with pytest.raises(RuntimeError, match="Triangulation is invalid"):
        mtri.TrapezoidMapTriFinder(triangulation)


def test_a_bbox_image_norm_scales_to_the_values_it_resamples():
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    fig, ax = plt.subplots()
    box = transforms.TransformedBbox(transforms.Bbox.from_bounds(0.2, 0.2, 0.5, 0.5),
                                     ax.transAxes)
    obj = image.BboxImage(box, data=z, cmap='gray', interpolation='bilinear',
                          origin='lower')
    ax.add_artist(obj)
    assert (obj.norm.vmin, obj.norm.vmax) == (None, None)
    made = obj.make_image(fig.canvas.get_renderer())[0]
    assert made.shape == (185, 248, 4)
    assert (obj.norm.vmin, obj.norm.vmax) == (-2.000244140625, 2.000244140625)


def polar_subplot():
    """A drawn polar subplot of a 4 x 4 inch figure, r from 0 to 9.4."""
    fig = plt.figure(figsize=(4, 4))
    ax = fig.add_subplot(projection='polar')
    ax.plot(X, VALUES)
    fig.canvas.draw()
    return fig, ax


def test_the_polar_inverse_rounds_as_matplotlibs_at_the_centre():
    _, ax = polar_subplot()
    centre = ax.transAxes.transform((0.5, 0.5))
    assert ax.transData.inverted().transform(centre).tolist() == [
        0.7853981633974483, 2.5121479338940403e-15]


def test_zooming_from_the_centre_reads_the_radial_frame_live():
    _, ax = polar_subplot()
    centre = ax.transAxes.transform((0.5, 0.5))
    limits = []
    for target in [(0.7, 0.6), (0.6, 0.55), (0.55, 0.52)]:
        ax.start_pan(*centre, 3)
        point = ax.transAxes.transform(target)
        for _ in range(2):
            ax.drag_pan(3, None, *point)
            limits.append(float(ax.get_ylim()[1]))
        ax.end_pan()
    assert limits[:3] == [5.617333549722717e-15, 5.617333549722717e-15, 0.0]
    assert all(math.isnan(limit) for limit in limits[3:])


def test_a_polar_axes_of_nan_rmax_fails_on_its_empty_gridlines():
    fig = plt.figure()
    ax = fig.add_subplot(projection='polar')
    ax.plot(X, VALUES)
    fig.canvas.draw()
    ax.set_rmax(float('nan'))
    with pytest.raises(ValueError, match=r"'vertices' must be 2D with shape \(N, 2\)"):
        fig.canvas.draw()


def test_a_failing_agg_filter_leaves_the_parasites_listed():
    fig = plt.figure()
    host = host_subplot(111, figure=fig)
    host.twinx().plot(X, np.sin(X))
    fig.canvas.draw()
    host.set(agg_filter=3)
    with pytest.raises(TypeError, match="'int' object is not callable"):
        fig.canvas.draw()
    host.set(agg_filter=None)
    for _ in range(2):
        with pytest.raises(AttributeError, match="'_axis_map'"):
            host.cla()


def test_a_zero_mutation_aspect_divides_by_zero():
    fig, ax = plt.subplots()
    obj = patches.FancyBboxPatch((0.2, 0.2), 0.4, 0.3, boxstyle='round,pad=0.05')
    ax.add_patch(obj)
    obj.set_mutation_aspect(0.0)
    with pytest.raises(ZeroDivisionError):
        fig.canvas.draw()
    obj.set_mutation_aspect(1.5)
    fig.canvas.draw()


def test_an_unsortable_zorder_fails_before_the_children_are_drawn():
    fig, ax = plt.subplots()
    ax.inset_axes([float('nan'), 0.5, 0.4, 0.4])
    ax.inset_axes([0.5, 0.5, 0.4, 0.4], zorder='3')
    with pytest.raises(TypeError, match="'<' not supported"):
        fig.canvas.draw()


@pytest.mark.parametrize("fill, hatch, reasons", [
    (True, None, "- cannot split filled path\n- path.should_simplify is False\n"),
    (False, "//", "- cannot split hatched path\n- path.should_simplify is False\n"),
])
def test_an_infinite_sketch_of_a_fill_exceeds_aggs_cell_limit(fill, hatch, reasons):
    fig, ax = plt.subplots()
    rect = patches.Rectangle((0.2, 0.2), 0.3, 0.3, fill=fill, hatch=hatch)
    ax.add_artist(rect)
    rect.set_sketch_params(float('inf'))
    with pytest.raises(OverflowError) as raised:
        fig.canvas.draw()
    assert str(raised.value) == (
        "Exceeded cell block limit in Agg, however for the following reasons:\n\n"
        f"{reasons}\nwe cannot automatically split up this path to draw.\n\n"
        "Please manually simplify your path.")


def test_agg_lists_the_path_collection_it_cannot_take():
    fig, ax = plt.subplots()
    X2, Y2 = np.meshgrid(np.linspace(-1, 1, 6), np.linspace(-2, 2, 5))
    ax.contour(X2, Y2, np.cos(X2) * np.sin(Y2), linewidths='ab')
    with pytest.raises(TypeError) as raised:
        fig.canvas.draw()
    message = str(raised.value)
    assert message.startswith("draw_path_collection(): incompatible function arguments.")
    assert "linewidths: typing.Annotated[numpy.typing.ArrayLike, numpy.float64]" in message
    assert "array(['a', 'b'" in message
