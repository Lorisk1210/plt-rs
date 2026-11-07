"""
Smoke tests of what pltrs (Python) does as matplotlib does where the
differential tester found it apart: tight layouts of 3-D axes with titles,
texts and axes measured with the renderer of another resolution, secondary
axes in another transform's coordinates, a figure's own alpha, transform
and clip box, axisartist's tick labels (their extents, positions,
anti-aliasing and math), contours removed before a draw, the colour a
failed fill takes, parasites drawn through their aux transform, the paths
and arrays collections hand out, `PdfFile`'s and `dviread`'s errors, and
`axline` on polar axes. The expected values were recorded from matplotlib
3.11.2 running the same code.
"""

import io

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, dviread, mathtext, patches, path, transforms
from pltrs.backends import backend_pdf
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.figure import Figure
from pltrs.mpl_toolkits import axisartist
from pltrs.mpl_toolkits.axes_grid1 import host_subplot
from pltrs.mpl_toolkits.axisartist import angle_helper, floating_axes, grid_finder

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def renderer_at_50_dpi():
    """The renderer of another figure, at 50 dpi."""
    return FigureCanvasAgg(Figure(figsize=(3, 2), dpi=50)).get_renderer()


def test_tight_layout_places_lowered_3d_titles_at_the_top():
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(8, 12), subplot_kw={'projection': '3d'})
    ax1.set_title("one")
    ax2.set_title("two")
    plt.tight_layout()
    assert ax1.get_position().bounds == pytest.approx(
        (0.1577470075197619, 0.513665931901349, 0.6845059849604761, 0.45633732330698407))
    assert ax2.get_position().bounds == pytest.approx(
        (0.15774700751976195, 0.019915931901349215, 0.684505984960476, 0.456337323306984))


def test_texts_measure_their_lines_with_the_renderer_they_are_given():
    fig, ax = plt.subplots()
    text = ax.text(0.5, 0.5, 'hello', ha='center')
    # The lines at the figure's renderer's resolution, placed at 200 dpi.
    assert text.get_window_extent(dpi=200).bounds == pytest.approx(
        (638.5, 468.5268229166666, 35.0, 27.77777777777777))
    assert text.get_window_extent(renderer_at_50_dpi()).bounds == pytest.approx(
        (320.0, 234.2634114583333, 16.0, 13.888888888888886))
    # The text keeps measuring with the renderer it was last given.
    assert text.get_window_extent(dpi=200).bounds == pytest.approx(
        (648.0, 468.5268229166666, 16.0, 27.77777777777777))


def test_axes_measure_their_texts_with_the_renderer_they_are_given():
    renderer = renderer_at_50_dpi()
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    ax.plot([0, 1, 2], [0, 1, 0], [3, 1, 4])
    assert ax.get_tightbbox(renderer).bounds == pytest.approx(
        (143.2, 43.68646720375199, 384.08997760138385, 378.713532796248))
    fig, ax = plt.subplots()
    ax.plot([1, 2, 30], [1, 2, 3])
    assert ax.get_tightbbox(renderer).bounds == pytest.approx(
        (56.27777777777777, 29.188888888888886, 519.7222222222222, 393.2111111111111))


def test_a_secondary_axis_faces_as_its_location_says():
    fig, ax = plt.subplots()
    sec = ax.secondary_yaxis('right', transform=transforms.Affine2D().scale(2, 3).translate(1, 1))
    fig.canvas.draw()
    assert sec.yaxis.get_ticks_position() == 'right'
    assert sorted(name for name, spine in sec.spines.items() if spine.get_visible()) == ['right']


def test_a_figure_keeps_its_own_alpha_transform_and_clip_box():
    fig = plt.figure()
    fig.set_alpha(0.5)
    buffer = io.BytesIO()
    fig.savefig(buffer, format='png')
    from PIL import Image
    buffer.seek(0)
    corner = np.asarray(Image.open(buffer))[0, 0]
    assert (fig.get_alpha(), fig.patch.get_alpha(), tuple(map(int, corner))) == (
        0.5, None, (255, 255, 255, 255))
    other = Figure()
    FigureCanvasAgg(other)
    fig.update_from(other.add_subplot().plot([1, 2])[0])
    assert type(fig.get_clip_box()).__name__ == 'TransformedBbox'
    assert type(fig.get_transform()).__name__ == 'CompositeGenericTransform'


def test_axisartist_tick_labels_measure_and_draw_as_matplotlibs():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    fig.canvas.draw()
    labels = ax.axis['bottom'].major_ticklabels
    assert labels.get_window_extent().bounds == pytest.approx(
        (565.0, 39.74118923611111, 22.0, 13.888888888888886))
    assert [type(v) for v in labels.get_position()] == [np.float64, np.float64]
    labels.set_x(float('nan'))
    assert np.isnan(labels.get_window_extent().x0)
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.axis['bottom'].major_ticklabels.set_antialiased(False)
    fig.canvas.draw()
    strip = np.asarray(fig.canvas.buffer_rgba())[430:450, 60:600, 0]
    assert len(np.unique(strip)) == 11


def test_axisartist_tick_labels_show_their_math_unparsed():
    polar = (transforms.Affine2D().scale(np.pi / 180, 1)
             + pltrs.projections.polar.PolarTransform())
    helper = floating_axes.GridHelperCurveLinear(
        polar, extremes=(0, 90, 2, 6), grid_locator1=angle_helper.LocatorDMS(4),
        tick_formatter1=angle_helper.FormatterDMS(),
        grid_locator2=grid_finder.MaxNLocator(4))
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=floating_axes.FloatingAxes, grid_helper=helper)
    labels = ax.axis['bottom'].major_ticklabels
    labels.set_parse_math(False)
    fig.canvas.draw()
    assert [b.width for b in labels.get_window_extents()] == pytest.approx(
        [14.337, 60.76, 90.902, 96.688], abs=1e-3)


def test_contours_removed_before_a_draw_leave_the_margins():
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    fig, ax = plt.subplots()
    ax.contour(z).remove()
    fig.canvas.draw()
    assert ax.get_xlim() == pytest.approx((-0.25, 5.25))


def test_a_fill_failing_on_its_step_takes_a_colour():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError):
        ax.fill_betweenx([0, 1], [0, 1], step=0.5)
    assert tuple(ax.fill_betweenx([0, 1], [0, 1]).get_facecolor()[0]) == pytest.approx(
        (1.0, 0.4980392156862745, 0.054901960784313725, 1.0))


def test_parasites_draw_through_their_aux_transform():
    fig = plt.figure()
    host = host_subplot(111, figure=fig)
    aux = host.get_aux_axes(transforms.Affine2D().scale(1, 2), viewlim_mode='transform')
    line, = aux.plot([0, 1], [0, 0.25])
    fig.canvas.draw()
    assert line.get_transform().transform([(1, 0.25)])[0] == pytest.approx((576.0, 237.6))


def test_collections_hand_out_paths_and_arrays_as_matplotlibs():
    fig, ax = plt.subplots()
    scatter = ax.scatter([1, 2], [1, 2], c=[3.0, 4.0])
    assert scatter.get_array().mask is np.ma.nomask
    calls = []
    fig.add_callback(lambda artist: calls.append(artist))
    fig.set_clip_box(None)
    assert calls == []
    ellipses = collections.EllipseCollection(
        [1], [0.5], [30], units='xy', offsets=[(0.5, 0.5)], offset_transform=ax.transData)
    ax.add_collection(ellipses)
    assert ellipses.get_paths()[0] is path.Path.unit_circle()
    assert ellipses.get_transforms().shape == (0, 3, 3)
    fig.canvas.draw()
    assert ellipses.get_transforms().shape == (1, 3, 3)
    mesh = ax.pcolormesh(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)))
    before = mesh.get_paths()
    mesh.set_paths()
    assert mesh.get_paths() is not before
    given = path.Path([(0, 0), (1, 1), (1, 0)])
    patch = patches.PathPatch(given)
    ax.add_patch(patch)
    assert patch.get_path() is given
    paths = [given]
    ellipses.set_paths(paths)
    assert ellipses.get_paths() is paths


def test_pdf_files_fail_as_matplotlibs_do():
    pdf = backend_pdf.PdfFile(io.BytesIO())
    pdf.newPage(4, 3)
    pdf.markerObject(path.Path.unit_circle(), transforms.Affine2D().scale(3), True, True,
                     1.0, 'miter', 'butt')
    with pytest.raises(AssertionError):
        pdf.writeMarkers()
    pdf.endStream()
    pdf.hatchPattern(((0, 0, 0, 1), (1, 1, 1, 1), '//', 1.0))
    with pytest.raises(TypeError, match="cannot unpack non-iterable Name object"), \
            pytest.warns(pltrs.MatplotlibDeprecationWarning):
        pdf.hatchPatterns
    pdf.fontName(pltrs.font_manager.FontProperties(family='DejaVu Sans'))
    with pytest.raises(KeyError):
        pdf.writeFonts()
    pdf.imageObject(np.linspace(0, 1, 24).reshape(2, 3, 4))
    with pytest.raises(OSError, match="cannot write mode F as PNG"):
        pdf.writeImages()


def test_luatex_names_are_checked_before_the_scale():
    with pytest.raises(ValueError, match="Invalid modern font name"):
        dviread.DviFont.from_luatex(2.0, b'cmr10')


def test_math_metrics_are_numpy_floats():
    prop = pltrs.font_manager.FontProperties(family='DejaVu Sans', size=12)
    parse = mathtext.MathTextParser('path').parse('$x^2$', 72, prop)
    assert type(parse.glyphs[0][5]) is np.float64
    rule = mathtext.MathTextParser('path').parse(r'$\frac{a}{b}$').rects[0]
    assert [type(v) for v in rule] == [float, np.float64, float, float]
    from pltrs.backends.backend_svg import RendererSVG
    width, height, descent = RendererSVG(100, 100, io.StringIO()).get_text_width_height_descent(
        '$x$', prop, True)
    assert type(width) is np.float64 and width == 7.199999999999999


def test_axline_fails_drawing_polar_axes():
    ax = plt.axes(projection='polar')
    ax.axline((0, 0), (1, 1))
    with pytest.raises(AttributeError, match="'PolarAxes' object has no attribute 'transLimits'"):
        plt.gcf().savefig(io.BytesIO(), format='png')
