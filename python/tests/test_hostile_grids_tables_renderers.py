"""
What the differential tester's hostile programs of grids, figures,
tables, pyplot's functions, FreeType fonts and the renderers
(``spec/diff/generators/hostile.py``) found pltrs doing otherwise than
matplotlib 3.11 in their second round: the margins, spacing and ratios
grids keep unchecked and fail on where they place their cells, tables
keyed by any position, pybind11's refusals of the extensions' arguments,
and the arguments matplotlib's renderers and writers compute with before
they draw. Each expected value and message is recorded from matplotlib.
"""

import io
import math
import re
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import font_manager, gridspec, mathtext, table, textpath
from pltrs.backend_bases import FigureCanvasBase
from pltrs.backends import backend_agg, backend_pdf, backend_svg
from pltrs.figure import Figure
from pltrs.ft2font import FT2Font
from pltrs.path import Path
from pltrs.transforms import Affine2D, Bbox

import build_facts

INT_PLUS_STR = re.escape("unsupported operand type(s) for +: 'int' and 'str'")


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _font():
    return font_manager.get_font(font_manager.findfont("DejaVu Sans"))


# -- grids keep their spacing and ratios unchecked -----------------------------

def test_subplots_adjust_keeps_text_spacing_and_fails_placing_subplots():
    fig = plt.figure()
    fig.add_subplot()
    with pytest.raises(TypeError, match=INT_PLUS_STR):
        fig.subplots_adjust(wspace='0.2')
    assert fig.subplotpars.wspace == '0.2'
    # Every later change places the subplots again, and fails again.
    with pytest.raises(TypeError, match=INT_PLUS_STR):
        fig.subplots_adjust(hspace=0.1)
    assert fig.subplotpars.hspace == 0.1
    fig.subplots_adjust(wspace=0.3)
    assert fig.subplotpars.wspace == 0.3


def test_gridspec_keeps_its_spacing_and_ratios_as_given():
    fig = plt.figure()
    gs = gridspec.GridSpec(2, 3, figure=fig, wspace='x')
    assert gs.wspace == 'x'
    with pytest.raises(TypeError, match=INT_PLUS_STR):
        gs.get_grid_positions(fig)
    with pytest.raises(TypeError, match=INT_PLUS_STR):
        fig.add_subplot(gs[0, 0])
    gs.wspace = 0.1
    gs.set_height_ratios('ab')
    assert gs.get_height_ratios() == 'ab'
    with pytest.raises(TypeError, match=INT_PLUS_STR):
        gs.get_grid_positions(fig)
    with pytest.raises(AttributeError,
                       match="'int' object has no attribute 'subplotpars'"):
        gridspec.GridSpec(1, 1).get_grid_positions(3)


def test_infinite_spacing_is_placed_with_numpy_warnings():
    fig = plt.figure()
    fig.add_subplot()
    with pytest.warns(RuntimeWarning, match="invalid value encountered in subtract"):
        fig.subplots_adjust(top=float('inf'))


def test_subfigures_place_their_grid_as_matplotlib_does():
    fig = plt.figure()
    fig.add_subplot()
    with pytest.raises(IndexError, match="array is 0-dimensional, but 1 were indexed"):
        fig.subfigures(width_ratios='x')
    with pytest.raises(TypeError, match=INT_PLUS_STR):
        fig.subfigures(wspace='0.2')
    assert len(fig.subfigs) == 1  # added before the grid failed
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute 'get_gridspec'"):
        fig.add_subfigure(None)


def test_tight_layout_computes_with_its_arguments_first():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError,
                       match="can't multiply sequence by non-int of type 'float'"):
        fig.tight_layout(pad='0.3')
    assert type(fig.get_layout_engine()).__name__ == "PlaceHolderLayoutEngine"
    with pytest.raises(TypeError, match=r"unsupported operand type\(s\) for \*: "
                       "'NoneType' and 'float'"):
        fig.tight_layout(pad=None)
    with pytest.raises(ValueError, match=r"not enough values to unpack \(expected 4, got 0\)"):
        fig.tight_layout(rect=())
    # A grid with none of the figure's Axes lays nothing out.
    gridspec.GridSpec(2, 2).tight_layout(fig, pad='0.3')
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute '_get_renderer'"):
        gridspec.GridSpec(2, 2).tight_layout(None)


def test_align_labels_take_the_subplots_of_the_axes_given():
    fig, axs = plt.subplots(2)
    with pytest.raises(AttributeError,
                       match="'numpy.int64' object has no attribute 'get_subplotspec'"):
        fig.align_xlabels(axs=0)
    fig.align_ylabels(axs=[])


# -- tables keyed by any position ----------------------------------------------

def test_cells_at_positions_that_are_no_integers():
    fig, ax = plt.subplots()
    tab = table.table(ax, [['a', 'b']])
    cell = tab.add_cell('ab', 0, 1, 1, text='x')
    assert tab['ab', 0] is cell
    assert len(tab.get_celld()) == 3
    with pytest.raises(TypeError,
                       match="'<' not supported between instances of 'int' and 'str'"):
        fig.canvas.draw()
    with pytest.raises(TypeError,
                       match="'>=' not supported between instances of 'str' and 'int'"):
        tab.contains(None)


def test_table_arguments_are_checked_as_its_cells_are_made():
    fig, ax = plt.subplots()
    with pytest.raises(IndexError, match="list index out of range"):
        table.table(ax, [['a']], colWidths=[])
    with pytest.raises(TypeError, match=INT_PLUS_STR):
        table.table(ax, [['a']], colWidths='ab')
    with pytest.raises(ValueError, match="'' is not a valid value for align"):
        table.table(ax, [['a']], cellLoc='')
    with pytest.raises(TypeError, match="'int' object is not subscriptable"):
        table.table(ax, [['a']], colLabels=0)
    with pytest.raises(IndexError, match="list index out of range"):
        table.table(ax, [['a']], rowColours=[])
    table.table(ax, [['a']], rowLoc='', edges=None)  # no row labels to align
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute 'get_figure'"):
        table.Table(None)


def test_table_keeps_a_box_it_cannot_be_placed_in():
    fig, ax = plt.subplots()
    tab = table.Table(ax, bbox=3)
    tab.add_cell(0, 0, 0.5, 0.1, text='x')
    ax.add_table(tab)
    with pytest.raises(TypeError, match="cannot unpack non-iterable int object"):
        fig.canvas.draw()
    tab.remove()
    tab = table.Table(ax, bbox=['a', 'b', 'c', 'd'])
    tab.add_cell(0, 0, 0.5, 0.1, text='x')
    ax.add_table(tab)
    with pytest.raises(TypeError, match="ufunc 'divide' not supported"):
        fig.canvas.draw()


def test_cells_compute_with_their_arguments_as_rectangles_do():
    with pytest.raises(TypeError, match=r"unsupported operand type\(s\) for \+: "
                       "'int' and 'NoneType'"):
        table.Cell((0, 0), None, 1)
    with pytest.raises(TypeError, match="'int' object is not iterable"):
        table.Cell((0, 0), 1, 1, visible_edges=3)
    with pytest.raises(TypeError, match=re.escape(build_facts.raised(lambda: {}[[]]))):
        table.Cell((0, 0), 1, 1, visible_edges=['B', 'T'])
    fig, ax = plt.subplots()
    tab = table.table(ax, [['a']])
    with pytest.raises(TypeError,
                       match="can't multiply sequence by non-int of type 'float'"):
        tab.scale('2', 1)
    assert table.Cell.get_x.__qualname__ == "Rectangle.get_x"
    assert table.Cell.set_width.__qualname__ == "Rectangle.set_width"
    assert Figure.remove.__qualname__ == "Artist.remove"


# -- pyplot's functions bind their arguments themselves ------------------------

def test_pyplot_functions_refuse_arguments_by_their_own_name():
    plt.figure()
    with pytest.raises(TypeError,
                       match="^margins\\(\\) got an unexpected keyword argument 'x1'"):
        plt.margins(x1=1)
    with pytest.raises(TypeError, match=r"^text\(\) missing 3 required positional "
                       "arguments: 'x', 'y', and 's'$"):
        plt.text()
    with pytest.raises(TypeError, match=r"^grid\(\) takes from 0 to 3 positional "
                       "arguments but 4 were given$"):
        plt.grid(1, 2, 3, 4)
    with pytest.warns(pltrs.MatplotlibDeprecationWarning,
                      match="Passing the range parameter of hist\\(\\) positionally"):
        plt.hist([1, 2, 3], 3, (0, 3))
    with pytest.warns(UserWarning, match="ignoring tight=None in get mode") as record:
        plt.margins(tight=None)
    assert record[0].filename == __file__


# -- FreeType fonts refuse arguments as pybind11 does --------------------------

def test_font_methods_refuse_arguments_as_pybind11_does():
    font = _font()
    module = type(font).__module__
    with pytest.raises(TypeError) as info:
        font.set_size('10', 72)
    assert str(info.value) == (
        "set_size(): incompatible function arguments. The following argument "
        "types are supported:\n"
        f"    1. (self: {module}.FT2Font, ptsize: typing.SupportsFloat | "
        "typing.SupportsIndex, dpi: typing.SupportsFloat | typing.SupportsIndex)"
        f" -> None\n\nInvoked with: {font!r}, '10', 72")
    with pytest.raises(TypeError, match=r"; kwargs: nonexistent_kw=1$"):
        font.get_num_glyphs(nonexistent_kw=1)
    with pytest.raises(TypeError, match=r"^load_char\(\): incompatible"):
        font.load_char(65, 2.0)
    with pytest.raises(TypeError, match=r"^draw_glyphs_to_bitmap\(\): incompatible"):
        font.draw_glyphs_to_bitmap(antialiased=[])
    font.set_charmap(True)
    font.draw_glyphs_to_bitmap(antialiased=None)
    assert font.get_name_index(None) == 0


# -- the Agg renderer's arguments ----------------------------------------------

def _agg_renderer():
    fig = Figure(figsize=(3, 2), dpi=50)
    canvas = backend_agg.FigureCanvasAgg(fig)
    canvas.draw()
    renderer = canvas.get_renderer()
    return canvas, renderer, renderer.new_gc()


def test_agg_renderer_converts_its_arguments_as_its_extension():
    canvas, renderer, gc = _agg_renderer()
    square = Path([(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)], closed=True)
    with pytest.raises(RuntimeError, match=r"^Unable to cast Python instance of "
                       r"type <class 'str'> to C\+\+ type '\?'"):
        renderer.draw_path(gc, square, None, 'ab')
    with pytest.raises(IndexError, match="tuple index out of range"):
        renderer.draw_path(gc, square, None, (1.0,))
    with pytest.raises(ValueError, match="RGBA value must be 3- or 4-tuple"):
        renderer.draw_path(gc, square, None, (1, 0, 0, 0, 0))
    renderer.draw_path(gc, square, None, [1.0, 0, 0])
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute '_linewidth'"):
        renderer.draw_path(None, square, None)
    with pytest.raises(ValueError, match="Invalid bounding box"):
        canvas.copy_from_bbox(3)
    with pytest.raises(TypeError, match=r"^restore_region\(\): incompatible"):
        canvas.restore_region(None)
    with pytest.raises(IndexError, match="pop from empty list"):
        renderer.stop_filter(None)
    with pytest.raises(ValueError, match="dpi must be positive"):
        backend_agg.RendererAgg(10, 10, -1)


def test_agg_renderer_draws_text_glyph_by_glyph():
    _, renderer, gc = _agg_renderer()
    prop = font_manager.FontProperties(family='DejaVu Sans', size=12)
    with pytest.raises(TypeError, match=r'can only concatenate str \(not "float"\) to str'):
        renderer.draw_text(gc, '10', 10, 'ab', prop, 0)
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute 'get_antialiased'"):
        renderer.draw_text(None, 10, 10, 'ab', prop, 0)
    with pytest.raises(AttributeError,
                       match="'int' object has no attribute 'get_fontfeatures'"):
        renderer.draw_text(gc, 10, 10, 'ab', prop, 0, mtext=0)
    with pytest.raises(TypeError, match=r"^_layout\(\): incompatible"):
        renderer.draw_text(gc, 10, 10, None, prop, 0)
    with pytest.raises(ValueError, match=build_facts.raised(lambda: math.cos(math.inf))):
        renderer.draw_mathtext(gc, 10, 10, '$x^2$', prop, float('inf'))
    with pytest.warns(RuntimeWarning, match="invalid value encountered in cast"), \
            pytest.raises(ValueError, match="cannot convert float NaN to integer"):
        renderer.draw_mathtext(gc, 10, 10, '$x^2$', prop, float('nan'))
    with pytest.raises(AttributeError, match="'str' object has no attribute 'copy'"):
        renderer.draw_mathtext(gc, 10, 10, '$x^2$', 'x', 0)
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute 'expandtabs'"):
        mathtext.MathTextParser('path').parse(None)
    renderer.draw_text(gc, 10, 10, 'ab', prop, 0, ismath='False')  # mathtext


# -- canvases' prints ----------------------------------------------------------

def test_raster_prints_check_their_metadata_and_keywords():
    fig = Figure()
    canvas = backend_agg.FigureCanvasAgg(fig)
    with pytest.raises(TypeError, match="'list' object is not a mapping"):
        canvas.print_png(io.BytesIO(), metadata=[])
    with pytest.raises(AttributeError, match="'int' object has no attribute 'encode'"):
        canvas.print_png(io.BytesIO(), metadata={'Title': 1})
    with pytest.raises(AttributeError, match="'int' object has no attribute 'copy'"):
        canvas.print_png(io.BytesIO(), pil_kwargs=0)
    with pytest.raises(ValueError, match="metadata not supported for format 'jpeg'"):
        canvas.print_jpg(io.BytesIO(), metadata={})
    with pytest.raises(AttributeError,
                       match="'list' object has no attribute 'setdefault'"):
        canvas.print_tiff(io.BytesIO(), pil_kwargs=[])


def test_print_figure_sets_its_colours_and_box_as_matplotlib_does():
    fig = plt.figure()
    with pytest.raises(ValueError, match="Invalid RGBA argument: ''"):
        fig.canvas.print_figure(io.BytesIO(), facecolor='')
    with pytest.raises(ValueError, match="Invalid RGBA argument: 3"):
        fig.canvas.print_figure(io.BytesIO(), edgecolor=3)
    with pytest.raises(AttributeError, match="'str' object has no attribute 'size'"):
        fig.canvas.print_figure(io.BytesIO(), bbox_inches='x')
    fig.canvas.print_figure(io.BytesIO(), bbox_inches=[])
    with pytest.raises(ValueError, match="dpi must be positive"):
        fig.canvas.print_figure(io.BytesIO(), dpi=-1)


def test_canvases_and_managers_take_their_figure_and_number_as_matplotlib():
    with pytest.raises(AttributeError, match="'int' object has no attribute 'set_canvas'"):
        FigureCanvasBase(3)
    with pytest.raises(ValueError,
                       match="Unknown format code 'd' for object of type 'float'"):
        FigureCanvasBase.new_manager(Figure(), 9.5)


# -- the SVG writer and renderer, the PDF file ---------------------------------

def test_svg_xml_writer_computes_as_matplotlib_does():
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", pltrs.MatplotlibDeprecationWarning)
        writer = backend_svg.XMLWriter(io.StringIO())
    with pytest.raises(TypeError, match=build_facts.raised(lambda: "a" in 1)):
        writer.start('g', attrib={'a': 1})
    writer.data('a')
    writer.data(None)
    with pytest.raises(TypeError, match="sequence item 1: expected str instance, "
                       "NoneType found"):
        writer.start('g')
    with pytest.raises(TypeError, match="sequence item 1"):
        writer.end()
    with pytest.raises(TypeError, match="'>' not supported between instances of "
                       "'int' and 'str'"):
        writer.close('0')
    with pytest.raises(TypeError, match="sequence item 1"):
        writer.comment(None)  # the data kept is written first
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", pltrs.MatplotlibDeprecationWarning)
        writer = backend_svg.XMLWriter(io.StringIO())
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'replace'"):
        writer.comment(None)


def test_svg_renderer_writes_its_size_and_groups_as_matplotlib():
    with pytest.raises(ValueError, match="Unknown format code 'f' for object of type 'str'"):
        backend_svg.RendererSVG('4', 3, io.StringIO())
    with pytest.raises(AttributeError, match="'int' object has no attribute 'write'"):
        backend_svg.RendererSVG(4, 3, 3)
    renderer = backend_svg.RendererSVG(4, 3, io.StringIO())
    with pytest.raises(TypeError, match=build_facts.raised(lambda: "a" in 3)):
        renderer.open_group('g', gid=3)
    square = Path([(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)], closed=True)
    renderer.draw_markers(renderer.new_gc(), None, Affine2D(), square, Affine2D())


def test_pdf_file_methods_take_their_arguments_as_matplotlib():
    fig = Figure()
    canvas = backend_pdf.FigureCanvasPdf(fig)
    pdf = backend_pdf.PdfFile(io.BytesIO())
    renderer = backend_pdf.RendererPdf(pdf, 72, 4, 3)
    with pytest.raises(TypeError, match="a bytes-like object is required, not 'NoneType'"):
        pdf.write(None)
    with pytest.raises(TypeError, match=re.escape(build_facts.raised(lambda: {}[[]]))):
        pdf.alphaState([0.5, 0.5])
    with pytest.raises(IndexError, match="tuple index out of range"):
        pdf.alphaState((0.5,))
    assert pdf.alphaState((0.5, 0.5)) is pdf.alphaState((0.5, 0.5))
    prop = font_manager.FontProperties(family='DejaVu Sans')
    assert pdf.fontName(prop) is pdf.fontName(prop)
    with pytest.raises(TypeError, match=r"unsupported operand type\(s\) for \*: "
                       "'int' and 'NoneType'"):
        pdf.newPage(None, 3)
    assert renderer.encode_string('ab', 3) == b'ab'
    with pytest.raises(KeyError):
        renderer.encode_string('ab', 0)
    assert canvas.figure is fig


# -- text as paths -------------------------------------------------------------

def test_text_as_paths_lays_out_as_matplotlib():
    prop = font_manager.FontProperties(family='DejaVu Sans')
    with pytest.raises(TypeError, match=r"^_layout\(\): incompatible"):
        textpath.TextToPath().get_text_path(prop, None)
    with pytest.raises(TypeError, match=r"^_layout\(\): incompatible"):
        textpath.TextToPath().get_text_path(prop, 'Ab', language=0)
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'replace'"):
        textpath.TextPath((0, 0), None)
    assert isinstance(_font(), FT2Font)
    assert np.isfinite(Bbox.unit().width)


# -- module-level arithmetic: bezier, mlab -------------------------------------

def test_bezier_arithmetic_of_arguments_that_are_no_plain_numbers():
    from pltrs import bezier
    with pytest.raises(TypeError, match=re.escape(
            "unsupported operand type(s) for -: 'int' and 'str'")):
        bezier.get_cos_sin('0', 0, 1, 1)
    x, y = bezier.get_intersection(np.int64(0), 0, 1, 0, 1, 1, 0, 1)
    assert type(x) is np.float64 and (x, y) == (1.0, 0.0)
    with pytest.raises(TypeError, match="ufunc 'arctan2' not supported"):
        bezier.check_if_parallel('1', 0, 2, 0)
    with pytest.raises(IndexError, match="list index out of range"):
        bezier.get_parallels([], 0.5)
    with pytest.raises(TypeError, match=re.escape(
            "can't multiply sequence by non-int of type 'float'")):
        bezier.get_parallels([(0, 0), (1, 1), (2, 0)], '0.5')
    with pytest.raises(TypeError, match=re.escape(
            "unsupported operand type(s) for -: 'int' and 'str'")):
        bezier.split_de_casteljau(np.array([[0.0, 0.0], [1.0, 2.0]]), '0.5')
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute 'iter_segments'"):
        bezier.split_path_inout(None, lambda xy: True)
    with pytest.raises(TypeError, match="'<' not supported between instances of "
                       "'float' and 'NoneType'"):
        bezier.split_path_inout(Path([(-0.5, 0), (2, 0)]),
                                lambda xy: xy[0] ** 2 + xy[1] ** 2 < 1,
                                tolerance=None)
    with pytest.raises(ValueError, match=re.escape(
            "not enough values to unpack (expected 2, got 0)")):
        bezier.BezierSegment('data')


def test_detrending_what_is_no_plain_array_as_numpy_does():
    from pltrs import mlab
    with pytest.raises(TypeError, match=re.escape(
            'can only concatenate str (not "int") to str')):
        mlab.detrend(np.arange(5.0), axis='both')
    with pytest.raises(TypeError, match=re.escape(
            "unsupported operand type(s) for /: 'NoneType' and 'int'")):
        mlab.detrend_mean(None)
    with pytest.warns(RuntimeWarning, match="Mean of empty slice"):
        mlab.detrend_mean(np.empty(0))
    assert mlab.detrend_mean(np.arange(4, dtype=np.float32)).dtype == np.float32
    assert mlab.detrend(np.arange(4.0), 'linear').tolist() == [0.0, 0.0, 0.0, 0.0]
    np.testing.assert_allclose(mlab.detrend(np.arange(6.0).reshape(2, 3), 'mean', axis=1),
                               [[-1.0, 0.0, 1.0], [-1.0, 0.0, 1.0]])
