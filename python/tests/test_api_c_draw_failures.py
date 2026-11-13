"""
Smoke tests of the values matplotlib keeps unchecked and only fails on
when it draws or measures, and of the other behaviour of the core 2-D
modules the differential tester's `api` programs found (task APIC). The
expected values were recorded from matplotlib 3.11.2 running the same
code.
"""

import io
import math
import warnings

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import (axis, bezier, collections, dates, image, lines, patches,
                   texmanager, ticker, tri)

import build_facts
from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def _fails(fig, kind, match):
    """The figure fails to draw and to save, with *kind* matching *match*."""
    with pytest.raises(kind, match=match):
        fig.canvas.draw()
    with pytest.raises(kind, match=match):
        fig.savefig(io.BytesIO(), format='pdf')


def test_unusable_values_fail_where_matplotlib_uses_them():
    fig, ax = plt.subplots()
    box = patches.FancyBboxPatch((0.1, 0.1), 0.5, 0.5)
    ax.add_patch(box)
    box.set_mutation_aspect('equal')
    assert box.get_mutation_aspect() == 'equal'
    with pytest.raises(TypeError, match="unsupported operand type"):
        box.get_path()
    _fails(fig, TypeError, "unsupported operand type")
    box.set_mutation_aspect(None)
    assert box.get_mutation_aspect() == 1
    fig.canvas.draw()

    polygon = patches.RegularPolygon((0.2, 0.3), 6, orientation='vertical')
    assert polygon.orientation == 'vertical'
    with pytest.raises(TypeError, match="must be real number, not str"):
        ax.add_patch(polygon)
    assert polygon not in ax.patches

    fig, ax = plt.subplots()
    stars = collections.StarPolygonCollection(
        5, rotation='vertical', offsets=[(0, 0)], offset_transform=ax.transData)
    ax.add_collection(stars)
    assert stars.get_rotation() == 'vertical'
    _fails(fig, TypeError, "bad operand type for unary -")
    assert collections.RegularPolyCollection(4).get_rotation() == 0

    fig, ax = plt.subplots()
    ellipses = collections.EllipseCollection([1], [1], [0], units='nope')
    ax.add_collection(ellipses)
    _fails(fig, ValueError, "Unrecognized units: 'nope'")

    fig, ax = plt.subplots()
    ax.add_collection(collections.EllipseCollection(
        [1, 2], [0.5, 1], [0, 30], units='xy', offsets=[[0, 0], [1, 1]],
        offset_transform=ax.transData)).set_array(np.ones((2, 3)))
    _fails(fig, ValueError, "Collections can only map rank 1 arrays")


def test_ticks_keep_a_location_that_is_no_number():
    fig, ax = plt.subplots()
    tick = ax.xaxis.get_major_ticks()[0]
    tick.update_position('nowhere')
    assert tick.get_loc() == 'nowhere'
    with pytest.raises(ValueError, match="could not convert string to float: 'nowhere'"):
        tick.draw(fig.canvas.get_renderer())
    fig.canvas.draw()  # the axis places its ticks anew


def test_inset_indicator_keeps_an_invalid_alpha():
    fig, ax = plt.subplots()
    indicator = ax.indicate_inset_zoom(ax.inset_axes([0.6, 0.6, 0.3, 0.3]))
    with pytest.raises(ValueError, match=r"alpha \(nan\) is outside 0-1 range"):
        indicator.set_alpha(float('nan'))
    assert np.isnan(indicator.get_alpha())
    assert indicator.rectangle.get_alpha() == 0.5
    with pytest.raises(ValueError, match=r"alpha \(nan\) is outside 0-1 range"):
        indicator.connectors
    _fails(fig, ValueError, r"alpha \(nan\) is outside 0-1 range")
    indicator.set_alpha(0.5)
    assert indicator.connectors[0].get_alpha() == 0.5


def test_images_without_data_fail_to_draw_and_relim():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 2])
    empty = ax.add_image(image.AxesImage(ax))
    with pytest.raises(RuntimeError, match="You must first set the image array"):
        ax.relim()
    _fails(fig, AttributeError, "'NoneType' object has no attribute 'size'")
    empty.set_data(np.ones((2, 2)))
    ax.relim()
    assert tuple(ax.dataLim.extents) == (-0.5, -0.5, 1.5, 2.0)
    fig.canvas.draw()


def test_spines_fail_as_matplotlib_lays_them_out():
    fig, ax = plt.subplots()
    spine = ax.spines['bottom']
    spine.set_patch_arc((0.5, 0.5), 0.5, 0, 90)
    spine.set_patch_line()
    assert spine.get_path().vertices.shape == (7, 2)
    with pytest.raises(AssertionError, match="unexpected vertices shape"):
        spine.get_window_extent()
    _fails(fig, AssertionError, "unexpected vertices shape")

    spine = plt.subplots()[1].spines['left']
    spine.set_position(('outward', 10))
    assert spine.get_position() == ('outward', 10)
    assert type(spine.get_position()[1]) is int
    spine.set_position('center')
    assert spine.get_position() == 'center'
    spine.set_bounds(1)
    assert spine.get_bounds()[0] == 1 and type(spine.get_bounds()[0]) is int

    fig, ax = plt.subplots()
    del ax.spines['top']
    _fails(fig, AttributeError, "'Spines' object does not contain a 'top' spine")


def test_scalar_formatter_follows_the_draw_and_its_format():
    fig, ax = plt.subplots()
    ax.plot([1e5, 2e5])
    fig.canvas.draw()
    formatter = ax.yaxis.get_major_formatter()
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")  # the deprecated attribute
        assert formatter.format == '%1.0f'
        assert len(formatter.locs) == 8
        formatter.format = 'svg'
    with pytest.raises(TypeError, match="not all arguments converted"):
        formatter(1e5)
    with pytest.raises(ValueError, match=build_facts.raised(lambda: math.log10(0.0))):
        formatter.format_data(-0.0)
    formatter.set_axis('z')
    with pytest.raises(AttributeError, match="'str' object has no attribute 'get_view_interval'"):
        fig.canvas.draw()


def test_formatters_and_locators_fail_on_values_as_matplotlib_does():
    with pytest.raises(ValueError, match="truth value of an array"):
        ticker.LogFormatterSciNotation().format_data(np.array([0.0, 0.5, 2.0]))
    with pytest.raises(TypeError, match="'<' not supported"):
        ticker.LogFormatterSciNotation().format_data('v')
    with pytest.raises(TypeError, match="'<=' not supported"):
        ticker.LogitFormatter().format_data('v')
    fig, ax = plt.subplots()
    percent = ticker.PercentFormatter(xmax=5)
    percent.set_axis(ax.yaxis)
    with pytest.raises(TypeError, match="unsupported format string passed to numpy.ndarray"):
        percent.format_data_short(np.array([0.0, 0.5]))
    locator = ticker.LogLocator()
    locator.set_axis(ax.xaxis)
    assert locator.view_limits(0.5, 50) == (0.5, 50)
    with pytest.warns(UserWarning, match="Data has no positive values"):
        assert locator.nonsingular(0, 0) == (1, 10)
    assert locator.nonsingular(-1, 10) == (1e-300, 10)
    with pytest.raises(TypeError, match="relativedelta only diffs datetime/date"):
        dates.AutoDateLocator().tick_values(19800.0, 19830.0)
    with pytest.raises(IndexError, match="too many indices for array"):
        dates.date2num('2024')


def test_date_formatter_unit_follows_draws_only():
    fig, ax = plt.subplots()
    ax.plot(np.arange('2024-01-01', '2024-01-08', dtype='datetime64[D]'), np.arange(7))
    formatter = ax.xaxis.get_major_formatter()
    assert formatter(19800.5) == '2024'
    fig.canvas.draw()
    assert formatter(19800.5) == '2024-03-18'


def test_base_axis_methods_are_abstract():
    fig, ax = plt.subplots()
    for name in ("get_view_interval", "get_data_interval", "get_tick_space",
                 "get_minpos"):
        with pytest.raises(NotImplementedError):
            getattr(axis.Axis, name)(ax.xaxis)
    with pytest.raises(NotImplementedError):
        axis.Axis.set_label_position(ax.xaxis, 'top')
    assert ax.xaxis.get_tick_space() > 0


def test_intervals_widen_with_pythons_min_and_max():
    fig, ax = plt.subplots()
    ax.plot(np.linspace(0, 2 * np.pi, 7), np.sin(np.linspace(0, 2 * np.pi, 7)))
    xaxis = ax.xaxis
    xaxis.set_view_interval(float('nan'), 4.0)
    assert np.isnan(xaxis.get_view_interval()).all()
    xaxis.set_view_interval(5, 4.0)
    assert list(xaxis.get_view_interval()) == [5, 4]
    xaxis.set_view_interval(0.0, -1)
    assert list(xaxis.get_view_interval()) == [5, -1]


def test_tick_params_reset_as_matplotlib_resets_them():
    fig, ax = plt.subplots()
    ax.xaxis.set_tick_params(labelsize=8)
    ax.xaxis.set_tick_params(reset=True)
    assert ax.xaxis.get_tick_params() == {'gridOn': False}


def test_axline_is_made_directly():
    line = lines.AxLine((0, 0), (1, 2), None, color='r')
    assert (line.get_xy1(), line.get_xy2(), line.get_slope()) == ((0, 0), (1, 2), None)
    with pytest.raises(TypeError, match="Exactly one of 'xy2' and 'slope' must be given"):
        lines.AxLine((0, 0), None, None)
    fig, ax = plt.subplots()
    ax.add_line(lines.AxLine((1, 1), None, 0.5))
    fig.canvas.draw()


def test_texmanager_names_documents_after_the_numbers_given():
    manager = texmanager.TexManager()
    assert manager.get_basefile('ab', 12).endswith(
        'b47e2d218a9dbbf8185cdb46ccda91a776beec1cf4a6df64c38f1119a6e2e164')
    assert manager.get_basefile('ab', 12, dpi=100).endswith(
        'c766ee34850fa177428bd6a3f1b33125d2e613fd76d9e7ec7295c1898ebaa290')
    with pytest.raises(TypeError, match="can't multiply sequence by non-int of type 'float'"):
        manager.get_rgba('ab', fontsize='x-small')


def test_bezier_values_have_matplotlibs_shapes():
    segment = bezier.BezierSegment([[0, 0], [1, 2], [3, 1]])
    assert segment(0.5).shape == (2,)
    assert segment([0.5, 1]).shape == (2, 2)
    with pytest.warns(Warning, match="point_at_t function was deprecated"):
        segment.point_at_t(0.5)
    left, right = bezier.split_de_casteljau(np.array([[0.0, 0.0], [1.0, 2.0], [3.0, 1.0]]), 0.5)
    assert isinstance(left, list) and left[1].tolist() == [0.5, 1.0]


def test_tri_analyzer_and_finder_types():
    triangulation = tri.Triangulation([0, 1, 0, 1, 0.5], [0, 0, 1, 1, 0.5])
    analyzer = tri.TriAnalyzer(triangulation)
    assert all(type(factor) is np.float64 for factor in analyzer.scale_factors)
    assert not isinstance(analyzer.circle_ratios(), np.ma.MaskedArray)
    with pytest.raises(NotImplementedError):
        tri.TriFinder(triangulation)(0.5, 0.25)
