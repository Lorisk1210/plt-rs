"""
Smoke tests of what pltrs (Python) does as matplotlib does in the
toolkits, 3-D and images: geographic limits the axis changes, the 3-D
artists matplotlib's draw fails on, 2-D artists in a 3-D axes, the colours
of 3-D collections not yet sorted, NaN view intervals, the children an
aux-transform box takes after joining its figure, the transform
`make_image` gives, the axisartist gridlines and grid finders, floating
curvilinear helpers before a draw, parasites keeping their own limits,
the warnings of 3-D tick updates and the data ratio of a program's
projection. The expected values were recorded from matplotlib 3.11.2
running the same code.
"""

import logging
import warnings

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import backend_bases, projections
from pltrs import collections as mcoll, image as mimage, offsetbox, patches as mpatches
from pltrs import text as mtext, transforms as mtransforms
from pltrs.axes import Axes
from pltrs.mpl_toolkits import axisartist
from pltrs.mpl_toolkits.axes_grid1 import host_subplot, mpl_axes
from pltrs.mpl_toolkits.axisartist import angle_helper, grid_finder
from pltrs.mpl_toolkits.axisartist import grid_helper_curvelinear
from pltrs.mpl_toolkits.mplot3d import art3d, proj3d

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)

Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
POLAR = mtransforms.Affine2D().scale(np.pi / 180, 1) + projections.polar.PolarTransform()


def axes3d():
    fig = plt.figure()
    return fig, fig.add_subplot(projection='3d')


def test_geographic_axes_let_their_axis_change_the_limits():
    fig = plt.figure()
    ax = fig.add_subplot(projection='hammer')
    ax.set(xinverted=True)
    assert ax.get_xlim() == (np.pi, -np.pi)
    fig.canvas.draw()
    with pytest.raises(TypeError, match="Please consider using Cartopy"):
        ax.set_xlim(0, 1)


def test_3d_artists_matplotlib_fails_to_draw():
    fig, ax = axes3d()
    line, = ax.plot([0, 1], [0, 1], [0, 1])
    line.set_data_3d([0], [1])
    assert line.get_data_3d() == ([0], [1])
    with pytest.raises(TypeError, match=r"missing 1 required positional argument: 'axes'"):
        fig.canvas.draw()
    line.set_data_3d([0, 1], [1, 0], [0, 1])
    fig.canvas.draw()

    fig, ax = axes3d()
    segments = art3d.Line3DCollection([[(0, 0, 0), (1, 1, 1)]])
    ax.add_collection3d(segments)
    segments.set_segments([[(0, 0), (1, 1)]])
    for fails in (fig.canvas.draw, segments.do_3d_projection):
        with pytest.raises(IndexError, match="index 2 is out of bounds for axis 2"):
            fails()


def test_2d_collections_in_3d_axes_fail_to_draw():
    fig, ax = axes3d()
    ax.add_collection(mcoll.LineCollection([[(0, 0), (1, 1)]]))
    with pytest.raises(AttributeError, match="'LineCollection' object has no attribute"):
        fig.canvas.draw()

    fig, ax = axes3d()
    with pytest.raises(ValueError, match=r"requested shape \(9,\)"):
        ax.contour(np.arange(6), np.arange(5), Z, offset=(1.0, -1.0))
    with pytest.raises(AttributeError, match="'QuadContourSet' object has no attribute"):
        fig.canvas.draw()
    assert ax.clabel() is None


def test_3d_view_intervals_and_inverse_projections_as_matplotlib():
    fig, ax = axes3d()
    ax.xaxis.set_view_interval(float('nan'), 4.0)
    assert np.isnan(ax.xaxis.get_view_interval()).all()
    fig.canvas.draw()
    assert proj3d.inv_transform([0.5], [0.5], [0.5], np.zeros((4, 4)))[0].tolist() == [0.0]


def test_unsorted_3d_colours_lose_their_last():
    fig, ax = axes3d()
    patches = [mpatches.Circle((x, x), 0.1) for x in range(3)]
    collection = art3d.Patch3DCollection(patches, facecolors=['r', 'g', 'b'],
                                         depthshade=False)
    ax.add_collection3d(collection, zs=0)
    scatter = ax.scatter([0, 1, 2], [2, 1, 0], [1, 1, 1], c=['r', 'g', 'b'])
    assert len(scatter.get_facecolor()) == 2
    fig.canvas.draw()
    assert (len(collection.get_facecolor()), len(scatter.get_facecolor())) == (2, 3)


def test_children_an_added_aux_box_takes_have_no_figure():
    fig, ax = plt.subplots()
    box = offsetbox.AuxTransformBox(mtransforms.Affine2D().scale(10))
    box.add_artist(mpatches.Rectangle((0, 0), 1, 1))
    ax.add_artist(offsetbox.AnchoredOffsetbox('center', child=box))
    label = mtext.Text(0, 0, 't')
    box.add_artist(label)
    assert label.get_figure() is None
    for fails in (fig.canvas.draw, lambda: box.get_bbox(fig.canvas.get_renderer())):
        with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'dpi'"):
            fails()


def test_make_image_gives_matplotlibs_composite():
    fig, ax = plt.subplots()
    image = mimage.BboxImage(ax.bbox, data=Z)
    made = image.make_image(fig.canvas.get_renderer())
    assert type(made[3]).__name__ == 'CompositeGenericTransform'
    assert np.round(made[3].get_matrix(), 4).tolist() == [
        [82.6667, 0.0, 0.0], [0.0, -73.92, 369.4], [0.0, 0.0, 1.0]]
    ax.add_artist(image)
    fig.canvas.draw()
    assert type(image.get_transform()).__name__ == 'BboxTransformTo'
    assert type(image.can_composite()) is bool


def test_axisartist_gridlines_and_grid_finders_fail_as_matplotlib():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.grid(True)
    ax.gridlines.set_axis('z')
    with pytest.raises(ValueError, match="'z' is not a valid value for axis"):
        fig.canvas.draw()

    fig = plt.figure()
    helper = grid_helper_curvelinear.GridHelperCurveLinear(POLAR)
    fig.add_subplot(axes_class=axisartist.Axes, grid_helper=helper)
    helper.grid_finder.update_transform((np.add, np.subtract))
    helper.grid_finder.update(extreme_finder=grid_finder.ExtremeFinderSimple(5, 5))
    for _ in range(2):
        with pytest.raises(ValueError, match=r"your input has shape \(25,\)"):
            fig.canvas.draw()


def test_floating_curvilinear_helpers_before_a_draw():
    fig = plt.figure()
    helper = grid_helper_curvelinear.GridHelperCurveLinear(
        POLAR, extreme_finder=angle_helper.ExtremeFinderCycle(
            20, 20, lon_cycle=360, lat_cycle=None, lon_minmax=None,
            lat_minmax=(0, np.inf)),
        grid_locator1=angle_helper.LocatorDMS(12),
        tick_formatter1=angle_helper.FormatterDMS())
    ax = fig.add_subplot(axes_class=axisartist.Axes, grid_helper=helper)
    ax.set_xlim(-5, 12)
    ax.set_ylim(-5, 10)
    floating = ax.new_floating_axis(0, 30)._axis_artist_helper
    floating.update_lim(ax)
    xy, angle = floating.get_axislabel_pos_angle(ax)
    assert (np.round(xy, 3).tolist(), round(float(angle), 3)) == ([432.81, 276.894], 25.993)
    assert len(floating.get_line(ax).vertices) == 100


def test_axis_artists_have_no_hit_test(caplog):
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=mpl_axes.Axes)
    mouse = backend_bases.MouseEvent('button_press_event', fig.canvas, 60, 40, 1)
    with caplog.at_level(logging.WARNING):
        assert ax.axis['left'].contains(mouse) == (False, {})
    assert "'SimpleAxisArtist' needs 'contains' method" in caplog.text
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    assert repr(ax.axis['bottom'].label) == "Text(0, 0, '__from_axes__')"


def test_parasites_keep_their_limits_until_the_host_changes():
    fig = plt.figure()
    host = host_subplot(111, figure=fig)
    parasite = host.twin()
    parasite.set_xlim(0, 10)
    assert parasite.get_xlim() == (0, 10)
    host.set_xlim(2, 3)
    assert parasite.get_xlim() == (2, 3)


def test_3d_ticks_warn_where_matplotlib_updates_them():
    fig, ax = axes3d()
    x = np.linspace(0, 2 * np.pi, 7)
    ax.scatter(x, np.sin(x), x)
    ax.set_ylim(-1, 1)
    ax.set_ymargin(0)
    ax.set_yscale('logit')
    ax.set_title('t')

    def warned(call):
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            call()
        return sum('rtol' in str(warning.message) for warning in caught)

    assert [warned(call) for call in (
        ax.yaxis.get_minorticklabels, ax.yaxis.get_majorticklabels,
        ax.get_tightbbox, fig.canvas.draw)] == [2, 1, 2, 3]


class _RatioAxes(Axes):
    """A projection of the program's giving its data ratio."""

    name = 'apid_ratio'
    asked = 0

    def get_data_ratio(self):
        type(self).asked += 1
        return 1.0


def test_a_programs_data_ratio_is_asked_where_matplotlib_asks_it():
    projections.register_projection(_RatioAxes)
    fig = plt.figure()
    ax = fig.add_subplot(projection='apid_ratio')
    ax.set_aspect(0.5)
    _RatioAxes.asked = 0
    fig.canvas.draw()
    # `Figure._get_draw_artists` and `Axes.draw` apply the aspect.
    assert _RatioAxes.asked == 2
