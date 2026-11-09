"""
What the differential tester's hostile programs of the module-level
functions, figures, renderers, writers and tables
(``spec/diff/generators/hostile.py``) found pltrs doing otherwise than
matplotlib 3.11 in their fourth round: values stored only when they
change or as given (an alpha, a patch's antialiasing, an annotation's
clipping), the resources of a PDF document kept unchecked until written,
constructors refused as pybind11 refuses them, memoised directories, the
statistics and dates of unusual data computed with matplotlib's
arithmetic, layouts of figures of no area or placed nowhere, and the order
in which texts, annotations and log scales compute with their arguments.
Each expected value and message is recorded from matplotlib.
"""

import datetime
import io
import math
import re
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import cbook, dates, font_manager, ft2font, path, text, transforms
from pltrs.backends import backend_pdf
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.figure import Figure


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


NAN_TO_INT = "cannot convert float NaN to integer"


# -- values stored as given, or only when they change --------------------------

def test_alpha_is_stored_only_when_it_changes():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1])
    line.set_alpha(0.5)
    line.set_alpha(np.float32(0.5))
    assert type(line.get_alpha()) is float


def test_patch_antialiasing_and_annotation_clip_read_back_as_given():
    fig, ax = plt.subplots()
    patch = ax.add_patch(pltrs.patches.Rectangle((0, 0), 1, 1))
    for value in (1, 'False', np.True_):
        patch.set_antialiased(value)
        assert patch.get_antialiased() is value
    note = ax.annotate('a', (0.5, 0.5))
    for value in (1, 'False', np.True_, None):
        note.set_annotation_clip(value)
        assert note.get_annotation_clip() is value


def test_a_figures_path_effects_are_kept_not_drawn():
    fig = plt.figure()
    fig.add_subplot().plot([0, 1])
    fig.set_path_effects('x')
    assert fig.get_path_effects() == 'x'
    fig.canvas.draw()


def test_a_cell_transform_set_to_none_reads_as_the_identity():
    fig, ax = plt.subplots()
    cell = ax.table([['a', 'b'], ['c', 'd']])[0, 1]
    cell.set_transform(None)
    matrix = cell.get_transform().get_matrix()
    assert matrix[0, 0] == pytest.approx(cell.get_width())
    assert matrix[0, 2] == pytest.approx(cell.get_x())


# -- the resources of a PDF document, kept until written -----------------------

def _pdf_file():
    file = backend_pdf.PdfFile(io.BytesIO())
    file.newPage(4, 3)
    return file


def test_pdf_markers_and_templates_fail_when_written():
    square = path.Path([(0, 0), (1, 0), (1, 1)])
    for args, error, message in [
            ((None, 'miter', 'butt'), TypeError, "unsupported operand type"),
            (('1.0', 'miter', 'butt'), TypeError, "bad operand type for unary -: 'str'"),
            ((1.0, 'MITER', 'butt'), KeyError, "'MITER'"),
            ((1.0, 'miter', None), KeyError, "None")]:
        file = backend_pdf.PdfFile(io.BytesIO())
        assert file.markerObject(square, None, True, True, *args).name == b'M0'
        with pytest.raises(error, match=re.escape(message)):
            file.writeMarkers()
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'get_extents'"):
        backend_pdf.PdfFile(io.BytesIO()).markerObject(
            None, transforms.Affine2D(), True, True, 1.0, 'miter', 'butt')
    gc = FigureCanvasAgg(Figure()).get_renderer().new_gc()
    file = backend_pdf.PdfFile(io.BytesIO())
    assert file.pathCollectionObject(gc, None, None, 1.0, True, True).name == b'P0'
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'get_extents'"):
        file.writePathCollectionTemplates()
    file = backend_pdf.PdfFile(io.BytesIO())
    file.pathCollectionObject(gc, square, 'x', 1.0, True, True)
    with pytest.raises(ValueError, match="Invalid affine transformation matrix"):
        file.writePathCollectionTemplates()


def test_pdf_shadings_and_images_fail_when_written():
    colors = np.array([[[1, 0, 0, 1], [0, 1, 0, 1], [0, 0, 1, 1]]], float)
    file = backend_pdf.PdfFile(io.BytesIO())
    name, _ = file.addGouraudTriangles(None, colors)
    assert name.name == b'GT0'
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'shape'"):
        file.writeGouraudTriangles()
    file = backend_pdf.PdfFile(io.BytesIO())
    file.addGouraudTriangles(np.empty((0, 3, 2)), colors)
    with pytest.raises(ValueError, match=re.escape(
            "cannot reshape array of size 12 into shape (0,4)")):
        file.writeGouraudTriangles()
    file = backend_pdf.PdfFile(io.BytesIO())
    file.addGouraudTriangles(np.array([[[np.nan, 0], [10, 0], [0, 10]]]), colors)
    with pytest.raises(ValueError, match="Can only output finite numbers in PDF"):
        file.writeGouraudTriangles()
    file = backend_pdf.PdfFile(io.BytesIO())
    assert file.imageObject(None).name == b'I1'
    with pytest.raises(TypeError, match="'NoneType' object is not subscriptable"):
        file.writeImages()
    file = backend_pdf.PdfFile(io.BytesIO())
    file.imageObject(np.arange(48.).reshape(3, 4, 4))
    with pytest.raises(OSError, match="cannot write mode F as PNG"):
        file.writeImages()


def test_pdf_fonts_are_named_whatever_they_are_and_fail_when_written():
    prop = font_manager.FontProperties(family='DejaVu Sans', size=12)
    file = _pdf_file()
    assert file.fontName('x').name == b'F1'
    with pytest.raises(KeyError):
        file.finalize()
    file = _pdf_file()
    assert [file.fontName(prop, subset=float('nan')).name for _ in range(2)] == [
        b'F1', b'F2']
    file = _pdf_file()
    file.fontName(prop, subset='0')
    file.fontName(prop, subset=0)
    with pytest.raises(TypeError, match="'<' not supported between instances"):
        file.finalize()


def test_pdf_embedding_reads_the_font_and_characters_first():
    font = font_manager.findfont('DejaVu Sans')
    for charmap, error, message in [
            ({}, ValueError, "min() iterable argument is empty"),
            ({'nonexistent_key': 1}, TypeError, 'can only concatenate str (not "int") to str'),
            ([], AttributeError, "'list' object has no attribute 'values'"),
            ({65: 36}, AssertionError, None)]:
        with pytest.raises(error, match=None if message is None else re.escape(message)):
            _pdf_file().embedTTF(font, 0, charmap)
    with pytest.raises(TypeError, match="'NoneType' object is not iterable"):
        _pdf_file().embedTTF(None, 0, {65: 36})


def test_pdf_states_and_hatches_are_keyed_by_the_values_given():
    gc = backend_pdf.GraphicsContextPdf(_pdf_file())
    first = gc.alpha_cmd(0.5, False, (0.5, 0.5))[0]
    assert gc.alpha_cmd(0.5, False, (0.5, 0.5, 0.5))[0] is not first
    assert gc.alpha_cmd(0.5, False, (0.5, 0.5))[0] is first
    assert gc.hatch_cmd('//', (0, 0, 0, 1), 1.0)[2] is not gc.hatch_cmd(
        '//', (0, 0, 0, 1), '1.0')[2]
    for hatch, color in [(3, (0, 0, 0, 1)), ('//', ()), ('//', 'ab')]:
        gc.hatch_cmd(hatch, color, 1.0)


def test_pdf_graphics_state_numbers_must_be_finite():
    renderer = backend_pdf.RendererPdf(_pdf_file(), 72, 3, 4)
    gc = renderer.new_gc()
    gc.set_linewidth(float('nan'))
    square = path.Path([(0, 0), (1, 0), (1, 1)])
    with pytest.raises(ValueError, match="Can only output finite numbers in PDF"):
        renderer.draw_path_collection(
            gc, transforms.IdentityTransform(), [square], [], [[0, 0], [10, 10]],
            transforms.IdentityTransform(), [(1, 0, 0, 1)], [(0, 0, 0, 1)],
            [float('nan')], [(0, None)], [True], [None], 'screen')


# -- constructors and functions as matplotlib's take their arguments -----------

def test_ft2font_constructors_refuse_as_pybind11():
    font = font_manager.findfont('DejaVu Sans')
    with pytest.raises(TypeError, match=re.escape(
            "__init__(): incompatible constructor arguments. The following argument "
            "types are supported:\n    1. ")) as info:
        ft2font.FT2Font(font, nonexistent_kw=1)
    assert info.value.args[0].endswith("; kwargs: nonexistent_kw=1")
    assert ".FT2Font(filename: object, hinting_factor: " in info.value.args[0]
    with pytest.raises(TypeError, match=r"Invoked with: None$"):
        ft2font.FT2Image(None)


def test_directories_are_remembered_and_take_keywords_once_known():
    assert pltrs.get_configdir(nonexistent_kw=1) == pltrs.get_configdir()
    assert pltrs.get_data_path(nonexistent_kw=1) == pltrs.get_data_path()
    assert pltrs.get_cachedir(nonexistent_kw=1) == pltrs.get_cachedir()


def test_tiff_targets_are_asked_where_they_stand():
    canvas = FigureCanvasAgg(Figure(figsize=(1, 1), dpi=10))
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'tell'"):
        canvas.print_tif(None)


def test_ttf_font_property_reads_the_family_name_first():
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'family_name'"):
        font_manager.ttfFontProperty(None)


def test_strip_math_of_what_is_no_string():
    with pytest.raises(TypeError, match="object of type 'NoneType' has no len()"):
        cbook.strip_math(None)
    assert cbook.strip_math(['a']) == ['a']


def test_boxplot_stats_of_unusual_data_as_matplotlib():
    stats, = cbook.boxplot_stats(np.array([3, 1, 4, 2, 5, 9, 2, 40]))
    assert stats['fliers'].dtype == np.int64
    values = np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])
    for args, kwargs, error, message in [
            ((np.array([False, True]),), {}, TypeError, "numpy boolean subtract"),
            ((values,), dict(whis=None), TypeError,
             "unsupported operand type(s) for *: 'NoneType' and 'float'"),
            ((values,), dict(bootstrap=-1), ValueError, "negative dimensions are not allowed"),
            ((values,), dict(bootstrap=0), IndexError, "index -1 is out of bounds")]:
        with pytest.raises(error, match=re.escape(message)):
            cbook.boxplot_stats(*args, **kwargs)
    stats, = cbook.boxplot_stats([], whis=None)
    assert math.isnan(stats['mean'])


def test_date_rules_are_checked_by_dateutil_as_made():
    for make, error, message in [
            (lambda: dates.YearLocator(month=float('nan')), TypeError,
             "'float' object is not iterable"),
            (lambda: dates.WeekdayLocator(byweekday='1'), AttributeError,
             "'str' object has no attribute 'n'"),
            (lambda: dates.HourLocator(byhour=[]), ValueError,
             "Invalid rrule byxxx generates an empty set."),
            (lambda: dates.rrulewrapper(dates.DAILY, nonexistent_kw=1), TypeError,
             "rrule.__init__() got an unexpected keyword argument 'nonexistent_kw'")]:
        with pytest.raises(error, match=re.escape(message)):
            make()


def test_day_numbers_of_unusual_types_convert_as_matplotlib():
    assert dates.num2timedelta(1.5) == datetime.timedelta(days=1, seconds=43200)
    for value, error, message in [
            (float('nan'), ValueError, NAN_TO_INT),
            (float('inf'), OverflowError, "cannot convert float infinity to integer"),
            (np.int64(1), TypeError, "unsupported type for timedelta days component: numpy.int64"),
            ('1.5', TypeError, "unsupported type for timedelta days component: str")]:
        with pytest.raises(error, match=re.escape(message)):
            dates.num2timedelta(value)
    with pytest.raises(TypeError, match="can't multiply sequence by non-int of type 'float'"):
        dates.num2date('19800.5')
    with pytest.raises(TypeError, match="unsupported operand type"):
        dates.num2date(None)


def test_set_epoch_refuses_once_dates_were_converted():
    dates.date2num(datetime.datetime(2024, 1, 1))
    with pytest.raises(RuntimeError, match="set_epoch must be called before dates plotted."):
        dates.set_epoch('')


# -- texts and annotations -------------------------------------------------------

def test_text_window_extent_sets_the_figure_resolution():
    fig, ax = plt.subplots()
    label = ax.text(0.5, 0.5, 'hi')
    with pytest.raises(TypeError, match="can't multiply sequence by non-int"):
        label.get_window_extent(dpi='72')
    assert fig.dpi == 100


def test_annotations_keep_what_they_cannot_place_until_drawn():
    note = text.Annotation('text', 'ab')
    assert note.xy == ('a', 'b')
    with pytest.raises(AttributeError, match="'int' object has no attribute 'copy'"):
        text.Annotation('text', (0.2, 0.3), arrowprops=0)
    with pytest.raises(TypeError, match="pop expected at most 1 argument, got 2"):
        text.Annotation('text', (0.2, 0.3), arrowprops=[])
    text.Text(multialignment='')


# -- layouts and scales ------------------------------------------------------------

def test_log_scale_stays_set_and_autoscaled_after_a_failing_plot():
    fig, ax = plt.subplots()
    ax.imshow(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)))
    with pytest.raises(AttributeError, match="unexpected keyword argument"):
        ax.semilogx([1, 2, 3], [1, 2, 3], nonexistent_kw=1)
    assert ax.get_xscale() == 'log'
    assert ax.get_xlim()[0] == pytest.approx(0.8912509381337455)


def test_constrained_layout_of_a_figure_of_no_area_warns_as_numpy():
    fig = plt.figure(layout='constrained')
    fig.subplots(2, 2)
    fig.set_figwidth(0.0)
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter('always')
        fig.canvas.draw()
    assert [str(w.message) for w in caught] == [
        "divide by zero encountered in scalar divide"]


def test_constrained_layout_with_a_nan_ratio_ends_without_warning():
    fig = plt.figure(layout='constrained')
    fig.subplot_mosaic('AB;CC', width_ratios=[float('nan'), 1])
    with warnings.catch_warnings():
        warnings.simplefilter('error')
        with pytest.raises(ValueError, match=NAN_TO_INT):
            fig.canvas.draw()


def test_an_infinite_resolution_sizes_the_canvas_at_no_number():
    fig = plt.figure()
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter('always')
        fig.set_dpi(float('inf'))
    assert [str(w.message) for w in caught].count(
        "invalid value encountered in scalar multiply") == 4
    with pytest.raises(ValueError, match=NAN_TO_INT):
        fig.canvas.get_width_height()


# -- colorbars, transforms and documents of the program's ---------------------------

def test_a_colorbar_failing_on_an_axes_leaves_its_locator():
    fig, ax = plt.subplots()
    ax.imshow(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)))
    with pytest.raises(ValueError, match="'nope' is not a valid value for extend"):
        pltrs.colorbar.Colorbar(ax, extend='nope')
    with pytest.raises(AttributeError, match="'Colorbar' object has no attribute '_boundaries'"):
        fig.canvas.draw()


def test_colorbar_values_and_boundaries_computed_as_matplotlib():
    fig, ax = plt.subplots()
    for kwargs, error, message in [
            (dict(values='data'), IndexError, "too many indices for array"),
            (dict(boundaries=0), IndexError, "too many indices for array"),
            (dict(boundaries=[]), IndexError, "index 0 is out of bounds for axis 0 with size 0")]:
        with pytest.raises(error, match=re.escape(message)):
            pltrs.colorbar.Colorbar(fig.add_axes((0, 0, 0.1, 0.1)), **kwargs)


def test_nonsingular_of_unusual_arguments_as_matplotlib():
    assert type(transforms.nonsingular(0, 0, expander=np.float32(0.001))[1]) is np.float32
    with pytest.raises(TypeError, match="bad operand type for unary -: 'str'"):
        transforms.nonsingular(0, 0, expander='0.001')
    with pytest.raises(ZeroDivisionError):
        transforms.nonsingular(0, 0, tiny=0.0)
    with pytest.raises(TypeError, match=re.escape(
            "unsupported operand type(s) for /: 'float' and 'NoneType'")):
        transforms.nonsingular(0, 0, tiny=None)


def test_pdf_documents_open_their_file_when_made():
    with pytest.raises(ValueError, match="fname must be a PathLike or file handle"):
        backend_pdf.PdfFile(None)
    with pytest.raises(TypeError, match="string argument expected, got 'bytes'"):
        backend_pdf.PdfFile(io.StringIO())
    buffer = io.BytesIO()
    document = backend_pdf.PdfFile(buffer)
    assert buffer.getvalue().startswith(b'%PDF-1.4\n')
    document.newPage(1, 1)
    document.finalize()
    document.close()
    assert buffer.getvalue().endswith(b'%%EOF\n')


def test_pdf_streams_of_unusual_lengths_fail_when_they_end():
    document = _pdf_file()
    stream = backend_pdf.Stream(document.reserveObject('s').id, 0, document)
    stream.write(b'0 0 m')
    with pytest.raises(AttributeError, match="'int' object has no attribute 'id'"):
        stream.end()


def test_odd_points_are_hit_tested_as_matplotlibs_patches():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match=re.escape(
            "point_in_path(): incompatible function arguments.")):
        fig.canvas.inaxes('ab')
    cell = ax.table([['a', 'b'], ['c', 'd']])[0, 1]
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'x'"):
        cell.contains(None)


def test_text_measures_keep_the_renderer_last_given():
    fig, ax = plt.subplots()
    note = ax.annotate('note', xy=(0.2, 0.2), xytext=(0.6, 0.7),
                       arrowprops=dict(arrowstyle='->'))
    with pytest.raises(TypeError, match="cannot create weak reference to 'str' object"):
        note.get_window_extent(renderer='x')
    with pytest.raises(TypeError, match="cannot create weak reference to 'str' object"):
        note.get_tightbbox(renderer=None)
    label = ax.text(0.5, 0.5, 'hi')
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter('always')
        label.get_window_extent(dpi=float('nan'))
    assert [str(w.message) for w in caught] == ["invalid value encountered in cast"]
