"""
Smoke tests of what pltrs (Python) does as matplotlib does in the
toolkits, found by the variant programs of the toolkit subjects: Lambert
axes centred elsewhere, 3-D axes moved to a grid block, labelled ticks of
log-scaled 3-D axes, the label angles and replaced aux transforms of
curvilinear helpers, the axes a draw leaves unticked, axis-artist parts
placed by transforms of their own, the own positions of parasites, the
PostScript backend's transparency log on draws that fail, and images of a
program's class computing their pixels in ``make_image``. The expected
values were recorded from matplotlib 3.11.2 running the same code.
"""

import io
import logging
import warnings

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import image as mimage, projections, transforms as mtransforms
from pltrs.mpl_toolkits import axisartist
from pltrs.mpl_toolkits.axes_grid1 import host_subplot
from pltrs.mpl_toolkits.axisartist import SubplotHost, angle_helper
from pltrs.mpl_toolkits.axisartist import grid_helper_curvelinear
from pltrs.mpl_toolkits.mplot3d import art3d

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)

POLAR = mtransforms.Affine2D().scale(np.pi / 180, 1) + projections.polar.PolarTransform()


def test_lambert_axes_take_their_centre():
    fig = plt.figure()
    ax = fig.add_subplot(projection='lambert', center_longitude=0.5, center_latitude=0.3)
    np.testing.assert_allclose(
        ax.transProjection.transform([(0.5, 0.3), (0, 0)]),
        [[0.0, 0.0], [-0.5000549545340379, -0.2705027823471586]], rtol=1e-12)
    ax = fig.add_axes([0.1, 0.1, 0.3, 0.3], projection='lambert', center_latitude=-0.2)
    np.testing.assert_allclose(
        ax.transProjection.transform([(0.0, -0.2), (0.3, 0.1)]),
        [[0.0, 0.0], [0.3007515671206188, 0.2932312699048469]], rtol=1e-12)


def test_3d_axes_move_to_a_grid_block():
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    ax.plot([0, 1], [0, 1], [0, 2])
    fig.canvas.draw()
    spec = fig.add_gridspec(2, 2)[0, 0]
    ax.set_subplotspec(spec)
    np.testing.assert_allclose(ax.get_position().bounds,
                               (0.16988636363636364, 0.53, 0.26249999999999996, 0.35))
    np.testing.assert_allclose(ax.get_position(original=True).bounds,
                               (0.125, 0.53, 0.35227272727272724, 0.35))
    assert len(ax.yaxis.get_majorticklocs()) == 5
    assert ax.get_subplotspec() is spec


def test_log_scaled_3d_ticks_take_labels():
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    ax.set_zlim3d(0, 3)
    ax.set_zscale('log')
    ax.set_zticks([0, 1, 2])
    assert len(ax.set_zticks([0, 1], labels=['a', 'b'])) == 2
    assert list(ax.get_zticks()) == [0, 1]


def curvilinear_axes():
    fig = plt.figure()
    helper = grid_helper_curvelinear.GridHelperCurveLinear(
        POLAR, extreme_finder=angle_helper.ExtremeFinderCycle(
            20, 20, lon_cycle=360, lat_cycle=None, lon_minmax=None, lat_minmax=(0, np.inf)),
        grid_locator1=angle_helper.LocatorDMS(12), tick_formatter1=angle_helper.FormatterDMS())
    ax = fig.add_subplot(axes_class=SubplotHost, grid_helper=helper)
    ax.set_aspect(1)
    ax.set_xlim(-5, 12)
    ax.set_ylim(-5, 10)
    return fig, ax, helper


def test_curvilinear_helpers_follow_their_grid_finder():
    fig, ax, helper = curvilinear_axes()
    floating = ax.new_floating_axis(0, 30)
    _, angle = floating._axis_artist_helper.get_axislabel_pos_angle(ax)
    assert type(angle) is np.float64
    assert angle == pytest.approx(25.993012801260022)
    helper.update_grid_finder(aux_trans=mtransforms.Affine2D().scale(2) + POLAR)
    fig.canvas.draw()
    np.testing.assert_allclose(
        floating._axis_artist_helper.get_line(ax).vertices[[0, -1]],
        [[0.0, 0.0], [8.189443, 14.184532]], atol=1e-6)


def test_axes_a_draw_leaves_unticked():
    # The hidden axes of axes on a curvilinear grid, and those of axes the
    # figure does not draw, are not ticked: a logit view outside (0, 1)
    # does not fail there.
    fig, ax, _ = curvilinear_axes()
    ax.set(ylim=(-1, 1))
    ax.set(yscale='logit')
    ax.set(yticks=[0.5])
    fig.canvas.draw()

    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.toggle_axisline(False)
    ax.set(animated=True, ylim=(-1, 1), yscale='logit', yticks=[0.5])
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        fig.canvas.draw()
    assert [str(w.message) for w in caught] == []


def test_axis_artist_parts_take_transforms_of_their_own():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    fig.canvas.draw()
    labels = ax.axis['bottom'].major_ticklabels
    labels.set_transform(mtransforms.IdentityTransform())
    fig.canvas.draw()
    np.testing.assert_allclose(
        [b.extents for b in labels.get_window_extents()][:2],
        [[-11.0, -23.0588, 11.0, -9.1699], [-10.8, -23.0588, 11.2, -9.1699]], atol=1e-4)


def test_parasites_keep_positions_of_their_own():
    fig = plt.figure()
    host = host_subplot(111, figure=fig)
    twin = host.twin()
    spec = fig.add_gridspec(2, 2)[0, 0]
    twin.set_subplotspec(spec)
    for original in (False, True):
        np.testing.assert_allclose(twin.get_position(original=original).bounds,
                                   (0.125, 0.53, 0.35227272727272724, 0.35))
    np.testing.assert_allclose(host.get_position().bounds, (0.125, 0.11, 0.775, 0.77))
    assert twin.get_subplotspec() is spec
    twin.set_position([0.2, 0.3, 0.4, 0.5])
    np.testing.assert_allclose(twin.get_position().bounds, (0.2, 0.3, 0.4, 0.5))
    np.testing.assert_allclose(host.get_position().bounds, (0.125, 0.11, 0.775, 0.77))


@pytest.fixture
def ps_log():
    """The messages the PostScript backend logs."""
    messages = []

    class Collect(logging.Handler):
        def emit(self, record):
            messages.append(record.getMessage())

    handler = Collect()
    logger = logging.getLogger("pltrs.backends.backend_ps")
    logger.addHandler(handler)
    yield messages
    logger.removeHandler(handler)


def test_ps_logs_transparency_drawn_before_a_failure(ps_log):
    # The panes of a 3-D axes are drawn before a line fails...
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    line, = ax.plot([0, 1], [0, 1], [0, 1])
    line.set_data_3d([0], [1])
    with pytest.raises(TypeError, match="missing 1 required positional argument: 'axes'"):
        fig.savefig(io.BytesIO(), format='ps')
    assert len(ps_log) == 1
    ps_log.clear()
    # ... but not before a collection fails its projection.
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    segments = art3d.Line3DCollection([[(0, 0, 0), (1, 1, 1)]])
    ax.add_collection3d(segments)
    segments.set_segments([[(0, 0), (1, 1)]])
    with pytest.raises(IndexError, match="index 2 is out of bounds"):
        fig.savefig(io.BytesIO(), format='ps')
    assert ps_log == []
    # A figure with a layout engine fails in the draw before the print.
    fig, ax = plt.subplots(layout='constrained')
    ax.tripcolor([0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4], alpha=0.5)
    ax.tripcolor([0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4], vmin=5)
    with pytest.raises(ValueError, match="minvalue must be less than or equal to maxvalue"):
        fig.savefig(io.BytesIO(), format='ps')
    assert ps_log == []


class ComputedImage(mimage.AxesImage):
    """An image computing its array when it is made, over the view (as
    mpl-scatter-density's images do)."""

    calls = 0

    def get_extent(self):
        return (*self.axes.get_xlim(), *self.axes.get_ylim())

    def make_image(self, renderer, magnification=1.0, unsampled=False):
        type(self).calls += 1
        self.set_data(np.outer(np.arange(4.0), np.arange(3.0)))
        return super().make_image(renderer, magnification, unsampled)


def test_program_images_draw_the_pixels_they_make():
    fig = plt.figure(figsize=(2, 2), dpi=50)
    ax = fig.add_axes([0, 0, 1, 1])
    image = ComputedImage(ax, origin='lower')
    image.set_data(np.array([[np.nan]]))
    ax.add_artist(image)
    fig.canvas.draw()
    pixels = np.asarray(fig.canvas.buffer_rgba())
    assert ComputedImage.calls == 1
    assert pixels[::25, ::25, 0].tolist() == [
        [15, 30, 14, 112], [30, 68, 48, 53], [30, 68, 68, 48], [30, 68, 68, 68]]
    assert int(pixels[..., :3].astype(int).sum()) == 2431238
