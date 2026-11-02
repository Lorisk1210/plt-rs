"""
Smoke tests of the public names pltrs (Python) adds to the projections
(polar, geo) and the toolkits (axes_grid1's parasite axes, axisartist,
mplot3d). The expected values were recorded from matplotlib 3.11.2
running the same code.
"""

import warnings

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import MatplotlibDeprecationWarning
from pltrs import axis as maxis, collections as mcoll, patches as mpatches
from pltrs.mpl_toolkits.axes_grid1 import host_subplot
from pltrs.mpl_toolkits.axisartist import (
    Axes, axis_artist, axislines, floating_axes, grid_helper_curvelinear as ghc)
from pltrs.mpl_toolkits.mplot3d import art3d, axes3d, axis3d, proj3d
from pltrs.path import Path
from pltrs.projections import PolarAxes, polar

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def _rounded(transform, points):
    return np.round(transform.transform(points), 4).tolist()


def test_polar_axis_text_transforms():
    fig = plt.figure()
    ax = fig.add_subplot(projection="polar")
    ax.set_rlim(0, 2)
    fig.canvas.draw()
    pts = np.array([[0.0, 1.0], [np.pi / 2, 0.5]])
    t, va, ha = ax.get_xaxis_text1_transform(5)
    assert (_rounded(t, pts), va, ha) == (
        [[328.0, 237.6], [328.0, 330.0]], "center", "center")
    t, va, ha = ax.get_xaxis_text2_transform(5)
    assert (_rounded(t, pts), va, ha) == (
        [[328.0, 237.6], [328.0, 330.0]], "center", "center")
    t, va, ha = ax.get_yaxis_text1_transform(5)
    assert (_rounded(t, pts), va, ha) == (
        [[413.3665, 272.9599], [310.32, 280.2832]], "bottom", "left")
    t, va, ha = ax.get_yaxis_text2_transform(5)
    assert (_rounded(t, pts), va, ha) == (
        [[413.3665, 279.9044], [310.32, 287.2277]], "center", "right")
    ax.set_thetalim(0, np.pi / 2)
    ax.set_theta_direction(-1)
    fig.canvas.draw()
    t, va, ha = ax.get_yaxis_text1_transform(5)
    assert (_rounded(t, pts), va, ha) == (
        [[328.0, 429.3444], [71.016, 371.6623]], "center", "right")
    t, va, ha = ax.get_yaxis_text2_transform(5)
    assert (_rounded(t, pts), va, ha) == (
        [[321.0556, 422.4], [64.0716, 364.7178]], "center", "left")


@pytest.mark.skipif(not hasattr(maxis.Axis, "clear"),
                    reason="the polar axes clear through Axis.clear")
def test_polar_axes_clear():
    ax = plt.figure().add_subplot(projection="polar")
    ax.xaxis.clear()
    ax.yaxis.clear()
    assert isinstance(ax.xaxis.get_major_formatter(), polar.ThetaFormatter)
    assert ax.xaxis.get_ticks_position() == "unknown"


def test_geo_axes_grid_and_text_transforms():
    fig = plt.figure()
    fig.add_subplot(projection="polar")
    ax = fig.add_subplot(2, 2, 1, projection="hammer")
    fig.canvas.draw()
    pts = np.array([[0.5, 0.3], [-1.0, -0.2]])
    t, va, ha = ax.get_xaxis_text1_transform(5)
    assert (_rounded(t, pts), va, ha) == (
        [[212.603, 342.4], [153.286, 342.4]], "bottom", "center")
    t, va, ha = ax.get_xaxis_text2_transform(5)
    assert (_rounded(t, pts), va, ha) == (
        [[212.603, 334.4], [153.286, 334.4]], "top", "center")
    t, va, ha = ax.get_yaxis_text1_transform(5)
    assert (_rounded(t, pts), va, ha) == (
        [[184.7273, 351.5029], [295.2075, 326.0825]], "center", "right")
    t, va, ha = ax.get_yaxis_text2_transform(5)
    assert (_rounded(t, pts), va, ha) == (
        [[200.7273, 351.5029], [311.2075, 326.0825]], "center", "left")
    ax.set_longitude_grid(45)
    ax.set_latitude_grid(30)
    ax.set_longitude_grid_ends(60)
    fig.canvas.draw()
    assert np.round(ax.xaxis.get_ticklocs(), 4).tolist() == [
        -2.3562, -1.5708, -0.7854, 0.0, 0.7854, 1.5708, 2.3562]
    assert np.round(ax.yaxis.get_ticklocs(), 4).tolist() == [
        -1.0472, -0.5236, 0.0, 0.5236, 1.0472]
    assert [t.get_text() for t in ax.get_xticklabels()][:4] == [
        "-135°", "-90°", "-45°", "0°"]
    formatter = ax.ThetaFormatter(round_to=15.0)
    assert (formatter(np.pi / 3), formatter(0.1)) == ("60°", "0°")
    assert type(ax.xaxis.get_major_formatter()).__name__ == "ThetaFormatter"


def test_parasite_axes_draw_clear_and_tightbbox():
    fig = plt.figure()
    host = host_subplot(111, figure=fig)
    par = host.twinx()
    host.plot([0, 1], [0, 1])
    par.plot([0, 1], [0, 10])
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    assert np.round(host.get_tightbbox(renderer).bounds, 1).tolist() == [
        48.3, 29.2, 555.4, 393.2]
    host.draw(renderer)
    par.clear()
    assert {c.get_visible() for c in par.get_children()} == {False}
    par.plot([0, 1], [5, 6])
    fig.canvas.draw()
    assert np.round(host.get_tightbbox(renderer).bounds, 1).tolist() == [
        48.3, 29.2, 527.7, 393.2]
    assert par.get_ylim() == pytest.approx((4.95, 6.05))


def test_axisartist_artists():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=Axes)
    ax.set_xlim(0, 10)
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    left = ax.axis["left"]
    ticks = left.major_ticks
    assert (ticks.get_color(), ticks.get_markeredgewidth(),
            ticks.get_markeredgecolor()) == ("black", 0.8, "black")
    assert left.label.get_color() == "black"
    ticks.set_color("red")
    assert ticks.get_color() == "red"
    assert isinstance(ticks, axis_artist.AttributeCopier)
    assert isinstance(left.label.get_ref_artist(), plt.Text)
    assert ticks.get_ref_artist() is not None
    assert left.major_ticklabels.get_ref_artist() is not None
    assert ticks.get_attribute_from_ref_artist("color") == "red"
    extents = left.major_ticklabels.get_window_extents(renderer)
    assert len(extents) == 6
    assert np.round(extents[0].bounds, 1).tolist() == [48.3, 45.9, 22.0, 13.9]
    assert np.round(left.major_ticklabels.get_texts_widths_heights_descents(
        renderer), 2).tolist()[:2] == [[22.0, 10.0, 0.0], [22.0, 10.0, 0.0]]
    left.label.set_default_alignment("bottom")
    left.label.set_default_angle("left")
    assert left.label.get_rotation() == 180.0
    left.set_ticklabel_direction("-")
    left.set_axislabel_direction("-")
    fig.canvas.draw()
    with pytest.warns(MatplotlibDeprecationWarning, match="locs_angles_labels"):
        assert isinstance(ticks.locs_angles_labels, list)
    with pytest.warns(MatplotlibDeprecationWarning, match="locs_angles_labels"):
        assert isinstance(left.major_ticklabels.locs_angles_labels, list)


def test_axisartist_helpers():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=Axes)
    ax.set_xlim(0, 10)
    fig.canvas.draw()
    major, _ = ax.axis["left"].get_helper().get_tick_iterators(ax)
    assert [(np.round(c, 3).tolist(), a, b, label) for c, a, b, label in major][:2] == [
        ([0.0, 0.0], 0, 90, "0.0"), ([0.0, 0.2], 0, 90, "0.2")]
    helper = axislines.FloatingAxisArtistHelperRectilinear(ax, 0, 0.5)
    assert helper.get_line(ax).vertices.tolist() == [[0.0, 0.5], [1.0, 0.5]]
    pos, angle = helper.get_axislabel_pos_angle(ax)
    assert (pos.tolist(), angle) == ([0.5, 0.5], 0)
    major, _ = helper.get_tick_iterators(ax)
    assert [(np.round(c, 3).tolist(), a, b, label) for c, a, b, label in major][:2] == [
        ([0.0, 0.5], 90, 0, "0"), ([2.0, 0.5], 90, 0, "2")]
    assert helper.get_tick_transform(ax) is not None
    assert helper.get_line_transform(ax) is not None
    assert helper.get_axislabel_transform(ax) is not None


def test_axisartist_curvilinear_helpers():
    grid_helper = ghc.GridHelperCurveLinear(PolarAxes.PolarTransform())
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=Axes, grid_helper=grid_helper)
    ax.set_xlim(-5, 5)
    ax.set_ylim(-5, 5)
    ax.axis["lat"] = axis = grid_helper.new_floating_axis(0, 1, axes=ax)
    fig.canvas.draw()
    helper = axis.get_helper()
    major = list(helper.get_tick_iterators(ax)[0])
    assert len(major) == 8
    assert [(np.round(c, 1).tolist(), round(a, 1), round(b, 1), label)
            for c, a, b, label in major][:2] == [
        ([328.0, 237.6], -161.2, 49.2, "$\\mathdefault{0.0}$"),
        ([349.4, 262.5], 154.4, 49.2, "$\\mathdefault{0.8}$")]
    assert np.round(helper.get_line(ax).vertices[:2], 3).tolist() == [
        [0.02, 0.031], [0.06, 0.094]]
    pos, angle = helper.get_axislabel_pos_angle(ax)
    assert (np.round(pos, 1).tolist(), round(float(angle), 2)) == ([427.7, 353.3], 49.25)
    assert helper.get_tick_transform(ax) is not None
    assert helper.get_line_transform(ax) is not None
    assert helper.get_axislabel_transform(ax) is not None
    fixed = ax.axis["bottom"].get_helper()
    assert type(fixed).__name__ == "FixedAxisArtistHelper"
    assert fixed.get_tick_transform(ax) is not None
    assert len(fixed.get_tick_iterators(ax)) == 2
    assert floating_axes.FloatingAxisArtistHelper.__mro__[1] is ghc.FloatingAxisArtistHelper


def test_floating_axes_fixed_helper():
    grid_helper = floating_axes.GridHelperCurveLinear(
        PolarAxes.PolarTransform(), extremes=(0, np.pi / 2, 1, 2))
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=floating_axes.FloatingAxes,
                         grid_helper=grid_helper)
    fig.canvas.draw()
    helper = ax.axis["left"].get_helper()
    assert helper.get_line(ax) is not None
    major, minor = helper.get_tick_iterators(ax)
    assert len(list(major)) > 0


def test_axes3d_z_names():
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        fig = plt.figure()
        ax = fig.add_subplot(projection="3d")
        ax.plot([0, 1], [0, 2], [0, 3])
        fig.canvas.draw()
        assert not ax.get_zinverted()
        ax.set_zinverted(True)
        assert ax.get_zinverted()
        assert ax.get_zlim() == pytest.approx((3.0625, -0.0625))
        ax.set_zinverted(False)
        assert [t.get_text() for t in ax.get_zticklabels()][:3] == ["−0.5", "0.0", "0.5"]
        assert (len(ax.get_zmajorticklabels()), len(ax.get_zminorticklabels())) == (9, 0)
        assert (len(ax.get_zgridlines()), len(ax.get_zticklines())) == (9, 18)
        assert ax.convert_zunits(3.5) == 3.5
        with pytest.raises(Exception, match="does not support 'semilogz'"):
            ax.semilogz()
        assert ax.update_datalim([[0, 0]]) is None
        assert ax.stem3D.__name__ == "stem"
        ax.set_top_view()
        dates = plt.figure().add_subplot(projection="3d")
        dates.zaxis_date()
        assert type(dates.zaxis.get_major_formatter()).__name__ == "AutoDateFormatter"


def test_axes3d_contour_sets_and_axis():
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        fig = plt.figure()
        ax = fig.add_subplot(projection="3d")
        ax.plot([0, 1], [0, 2], [0, 3])
        fig.canvas.draw()
        xys, loc = ax.zaxis.active_pane()
        assert np.round(xys, 4).tolist() == [
            [-0.081, -0.0494, -0.9966], [0.0229, -0.0833, -1.0505],
            [0.0781, -0.0267, -0.9604], [-0.0199, 0.0018, -0.9151]]
        assert round(loc, 4) == -0.0625
        assert (ax.zaxis.get_rotate_label("abc"),
                ax.zaxis.get_rotate_label("abcde")) == (False, True)
        ax.zaxis.set_rotate_label(True)
        assert ax.zaxis.get_rotate_label("a") is True
        assert ax.zaxis.adir == "z"
        assert ax.xaxis.d_interval.tolist() == [0.0, 1.0]
        assert ax.zaxis.v_interval.tolist() == [-0.0625, 3.0625]
        ax.zaxis.v_interval = (0, 5)
        assert ax.get_zlim() == pytest.approx((-0.0625, 5.0))
        x = y = np.linspace(-1, 1, 5)
        X, Y = np.meshgrid(x, y)
        cset = ax.contour(X, Y, X**2 + Y**2)
        assert np.round(cset.levels, 3).tolist() == [
            0.0, 0.25, 0.5, 0.75, 1.0, 1.25, 1.5, 1.75, 2.0]
        renderer = fig.canvas.get_renderer()
        ax.xaxis.draw_pane(renderer)
        ax.xaxis.draw_grid(renderer)
        ax.xaxis.init3d()
        assert isinstance(ax.xaxis, axis3d.Axis)


def test_axes3d_places_contour_sets_of_its_plane():
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        fig = plt.figure()
        ax = fig.add_subplot(projection="3d")
        ax.plot([0, 1], [0, 2], [0, 3])
        fig.canvas.draw()
        x = y = np.linspace(-1, 1, 5)
        X, Y = np.meshgrid(x, y)
        plane = type(plt.figure().add_subplot())
        filled = plane.contourf(ax, X, Y, X**2 + Y**2)
        assert np.round(ax._add_contourf_set(filled, "z", -1), 4).tolist() == [
            0.125, 0.375, 0.625, 0.875, 1.125, 1.375, 1.625, 1.875]
        ax.add_contour_set(plane.contour(ax, X, Y, X + Y), zdir="z", offset=0)
        ax.add_contour_set(plane.contour(ax, X, Y, X + Y), extend3d=True, stride=2)
        fig.canvas.draw()
        assert len(ax.collections) == 3
        assert np.round(ax.get_zlim(), 4).tolist() == [-2.3594, 3.1094]
        filled = plane.contourf(ax, X, Y, X * Y)
        ax.add_contourf_set(filled, zdir="x", offset=2)


def test_art3d_collections_and_patches():
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        fig = plt.figure()
        ax = fig.add_subplot(projection="3d")
        ax.plot([0, 1], [0, 2], [0, 3])
        fig.canvas.draw()
        col = mcoll.PolyCollection([[(0, 0), (1, 0), (1, 1)]])
        art3d.collection_2d_to_3d(col, zs=2, zdir="y")
        assert type(col).__name__ == "PolyCollection3D"
        assert isinstance(col, art3d.Collection3D)
        ax.add_collection3d(col)
        fig.canvas.draw()
        assert round(float(col.do_3d_projection()), 4) == -0.9769
        poly = art3d.Poly3DCollection([[(0, 0, 0), (1, 0, 0), (1, 1, 1)]])
        poly.set_zsort("max")
        poly.set_sort_zpos(1.5)
        ax.add_collection3d(poly)
        fig.canvas.draw()
        assert round(float(poly.do_3d_projection()), 4) == -1.0167
        poly.set_verts_and_codes([[(0, 0, 0), (1, 0, 0), (1, 1, 1)]], [[1, 2, 2]])
        poly.set_3d_properties()
        fig.canvas.draw()
        assert round(float(poly.do_3d_projection()), 4) == -1.0417
        assert poly.get_vector([[(0, 0, 0), (1, 0, 0), (1, 1, 1)]]) is None
        with pytest.raises(KeyError):
            poly.set_zsort("bad")
        lines = art3d.Line3DCollection([[(0, 0, 0), (1, 1, 1)]])
        lines.set_sort_zpos(2)
        ax.add_collection3d(lines)
        points = art3d.Patch3DCollection([plt.Circle((0, 0), 0.1)])
        points.set_sort_zpos(1)
        paths = art3d.Path3DCollection([])
        paths.set_sort_zpos(1)
        circle = plt.Circle((0, 0), 0.5)
        ax.add_patch(circle)
        art3d.patch_2d_to_3d(circle, z=1)
        fig.canvas.draw()
        assert round(float(circle.do_3d_projection()), 4) == -1.0406
        patch = mpatches.PathPatch(Path([[0, 0], [1, 0], [1, 1]]))
        ax.add_patch(patch)
        art3d.pathpatch_2d_to_3d(patch, z=0.5, zdir="x")
        fig.canvas.draw()
        assert type(patch).__name__ == "PathPatch3D"
        assert round(float(patch.do_3d_projection()), 4) == -1.0178
        assert art3d.rcParams["axes3d.depthshade"] is True


def test_axes3d_top_view():
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        fig = plt.figure()
        ax = fig.add_subplot(projection="3d")
        ax.plot([0, 1], [0, 2], [0, 3])
        fig.canvas.draw()
        ax.set_top_view()
        fig.canvas.draw()
        assert np.round(ax.get_xlim(), 4).tolist() == [-0.0729, 1.0729]
        assert np.round(ax.get_ylim(), 4).tolist() == [-0.1458, 2.1458]


def test_proj3d_clip():
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        fig = plt.figure()
        ax = fig.add_subplot(projection="3d")
        ax.plot([0, 1], [0, 2], [0, 3])
        fig.canvas.draw()
        result = proj3d.proj_transform_clip(
            np.array([0., 1.]), np.array([0., 1.]), np.array([0., 1.]), ax.M)
        assert [np.round(np.asarray(a, float), 4).tolist() for a in result] == [
            [-0.0706, 0.0457], [-0.0464, -0.0258], [-0.9951, -1.0156], [1.0, 1.0]]
    assert axes3d.Axes3D.zaxis_date is not None
