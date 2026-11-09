"""
What the differential tester's programs using the API as documented
(``generated/api``, ``orders``, ``artists``, ``modules``, ``rest``) found
pltrs doing otherwise than matplotlib 3.11: legends, artists added in an
unusual order, offset boxes, axis artists, PDF documents written by hand
and 3-D axes. Each expected value is recorded from matplotlib.
"""

import io
import shutil

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, lines, patches
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.figure import Figure
from pltrs.mpl_toolkits import axisartist
from pltrs.offsetbox import AnnotationBbox, OffsetImage, TextArea, VPacker


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))


def small_renderer():
    """The renderer of a 3 x 2 in figure at 50 dpi, as the generated
    programs measure with."""
    fig = Figure(figsize=(3, 2), dpi=50)
    canvas = FigureCanvasAgg(fig)
    fig.add_subplot()
    canvas.draw()
    return canvas.get_renderer()


def pixels(fig):
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba())


# -- legends ---------------------------------------------------------------------

@pytest.mark.parametrize("handle", ["errorbar", "line collection"])
def test_marker_scale_with_handles_drawing_no_line_of_their_own(handle):
    fig, ax = plt.subplots()
    if handle == "errorbar":
        ax.errorbar([1], [1], yerr=0.1, label='err')
    else:
        ax.add_collection(collections.LineCollection([[(0, 0), (1, 1)]], label='lc'))
    leg = ax.legend(fontsize='small', markerscale=2)
    assert leg.markerscale == 2 and len(leg.legend_handles) == 1


def test_a_legend_shows_the_artists_of_another_figure():
    fig1, ax1 = plt.subplots()
    fig2, ax2 = plt.subplots()
    line, = ax1.plot([1, 2], label='one')
    leg = ax2.legend(handles=[line])
    assert [t.get_text() for t in leg.get_texts()] == ['one']
    assert isinstance(leg.legend_handles[0], lines.Line2D)


# -- artists added after being given a figure --------------------------------------

def test_an_artist_only_given_its_figure_takes_the_data_coordinates_of_an_axes():
    fig, ax = plt.subplots()
    rect = patches.Rectangle((0.1, 0.2), 0.5, 0.3, angle=20)
    rect.set_figure(fig)
    ax.add_artist(rect)
    assert rect.is_transform_set()
    image = pixels(fig)
    # matplotlib draws the rectangle in the Axes' data coordinates.
    assert image[300, 150].tolist() == [31, 119, 180, 255]


def test_an_artist_only_given_its_figure_takes_the_figure_fractions():
    fig, ax = plt.subplots()
    rect = patches.Rectangle((0.1, 0.2), 0.5, 0.3, angle=20)
    rect.set_figure(fig)
    fig.add_artist(rect)
    assert np.round(rect.get_transform().transform([[0, 0], [1, 1]]), 3).tolist() == [
        [64.0, 96.0], [299.034, 313.401]]


def test_a_line_of_an_axes_added_to_its_figure_with_clipping():
    fig, ax = plt.subplots()
    first, = ax.plot([1, 2])
    second, = ax.plot([2, 1])
    fig.add_artist(first)
    fig.add_artist(second, clip=True)
    extra = fig.get_default_bbox_extra_artists()
    assert [a.get_label() for a in extra if isinstance(a, lines.Line2D)] == [
        '_child0', '_child1', '_child1']
    assert second.get_clip_box().extents.tolist() == [0.0, 0.0, 640.0, 480.0]
    assert first.get_clip_box().extents.tolist() == [80.0, 52.8, 576.0, 422.4]


def test_clipping_to_the_background_of_another_axes():
    fig, (ax1, ax2) = plt.subplots(1, 2)
    rect = fig.add_artist(patches.Rectangle((0, 0), 1, 1))
    rect.set_clip_path(ax2.patch)
    image = pixels(fig)
    assert image[240, 150].tolist() == [255, 255, 255, 255]
    assert image[240, 450].tolist() == [31, 119, 180, 255]
    assert np.round(rect.get_tightbbox().bounds, 6).tolist() == [
        350.545455, 52.8, 225.454545, 369.6]


def test_clipping_to_the_background_of_an_axes_of_another_figure():
    fig, ax = plt.subplots()
    rect = ax.add_patch(patches.Rectangle((0.2, 0.2), 0.5, 0.5))
    fig2, ax2 = plt.subplots()
    rect.set_clip_path(ax2.patch)
    fig.canvas.draw()
    assert np.round(rect.get_tightbbox().bounds, 6).tolist() == [179.2, 126.72, 248.0, 184.8]


# -- offset boxes ------------------------------------------------------------------

def annotation_box_tree():
    fig, ax = plt.subplots()
    image, text = OffsetImage(Z, zoom=2), TextArea('hello')
    packer = VPacker(children=[image, text], pad=2, sep=3)
    ax.add_artist(AnnotationBbox(packer, (0.5, 0.5)))
    return fig, ax, packer, image, text


def test_the_artists_of_an_annotation_box_are_in_no_axes():
    fig, ax, packer, image, text = annotation_box_tree()
    fig.canvas.draw()
    assert image.image.axes is None and text._text.axes is None
    fig2, ax2 = plt.subplots()
    with pytest.raises(RuntimeError, match="more than one figure"):
        ax2.add_artist(image)
    assert image.axes is ax2 and image.image.axes is ax2


def test_the_boxes_of_a_cleared_annotation_box_keep_their_offsets():
    fig, ax, packer, image, text = annotation_box_tree()
    fig.canvas.draw()
    fig.clear()
    assert [np.round(box.get_window_extent().bounds, 6).tolist()
            for box in (packer, image, text)] == [
        [307.722222, 218.62615, 40.555556, 37.9477],
        [310.5, 239.907183, 16.666667, 13.888889],
        [310.5, 221.403928, 35.0, 14.336589]]


# -- axis artists, titles, ticks ---------------------------------------------------

def test_axis_artists_measure_with_the_renderer_given():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    bounds = ax.axis['bottom'].get_tightbbox(small_renderer()).bounds
    assert np.round(bounds, 6).tolist() == [75.0, 39.6023, 506.0, 13.888889]


def test_axis_artists_measure_the_axes_as_they_are():
    # Before a draw, the aspect is not applied yet.
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.set_aspect(1)
    ax.set_xlim(-5, 12)
    ax.set_ylim(-5, 10)
    bounds = ax.axis['left'].get_tightbbox(fig.canvas.get_renderer()).bounds
    assert np.round(bounds, 6).tolist() == [49.277778, 52.8, 30.722222, 376.544444]


def test_a_hidden_title_is_lifted_far_up():
    fig, ax = plt.subplots()
    title = ax.set_title('title')
    title.set_visible(False)
    fig.canvas.draw()
    assert title.get_position()[1] == pytest.approx(2.1428571428571432, abs=1e-12)


def test_a_hidden_tick_label_has_a_null_tight_box():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    label = ax.get_xticklabels()[0]
    fig.canvas.draw()
    label.set_visible(False)
    bbox = label.get_tightbbox()
    assert (bbox.x0, bbox.x1) == (np.inf, -np.inf)


def test_panning_a_polar_axes_leaves_its_aspect_unapplied():
    fig = plt.figure()
    ax = fig.add_subplot(projection='polar')
    ax.start_pan(60, 40, 1)
    assert str(ax) == 'PolarAxes(0.125,0.11;0.775x0.77)'


@pytest.mark.parametrize("cleared", ["figure", "axes"])
def test_clearing_a_3d_axes_leaves_its_axes_stale(cleared):
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    fig.canvas.draw()
    if cleared == "figure":
        fig.clear()
    else:
        ax.cla()
    assert (ax.xaxis.stale, ax.yaxis.stale, ax.zaxis.stale) == (True, True, True)


# -- PDF documents written by hand --------------------------------------------------

@pytest.mark.skipif(shutil.which("kpsewhich") is None, reason="needs TeX")
def test_a_type1_descriptor_beside_an_open_page_stream():
    from pltrs import _type1font, dviread
    from pltrs.backends.backend_pdf import PdfFile
    font = _type1font.Type1Font(dviread.find_tex_file('cmr10.pfb'))
    pdf = PdfFile(io.BytesIO())
    pdf.newPage(4, 3)
    with pytest.raises(AssertionError, match="^$"):
        pdf.createType1Descriptor(font)
    with pytest.raises(AssertionError, match=r"^No offset for object 14 \(font file\)$"):
        pdf.finalize()


@pytest.mark.skipif(shutil.which("latex") is None, reason="needs TeX")
def test_a_tex_font_named_but_never_drawn_cannot_be_embedded():
    from pltrs import dviread, texmanager
    from pltrs.backends.backend_pdf import PdfFile
    from pltrs.font_manager import FontPath
    with dviread.Dvi(texmanager.TexManager.make_dvi('ab', 12), 72) as dvi:
        font = next(iter(dvi)).text[0].font
    pdf = PdfFile(io.BytesIO())
    name = pdf.dviFontName(font)
    with pytest.raises(KeyError) as raised:
        pdf.finalize()
    assert raised.value.args == (FontPath(font.texname.decode('ascii'), 0),)
    assert str(name).startswith('/F-')
