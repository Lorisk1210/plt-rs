"""
What the differential tester's hostile programs of the module-level
functions, figures, renderers, writers and tables
(``spec/diff/generators/hostile.py``) found pltrs doing otherwise than
matplotlib 3.11 in their third round: the order in which constructors and
functions compute with their arguments (a legend's paddings, a colorbar's
Axes, a figure's patch), values kept unchecked until drawn (a cell's size
or filter, a figure image's offset, a colorbar's box aspect), FreeType
refusing font sizes of no pixel, pybind11's conversions of ``_path``, and
PDF streams buffered as matplotlib's. Each expected value and message is
recorded from matplotlib.
"""

import io
import math
import re

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colorbar, legend, mlab, transforms
from pltrs.backends import backend_pdf, backend_ps, backend_svg
from pltrs.figure import Figure, SubFigure
from pltrs.gridspec import SubplotParams
from pltrs.path import Path

import build_facts


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


NAN_TO_INT = "cannot convert float NaN to integer"
#: The package classes are named after in messages (matplotlib's, or
#: pltrs's under its own name).
PACKAGE = "(matplotlib|pltrs)"


# -- layouts measure the Axes, which fail placed nowhere ------------------------

def test_tight_layout_measures_axes_placed_nowhere():
    fig, ax = plt.subplots()
    fig.tight_layout(pad=float('nan'))
    assert math.isnan(fig.subplotpars.left)
    with pytest.raises(ValueError, match=NAN_TO_INT):
        fig.tight_layout(pad=0.3)
    with pytest.raises(ValueError, match=NAN_TO_INT):
        fig.canvas.draw()


def test_autofmt_xdate_takes_the_labels_first_then_checks_them():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match=re.escape(
            "'' is not a valid value for align. Supported values are "
            "'center', 'right', 'left'")):
        fig.autofmt_xdate(ha='')
    with pytest.raises(ValueError, match="rotation must be 'vertical', "
                       "'horizontal' or a number, not 45"):
        fig.autofmt_xdate(rotation='45')
    fig.autofmt_xdate(bottom=float('nan'))
    with pytest.raises(ValueError, match=NAN_TO_INT):
        fig.autofmt_xdate(ha='')


# -- FreeType refuses sizes of no pixel per em ----------------------------------

def test_texts_at_tiny_resolutions_fail_as_freetype_refuses_them():
    fig, ax = plt.subplots()
    fig.set_dpi(2.5)
    with pytest.raises(RuntimeError, match=re.escape(
            f"FT_Set_Char_Size (ft2font.cpp line {build_facts.SET_CHAR_SIZE_LINE}) failed "
            "with error 0x97: invalid ppem value")):
        fig.canvas.draw()
    fig.set_dpi(100)
    fig.canvas.draw()


# -- figures check their patch as matplotlib's makes it -------------------------

def test_figure_patch_checks_its_colours_and_width():
    with pytest.raises(ValueError, match="Invalid RGBA argument: ''"):
        Figure(edgecolor='')
    with pytest.raises(ValueError, match="Invalid RGBA argument: 3"):
        Figure(edgecolor=3)
    assert Figure(linewidth='2.0').get_linewidth() == 2.0
    with pytest.raises(TypeError, match="can't multiply sequence by non-int of "
                       "type 'numpy.float64'"):
        Figure(dpi='72')
    fig = Figure()
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute '_root_figure'"):
        SubFigure(None, fig.add_gridspec(1, 2)[0, 1])
    with pytest.raises(ValueError, match="Invalid RGBA argument: 'Y'"):
        SubFigure(fig, fig.add_gridspec(1, 2)[0, 1], facecolor='Y')


def test_figure_resets_the_subplot_parameters_it_is_given():
    params = SubplotParams(left=0.3)
    fig = Figure(subplotpars=params)
    assert fig.subplotpars is params
    assert params.left == 0.125
    with pytest.raises(AttributeError, match="'int' object has no attribute 'reset'"):
        Figure(subplotpars=0)


def test_add_axes_places_what_is_no_sequence_as_a_subplot():
    fig = Figure()
    with pytest.raises(ValueError, match="Single argument to subplot must be a "
                       "three-digit integer, not 0"):
        fig.add_axes(0)
    with pytest.raises(TypeError, match=re.escape(
            "Bbox.from_bounds() missing 4 required positional arguments")):
        fig.add_axes([])


# -- renderers keep their size and compute their magnification ------------------

def test_vector_renderers_keep_any_size():
    backend_ps.RendererPS(-1, 3, io.StringIO())
    backend_ps.RendererPS(4, float('nan'), io.StringIO(), imagedpi=0)
    with pytest.raises(TypeError, match=re.escape(
            "unsupported operand type(s) for /: 'str' and 'int'")):
        backend_ps.RendererPS(4, 3, io.StringIO(), imagedpi='72')
    backend_svg.RendererSVG(0, 3, io.StringIO())
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'write'"):
        backend_svg.RendererSVG(4, 3, None)
    with pytest.raises(TypeError, match="'list' object is not a mapping"):
        backend_svg.RendererSVG(4, 3, io.StringIO(), metadata=[])


def test_postscript_helpers_compute_as_matplotlibs(tmp_path):
    eps = tmp_path / "plain.eps"
    plt.figure().savefig(eps)
    with pytest.raises(TypeError, match=re.escape(
            "unsupported operand type(s) for +: 'NoneType' and 'str'")):
        backend_ps.pstoeps(None)
    with pytest.raises(TypeError, match="cannot unpack non-iterable int object"):
        backend_ps.pstoeps(str(eps), bbox=3)
    with pytest.raises(TypeError, match="cannot unpack non-iterable NoneType object"):
        backend_ps.pstoeps(str(eps), rotated=1)
    with pytest.raises(TypeError, match=re.escape(
            "unsupported operand type(s) for +: 'int' and 'str'")):
        backend_ps.gs_distill(3, eps=True)


# -- legends compute with their paddings as matplotlib's constructor ------------

def test_legend_computes_with_its_paddings_in_order():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1])
    sequence = "can't multiply sequence by non-int of type 'float'"
    with pytest.raises(TypeError, match=sequence):
        legend.Legend(ax, [line], ['l'], markerscale='x')
    with pytest.raises(TypeError, match=sequence):
        legend.Legend(ax, [line], ['l'], borderpad='x')
    with pytest.raises(TypeError, match=re.escape(
            "unsupported operand type(s) for -: 'str' and 'float'")):
        legend.Legend(ax, [line], ['l'], handleheight='x')
    with pytest.raises(ValueError, match="number sections must be larger than 0."):
        legend.Legend(ax, [line], ['l'], ncol=0)
    with pytest.raises(ValueError, match="Invalid labelcolor: 0"):
        legend.Legend(ax, [line], ['l'], labelcolor=0)
    with pytest.raises(ValueError, match="Invalid bbox: 0"):
        legend.Legend(ax, [line], ['l'], bbox_to_anchor=0)
    with pytest.raises(TypeError, match="alpha must be numeric or None"):
        legend.Legend(ax, [line], ['l'], framealpha='x')
    legend.Legend(ax, [line], ['l'], mode='nope')
    legend.Legend(ax, [line], ['l'], bbox_transform=0)


def test_legend_keeps_a_pad_to_its_parent_that_is_no_number():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1])
    leg = ax.legend([line], ['l'], borderaxespad='x')
    assert leg.borderaxespad == 'x'
    with pytest.raises(TypeError, match=re.escape(
            "_compat_get_offset.<locals>.<lambda>() missing 2 required positional "
            "arguments: 'bbox' and 'renderer'")):
        fig.canvas.draw()


# -- colorbars and their Axes ---------------------------------------------------

def test_colorbar_checks_in_matplotlibs_order():
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute '_axes_locator'"):
        colorbar.Colorbar(None)
    with pytest.raises(AttributeError, match="'int' object has no attribute 'cmap'"):
        colorbar.Colorbar(ax, mappable=3)
    with pytest.raises(ValueError, match=re.escape(
            "'nope' is not a valid value for extend. Supported values are "
            "'neither', 'both', 'min', 'max'")):
        colorbar.Colorbar(ax, extend='nope')
    with pytest.raises(ValueError, match=re.escape(
            "'locs' must be 1D with shape (N,), but your input has shape ()")):
        colorbar.Colorbar(ax, ticks='ab')
    with pytest.raises(TypeError, match=f"'formatter' must be an instance of "
                       f"{PACKAGE}.ticker.Formatter, not a int"):
        colorbar.Colorbar(ax, format=3)
    with pytest.raises(ValueError, match=re.escape(
            "'x' is not a valid value for extendfrac. Supported values are 'auto'")):
        colorbar.Colorbar(ax, extendfrac='x')
    with pytest.raises(TypeError, match=re.escape(
            "Colorbar.__init__() got an unexpected keyword argument 'nonexistent_kw'")):
        fig.colorbar(ax.imshow([[0, 1]]), nonexistent_kw=1)


def test_make_axes_computes_as_matplotlib_and_moves_the_parents_first():
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute 'get_figure'"):
        colorbar.make_axes(None)
    with pytest.raises(TypeError, match=re.escape(
            "unsupported operand type(s) for -: 'int' and 'str'")):
        colorbar.make_axes(ax, fraction='0.1')
    with pytest.raises(TypeError, match="cannot use 'list' as a dict key"):
        colorbar.make_axes(ax, location=[])
    with pytest.raises(ValueError, match=re.escape(
            "all entries in rect must be finite not Bbox(x0=nan, y0=")):
        colorbar.make_axes(ax, fraction=float('nan'))
    assert math.isnan(ax.get_position(original=True).x0)


def test_make_axes_gridspec_takes_any_aspect_and_fails_drawing():
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute 'get_subplotspec'"):
        colorbar.make_axes_gridspec(None)
    colorbar.make_axes_gridspec(ax, aspect=-1.5)
    with pytest.raises(ValueError, match="'box_aspect' and 'fig_aspect' must be positive"):
        fig.canvas.draw()


# -- spectra of odd arguments, as matplotlib's mlab computes them ---------------

def test_mlab_spectra_compute_odd_arguments_as_matplotlib():
    x = np.sin(np.arange(64.0))
    y = np.cos(np.arange(64.0))
    with pytest.raises(TypeError, match="len\\(\\) of unsized object"):
        mlab.cohere(x, None, NFFT=16)
    with pytest.raises(ZeroDivisionError):
        mlab.csd(x, y, NFFT=16, Fs=0)
    with pytest.raises(ValueError, match="The window length must match the data's "
                       "first dimension"):
        mlab.cohere(x, y, NFFT=16, window='x')
    with pytest.raises(ValueError, match=re.escape(
            "operands could not be broadcast together with shapes (9,4) (9,3)")):
        mlab.csd(x, y[:-16], NFFT=16)


# -- PDF streams, objects and paths ---------------------------------------------

def test_pdf_streams_of_inline_length_are_independent():
    pdf = backend_pdf.PdfFile(io.BytesIO())
    backend_pdf.Stream(-1, None, pdf)
    backend_pdf.Stream(0, None, pdf)
    with pytest.raises(TypeError, match="list indices must be integers or slices, "
                       "not float"):
        backend_pdf.Stream(13.5, None, pdf)
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'fh'"):
        backend_pdf.Stream(pdf.reserveObject('s').id, None, None)
    with pytest.raises(TypeError, match="a bytes-like object is required, not 'str'"):
        backend_pdf.Stream(pdf.reserveObject('s').id, None, pdf).write('x')
    with pytest.raises(IndexError, match="list index out of range"):
        pdf.recordXref(10 ** 6)


def test_pdf_pages_and_notes_take_what_matplotlib_writes():
    pdf = backend_pdf.PdfFile(io.BytesIO())
    pdf.newPage('4', 3)
    pdf.newTextnote('note', positionRect=[])
    pdf.newTextnote('note', positionRect='ab')
    pdf.newTextnote('note', positionRect=None)


def test_pdf_path_operations_convert_as_pybind11():
    square = Path([(0, 0), (1, 0), (1, 1)])
    affine = transforms.Affine2D()
    operations = backend_pdf.PdfFile.pathOperations
    assert operations(None, affine)[0].pdfRepr() == b''
    with pytest.raises(ValueError, match="Invalid affine transformation matrix"):
        operations(square, 3)
    with pytest.raises(ValueError, match="Invalid bounding box"):
        operations(square, affine, clip=1)
    with pytest.raises(TypeError, match=re.escape(
            "convert_to_string(): incompatible function arguments")):
        operations(square, affine, simplify='x')
    with pytest.raises(RuntimeError, match="Unable to cast Python instance of type "
                       "<class 'int'>"):
        operations(square, affine, sketch=0)


# -- tables draw their cells as matplotlib's ------------------------------------

def _cell():
    fig, ax = plt.subplots()
    tab = ax.table([['a', 'b'], ['c', 'd']])
    return fig, tab[0, 1]


def test_cells_keep_sizes_and_filters_until_drawn():
    fig, cell = _cell()
    cell.set_width('4.0')
    assert cell.get_width() == '4.0'
    # Fitting the font compares the width the text needs with it first.
    with pytest.raises(TypeError, match="ufunc 'greater' did not contain a loop"):
        fig.canvas.draw()
    cell.set_fontsize(1)
    with pytest.raises(TypeError, match="'>' not supported between instances of "
                       "'str' and 'float'"):
        fig.canvas.draw()
    cell.set_width(0.4)
    fig.canvas.draw()
    cell.set_agg_filter(3)
    with pytest.raises(TypeError, match="'int' object is not callable"):
        fig.canvas.draw()
    cell.set_agg_filter(None)
    cell.set_transform(3)
    with pytest.raises(AttributeError, match="'int' object has no attribute 'inverted'"):
        fig.canvas.draw()


def test_cells_measure_their_text_with_the_renderer_given():
    fig, cell = _cell()
    cell.set_fontsize(None)
    assert cell.get_fontsize() == 10.0
    with pytest.raises(TypeError, match="cannot create weak reference to 'int' object"):
        cell.get_text_bounds(3)
    # The text keeps the renderer it was given last.
    with pytest.raises(TypeError, match="cannot create weak reference to 'int' object"):
        cell.get_required_width(None)


# -- module functions -----------------------------------------------------------

def test_streamplot_reads_the_axes_transform_first():
    from pltrs import streamplot
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute 'transData'"):
        streamplot.streamplot(None, np.arange(6), np.arange(5), np.ones((5, 6)),
                              np.ones((5, 6)))


def test_intervals_of_what_is_no_number_compare_as_python():
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        with pytest.raises(TypeError, match="'<=' not supported between instances "
                           "of 'str' and 'float'"):
            transforms.interval_contains('ab', 0.5)
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        assert transforms.interval_contains_open((0, 1), 0.5)


def test_figimage_keeps_an_offset_that_is_no_number():
    fig = plt.figure()
    image = fig.figimage(np.arange(6.0).reshape(2, 3), xo=None)
    assert image.ox is None
    with pytest.raises(TypeError, match=re.escape(
            "unsupported operand type(s) for /: 'NoneType' and 'float'")):
        fig.canvas.draw()


def test_sci_names_matplotlibs_classes():
    plt.subplots()
    with pytest.raises(TypeError, match=f"'im' must be an instance of "
                       f"{PACKAGE}.collections.Collection or "
                       f"{PACKAGE}.image.AxesImage, not a None"):
        plt.sci(None)


def test_stale_reads_back_as_given():
    fig = plt.figure()
    fig.stale = None
    assert fig.stale is None
    fig.stale = 'False'
    assert fig.stale == 'False'
    fig.canvas.draw()
    assert fig.stale is False


def test_path_segments_take_matrices_as_pybind11():
    square = Path([(0, 0), (1, 0), (1, 1)])
    with pytest.raises(ValueError, match="Invalid affine transformation matrix"):
        list(square.iter_segments(3))
    with pytest.raises(ValueError, match="Invalid affine transformation matrix"):
        list(square.iter_segments('x'))


def test_postscript_renderer_lays_text_out_as_matplotlib(caplog):
    from pltrs.font_manager import FontProperties
    renderer = backend_ps.RendererPS(4, 3, io.StringIO())
    gc = renderer.new_gc()
    prop = FontProperties(size=12)
    with pytest.raises(TypeError, match=re.escape("_layout(): incompatible function arguments")):
        renderer.draw_text(gc, 10, 10, None, prop, 0)
    with pytest.raises(TypeError, match="argument after \\*\\* must be a mapping, not int"):
        renderer.draw_text(gc, 10, 10, 'ab', 3, 0)
    # Without usetex, TeX is drawn as plain text, which it says once.
    with caplog.at_level("WARNING"):
        renderer.draw_tex(gc, 10, 10, 'ab', prop, 0)
        renderer.draw_tex(gc, 10, 10, 'cd', prop, 0)
    assert [record.getMessage() for record in caplog.records] == [
        "The PS backend determines usetex status solely based on rcParams['text.usetex'] "
        "and does not support having usetex=True only for some elements; this element "
        "will thus be rendered as if usetex=False."]
    with pytest.raises(AttributeError, match="'int' object has no attribute 'get_fontfeatures'"):
        renderer.draw_tex(gc, 10, 10, 'ab', prop, 0, mtext=0)


def test_pdf_hatch_patterns_keep_their_names():
    pdf = backend_pdf.PdfFile(io.BytesIO())
    gc = backend_pdf.GraphicsContextPdf(pdf)
    first = gc.hatch_cmd('//', (0, 0, 0, 1), 1.0)[2]
    assert gc.hatch_cmd('//', [0, 0, 0, 1], 1.0)[2] is first


def test_saving_a_pdf_with_a_number_it_cannot_write_raises_matplotlibs_error():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1], linewidth=float('inf'))
    with pytest.raises(ValueError, match="Can only output finite numbers in PDF"):
        fig.savefig(io.BytesIO(), format='pdf')


def test_constrained_layout_computes_with_the_spacing_of_subfigure_grids():
    fig = plt.figure(layout='constrained')
    fig.subplots(2, 2)
    fig.subfigures(wspace='0.2')
    with pytest.raises(TypeError, match=re.escape(
            "unsupported operand type(s) for /: 'str' and 'int'")):
        fig.canvas.draw()


def test_canvas_inaxes_reads_the_point_as_matplotlibs_patches():
    fig, ax = plt.subplots()
    with pytest.raises(IndexError, match="tuple index out of range"):
        fig.canvas.inaxes((60,))
    with pytest.raises(TypeError, match="'NoneType' object is not subscriptable"):
        fig.canvas.inaxes(None)
    assert plt.figure().canvas.inaxes(None) is None


# -- fitted fonts, SVG triangles, Sankey diagrams and prints --------------------

def test_table_fonts_fitted_when_drawn_never_grow_back():
    fig, ax = plt.subplots()
    cell = ax.table([['a rather long text']])[0, 0]
    cell.set_width(-1.5)
    fig.canvas.draw()
    assert cell.get_fontsize() == 1
    cell.set_width(4.0)
    fig.canvas.draw()
    assert cell.get_fontsize() == 1
    # Fitted down to 1 point, the fonts are no longer compared with the
    # widths: the cells are, as they are aligned.
    cell.set_width('4.0')
    with pytest.raises(TypeError, match=re.escape(
            "'>' not supported between instances of 'str' and 'float'")):
        fig.canvas.draw()
    cell.set_width(4.0)
    cell.set_fontsize(12)
    fig.canvas.draw()
    assert cell.get_fontsize() == 12


def test_svg_gouraud_triangles_compute_their_gradients_as_matplotlib():
    renderer = backend_svg.RendererSVG(4, 3, io.StringIO(), image_dpi=72)
    gc = renderer.new_gc()
    triangle = np.array([[[0, 0], [10, 0], [0, 10]]], float)
    identity = transforms.IdentityTransform()
    flags = np.array([[[True, False, False, True], [False, True, False, True],
                       [False, False, True, True]]])
    with pytest.raises(ValueError, match=re.escape(
            "Invalid RGBA argument: array([ True, False, False,  True])")):
        renderer.draw_gouraud_triangles(gc, triangle, flags, identity)
    masked = np.ma.masked_invalid(np.array([[[np.nan, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0],
                                             [0.0, 0.0, 1.0, 1.0]]]))
    with pytest.raises(ValueError, match=re.escape(
            "Invalid RGBA argument: masked_array(data=[--, 0.0, 0.0, 1.0]")):
        renderer.draw_gouraud_triangles(gc, triangle, masked, identity)
    with pytest.raises(np.exceptions.AxisError, match="axis 0 is out of bounds"):
        renderer.draw_gouraud_triangles(gc, triangle, 'data', identity)
    # A transparent triangle is skipped, whatever its corners' colours.
    renderer.draw_gouraud_triangles(gc, triangle, np.zeros((1, 3, 4)), identity)


def test_sankey_keeps_what_is_no_axes_until_it_places_a_diagram_on_it():
    from pltrs.sankey import Sankey
    sankey = Sankey(ax=3)
    with pytest.raises(AttributeError, match="'int' object has no attribute 'text'"):
        sankey.add(flows=[1, -1])
    with pytest.raises(AttributeError, match="'int' object has no attribute 'axis'"):
        sankey.finish()


def test_sankey_computes_with_its_scale_format_and_margin_where_matplotlib_does():
    from pltrs.sankey import Sankey
    with pytest.raises(TypeError, match="ufunc 'multiply' did not contain a loop"):
        Sankey(scale='x', flows=[1.0, -1.0])
    with pytest.raises(TypeError, match='format must be callable or a format string'):
        Sankey(format=3, unit='u', flows=[1.0, -1.0], labels=['a', 'b'])
    sankey = Sankey(margin='x', flows=[1.0, -1.0])
    with pytest.raises(TypeError, match="ufunc 'subtract' did not contain a loop"):
        sankey.finish()


def test_prints_read_their_area_only_from_print_figure():
    fig, ax = plt.subplots()
    ax.plot([1, 2])
    backend_svg.FigureCanvasSVG(fig).print_svg(io.StringIO(), bbox_inches_restore='x')
    backend_pdf.FigureCanvasPdf(fig).print_pdf(io.BytesIO(), bbox_inches_restore='x')
    backend_pdf.FigureCanvasPdf(fig).print_pdf(io.BytesIO(), metadata=[])
    with pytest.raises(TypeError, match=re.escape(
            "FigureCanvasSVG.print_svg() got an unexpected keyword argument "
            "'nonexistent_kw'")):
        backend_svg.FigureCanvasSVG(fig).print_svgz(io.BytesIO(), nonexistent_kw=1)


def test_colours_of_numpy_arrays_are_numbers_alone():
    from pltrs import colors
    with pytest.raises(ValueError, match=re.escape(
            "Invalid RGBA argument: array(['a', 'b', 'c'], dtype='<U1')")):
        colors.to_rgba(np.array(['a', 'b', 'c']))
    assert colors.to_rgba(np.array([1, 0, 0])) == (1.0, 0.0, 0.0, 1)
    assert colors.to_rgba((True, False, False)) == (1.0, 0.0, 0.0, 1)


def test_python_drawn_artists_fail_at_their_place_in_the_draw():
    # The figure text, kept with an x of None, is drawn after the Axes on a
    # grid of a NaN height ratio, which fail first; drawn before them, it
    # fails first.
    fig = plt.figure()
    fig.subplots(height_ratios=[float('nan')])
    text = fig.text(None, 0.25, 'text')
    with pytest.raises(ValueError, match=NAN_TO_INT):
        fig.canvas.draw()
    text.set_zorder(-1)
    with pytest.raises(TypeError, match="float.. argument must be a string or a real number"):
        fig.canvas.draw()


def test_figure_images_take_their_keywords_and_offsets_as_matplotlib():
    from pltrs.image import FigureImage
    fig = plt.figure()
    with pytest.raises(AttributeError, match=re.escape(
            "FigureImage.set() got an unexpected keyword argument 'nonexistent_kw'")):
        fig.figimage(np.arange(6), nonexistent_kw=1)
    image = FigureImage(fig, offsetx=None)
    assert image.ox is None


def test_axis_labels_keep_their_usetex():
    fig, ax = plt.subplots()
    assert not ax.xaxis.label.get_usetex()
    ax.set_xlabel('x', usetex=True)
    assert ax.xaxis.label.get_usetex()
    assert not ax.yaxis.label.get_usetex()
    ax.xaxis.label.set_usetex(False)
    assert not ax.xaxis.label.get_usetex()
