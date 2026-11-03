"""
matplotlib's transform and bbox trees over pltrs (Rust)'s values: the
classes, the structure and the identity of ``ax.transData`` and its
friends, their leaves following the figure (and the figure following
leaves the program sets), and the window extents computed from them as
they are. The expected values were recorded from matplotlib 3.11.2
running the same code.
"""

import pickle

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import patches, transforms
from pltrs.offsetbox import AnchoredText

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def test_the_trees_are_matplotlibs_and_kept():
    fig, ax = plt.subplots()
    assert type(ax.transData) is transforms.CompositeGenericTransform
    assert type(ax.transAxes) is transforms.BboxTransformTo
    assert type(ax.transLimits) is transforms.BboxTransformFrom
    assert type(ax.transScale) is transforms.TransformWrapper
    assert type(ax.bbox) is transforms.TransformedBbox
    assert type(ax.viewLim) is transforms.Bbox
    assert type(ax.dataLim) is transforms.Bbox
    assert type(fig.transFigure) is transforms.BboxTransformTo
    assert type(fig.dpi_scale_trans) is transforms.Affine2D
    assert ax.transData is ax.transData and ax.bbox is ax.bbox
    assert fig.transSubfigure is fig.transFigure
    assert ax.get_window_extent() is ax.bbox
    assert ax.transData.contains_branch(ax.transAxes)
    assert ax.transData.depth == 3
    blended = ax.get_xaxis_transform()
    assert type(blended) is transforms.BlendedGenericTransform
    assert blended is ax.get_xaxis_transform()
    line, = ax.plot([1, 2], [3, 5])
    assert line.get_transform() is ax.transData
    assert type(line.get_clip_box()) is transforms.TransformedBbox
    assert line.get_clip_box() is line.get_clip_box()
    assert str(ax.transData).startswith("CompositeGenericTransform(\n    TransformWrapper(")


def test_leaves_follow_the_figure():
    fig, ax = plt.subplots()
    ax.plot([1, 2], [3, 5])
    np.testing.assert_allclose(ax.viewLim.get_points(), [[0.95, 2.9], [2.05, 5.1]])
    np.testing.assert_allclose(ax.dataLim.get_points(), [[1, 3], [2, 5]])
    np.testing.assert_allclose(ax.transData.transform([1.5, 4]), [328, 237.6])
    data_box = transforms.TransformedBbox(transforms.Bbox.unit(), ax.transAxes)
    data_box.get_points()
    fig.set_size_inches(4, 3)
    np.testing.assert_allclose(fig.bbox.get_points(), [[0, 0], [400, 300]])
    np.testing.assert_allclose(data_box.get_points(), [[50, 33], [360, 264]])
    ax.set_position([0.2, 0.2, 0.5, 0.5])
    np.testing.assert_allclose(ax.transAxes.transform([1, 1]), [280, 210])
    ax.set_xscale("log")
    assert type(ax.transScale._child) is transforms.BlendedGenericTransform
    ax.viewLim.set(transforms.Bbox([[1, 0], [100, 10]]))
    assert ax.get_xlim() == (1, 100) and ax.get_ylim() == (0, 10)
    fig.dpi = 50
    np.testing.assert_allclose(fig.dpi_scale_trans.get_matrix()[0, 0], 50)


def test_subfigures_polar_and_geographic_trees():
    fig = plt.figure()
    right = fig.subfigures(1, 2)[1]
    ax = right.add_subplot()
    assert type(right.transSubfigure) is transforms.BboxTransformTo
    np.testing.assert_allclose(ax.bbox.get_points(),
                               [[360, 52.8], [608, 422.4]])
    polar = plt.figure().add_subplot(projection="polar")
    assert type(polar.transProjectionAffine).__name__ == "PolarAffine"
    np.testing.assert_allclose(polar.transData.transform([0, 1]), [576, 237.6])
    polar.set_theta_offset(np.pi / 2)
    np.testing.assert_allclose(polar.transData.transform([0, 1]), [328, 422.4])
    hammer = plt.figure().add_subplot(projection="hammer")
    np.testing.assert_allclose(hammer.transData.transform([0.5, 0.5]),
                               [367.58449977, 302.7331349])


def test_trees_pickle_and_build_again():
    fig, ax = plt.subplots()
    ax.set_xlim(0, 10)
    copy = pickle.loads(pickle.dumps(fig))
    copied = copy.axes[0]
    assert copied.transData is copied.transData
    np.testing.assert_allclose(copied.transData.transform([5, 0.5]),
                               ax.transData.transform([5, 0.5]))


def test_window_extents_read_the_trees_as_they_are():
    fig, ax = plt.subplots()
    bar = ax.bar([1, 2, 3], [3, 1, 2])[1]
    # The view limits are not autoscaled yet.
    np.testing.assert_allclose(bar.get_window_extent().get_points(),
                               [[873.6, 52.8], [1270.4, 422.4]])
    assert bar.get_tightbbox() is None
    ax.get_xlim()
    line, = ax.plot([0, 6], [0, 1])
    np.testing.assert_allclose(line.get_window_extent().get_points(),
                               [[5.922077922077932, 52.8], [972.1558441558441, 170.13333333]])
    scatter = ax.scatter([1, 2], [3, 4])
    assert not np.isfinite(scatter.get_window_extent().get_points()).any()
    fig.canvas.draw()
    np.testing.assert_allclose(bar.get_window_extent().get_points(),
                               [[222.7878787878788, 52.8], [282.90909090909093, 140.8]])
    fill = ax.fill_between([0, 1, 2], [0, 1, 0])
    np.testing.assert_allclose(fill.get_window_extent().get_points(),
                               [[102.54545454545455, 52.8], [252.84848484848487, 140.8]])


def test_patch_transforms_are_matplotlibs():
    fig, ax = plt.subplots()
    rectangle = ax.add_patch(patches.Rectangle((0.1, 0.2), 0.3, 0.4, angle=30))
    assert type(rectangle.get_patch_transform()) is transforms.CompositeGenericTransform
    np.testing.assert_allclose(
        rectangle.get_transform().get_matrix(),
        [[128.86458, -99.2, 129.6], [55.44, 128.033196, 126.72], [0, 0, 1]], atol=1e-6)
    polygon = ax.add_patch(patches.RegularPolygon((0.5, 0.5), 5, radius=0.1))
    assert polygon.get_patch_transform() is polygon.get_patch_transform()
    wedge = ax.add_patch(patches.Wedge((0.5, 0.5), 0.2, 10, 50))
    assert type(wedge.get_patch_transform()) is transforms.IdentityTransform
    assert wedge.get_transform() is ax.transData
    shadow = patches.Shadow(rectangle, 0.02, -0.02)
    assert shadow.get_transform().get_affine() is not None
    assert type(shadow.get_patch_transform()) is transforms.CompositeGenericTransform


def test_axis_parts_and_added_artists_have_matplotlibs_transforms():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    label = ax.get_xticklabels()[0]
    assert type(label.get_transform()) is transforms.CompositeGenericTransform
    assert label.get_transform() is label.get_transform()
    assert label.is_transform_set()
    assert type(ax.xaxis.get_clip_box()) is transforms.TransformedBbox
    np.testing.assert_allclose(ax.xaxis.get_clip_box().get_points(),
                               [[80, 52.8], [576, 422.4]])
    box = ax.add_artist(AnchoredText("anchored", loc="upper left"))
    assert box.get_transform() is ax.transData and box.is_transform_set()
    assert type(box.get_clip_box()) is transforms.TransformedBbox
    assert ax.titleOffsetTrans is ax.titleOffsetTrans
    x1, y1 = ax.get_xaxis_text1_transform(3.5)[0].transform([1, 0])
    assert (x1, y1) == pytest.approx((328.0, 47.93888888888888))
