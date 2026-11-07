"""
What the differential tester's hostile programs of figures, texts,
annotations, tables and the vector renderers
(``spec/diff/generators/hostile.py``) found pltrs doing otherwise than
matplotlib 3.11: values matplotlib keeps unchecked and fails on where it
draws or measures, the attributes it reads of wrong arguments first, and
the renderers' checks of their arguments. Each expected value and message
is recorded from matplotlib.
"""

import io

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import table, text, transforms
from pltrs.backends import backend_agg, backend_pdf, backend_ps, backend_svg
from pltrs.backend_bases import GraphicsContextBase
from pltrs.offsetbox import AnnotationBbox, TextArea
from pltrs.path import Path


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


# -- artists' flags and texts as given -----------------------------------------

def test_flags_read_back_as_given():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1])
    line.set_visible('')
    line.set_clip_on(1)
    line.set_snap('False')
    line.set_in_layout(np.True_)
    line.set_gid(3)
    line.set_url(3)
    assert line.get_visible() == '' and line.get_clip_on() == 1
    assert line.get_snap() == 'False' and type(line.get_in_layout()) is np.bool
    assert line.get_gid() == 3 and line.get_url() == 3
    line.set_visible(True)
    assert line.get_visible() is True


def test_visibility_kept_only_when_it_changes():
    fig = plt.figure()
    fig.set_frameon(1)
    assert fig.get_frameon() is True
    fig.set_frameon(None)
    assert fig.get_frameon() is None
    fig.set_snap(None)
    assert fig.get_snap() is None


def test_sketch_params_no_number_is_kept():
    fig = plt.figure()
    fig.set_sketch_params('asinh', 100, 2)
    assert fig.get_sketch_params() == ('asinh', 100, 2)
    fig.canvas.draw()


def test_mouseover_of_a_figure_with_axes():
    fig = plt.figure()
    fig.set_mouseover(True)
    fig.add_subplot()
    with pytest.raises(AttributeError,
                       match="^'list' object has no attribute '_mouseover_set'$"):
        fig.set_mouseover(False)
    assert fig.get_mouseover() is False


def test_figure_size_of_none():
    fig = plt.figure()
    with pytest.raises(ValueError, match="^Figure.set_size_inches does not accept "
                                         "None; provide both width and height "
                                         "explicitly$"):
        fig.set_figwidth(None)


def test_figure_resolution_no_number_is_kept():
    fig = plt.figure()
    with pytest.raises(TypeError, match="^can't multiply sequence by non-int of "
                                        "type 'numpy.float64'$"):
        fig.dpi = '72'
    assert fig.dpi == '72'
    with pytest.raises(TypeError) as raised:
        fig.canvas.draw()
    assert str(raised.value).endswith("\n\nInvoked with: 6, 4, '72'")
    fig.dpi = 72
    fig.canvas.draw()
    assert fig.canvas.get_width_height() == (460, 345)


def test_agg_renderer_refuses_what_its_extension_refuses():
    with pytest.raises(TypeError, match="Invoked with: -1, 30, 72$"):
        backend_agg.RendererAgg(-1, 30, 72)
    assert backend_agg.RendererAgg(40.5, 30, 72).width == 40.5


def test_relative_constrained_layout_pads_are_floats():
    fig = plt.figure(layout="constrained")
    pads = fig.get_constrained_layout_pads(relative=True)
    assert [type(pad) for pad in pads] == [float] * 4
    assert pads[:2] == (0.006510937499999999, 0.00868125)


def test_figure_reads_what_matplotlib_reads_first():
    fig = pltrs.figure.Figure()
    with pytest.raises(AttributeError,
                       match="^'NoneType' object has no attribute 'set_figure'$"):
        fig.add_artist(None)
    fig.canvas = 'x'
    with pytest.raises(AttributeError,
                       match="^'str' object has no attribute 'print_figure'$"):
        fig.savefig(io.BytesIO())


# -- text positions --------------------------------------------------------------

def test_text_position_kept_unchecked():
    fig = plt.figure()
    t = fig.text(None, 0.25, 'text')
    assert t.get_position() == (None, 0.25)
    with pytest.raises(TypeError, match="^float\\(\\) argument must be a string "
                                        "or a real number, not 'NoneType'$"):
        fig.canvas.draw()
    t.set_x('0.5')
    fig.canvas.draw()
    assert t.get_position() == ('0.5', 0.25)
    t.set_x('x')
    with pytest.raises(ValueError, match="^could not convert string to float: 'x'$"):
        t.get_window_extent()
    t.set_text('')
    fig.canvas.draw()


def test_text_position_in_units_of_its_axes():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    t = ax.text(0.5, 0.5, 'hello')
    t.set_position('ab')
    assert t.get_position() == ('a', 'b')
    with pytest.raises(pltrs.units.ConversionError,
                       match="^Failed to convert value\\(s\\) to axis units: 'a'$"):
        fig.canvas.draw()
    t.set_x(None)
    with pytest.raises(pltrs.units.ConversionError,
                       match="^Failed to convert value\\(s\\) to axis units: None$"):
        fig.canvas.draw()


def test_category_converter_set_without_categories_refuses_names():
    fig, ax = plt.subplots()
    for _ in range(2):
        with pytest.raises(pltrs.units.ConversionError,
                           match="^Failed to convert value\\(s\\) to axis units: '0.5'$"):
            ax.xaxis.convert_units('0.5')


def test_text_draw_refuses_what_is_no_renderer():
    fig, ax = plt.subplots()
    t = ax.text(0.5, 0.5, 'x')
    with pytest.raises(AttributeError,
                       match="^'NoneType' object has no attribute '_rasterizing'$"):
        t.draw(None)
    t.set_rasterized(True)
    with pytest.raises(AttributeError,
                       match="^'int' object has no attribute '_raster_depth'$"):
        t.draw(3)
    with pytest.raises(TypeError, match="^Text.draw\\(\\) takes 2 positional "
                                        "arguments but 3 were given$"):
        t.draw(None, None)
    with pytest.raises(TypeError, match="^cannot create weak reference to 'int' object$"):
        t.get_window_extent(renderer=3)
    assert pltrs.artist.Artist.draw(fig, None) is None
    with pytest.raises(AttributeError,
                       match="^'NoneType' object has no attribute '_raster_depth'$"):
        pltrs.figure.FigureBase.draw(fig, None, None)


# -- annotations -----------------------------------------------------------------

def test_annotation_coordinates_kept_unchecked():
    fig, ax = plt.subplots()
    note = ax.annotate('a', (1, 0.5), xytext=(2, 0.2), arrowprops={})
    note.xycoords = 3
    with pytest.raises(TypeError, match="^'xycoords' must be an instance of str, "
                                        "tuple\\[str, str\\], Artist, Transform, or "
                                        "Callable, not a int$"):
        fig.canvas.draw()
    note.xycoords = 'data'
    note.set_anncoords('axes fractions')
    with pytest.raises(ValueError, match="^'fractions' is not a recognized unit$"):
        note.get_window_extent()
    note.set_anncoords('data')
    note.xy = ()
    with pytest.raises(ValueError, match="^not enough values to unpack "
                                         "\\(expected 2, got 0\\)$"):
        fig.canvas.draw()
    note.xy = (1, 2)
    assert note.xy == (1, 2)
    fig.canvas.draw()


def test_annotation_made_with_what_matplotlib_keeps():
    assert text.Annotation('a', (0, 0), xycoords='').xycoords == ''
    with pytest.raises(ValueError, match="^not enough values to unpack "
                                         "\\(expected 2, got 0\\)$"):
        text.Annotation('a', (0, 0), xytext=())
    with pytest.warns(UserWarning, match="^You have used the `textcoords` kwarg, "
                                         "but not the `xytext` kwarg."):
        text.Annotation('a', (0, 0), textcoords=0)
    fig, ax = plt.subplots()
    ax.annotate('c', (0.5, 0.5), xycoords='')
    with pytest.raises(ValueError, match="^'' is not a valid coordinate$"):
        fig.canvas.draw()


def test_annotation_box_position_kept_unchecked():
    fig, ax = plt.subplots()
    box = AnnotationBbox(TextArea('ab'), (0.5, 0.5), xybox=(0.8, 0.8))
    ax.add_artist(box)
    box.xyann = None
    assert box.xybox is None
    with pytest.raises(TypeError, match="^cannot unpack non-iterable NoneType object$"):
        fig.canvas.draw()
    box.xyann = (0.8, 0.8)
    fig.canvas.draw()
    with pytest.raises(TypeError, match="^cannot create weak reference to 'int' object$"):
        box.get_window_extent(renderer=3)


# -- tables ----------------------------------------------------------------------

def test_cell_checks_as_its_text_and_rectangle():
    with pytest.raises(ValueError, match="^'best' is not a valid value for align. "
                                         "Supported values are 'center', 'right', "
                                         "'left'$"):
        table.Cell((0, 0), 1, 1, loc='best')
    with pytest.raises(TypeError, match="^'NoneType' object is not subscriptable$"):
        table.Cell(None, 1, 1)
    assert table.Cell((0, 0), 1, 1, text=None).get_text().get_text() == ''


# -- the vector renderers --------------------------------------------------------

def _collection(renderer, gc, offsets):
    """Draw a square at each of *offsets* through *renderer*."""
    renderer.draw_path_collection(
        gc, transforms.IdentityTransform(), [Path.unit_rectangle()], [], offsets,
        transforms.IdentityTransform(), [(1, 0, 0, 1)], [(0, 0, 0, 1)], [1.0],
        [(0, None)], [True], [None], 'screen')


def _renderers():
    """A renderer of each vector format, writing to a file of its own."""
    pdf = backend_pdf.PdfFile(io.BytesIO())
    pdf.newPage(4, 3)
    return [backend_pdf.RendererPdf(pdf, 72, 3, 4),
            backend_ps.RendererPS(4, 3, io.StringIO()),
            backend_svg.RendererSVG(4, 3, io.StringIO())]


@pytest.mark.parametrize("index", range(3))
def test_small_collections_drawn_path_by_path(index):
    renderer = _renderers()[index]
    gc = renderer.new_gc()
    _collection(renderer, gc, [[0, 0]])
    with pytest.raises(AttributeError,
                       match="^'IdentityTransform' object has no attribute 'translate'$"):
        _collection(renderer, gc, [[0, 0], [10, 10]])


def test_renderers_read_the_graphics_context_as_matplotlib():
    pdf, ps, svg = _renderers()
    square = Path.unit_rectangle()
    affine = transforms.Affine2D()
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute "
                                             "'_fillcolor' and no __dict__ for setting "
                                             "new attributes$"):
        pdf.draw_path(None, square, affine)
    for renderer in (ps, svg):
        with pytest.raises(AttributeError, match="^'NoneType' object has no "
                                                 "attribute 'get_hatch_path'$"):
            renderer.draw_path(None, square, affine)
    gc = GraphicsContextBase()
    with pytest.raises(ValueError, match="^Invalid affine transformation matrix$"):
        pdf.draw_path(gc, square, 3)
    pdf.draw_path(gc, square, None)
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for \\+: "
                                        "'int' and 'Affine2D'$"):
        svg.draw_path(gc, square, 3)
    with pytest.raises(TypeError, match="^RendererPS.set_color\\(\\) missing 3 "
                                        "required positional arguments: 'r', 'g', "
                                        "and 'b'$"):
        ps.draw_path(gc, square, affine, rgbFace=())


def test_renderers_format_their_arguments_as_matplotlib():
    pdf, ps, svg = _renderers()
    gc = GraphicsContextBase()
    prop = pltrs.font_manager.FontProperties(size=12)
    with pytest.raises(ValueError, match="^Unknown format code 'f' for object of "
                                         "type 'str'$"):
        ps.set_color('1', 0, 0)
    with pytest.raises(KeyError, match="^''$"):
        ps.set_linejoin('')
    with pytest.raises(ValueError, match="^Can only output finite numbers in PDF$"):
        pdf.draw_text(gc, float('nan'), 10, 'ab', prop, 0)
    with pytest.raises(TypeError, match="^bad operand type for unary -: 'str'$"):
        svg.draw_text(gc, 10, 10, 'ab', prop, '0')
    pdf.draw_text(gc, 10, 10, 'ab', prop, 0, ismath='False')
