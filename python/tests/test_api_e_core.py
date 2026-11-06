"""
Smoke tests of what the differential tester's ``api`` and ``unreached``
programs found in the core 2-D modules, the backends and the writers
(task APIE). The expected values were recorded from matplotlib 3.11.2
running the same code.
"""

import datetime
import io
import logging

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import (colors, dates, dviread, font_manager, ft2font, gridspec,
                   hatch, layout_engine, patches, path, table, ticker,
                   transforms, tri)
from pltrs.backends import backend_pdf, backend_pgf

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def _font():
    return ft2font.FT2Font(font_manager.findfont('DejaVu Sans'))


def test_ft2font_glyph_lists_and_bitmaps():
    font = _font()
    font.set_size(12, 72)
    font.set_text('Hi')
    font.set_text('Text')
    assert font.get_num_glyphs() == 4
    font.set_text('')
    assert font.get_num_glyphs() == 0
    font.set_text('abc', 0, flags=ft2font.LoadFlags.NO_HINTING)
    assert font.get_num_glyphs() == 3
    font.load_char(65)
    assert font.get_num_glyphs() == 4
    font.draw_glyphs_to_bitmap()
    smooth = font.get_image().sum()
    # The glyphs are bitmaps once drawn: another antialiasing changes nothing.
    font.draw_glyphs_to_bitmap(antialiased=False)
    assert font.get_image().sum() == smooth
    with pytest.raises(RuntimeError, match=r"^FT_Select_Charmap \(ft2font.cpp line 280\) "
                       r"failed with error 0x06: invalid argument$"):
        font.select_charmap(0)
    with pytest.raises(RuntimeError, match=r"^charcode loading \(ft2font.cpp line 544\)"):
        with pytest.warns(UserWarning, match="missing from font"):
            font.load_char(0x10ffff)


def test_ft2font_classes_without_constructors():
    with pytest.raises(RuntimeError, match="^Glyph is not constructible$"):
        ft2font.Glyph()
    with pytest.raises(RuntimeError, match="^LayoutItem is not constructible$"):
        ft2font.LayoutItem()
    with pytest.raises(TypeError, match=r"Invoked with: 1; kwargs: b=2$"):
        ft2font.Glyph(1, b=2)
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        image = ft2font.FT2Image(3, 2)
    assert memoryview(image).shape == (2, 3)


def test_hatch_patterns_write_as_matplotlib_writes():
    with pytest.raises(ValueError, match=r"from shape \(12,\) into shape \(2,\)"):
        hatch.HorizontalHatch('--', 6).set_vertices_and_codes(
            np.zeros((4, 2)), np.ones(4, dtype=np.uint8))
    with pytest.raises(ValueError, match="Output array is the wrong shape"):
        hatch.SmallCircles('oo', 6).set_vertices_and_codes(
            np.zeros((4, 2)), np.ones(4, dtype=np.uint8))
    assert hatch.SmallFilledCircles('x.', 6.0).num_vertices == 1196.0


def test_values_fail_as_matplotlib_fails():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2])
    with pytest.raises(TypeError, match="unsupported operand type"):
        line.set(dashes=(0, [3, 1]))
    with pytest.raises(AttributeError, match="has no attribute 'translate'"):
        backend_pdf.Name(3)
    with pytest.raises(ZeroDivisionError):
        ticker.scale_range(5, 5)
    with pytest.raises(ValueError, match="Found a maximum value of 2.0$"):
        colors.rgb_to_hsv(np.array([2.0, 0, 0]))
    with pytest.raises(ValueError, match="^RGBA values should be within 0-1 range$"):
        colors.to_rgba((1, 2, 3))
    with pytest.raises(ValueError, match="^RGBA sequence should have length 3 or 4$"):
        colors.to_rgba((1, 0, 0, 1, 0))
    with pytest.raises(ValueError, match="must have a length of at least 3"):
        tri.Triangulation([0, 1], [0, 1])
    with pytest.raises(KeyError, match="nope"):
        font_manager.fontManager.score_weight('nope', 400)
    assert font_manager.fontManager.score_weight('bold', 2000) == pytest.approx(1.285)
    with pytest.raises(FileNotFoundError, match="named 'nope.tfm' in your texmf"):
        dviread.find_tex_file('nope.tfm')
    with pytest.raises(AttributeError, match="'Bbox' object has no attribute '_transform'"):
        line.update_from(transforms.Bbox.unit())
    locator = ticker.IndexLocator(3, 1)
    locator.set_axis(ax.xaxis)
    fig2, ax2 = plt.subplots()
    locator.set_axis(ax2.xaxis)
    with pytest.raises(ValueError, match="Maximum allowed size exceeded"):
        locator()


def test_abstract_bases_stay_abstract():
    fig = plt.figure()
    grid = gridspec.GridSpec(2, 3, figure=fig)
    assert gridspec.GridSpecBase.get_subplot_params(grid) is None
    nested = gridspec.GridSpecFromSubplotSpec(2, 1, subplot_spec=grid[0, 1])
    left = nested.get_subplot_params().left
    assert isinstance(left, np.float64) and left == pytest.approx(0.3985294117647059)
    with pytest.raises(NotImplementedError):
        layout_engine.LayoutEngine.execute(layout_engine.TightLayoutEngine(), fig)
    with pytest.raises(NotImplementedError, match="Derived must override"):
        patches.Patch().get_path()
    ax = fig.add_subplot()
    cell = ax.table([['a', 'b']])[0, 1]
    with pytest.raises(NotImplementedError, match="Derived must override"):
        patches.Patch.get_path(cell)
    for patch, count in [(patches.Polygon([[0, 0], [1, 0], [0.5, 1]]), 4),
                         (patches.Wedge((0.5, 0.5), 0.3, 30, 120, width=0.1), 15),
                         (patches.Ellipse((0, 0), 1, 2), 26),
                         (patches.CirclePolygon((0, 0), 1), 21),
                         (patches.StepPatch([1, 2], [0, 1, 2]), 6)]:
        assert patch.get_path().vertices.shape == (count, 2)


def test_rrule_wrapper_hands_dateutil_what_it_lacks():
    rule = dates.rrulewrapper(dates.DAILY, interval=2, dtstart=datetime.datetime(2024, 3, 1), count=4)
    with pytest.raises(AttributeError, match="'rrule' object has no attribute 'nope'"):
        rule.nope
    assert rule.count() == 4
    assert list(rule.xafter(datetime.datetime(2024, 3, 1), count=2)) == [
        datetime.datetime(2024, 3, 3), datetime.datetime(2024, 3, 5)]
    # dateutil keeps what it is given: the locators fail where they use it.
    dates.MonthLocator(interval=(0.0, 2.0))
    dates.WeekdayLocator(interval=(3.0, 1.0))


def test_pdf_pages_take_any_finite_size():
    pdf = backend_pdf.PdfFile(io.BytesIO())
    pdf.newPage(4, 3)
    pdf.newPage(0, 0)
    with pytest.raises(AssertionError):
        pdf.beginStream(pdf.reserveObject('stream').id, None)
    fig, ax = plt.subplots()
    assert backend_pgf.FigureCanvasPgf(fig).get_default_filetype() == 'pdf'


def test_rc_files_not_utf8(caplog):
    with caplog.at_level(logging.WARNING):
        with pytest.raises(UnicodeDecodeError, match="can't decode byte 0x9e in position 18"):
            pltrs.rc_params_from_file(font_manager.findfont('DejaVu Sans'))
    assert "Cannot decode configuration file" in caplog.text
    assert pltrs.ExecutableNotFoundError.__module__ == pltrs.__name__


def test_categories_and_units():
    fig, ax = plt.subplots()
    ax.plot(['a', 'b', 'c'], [1, 2, 3])
    units = ax.xaxis.units
    units.update(['z'])
    assert units._mapping['z'] == 3
    days = np.arange('2024-01-01', '2024-01-08', dtype='datetime64[D]')
    fig, ax = plt.subplots()
    ax.plot(['a', 'b', 'c'], [1, 2, 3])
    with pytest.warns(UserWarning, match="potentially incompatible converter"):
        with pytest.raises(TypeError, match="tz must be string or tzinfo subclass"):
            ax.xaxis.update_units(days)
    fig, ax = plt.subplots()
    ax.plot(days, np.arange(7))
    with pytest.raises(ValueError, match="inches is not a valid timezone"):
        ax.xaxis.set_units('inches')


def test_values_read_back_with_matplotlib_types():
    fig, ax = plt.subplots()
    rectangle = patches.Rectangle((0, 0), 1, 1)
    rectangle.set_edgegapcolor('r')
    assert rectangle.get_edgegapcolor() == (1.0, 0.0, 0.0, 1)
    assert type(rectangle.get_edgegapcolor()[3]) is int
    cell = ax.table([['a', 'b'], ['c', 'd']])[0, 1]
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    bounds = cell.get_text_bounds(renderer)
    assert all(isinstance(v, np.float64) for v in bounds)
    assert cell.get_required_width(renderer) == bounds[2] * (1.0 + 2.0 * cell.PAD)
    assert type(pltrs.colormaps).__name__ == 'ColormapRegistry'
    a = ax.imshow(np.ones((5, 6)))
    fig.canvas.draw()
    _, x, y = pltrs.image.composite_images([a], fig.canvas.get_renderer())
    assert isinstance(x, np.float64) and isinstance(y, np.float64)
    collection = tri.tripcolor(ax, [0, 1, 0, 1], [0, 0, 1, 1], facecolors=[1, 2])
    assert collection.get_array().dtype == np.int64


def test_triangular_contour_sets_contour_their_arguments():
    fig, ax = plt.subplots()
    triangulation = tri.Triangulation([0, 1, 0, 1, 0.5], [0, 0, 1, 1, 0.5])
    contours = tri.TriContourSet(ax, triangulation, [1, 2, 3, 4, 0])
    assert list(contours.levels) == [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0]
    filled = tri.TriContourSet(ax, triangulation, [1, 2, 3, 4, 0], levels=2, filled=True)
    assert filled.filled and len(filled.levels) > 1


def test_update_from_a_loose_artist_drops_the_clip_box():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1])
    assert line.get_clip_box() is not None
    line.update_from(plt.Line2D([0], [0]))
    assert line.get_clip_box() is None


def test_unreached_names():
    path_ = path.Path.make_compound_path(path.Path.unit_rectangle(), path.Path.unit_circle())
    assert len(path_.vertices) == 31
    assert len(path.Path.unit_circle_righthalf().vertices) == 14
    fig, ax = plt.subplots()
    tab = table.table(ax, cellText=[['a']], loc='top')
    tab[2, 0] = table.Cell((0, 0), 0.1, 0.1, text='new')
    assert tab[2, 0].get_text().get_text() == 'new'
    with pytest.raises(KeyError, match="Only tuples length 2"):
        tab['x'] = table.Cell((0, 0), 0.1, 0.1)
    with pytest.raises(ValueError, match="'nope' is not a valid value for location"):
        pltrs.colorbar.make_axes(ax, location='nope')


def test_arrow_outline_counts_in_the_data_limits():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    ax.add_patch(patches.FancyArrowPatch((0.2, 0.2), (0.8, 0.5), arrowstyle='-|>',
                                         mutation_scale=20, shrinkA=5, shrinkB=5))
    ax.add_patch(patches.FancyArrowPatch(
        path=path.Path([(0, 0), (2, 1), (3, 0)], [1, 3, 3]), arrowstyle='simple',
        mutation_scale=40))
    # Measured with the view as it stood, (0, 1), not autoscaled first.
    assert np.allclose(ax.dataLim.bounds, (-2.816e-03, -1.0141e-02, 3.002816, 1.010141),
                       atol=1e-6)
    ax.relim()
    ax.autoscale_view()
    assert np.allclose(ax.get_xlim(), (-0.152956, 3.150141), atol=1e-6)


def test_core_behaviour_handed_on():
    fig, ax = plt.subplots()
    text = ax.text(0.5, 0.5, 'x')
    with pytest.raises(ValueError, match="Invalid RGBA argument: 'notacolor'"):
        text.set_backgroundcolor('notacolor')
    loose = plt.Line2D([0, 1], [0, 1])
    from pltrs import backend_bases
    mouse = backend_bases.MouseEvent('button_press_event', fig.canvas, 60, 40, 1)
    hit, details = loose.contains(mouse)
    assert not hit and list(details) == ['ind'] and details['ind'].size == 0
    line, = ax.plot([0, 1])
    assert list(line.contains(mouse)[1]) == ['ind']
    from pltrs import collections
    group = collections.PatchCollection([patches.Circle((0, 0), 1)])
    with pytest.raises(AttributeError, match="'Path' object has no attribute 'get_transform'"):
        group.set_paths([path.Path.unit_circle()])
    with pytest.warns(UserWarning, match=r"Log scale: values of z <= 0 have been masked"):
        ax.contour(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)),
                   norm=colors.LogNorm(1, 10))
    polar = fig.add_subplot(projection='polar')
    with pytest.raises(ValueError, match="for Axes which override 'get_data_ratio'"):
        polar.axis('equal')


def test_pyplot_loads_matplotlibs_modules():
    for name in ("colorbar", "dviread", "hatch", "inset", "spines", "stackplot", "tri"):
        assert hasattr(pltrs, name)
    # pyplot's own functions keep their names.
    assert callable(plt.colorbar) and callable(plt.stackplot)
