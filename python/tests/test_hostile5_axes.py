"""
What the differential tester's hostile programs (fifth round) found pltrs
doing otherwise than matplotlib 3.11 with 2-D Axes, their axes, ticks and
tick helpers, paths, polar axes, patches, collections and images. Each
expected value is recorded from matplotlib.
"""

import io
import re
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.collections as mcollections
import pltrs.dates as mdates
import pltrs.hatch as mhatch
import pltrs.patches as mpatches
import pltrs.pyplot as plt
import pltrs.ticker as mticker
import pltrs.units as munits
from pltrs.bezier import BezierSegment
from pltrs.path import Path

import build_facts


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


X = np.linspace(0, 2 * np.pi, 7)
Y = np.sin(X)
Z = np.ones((5, 6))


def drawn_plot(projection=None):
    fig = plt.figure()
    ax = fig.add_subplot(projection=projection)
    ax.plot(X, np.arange(7.0) + 2)
    fig.canvas.draw()
    return fig, ax


def draw_error(fig):
    """The exception drawing *fig* raises, None if it draws."""
    try:
        fig.canvas.draw()
    except Exception as err:  # noqa: BLE001 - the error is what is compared
        return type(err), str(err)
    return None


# -- panning ----------------------------------------------------------------------

@pytest.mark.parametrize("start, message", [
    ((None, 40, 1), "unsupported operand type(s) for -: 'int' and 'NoneType'"),
    (('60', 40, 1), "unsupported operand type(s) for -: 'int' and 'str'"),
    ((60, None, 1), "unsupported operand type(s) for -: 'int' and 'NoneType'"),
])
def test_a_pan_keeps_its_start_and_fails_to_drag_from_it(start, message):
    fig, ax = drawn_plot()
    ax.start_pan(*start)
    with pytest.raises(TypeError, match=f"^{re.escape(message)}$"):
        ax.drag_pan(1, None, 70, 50)
    ax.end_pan()


@pytest.mark.parametrize("button", [-1, None, '1', 1.0, 2])
def test_any_button_starts_a_rectilinear_pan(button):
    fig, ax = drawn_plot()
    ax.start_pan(60, 40, button)
    ax.drag_pan(1, None, 70, 50)
    ax.end_pan()
    with pytest.raises(AttributeError, match="_pan_start"):
        ax.end_pan()


# -- axes and ticks ------------------------------------------------------------------

@pytest.mark.parametrize("event", [None, 3, 'x'])
def test_an_axis_reads_the_position_of_what_it_is_asked_about(event):
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError, match=f"^'{type(event).__name__}' object has no "
                                             "attribute 'x'$"):
        ax.xaxis.contains(event)


@pytest.mark.parametrize("pad, message", [
    ('0.3', 'can only concatenate str \\(not "float"\\) to str'),
    (None, "unsupported operand type\\(s\\) for \\+: 'NoneType' and 'float'"),
])
def test_a_tick_pad_fails_where_matplotlib_adds_it(pad, message):
    fig, ax = drawn_plot('polar')
    xtick = ax.xaxis.get_major_ticks()[1]
    with pytest.raises(TypeError, match=message):
        xtick.set_pad(pad)
    assert xtick.get_pad() == pad
    # A theta tick pads its labels anew at each draw.
    assert draw_error(fig)[0] is TypeError
    xtick.set_pad(0.3)
    assert draw_error(fig) is None


def test_a_hidden_tick_draws_nothing():
    fig, ax = drawn_plot()

    def drawn_ticks():
        buf = io.BytesIO()
        fig.savefig(buf, format='svg')
        return buf.getvalue().count(b'<g id="xtick_')

    shown = drawn_ticks()
    ax.xaxis.get_major_ticks()[1].set_visible(False)
    assert drawn_ticks() == shown - 1


@pytest.mark.parametrize("bad, message", [(3, "'int' object is not callable"),
                                          ('x', "'str' object is not callable")])
def test_a_tick_is_drawn_through_its_filter(bad, message):
    fig, ax = drawn_plot()
    tick = ax.yaxis.get_major_ticks()[1]
    tick.set_agg_filter(bad)
    assert draw_error(fig) == (TypeError, message)
    tick.set_agg_filter(lambda img, dpi: (img, 0, 0))
    assert draw_error(fig) is None


def test_a_radial_locator_fails_without_an_axis_to_read():
    fig, ax = drawn_plot('polar')
    locator = ax.yaxis.get_major_locator()
    locator.set_axis(None)
    assert draw_error(fig) == (AttributeError, "'NoneType' object has no attribute 'axes'")
    locator.set_axis(3)
    assert draw_error(fig) == (AttributeError, "'int' object has no attribute 'axes'")
    locator.set_axis(ax.yaxis)
    assert draw_error(fig) is None


def test_a_radial_axis_installs_a_locator_of_no_base():
    fig, ax = drawn_plot('polar')
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'set_axis'"):
        ax.yaxis.set_major_locator(None)
    assert ax.yaxis.get_major_locator().base is None
    assert draw_error(fig) == (AttributeError, "'NoneType' object has no attribute 'axis'")


def test_label_coordinates_are_kept_and_placed_when_drawn():
    fig, ax = plt.subplots()
    ax.set_xlabel('label')
    ax.xaxis.set_label_coords(None, -0.1)
    assert draw_error(fig) == (
        TypeError, "float() argument must be a string or a real number, not 'NoneType'")
    ax.xaxis.set_label_coords(0.5, -0.1, transform=3)
    assert draw_error(fig) == (AttributeError, "'int' object has no attribute 'transform'")
    ax.xaxis.set_label_coords(0.5, -0.1)
    assert draw_error(fig) is None


def test_a_refused_subplot_spec_is_kept():
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError):
        ax.set_subplotspec(3)
    assert ax.get_subplotspec() == 3
    ax.set_subplotspec(fig.add_gridspec(2, 2)[1, 1])
    assert ax.get_subplotspec().get_geometry() == (2, 2, 3, 3)


def test_a_rasterization_zorder_that_is_no_number():
    fig, ax = plt.subplots()
    ax.plot([1, 2])
    ax.set_rasterization_zorder(np.float32(0.5))
    assert type(ax.get_rasterization_zorder()) is np.float32
    ax.set_rasterization_zorder('0.5')
    assert ax.get_rasterization_zorder() == '0.5'
    assert draw_error(fig) == (
        TypeError, "'<' not supported between instances of 'float' and 'str'")


def test_a_zorder_that_is_no_number_fails_the_sort_of_the_draw():
    fig, ax = plt.subplots()
    ax.add_collection(mcollections.LineCollection([[(0, 0), (1, 1)]], zorder='3'))
    assert draw_error(fig) == (
        TypeError, "'<' not supported between instances of 'float' and 'str'")


def test_a_twin_of_a_bad_zorder_stays_as_made():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError):
        ax.twinx(delta_zorder='0.0')
    assert len(fig.axes) == 2
    twin = fig.axes[1]
    assert twin.patch.get_visible() and twin.xaxis.get_visible()
    assert len(ax._twinned_axes.get_siblings(ax)) == 1
    assert [a.get_adjustable() for a in fig.axes] == ['datalim', 'datalim']


def test_a_polar_clear_keeps_the_radial_labels_angle():
    fig, ax = drawn_plot('polar')
    ax.set_rlabel_position(45)
    ax.set_theta_offset(1)
    ax.cla()
    assert ax.get_rlabel_position() == 45.0
    assert ax.get_theta_offset() == 0


def test_a_bad_radial_label_angle_clears_it():
    fig, ax = drawn_plot('polar')
    with pytest.raises(TypeError):
        ax.set_rlabel_position('45')
    assert ax.get_rlabel_position() == 0.0


# -- plotting methods -------------------------------------------------------------

@pytest.mark.parametrize("kwargs, error, message", [
    (dict(minlength=-1.5), IndexError, "index -1 is out of bounds for axis 0 with size 0"),
    (dict(minlength=None), TypeError,
     "'>' not supported between instances of 'float' and 'NoneType'"),
    (dict(num_arrows=2.5), TypeError, "'float' object cannot be interpreted as an integer"),
    (dict(num_arrows=float('nan')), TypeError,
     "'float' object cannot be interpreted as an integer"),
])
def test_streamplot_arguments_fail_where_matplotlib_computes_with_them(kwargs, error,
                                                                     message):
    fig, ax = plt.subplots()
    with pytest.raises(error, match=f"^{re.escape(message)}$"):
        ax.streamplot(np.arange(6), np.arange(5), Z, Z, **kwargs)


def test_streamplot_keeps_a_zorder_that_is_no_number():
    fig, ax = plt.subplots()
    ax.streamplot(np.arange(6), np.arange(5), Z, Z, zorder='3')
    assert draw_error(fig) == (
        TypeError, "'<' not supported between instances of 'float' and 'str'")


@pytest.mark.parametrize("kwargs, error, message", [
    (dict(lineoffsets=None), TypeError,
     "unsupported operand type(s) for +: 'NoneType' and 'float'"),
    (dict(linelengths=None), TypeError,
     "unsupported operand type(s) for /: 'NoneType' and 'int'"),
    (dict(linewidths='ab'), TypeError, "len() of unsized object"),
    (dict(linestyles='DASHED'), ValueError, "Do not know how to convert ['DASHED'] to dashes"),
])
def test_eventplot_arguments_fail_as_matplotlib_s_rows(kwargs, error, message):
    fig, ax = plt.subplots()
    with pytest.raises(error) as info:
        ax.eventplot([1, 2, 4], **kwargs)
    assert str(info.value) == message


def test_eventplot_of_none():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="^object of type 'NoneType' has no len\\(\\)$"):
        ax.eventplot(None)


def test_errorbar_of_a_refused_colour_leaves_its_first_bars():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="^'' is not a valid color value.$"):
        ax.errorbar(np.arange(6), np.arange(6) * 0.5, yerr=0.2, ecolor='')
    assert (len(ax.lines), len(ax.collections), len(ax.containers)) == (1, 1, 0)
    assert draw_error(fig) == (ValueError, "'' is not a valid color value.")
    assert draw_error(fig) is None


def test_errorbar_keeps_widths_given_as_text():
    fig, ax = plt.subplots()
    container = ax.errorbar(np.arange(6), np.arange(6) * 0.5, yerr=0.2, capsize=3,
                            elinewidth='2', capthick='2')
    assert container[2][0].get_linewidth().tolist() == ['2']
    assert container[1][0].get_markeredgewidth() == '2'


def test_hlines_refuse_their_colours_before_adding():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="^Invalid RGBA argument: 0$"):
        ax.hlines([1, 2], 0, 1, colors=[0, 1])
    assert len(ax.collections) == 0
    assert draw_error(fig) is None


@pytest.mark.parametrize("args, kwargs, error, message", [
    ((Z,), dict(shading='gouraud'), ValueError,
     "For X (6) and Y (5) with flat shading, A should have shape (4, 5, 3) or "
     "(4, 5, 4) or (4, 5) or (20,), not (5, 6)"),
    ((np.arange(7), np.arange(6), Z), dict(shading='nearest'), TypeError,
     "Dimensions of C (5, 6) are incompatible with X (7) and/or Y (6); see help(pcolor)"),
    ((np.arange(6), np.arange(5), Z), dict(shading='flat'), TypeError,
     "Dimensions of C (5, 6) should be one smaller than X(6) and Y(5) while using "
     "shading='flat' see help(pcolor)"),
])
def test_pcolor_refuses_meshes_as_matplotlib(args, kwargs, error, message):
    fig, ax = plt.subplots()
    with pytest.raises(error) as info:
        ax.pcolor(*args, **kwargs)
    assert str(info.value) == message


def test_an_unknown_shading_warns_and_is_auto():
    fig, ax = plt.subplots()
    with pytest.warns(UserWarning, match="shading value 'nope' not in list of valid values"):
        ax.pcolormesh(Z, shading='nope')


def test_violin_computes_its_line_ends_first():
    fig, ax = plt.subplots()
    values = np.linspace(0, 1, 20)
    with pytest.raises(np._core._exceptions._UFuncNoLoopError, match="'multiply'"):
        ax.violinplot([values, values + 1], widths='0.3')
    with pytest.raises(TypeError, match="^'NoneType' object is not subscriptable$"):
        ax.violin([None])


def test_bxp_reads_the_label_first():
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute 'get'$"):
        ax.bxp([None])


def test_xcorr_of_booleans_sums_them_as_numpy():
    fig, ax = plt.subplots()
    x = np.array([False, False, True, False, True, True, False])
    lags, c, *_ = ax.xcorr(x, np.arange(7.0), maxlags=3)
    np.testing.assert_allclose(c, [0.5241424183609592, 1.0482848367219184,
                                   1.4675987714106857, 1.1531133203941102,
                                   0.8386278693775346, 0.5241424183609592,
                                   0.3144854510165755])


def test_cohere_shifts_its_frequencies_in_place():
    fig, ax = plt.subplots()
    with pytest.raises(np._core._exceptions._UFuncOutputCastingError):
        ax.cohere(np.sin(np.arange(256) * 0.3), np.cos(np.arange(256) * 0.2),
                  NFFT=64, Fc=None)


def test_plot_takes_units_before_checking_its_shapes():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="x and y must have same first dimension"):
        ax.plot('data', Y)
    assert type(ax.xaxis.get_converter()).__name__ == 'StrCategoryConverter'


# -- collections ---------------------------------------------------------------------

@pytest.mark.parametrize("cls, args, message", [
    (mcollections.PolyCollection, ([[[0, 0], [1, 1], [1, 0]]],),
     "object of type 'NoneType' has no len()"),
    (mcollections.LineCollection, ([[[0, 0], [1, 1]]],),
     "'vertices' must be 2D with shape (N, 2), but your input has shape ()"),
])
def test_paths_set_are_checked_as_matplotlib_s(cls, args, message):
    collection = cls(*args)
    with pytest.raises((TypeError, ValueError)) as info:
        collection.set_paths([None])
    assert str(info.value) == message


def test_a_polygon_set_that_fails_keeps_those_made_before():
    fig, ax = plt.subplots()
    mesh = ax.pcolor(np.arange(12.0).reshape(3, 4))
    with pytest.raises(TypeError):
        mesh.set_paths(None)
    assert len(mesh.get_paths()) == 0


def test_paths_set_again_are_new_objects():
    collection = mcollections.EventCollection([1, 2])
    collection.set_paths([[(0, 1), (2, 3)]])
    first = collection.get_paths()
    collection.set_paths([[(0, 1), (2, 3)]])
    assert collection.get_paths() is not first


def test_a_refused_colour_fails_the_first_draw_only():
    fig, ax = plt.subplots()
    collection = mcollections.EventCollection([1, 2, 4], linelength=0.5)
    ax.add_collection(collection)
    with pytest.raises(ValueError):
        collection.set_colors('')
    assert draw_error(fig) == (ValueError, "'' is not a valid color value.")
    assert draw_error(fig) is None


def test_line_widths_given_as_text_fail_raster_draws_only():
    fig, ax = plt.subplots()
    collection = mcollections.EllipseCollection(
        [1, 2], [0.5, 1], [0, 30], units='xy', offsets=[[0, 0], [1, 1]],
        offset_transform=ax.transData, hatch='/')
    ax.add_collection(collection)
    collection.set_linewidth('2')
    assert collection.get_linewidth().tolist() == ['2']
    assert draw_error(fig)[0] is RuntimeError
    fig.savefig(io.BytesIO(), format='pdf')
    collection.set_linewidth(2)
    collection.set_hatch_linewidth('3')
    assert draw_error(fig)[0] is RuntimeError
    with pytest.raises(ValueError, match="^Unknown format code 'g' for object of type 'str'$"):
        fig.savefig(io.BytesIO(), format='ps')


def test_a_hidden_ellipse_collection_still_makes_its_transforms():
    fig, ax = plt.subplots()
    collection = mcollections.EllipseCollection(
        [1, 2], [0.5, 1], [0, 30], units='xy', offsets=[[0, 0], [1, 1]],
        offset_transform=ax.transData)
    ax.add_collection(collection)
    collection.set(visible=False, widths=0.3)
    assert draw_error(fig) == (
        ValueError, "could not broadcast input array from shape (2,) into shape (1,)")


@pytest.mark.parametrize("kwargs, error, message", [
    (dict(positions='ab'), ValueError, "positions must be one-dimensional"),
    (dict(positions=[1, 2], lineoffset=None), TypeError,
     "unsupported operand type(s) for +: 'NoneType' and 'float'"),
    (dict(positions=[1, 2], lineoffset='1'), TypeError,
     'can only concatenate str (not "float") to str'),
    (dict(positions=[1, 2], orientation='nope', linestyle='DASHED'), ValueError,
     "Do not know how to convert 'DASHED' to dashes"),
])
def test_event_collections_check_as_matplotlib_s(kwargs, error, message):
    with pytest.raises(error) as info:
        mcollections.EventCollection(**kwargs)
    assert str(info.value) == message


# -- paths, patches and transforms -----------------------------------------------------

@pytest.mark.parametrize("args, error, message", [
    ((3,), AttributeError, "'int' object has no attribute 'count'"),
    (('//', -1.5), ValueError, "negative dimensions are not allowed"),
    (('o', -1), ValueError, "need at least one array to concatenate"),
])
def test_hatches_of_no_pattern_or_negative_density(args, error, message):
    with pytest.raises(error, match=f"^{re.escape(message)}$"):
        mhatch.get_path(*args)


def test_bezier_points_that_are_no_numbers():
    segment = BezierSegment([[0, 0], [1, 1], [2, 0]])
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for -: 'int' and "
                                        "'NoneType'$"):
        segment(None)
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        segment(float('inf'))
    assert [str(w.message) for w in caught] == build_facts.as_numpy_warns(
        ["invalid value encountered in matmul"])


def test_curved_extents_iterate_with_the_keywords_given():
    path = Path([[0, 0], [1, 1], [2, 0]], [Path.MOVETO, Path.CURVE3, Path.CURVE3])
    with pytest.raises(TypeError, match="iter_segments\\(\\) got an unexpected keyword"):
        path.get_extents(nonexistent_kw=1)


def test_lambert_transforms_keep_their_centre():
    fig = plt.figure()
    ax = fig.add_subplot(projection='lambert')
    transform = ax.LambertTransform(None, 0, 50)
    assert transform._center_longitude is None
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for -: 'float' and "
                                        "'NoneType'$"):
        transform.transform([[0.1, 0.2]])


def test_a_centre_of_three_items_reads_back_as_given():
    ellipse = mpatches.Ellipse((0.5, 0.5), 0.3, 0.2)
    ellipse.set_center((0.4, 0.4, 0.4))
    assert ellipse.get_center() == (0.4, 0.4, 0.4)


def test_a_hidden_quiver_key_is_drawn():
    fig, ax = plt.subplots()
    q = ax.quiver([0, 1], [0, 1], [1, 1], [1, 1])
    key = ax.quiverkey(q, 0.8, 0.9, 1, 'one', labelpos='W')
    key.set_visible(False)
    buf = io.BytesIO()
    fig.savefig(buf, format='svg')
    assert b'one' in buf.getvalue()


# -- tick helpers --------------------------------------------------------------------

def test_auto_date_locator_measures_its_distances_first():
    locator = pltrs.dates.AutoDateLocator()
    with pytest.raises(AttributeError, match="^'float' object has no attribute 'days'$"):
        locator.tick_values(0.0, 19800.0)
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for -: 'float' and "
                                        "'NoneType'$"):
        locator.tick_values(None, 19800.0)


def test_eng_formatter_compares_what_is_no_number():
    with pytest.raises(TypeError, match="^'<' not supported between instances of 'str' and "
                                        "'int'$"):
        mticker.EngFormatter()('1500')


def test_logit_locator_is_called_as_max_n_locator():
    with pytest.raises(TypeError, match="^MaxNLocator.__call__\\(\\) takes 1 positional "
                                        "argument but 2 were given$"):
        mticker.LogitLocator()(None)


def test_an_offset_of_numpy_truth_shifts_the_labels():
    fig, ax = plt.subplots()
    ax.plot([1000, 1001], [1, 2])
    formatter = mticker.ScalarFormatter()
    ax.xaxis.set_major_formatter(formatter)
    formatter.useOffset = np.True_
    fig.canvas.draw()
    assert [t.get_text() for t in ax.get_xticklabels()][:2] == ['998.8', '999.0']


def test_no_null_minor_ticks_are_compared_with_the_majors():
    fig, ax = drawn_plot()
    ax.yaxis.set_view_interval(float('inf'), 4.0)
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        fig.canvas.draw()
    assert not [w for w in caught if "rtol or atol" in str(w.message)]


def test_barbs_at_integer_positions_fail_draws_once_the_axes_have_units():
    fig, ax = plt.subplots()
    barbs = ax.barbs([0, 1], [0, 1], [10, -20], [15, 5])
    assert barbs.get_offsets().dtype == np.int64
    fig.canvas.draw()
    ax.barh('ab', [3, 1, 2])
    with pytest.raises(TypeError, match="^Cannot convert fill_value nan to dtype int64$"):
        fig.canvas.draw()
    barbs.set_offsets(np.array([[0, 0], [1, 1]]))
    fig.canvas.draw()


@pytest.mark.parametrize("bottom, units", [(None, False), ('0.1', True)])
def test_stem_takes_a_bottom_as_vlines_takes_it(bottom, units):
    fig, ax = plt.subplots()
    stems = ax.stem(np.array([3.0, 1.0, 4.0]), bottom=bottom)
    assert len(stems) == 3
    assert (ax.yaxis.units is not None) == units
    fig.canvas.draw()


@pytest.mark.parametrize("yrange, align, error, message", [
    ('ab', 'bottom', ValueError, "setting an array element with a sequence. The requested "
     "array has an inhomogeneous shape after 3 dimensions. The detected shape was (2, 4, 2) "
     "+ inhomogeneous part."),
    ('ab', 'center', TypeError, "unsupported operand type(s) for /: 'list' and 'int'"),
    ((0, [1]), 'bottom', TypeError, "unsupported operand type(s) for +: 'int' and 'list'"),
])
def test_broken_barh_sums_its_height_as_matplotlib_does(yrange, align, error, message):
    fig, ax = plt.subplots()
    with pytest.raises(error, match=f"^{re.escape(message)}$"):
        ax.broken_barh([(1, 2), (4, 1)], yrange, align=align)


def test_broken_barh_of_no_ranges_takes_a_height_in_a_list():
    fig, ax = plt.subplots()
    assert len(ax.broken_barh([], 'ab').get_paths()) == 0


@pytest.mark.parametrize("kwargs, error, message", [
    (dict(positions=[]), ValueError, "shape mismatch: objects cannot be broadcast to a single "
     "shape.  Mismatch is between 'x' with shape (0,) and 'height' with shape (2,)."),
    (dict(colors=[0, 1]), TypeError, "object of type 'int' has no len()"),
    (dict(colors=3), TypeError, "'int' object is not iterable"),
])
def test_grouped_bar_hands_positions_and_colours_to_bar(kwargs, error, message):
    fig, ax = plt.subplots()
    with pytest.raises(error, match=f"^{re.escape(message)}$"):
        ax.grouped_bar([[1, 2], [3, 4]], **kwargs)


def test_grouped_bar_colours_of_a_dataset_each():
    fig, ax = plt.subplots()
    bars = ax.grouped_bar([[1, 2], [3, 4]], colors=[['r', 'g'], ['b', 'k']])
    assert [[p.get_facecolor() for p in c] for c in bars.bar_containers] == [
        [(1.0, 0.0, 0.0, 1.0), (0.0, 0.5, 0.0, 1.0)], [(0.0, 0.0, 1.0, 1.0), (0.0, 0.0, 0.0, 1.0)]]
    fig, ax = plt.subplots()
    bars = ax.grouped_bar([[1, 2], [3, 4]], positions=[0])
    assert [len(c) for c in bars.bar_containers] == [2, 2]


@pytest.mark.parametrize("channels, masked", [(4, True), (3, False), (None, False)])
def test_unsampled_rgba_images_are_masked_as_their_data(channels, masked):
    data = np.linspace(0, 1, 24).reshape(2, 3, 4)
    data = data[..., 0] if channels is None else data[..., :channels]
    fig, ax = plt.subplots()
    image = ax.imshow(data, origin='lower', extent=(0, 3, 0, 2), interpolation='nearest')
    ax.set_xlim(1.2, 3)
    pixels = image.make_image(fig.canvas.get_renderer(), unsampled=True)[0]
    assert isinstance(pixels, np.ma.MaskedArray) == masked
    assert pixels.shape == (2, 2, 4)


def test_log_formatters_called_as_log_formatter_label_as_it():
    fig, ax = plt.subplots()
    ax.set_yscale('log')
    formatter = mticker.LogFormatterSciNotation()
    formatter.set_axis(ax.yaxis)
    assert mticker.LogFormatter.__call__(formatter, -1) == '1'
    assert mticker.LogFormatter.__call__(formatter, 0) == '0'
    assert formatter(0) == '$\\mathdefault{0}$'


@pytest.mark.parametrize("kwargs, expected", [
    ({}, '1.234500051e3'), (dict(useMathText=True), '1.234500051 \\times 10^{3}')])
def test_scalar_format_data_computes_in_the_values_precision(kwargs, expected):
    assert mticker.ScalarFormatter(**kwargs).format_data(np.float32(1234.5)) == expected
    assert mticker.ScalarFormatter(**kwargs).format_data(np.longdouble(1234.5)) != expected


def test_dates_of_numpy_singles_count_microseconds_in_single_precision():
    assert str(mdates.num2date(np.float32(19800.25))) == '2024-03-18 05:59:37.007104+00:00'
    assert str(mdates.num2date(np.array([19800.25], dtype=np.float32))[0]) == (
        '2024-03-18 06:00:00+00:00')
    formatter = mdates.ConciseDateFormatter(mdates.AutoDateLocator())
    assert formatter.format_data_short(np.float32(19800.25)) == '2024-03-18 05:59:37'


@pytest.mark.parametrize("kwargs, error, made", [
    (dict(textprops={'nonexistent_key': 1}), AttributeError, (3, 1)),
    (dict(wedgeprops={'nonexistent_key': 1}), AttributeError, (0, 0)),
    (dict(textprops={'fontsize': 'x'}), ValueError, (3, 1)),
])
def test_pie_fails_on_properties_where_matplotlib_does(kwargs, error, made):
    fig, ax = plt.subplots()
    ax.set_yscale('log')
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        with pytest.raises(error):
            ax.pie([1, 2, 3], labels=['a', 'b', 'c'], **kwargs)
    assert not caught, "no limits are set"
    assert (len(ax.patches), len(ax.texts)) == made


@pytest.mark.parametrize("scale, shape, expected", [
    ('linear', (0, 0), ['identical x', 'identical y']),
    ('log', (0, 6), ['non-positive y', 'non-positive y']),
    ('log', (6, 0), ['identical x', 'non-positive y']),
    ('log', (0, 0), ['identical x', 'non-positive y', 'non-positive y']),
])
def test_spy_of_no_rows_or_columns_warns_of_its_limits_in_order(scale, shape, expected):
    fig, ax = plt.subplots()
    ax.set_yscale(scale)
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        ax.spy(np.empty(shape))
    found = []
    for w in caught:
        message = str(w.message)
        name = re.search(r"(\w)lim", message)[1]
        found.append(("identical " if "identical" in message else "non-positive ") + name)
    assert found == expected


@pytest.mark.parametrize("y", ['0.25', 'ab'])
def test_a_title_keeps_a_height_of_text_until_drawn(y):
    fig, ax = plt.subplots()
    title = ax.set_title('t', y=y)
    assert title.get_position() == (0.5, y)
    with pytest.raises(munits.ConversionError,
                       match=f"^Failed to convert value\\(s\\) to axis units: '{y}'$"):
        fig.canvas.draw()


def test_clearing_an_axes_makes_new_ticks():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    ticks = ax.xaxis.get_major_ticks()
    labels = ax.get_xticklabels()
    ax.cla()
    assert not any(a is b for a, b in zip(ticks, ax.xaxis.get_major_ticks()))
    assert not any(a is b for a, b in zip(labels, ax.set_xticklabels([])))
