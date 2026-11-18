"""
2-D state as matplotlib 3.11 keeps it, which the differential tester and
the ecosystem suites found pltrs answering otherwise: the background of
polar Axes after a draw, the transforms of text areas, aux-transform boxes
outside any figure, box-plot parts given the colour None, the legend of a
cleared Axes, the arrays of ``pcolor`` and values read back as given. Every
expected value was recorded from matplotlib 3.11.2.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import patches as mpatches, transforms as mtransforms
from pltrs.offsetbox import (AnchoredOffsetbox, AuxTransformBox, DrawingArea, TextArea,
                             VPacker)
from build_facts import DEFAULT_INT


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_polar_background_fitted_by_draws():
    fig = plt.figure()
    ax = fig.add_subplot(projection="polar")
    patch = ax.patch
    assert (patch.width, len(patch.get_path().vertices)) == (None, 51)
    ax.plot([0, 1], [1, 2])
    fig.canvas.draw()
    assert isinstance(patch.center, np.ndarray) and list(patch.center) == [0.5, 0.5]
    assert (patch.r, patch.theta1, patch.theta2, patch.width) == (0.5, 0.0, 360.0, 0.5)
    assert len(patch.get_path().vertices) == 99
    ax.set_thetamin(10)
    ax.set_thetamax(200)
    ax.set_rorigin(-1)
    assert patch.theta2 == 360.0, "until the next draw"
    fig.canvas.draw()
    assert list(patch.center) == pytest.approx([0.5055530130239534, 0.3324487854121474])
    assert type(patch.r) is np.float64 and patch.r == pytest.approx(0.502074628742202)
    assert (patch.theta1, patch.theta2) == (10.0, 200.0)
    assert patch.width == pytest.approx(0.33745999636770957)
    assert len(patch.get_path().vertices) == 51


def test_text_areas_place_their_text_through_their_transforms():
    fig, ax = plt.subplots()
    area = TextArea("hello")
    area2 = TextArea("a\nb", multilinebaseline=True)
    assert area._text.get_position() == (0, 0)
    assert type(area._text.get_transform()).__name__ == "CompositeAffine2D"
    ax.add_artist(AnchoredOffsetbox(loc="upper left",
                                    child=VPacker(children=[area, area2])))
    fig.canvas.draw()
    assert area._text.get_position() == (0, 0)
    assert area.offset_transform.get_matrix()[:2, 2].tolist() == [92.5, 398.9]
    assert area._text.get_transform().get_matrix()[:2, 2].tolist() == [92.5, 398.9]
    assert area2.offset_transform.get_matrix()[:2, 2].tolist() == [92.5, 373.17018229166666]
    assert area2._baseline_transform.get_matrix()[1, 2] == pytest.approx(-6.666395399305556)
    assert area2._text.get_window_extent().bounds == pytest.approx(
        (92.5, 361.77695312500003, 9.0, 33.786458333333314))


def test_drawing_areas_transforms_follow_the_draw():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    drawing = DrawingArea(20, 10, 2, 3)
    drawing.add_artist(mpatches.Circle((5, 5), 4))
    ax.add_artist(AnchoredOffsetbox(
        loc="upper left", child=VPacker(children=[TextArea("t"), drawing])))
    assert drawing.offset_transform.get_matrix()[:2, 2].tolist() == [0, 0]
    fig.canvas.draw()
    assert drawing.offset_transform.get_matrix()[:2, 2].tolist() == pytest.approx(
        [65.27777777777777, 227.44118923611111])
    assert drawing.dpi_transform.get_matrix()[0, 0] == pytest.approx(100 / 72)


def test_annulus_semi_axes_kept_as_given():
    annulus = mpatches.Annulus((0, 0), (2, 1), 0.5)
    assert (annulus.a, annulus.b) == (2, 1) and type(annulus.a) is int
    annulus.set_semimajor(3)
    assert annulus.get_radii() == (3.0, 1) and type(annulus.a) is float
    annulus.set_radii(4)
    assert (annulus.a, annulus.b) == (4.0, 4.0) and type(annulus.b) is float


def test_aux_transform_boxes_measured_outside_any_figure():
    fig = plt.figure()
    aux = AuxTransformBox(mtransforms.Affine2D().scale(2))
    aux.add_artist(mpatches.Rectangle((0, 0), 10, 5))
    renderer = fig.canvas.get_renderer()
    assert aux.get_bbox(renderer).bounds == (0, 0, 20, 10)
    assert aux.get_transform().get_matrix().tolist() == [[2, 0, 0], [0, 2, 0], [0, 0, 1]]
    assert aux.get_window_extent(renderer).bounds == (0, 0, 20, 10)


def test_boxplot_parts_given_no_colour_take_the_cycle():
    fig, ax = plt.subplots()
    cycle = plt.rcParams["axes.prop_cycle"].by_key()["color"]
    ax.plot([0, 1])
    parts = ax.boxplot([[1, 2, 3, 4, 10], [2, 3, 4]], medianprops={"color": None},
                       whiskerprops={"color": None, "linestyle": "--"},
                       boxprops={"color": None})
    cycle = [pltrs.colors.to_hex(c) for c in cycle]

    def colors(lines):
        return [pltrs.colors.to_hex(line.get_color()) for line in lines]

    assert colors(parts["boxes"]) == [cycle[1], cycle[5]]
    assert colors(parts["whiskers"]) == [cycle[2], cycle[3], cycle[6], cycle[7]]
    assert colors(parts["medians"]) == [cycle[4], cycle[8]]
    assert [c.get_color() for c in parts["caps"]] == ["black"] * 4
    assert colors(ax.plot([0, 1])) == [cycle[9]]
    fig, ax = plt.subplots()
    parts = ax.boxplot([[1, 2, 3]], patch_artist=True, boxprops={"color": None})
    face = parts["boxes"][0].get_facecolor()
    assert face == (*pltrs.colors.to_rgb(cycle[0]), 1) and type(face[3]) is int


def test_cleared_legends_keep_their_axes():
    fig, ax = plt.subplots()
    ax.plot([0, 1], label="a")
    legend = ax.legend()
    ax.clear()
    assert legend.axes is ax and ax.get_legend() is None and legend.figure is fig
    ax.plot([0, 1], label="b")
    ax.legend().remove()
    assert legend.axes is ax
    fig, ax = plt.subplots()
    ax.plot([0, 1], label="a")
    removed = ax.legend()
    removed.remove()
    assert removed.axes is None and removed.figure is None


def test_pcolor_keeps_every_value():
    fig, ax = plt.subplots()
    masked = ax.pcolor(np.ma.masked_less(np.arange(12.).reshape(3, 4), 3))
    values = masked.get_array()
    assert np.ma.isMA(values) and values.shape == (3, 4) and values.mask.sum() == 3
    assert len(masked.get_paths()) == 9
    assert ax.pcolor(np.arange(6).reshape(2, 3)).get_array().dtype == DEFAULT_INT


def test_alphas_and_label_pads_read_back_as_given():
    fig, ax = plt.subplots()
    patch = mpatches.Polygon([[0, 0], [0, 1]], closed=False)
    ax.add_patch(patch)
    patch.set_facecolor((1.0, 0.0, 0.0, 1))
    patch.set_alpha(1)
    assert patch.get_alpha() == 1 and type(patch.get_alpha()) is int
    assert patch.get_facecolor() == (1.0, 0.0, 0.0, 1)
    assert type(patch.get_facecolor()[3]) is int
    ax.set_xlabel("x", labelpad=12)
    assert type(ax.xaxis.labelpad) is int
    ax.xaxis.labelpad = 3.5
    assert ax.xaxis.labelpad == 3.5
