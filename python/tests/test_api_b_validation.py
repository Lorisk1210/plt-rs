"""
Smoke tests of the checks, errors, warnings and types pltrs (Python)
matches in the core 2-D modules, found by the differential tester's
`api` programs. The expected values were recorded from matplotlib 3.11.2
running the same code.
"""

import io

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import (artist, bezier, collections, contour, dates, lines, patches,
                   streamplot, style, table, ticker)

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)

Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
VALUES = np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])


def test_a_draw_ends_at_its_first_problem():
    fig, ax = plt.subplots()
    ax.imshow(Z, vmin=5)
    ax.plot([0, 1], [0, 1])
    with pytest.raises(ValueError, match="minvalue must be less than or equal to maxvalue"):
        fig.canvas.draw()


def test_non_positive_log_limits_warn_where_set():
    fig, ax = plt.subplots()
    ax.bar([1, 2, 3], [3, 1, 2], log=True)
    with pytest.warns(UserWarning, match="Attempt to set non-positive ylim on a log-scaled"):
        ax.pie([1, 2, 3])
    assert ax.get_ylim()[1] == 1.25


def test_plotting_methods_check_their_arguments():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match=r"alpha \(1.5\) is outside 0-1 range"):
        ax.eventplot([1, 2, 4], alpha=1.5)
    with pytest.raises(ValueError, match=r"alpha \(nan\) is outside 0-1 range"):
        ax.pcolorfast(np.ones((3, 3)), alpha=float('nan'))
    with pytest.raises(ValueError, match=r"alpha \(1.5\) is outside 0-1 range"):
        ax.contour(Z, alpha=1.5)
    with pytest.raises(TypeError, match="'str' object is not callable"):
        ax.xcorr(VALUES, VALUES[::-1], maxlags=3, detrend='mean')
    with pytest.raises(KeyError, match="cilo"):
        ax.bxp([{'med': 2, 'q1': 1, 'q3': 3, 'whislo': 0, 'whishi': 4,
                 'fliers': [5]}], shownotches=True)
    with pytest.raises(TypeError, match='can only concatenate str'):
        ax.fill_betweenx([1, 2, 3], [1, 2, 1], step=0.5)
    with pytest.raises(KeyError, match="steps-bad"):
        ax.fill_between([1, 2, 3], [1, 2, 1], step='bad')
    ax.fill_between([], [], step='bad')  # nothing to fill, nothing looked up
    with pytest.raises(ValueError, match="need at least one array to concatenate"):
        ax.stackplot([1, 2, 3])
    with pytest.raises(ValueError, match="Supported values are None, 'default', 'linear', 'dB'"):
        ax.magnitude_spectrum(np.sin(np.arange(128) * 0.3), scale=2.0)
    with pytest.raises(ValueError, match="noverlap must be less than NFFT"):
        ax.specgram(np.sin(np.arange(512) * 0.3), NFFT=64, mode='expand')
    with (pytest.warns(UserWarning, match="Positional parameter c has no effect"),
          pytest.raises(TypeError, match="Image data of dtype <U1 cannot be converted")):
        ax.tripcolor([0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4], facecolors=['r', 'g'])
    with pytest.raises(AssertionError):
        ax.grouped_bar([[1, 2], [3, 4]], labels=[])
    with pytest.raises(ValueError, match="^Axis limits cannot be NaN or Inf$"):
        ax.set_xlim(np.nan, 1)


def test_plotting_methods_return_matplotlib_types():
    fig, ax = plt.subplots()
    medians = ax.boxplot(VALUES, label=3)['medians']
    assert medians[0].get_label() == '3'
    *_, line = ax.psd(np.sin(np.arange(256) * 0.3), NFFT=64, return_line=True)
    assert type(line) is list and isinstance(line[0], lines.Line2D)
    sublist = ax.ArtistList(ax, 'lines', valid_types=lines.Line2D)
    assert repr(sublist) == f'<Axes.ArtistList of {len(ax.lines)} lines>'


def test_figures_check_and_keep_what_matplotlib_does():
    fig = plt.figure()
    with pytest.raises(ValueError, match=r"figure size must be positive finite not \[nan"):
        fig.set_figwidth(float('nan'))
    with pytest.warns(RuntimeWarning, match="invalid value encountered in cast"):
        fig.set_dpi(float('nan'))
    fig.set_dpi(100)
    with pytest.warns(pltrs.MatplotlibDeprecationWarning, match="set at instantiation"):
        fig.figure = fig
    fig.set_facecolor((0.1, 0.2, 0.3))
    assert fig.get_facecolor() == (0.1, 0.2, 0.3, 1)
    assert type(fig.get_facecolor()[3]) is int
    fig.add_subplot()
    with pytest.raises(AttributeError, match="'list' object has no attribute 'xaxis'"):
        fig.convert_xunits(2)
    fig.set_transform(0.5)
    assert fig.get_transform() == 0.5
    with pytest.raises(NotImplementedError, match="cannot remove artist"):
        fig.remove()
    subfig = fig.subfigures()
    subfig.dpi = 72
    assert fig.dpi == 72 and type(subfig.dpi) is int
    fig.autofmt_xdate(rotation='vertical')
    fig.set_dpi(-100)
    with pytest.raises(ValueError, match="^dpi must be positive$"):
        fig.canvas.draw()


def test_artists_check_their_colours_and_shapes():
    with pytest.raises(ValueError, match="'notacolor' is not a valid value for color"):
        lines.Line2D([0, 1, 2], [2, 0, 1], color='notacolor')
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="'notacolor' is not a valid value for color"):
        ax.plot([1, 2], color='notacolor')
    with pytest.raises(ValueError, match=r"not enough values to unpack \(expected 2, got 1\)"):
        patches.Polygon((0.2, 0.3))
    with pytest.raises(ValueError, match="Invalid RGBA argument: 'notacolor'"):
        ax.spines['left'].set_color('notacolor')
    with pytest.raises(AttributeError, match="'int' object has no attribute 'ndim'"):
        collections.FillBetweenPolyCollection('x', np.arange(3.0), np.arange(3.0), 0)
    assert issubclass(patches.BoxStyle.LArrow, patches.BoxStyle.RArrow)
    assert issubclass(patches.BoxStyle.DArrow, patches.BoxStyle.RArrow)
    assert issubclass(patches.BoxStyle.Roundtooth, patches.BoxStyle.Sawtooth)


def test_ellipse_sizes_broadcast_as_numpy_does_when_drawn():
    fig, ax = plt.subplots()
    ellipses = collections.EllipseCollection(
        [1, 2], [0.5, 1], [0, 30], units='xy', offsets=[[0, 0], [1, 1]],
        offset_transform=ax.transData)
    ax.add_collection(ellipses)
    ellipses.set_widths(1)
    with pytest.raises(ValueError, match=r"could not broadcast input array from shape \(2,\) "
                                         r"into shape \(1,\)"):
        fig.canvas.draw()


def test_contour_sets_keep_their_arguments_and_errors():
    fig, ax = plt.subplots()
    segments = [[[(0, 0), (1, 1), (0, 1)]], [[(0.2, 0.2), (0.5, 0.5)]]]
    cs = contour.ContourSet(ax, [0, 1], segments, origin='lower', extent=(0, 4, 0, 3))
    assert (cs.origin, cs.extent) == ('lower', (0, 4, 0, 3))
    with pytest.raises(ValueError, match="^minvalue must be less than or equal to maxvalue$"):
        contour.ContourSet(ax, [0, 1], segments, vmin=5)
    cs = ax.contour(Z, levels=3)
    cs.clabel()
    cs.set(paths=cs.get_paths()[:2])
    with pytest.raises(IndexError, match="list index out of range"):
        cs.add_label_near(2, 2)


def test_tickers_and_dates_fail_and_type_as_matplotlib():
    with pytest.raises(ValueError, match="cannot convert float NaN to integer"):
        ticker.LogitFormatter().format_data(float('nan'))
    assert ticker.EngFormatter(unit='m').format_eng(1234.5) == '1.2345 km'
    assert ticker.EngFormatter(unit='m').format_eng(-1e9) == '-1 Gm'
    fig, ax = plt.subplots()
    ax.set_xscale('symlog')
    ax.plot([-100, 100])
    ticks = ax.xaxis.get_major_locator()()
    assert type(ticks) is list and ticks[1] == 0 and type(ticks[1]) is int
    locator = ticker.LinearLocator(presets={(0, 1): [0, 0.5, 1]})
    ax.yaxis.set_major_locator(locator)
    ax.set_ylim(0, 1)
    assert locator() == [0, 0.5, 1]
    with pytest.raises(TypeError, match="'tuple' object cannot be interpreted as an integer"):
        dates.HourLocator(interval=(0.0, 2.0))
    with pytest.raises(ValueError, match="cannot convert float NaN to integer"):
        dates.num2date(float('nan'))
    with pytest.raises(OverflowError, match="int too big to convert"):
        dates.num2date(1e10)


def test_colorbar_ticks_keep_their_type():
    fig, ax = plt.subplots()
    colorbar = fig.colorbar(ax.imshow(Z))
    colorbar.set_ticks([0, 1, 2])
    assert colorbar.get_ticks().dtype == np.int64


def test_module_functions_of_matplotlib():
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        assert pltrs.sanitize_sequence({'a': 1}.keys()) == ['a']
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        assert pltrs.validate_backend('agg') == 'agg'
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        assert style.update_nested_dict({'a': {'x': 1}}, {'a': {'y': 2}, 'b': {}}) == {
            'a': {'x': 1, 'y': 2}, 'b': {}}
    with pytest.warns(pltrs.MatplotlibDeprecationWarning):
        sheets = style.read_style_directory(pltrs.get_data_path() + '/stylelib')
    assert sheets['ggplot']['axes.facecolor'] == '#E5E5E5'
    assert streamplot.StreamMask.__module__ == streamplot.__name__
    assert artist.ArtistInspector.__module__ == artist.__name__


def test_bezier_helpers_keep_their_operands_types():
    assert bezier.get_normal_points(0, 0, 1, 0, 2) == (0, -2, 0, 2)
    assert bezier.find_control_points(0, 0, 0.5, 1, 1, 0) == [(0, 0), (0.5, 2.0), (1, 0)]
    left, right = bezier.make_wedged_bezier2([(0, 0), (1, 1), (2, 0)], 0.5)
    assert left[2] == (2.0, 0.0) and type(right[0][0]) is float
    with pytest.raises(ValueError, match="The path does not intersect with the patch") as info:
        bezier.split_path_inout(pltrs.path.Path([(5, 5), (6, 6)]),
                                lambda xy: xy[0] ** 2 + xy[1] ** 2 < 1)
    assert type(info.value) is ValueError


def test_tables_polar_axes_and_text_paths():
    fig, ax = plt.subplots()
    cells = table.table(ax, cellText=[['a', 'b']])
    with pytest.raises(KeyError):
        cells[(3, 0), table.Cell((0, 0), 1, 1)]
    polar = fig.add_subplot(projection='polar')
    with pytest.raises(KeyError) as info:
        polar.set_theta_zero_location('X')
    assert str(info.value) == "'X'"
    to_path = pltrs.textpath.TextToPath()
    verts, codes = to_path.get_text_path(pltrs.font_manager.FontProperties(size=12), 'Ab')
    assert type(verts) is list and verts[0].shape == (2,)
    assert type(codes[0]) is np.uint8
    verts, codes = to_path.get_text_path(pltrs.font_manager.FontProperties(size=12), '')
    assert verts.shape == (0, 2) and codes == []


def test_axes_positions_images_and_saving():
    fig, ax = plt.subplots()
    ax.set_aspect('equal')
    with pytest.raises(AttributeError, match="'list' object has no attribute 'frozen'"):
        ax.apply_aspect([0.1, 0.1, 0.5, 0.5])
    with pytest.raises(ValueError, match="^metadata not supported for format 'jpeg'$"):
        fig.canvas.print_jpg(io.BytesIO(), metadata={'Title': 't'})
    image = pltrs.image.AxesImage(ax)
    with pytest.raises(RuntimeError, match="You must first set the image array"):
        image.get_extent()
