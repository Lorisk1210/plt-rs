"""
What the differential tester's hostile programs (fourth round) found pltrs
doing otherwise than matplotlib 3.11 with 2-D Axes, their axes and tick
helpers, contour labels, polar axes, patches and collections. Each
expected value is recorded from matplotlib.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.category import StrCategoryConverter
from pltrs.units import ConversionError


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
VALUES = np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])


def labelled_contours():
    fig, ax = plt.subplots()
    cs = ax.contour(Z, levels=3)
    cs.clabel()
    return fig, ax, cs


# -- annotations, lines and spans -------------------------------------------------

def test_a_clear_forgets_annotations_that_cannot_be_placed():
    fig, ax = plt.subplots()
    ax.annotate('a', (0.5, 0.5), xycoords='')
    with pytest.raises(ValueError, match="^'' is not a valid coordinate$"):
        fig.canvas.draw()
    ax.cla()
    assert len(ax.texts) == 0
    fig.canvas.draw()


@pytest.mark.parametrize("arrowprops, error, message", [
    (0, AttributeError, "'int' object has no attribute 'copy'"),
    ([], TypeError, "pop expected at most 1 argument, got 2"),
])
def test_arrow_properties_that_are_no_dict(arrowprops, error, message):
    fig, ax = plt.subplots()
    with pytest.raises(error, match=f"^{message}$"):
        ax.annotate('a', (0.5, 0.5), arrowprops=arrowprops)


def test_reference_lines_compare_their_position_with_the_bounds():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="^'>' not supported between instances of "
                                        "'float' and 'NoneType'$"):
        ax.axhline(None)
    with pytest.raises(TypeError, match="^unsupported operand type"):
        ax.axhspan(0.2, '0.4')


def test_an_infinite_line_of_points_that_are_none_is_added_then_fails():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="^setting an array element with a sequence"):
        ax.axline((), (1, 1))
    assert len(ax.lines) == 1
    assert ax.lines[0].get_xy1() == ()
    with pytest.raises(TypeError, match=r"^affine_transform\(\): incompatible function"):
        fig.canvas.draw()


def test_an_infinite_line_checks_its_properties_before_its_geometry():
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError, match="^AxLine.set\\(\\) got an unexpected "
                                             "keyword argument 'nonexistent_kw'$"):
        ax.axline((0, 0), (1, 1), slope=0, nonexistent_kw=1)


def test_an_infinite_line_keeps_a_slope_that_is_no_number():
    fig, ax = plt.subplots()
    line = ax.axline((0, 0), slope=0.5)
    line.set_slope('x')
    assert line.get_slope() == 'x'
    with pytest.raises(TypeError, match="^ufunc 'isinf' not supported"):
        fig.canvas.draw()


def test_data_limits_take_no_value_that_is_no_number():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="^ufunc 'isfinite' not supported"):
        ax.update_datalim(None)


def test_arrows_convert_their_geometry_with_the_units():
    fig, ax = plt.subplots()
    with pytest.raises(ConversionError, match="^Failed to convert value\\(s\\) to "
                                              "axis units: '0'$"):
        ax.arrow('0', 0, 0.5, 0.5)
    assert isinstance(ax.xaxis.get_converter(), StrCategoryConverter)
    ax.cla()
    assert ax.xaxis.get_converter() is None


def test_a_conversion_failing_keeps_the_converter():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="^'value' must be an instance of str or "
                                        "bytes, not a float$"):
        ax.axhspan('0.2', 0.4)
    assert isinstance(ax.yaxis.get_converter(), StrCategoryConverter)
    with pytest.raises(ConversionError, match=r"^Failed to convert value\(s\) to "
                                              r"axis units: \[None, 0.4\]$"):
        ax.axhspan(None, 0.4)


def test_an_explicit_converter_of_none_refuses_others():
    fig, ax = plt.subplots()
    ax.plot(['a', 'b'], [1, 2])
    with pytest.warns(UserWarning, match="^This axis already has a converter set"):
        ax.xaxis.set_converter(None)
    assert ax.xaxis.get_converter() is None
    with pytest.raises(RuntimeError, match="^Axis already has an explicit converter set$"):
        ax.xaxis.set_ticks('ab')


def test_category_ticks_of_no_number_fail_the_draw():
    fig, ax = plt.subplots()
    ax.plot(['a', 'b', 'c'], [1, 2, 3])
    ax.xaxis.set_ticks([float('nan'), 1, 2])
    assert np.isnan(ax.xaxis.get_view_interval()).all()
    with pytest.raises(ValueError, match="^cannot convert float NaN to integer$"):
        fig.canvas.draw()
    ax.xaxis.set_ticks([0, 1, 2])
    assert ax.xaxis.get_view_interval().tolist() == [2.0, 0.0]


# -- plotting methods -------------------------------------------------------------

def test_grouped_bars_check_their_spacing_as_matplotlib():
    fig, ax = plt.subplots()
    with pytest.raises(StopIteration):
        ax.grouped_bar([])
    with pytest.raises(ValueError, match="^'positions' must be equidistant$"):
        ax.grouped_bar([[1, 2], [3, 4]], positions=[float('nan'), 1.5])
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for \\+: "
                                        "'int' and 'str'$"):
        ax.grouped_bar([[1, 2], [3, 4]], group_spacing='2')


def test_bars_name_their_arguments_in_matplotlib_order():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="Mismatch is between 'width' with shape "
                                         r"\(3,\) and 'y' with shape \(0,\)\.$"):
        ax.barh([], [3, 1, 2])


def test_bar_labels_measure_their_padding():
    fig, ax = plt.subplots()
    bars = ax.bar([1, 2], [3, 4])
    with pytest.raises(TypeError, match="^len\\(\\) of unsized object$"):
        ax.bar_label(bars, padding='3')
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for \\*: "
                                        "'int' and 'NoneType'$"):
        ax.bar_label(bars, padding=None)


def test_stems_take_formats_as_matplotlib():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="^object of type 'int' has no len\\(\\)$"):
        ax.stem(VALUES, linefmt=3)


def test_empirical_distributions_compute_as_matplotlib():
    fig, ax = plt.subplots()
    with pytest.raises(IndexError, match="^index 0 is out of bounds for axis 0 with size 0$"):
        ax.ecdf(np.empty((0,)))
    with pytest.raises(IndexError, match="^index 6 is out of bounds"):
        ax.ecdf(VALUES, weights=np.ones(6))
    line = ax.ecdf(VALUES, weights=np.ones((1, 7)))
    assert line.get_ydata()[-1] == pytest.approx(1.0)


@pytest.mark.parametrize("kwargs, error, message", [
    (dict(bins=-1), ValueError, "`bins` must be positive, when an integer"),
    (dict(bins=4.5), TypeError, "`bins` must be an integer, a string, or an array"),
    (dict(bins='4'), ValueError, "'4' is not a valid estimator for `bins`"),
    (dict(range=[]), ValueError, r"not enough values to unpack \(expected 2, got 0\)"),
])
def test_histograms_bin_unusual_arguments_with_numpy(kwargs, error, message):
    fig, ax = plt.subplots()
    with pytest.raises(error, match=f"^{message}$"):
        ax.hist(VALUES, **kwargs)


def test_insets_place_themselves_with_their_locator_first():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="^Bbox.from_bounds\\(\\) missing 3 required "
                                        "positional arguments"):
        ax.inset_axes([0.5])
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for -: "
                                        "'int' and 'BboxTransformTo'$"):
        ax.inset_axes([0.5, 0.5, 0.4, 0.4], transform=3)


def test_twins_take_their_class_and_z_order_as_matplotlib():
    fig, ax = plt.subplots()
    assert ax.twinx(axes_class=0).get_zorder() == 0
    with pytest.raises(TypeError, match="^'str' object is not callable$"):
        ax.twinx(axes_class='x')
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for \\+: "
                                        "'int' and 'str'$"):
        ax.twiny(delta_zorder='0.0')
    assert len(fig.axes) == 3


def test_the_axes_test_points_as_its_patch_does():
    fig, ax = plt.subplots()
    with pytest.raises(IndexError, match="^tuple index out of range$"):
        ax.contains_point(())
    with pytest.raises(TypeError, match="^'NoneType' object is not subscriptable$"):
        ax.contains_point(None)
    assert not ax.contains_point((60, 40, 40))


def test_spy_masks_the_values_first():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="^bad operand type for abs\\(\\): 'NoneType'$"):
        ax.spy(None)
    with pytest.raises(TypeError, match=r"^Invalid shape \(3,\) for image data$"):
        ax.spy(np.array([True, False, True]))


def test_boxplots_compute_unusual_statistics_as_matplotlib():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="^whis must be a float or list of percentiles$"):
        ax.boxplot(VALUES, whis='x')
    with pytest.raises(ValueError, match="^negative dimensions are not allowed$"):
        ax.boxplot(VALUES, bootstrap=-1)
    with pytest.raises(TypeError, match="^object of type 'int' has no len\\(\\)$"):
        ax.boxplot(VALUES, sym=3)


def test_error_bars_compare_their_cap_size():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match="^'>' not supported between instances of "
                                        "'str' and 'int'$"):
        ax.errorbar(VALUES, VALUES, yerr=0.2, capsize='x')
    with pytest.raises(TypeError, match="^'int' object is not iterable$"):
        ax.errorbar(VALUES, VALUES, yerr=0.2, elinestyle=3)


def test_correlations_compute_unusual_series_with_numpy():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="^first array argument cannot be empty$"):
        ax.acorr(np.empty((0,)))
    with pytest.raises(ValueError, match="^arange: cannot compute length$"):
        ax.xcorr(VALUES, VALUES[::-1], maxlags=float('nan'))
    with pytest.raises(TypeError, match="^slice indices must be integers"):
        ax.xcorr(VALUES, VALUES[::-1], maxlags=3.5)


def test_fill_between_data_convert_with_the_units():
    fig, ax = plt.subplots()
    poly = ax.fill_between(VALUES, VALUES + 1, VALUES - 1)
    with pytest.raises(TypeError, match="^ufunc 'isfinite' not supported"):
        poly.set_data(None, VALUES + 1, VALUES - 1)


# -- contour labels ---------------------------------------------------------------

def test_contour_labels_of_any_level_and_colour():
    fig, ax, cs = labelled_contours()
    count = len(cs.labelTexts)
    cs.add_label(1, 1, 0, 0.5, 0.5)
    assert len(cs.labelTexts) == count + 1
    assert cs.labelTexts[-1].get_text() == '0.5'
    assert cs.labelCValues[-1] == 0.5
    assert cs.labelXYs[-1] == (1, 1)
    with pytest.raises(ValueError, match="^rotation must be 'vertical', 'horizontal' "
                                         "or a number, not 0$"):
        cs.add_label(1, 1, '0', 0.5, 0.5)
    with pytest.raises(TypeError, match="^unsupported operand type\\(s\\) for -: "
                                        "'NoneType' and 'float'$"):
        cs.add_label(1, 1, 0, 0.5, None)
    cs.pop_label()
    assert len(cs.labelTexts) == count


def test_contour_labelling_keeps_matplotlibs_attributes():
    fig, ax, cs = labelled_contours()
    assert cs.labelMappable is cs
    assert cs.labelIndiceList == [0, 1, 2, 3, 4]
    assert cs._clabel_zorder == 4
    assert cs.labelFmt.format_ticks([0.5]) == ['0.5']


def test_contour_label_methods_compute_unusual_arguments_as_matplotlib():
    fig, ax, cs = labelled_contours()
    with pytest.raises(TypeError, match="^can't multiply sequence by non-int of type "
                                        "'float'$"):
        cs.too_close(0.5, 0.25, '1.5')
    with pytest.raises(TypeError, match="^cannot unpack non-iterable NoneType object$"):
        cs.find_nearest_contour(float('nan'), 1)
    with pytest.raises(TypeError, match="^'int' object is not iterable$"):
        cs.find_nearest_contour(1, 1, indices=0)
    with pytest.raises(TypeError, match="^object of type 'NoneType' has no len\\(\\)$"):
        cs.print_label(None, 5)
    with pytest.raises(OverflowError, match="^cannot convert float infinity to integer$"):
        cs.locate_label(np.array([[0, 0], [10, 0], [20, 0], [30, 5]]), float('inf'))
    with pytest.raises(TypeError, match="^'float' object cannot be interpreted as an "
                                        "integer$"):
        cs.pop_label(index=1.5)


def test_contour_labels_near_a_point_convert_it_and_cut_with_the_spacing():
    fig, ax, cs = labelled_contours()
    with pytest.raises(TypeError, match="^list indices must be integers or slices, "
                                        "not NoneType$"):
        cs.add_label_near(float('nan'), 2)
    with pytest.raises(ConversionError):
        cs.add_label_near('2', 2)
    with pytest.raises(TypeError, match="^bad operand type for unary -: 'str'$"):
        ax.contour(Z, levels=3).clabel(inline_spacing='5')


def test_contour_labels_format_templates_with_numpy_floats():
    fig, ax = plt.subplots()
    texts = ax.contour(Z, levels=3).clabel(fmt='')
    assert [text.get_text() for text in texts] == [''] * len(texts)


def test_contour_label_levels_that_do_not_match():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError, match="^Specified levels \\['a', 'b'\\] don't match "
                                         "available levels"):
        ax.contour(Z).clabel(levels='ab')


# -- polar axes -------------------------------------------------------------------

@pytest.mark.parametrize("offset", [float('nan'), None])
def test_polar_axes_of_no_angle_fail_their_draw(offset):
    fig = plt.figure()
    ax = fig.add_subplot(projection='polar')
    ax.set_theta_offset(offset)
    with pytest.raises(ValueError, match="^cannot convert float NaN to integer$"):
        fig.canvas.draw()


def test_polar_tick_parameters_read_back_as_matplotlib():
    ax = plt.figure().add_subplot(projection='polar')
    assert ax.xaxis.get_tick_params() == {
        'left': False, 'right': False, 'labelleft': True, 'labelright': False,
        'gridOn': True}


def test_a_theta_locator_of_no_axis_fails_the_draw():
    fig = plt.figure()
    ax = fig.add_subplot(projection='polar')
    ax.plot(np.linspace(0, 6, 7), VALUES)
    ax.xaxis.get_major_locator().set_axis(3)
    with pytest.raises(AttributeError, match="^'int' object has no attribute "
                                             "'get_view_interval'$"):
        fig.canvas.draw()


# -- names, formatters and meshes -------------------------------------------------

def test_artist_lists_are_named_in_the_axes_class():
    fig, ax = plt.subplots()
    with pytest.raises(TypeError, match=r"^_AxesBase.ArtistList.__init__\(\) missing 2 "
                                        "required positional arguments"):
        ax.ArtistList()


def test_child_axes_that_hold_no_attributes():
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError, match="^'NoneType' object has no attribute '_axes' "
                                             "and no __dict__ for setting new attributes$"):
        ax.add_child_axes(None)


def test_log_formatters_round_infinite_exponents():
    from pltrs import ticker
    formatter = ticker.LogFormatterSciNotation()
    with pytest.raises(OverflowError, match="^cannot convert float infinity to integer$"):
        formatter(float('inf'))
    with pytest.raises(TypeError, match="^bad operand type for abs\\(\\): 'str'$"):
        ticker.LogFormatter.__call__(formatter, '100')


def test_mesh_arrays_and_colours_as_matplotlib():
    fig, ax = plt.subplots()
    mesh = ax.pcolormesh(Z)
    with pytest.raises(AttributeError, match="^'str' object has no attribute 'shape'$"):
        mesh.set_array('data')
    with pytest.raises(TypeError, match=r"^_MeshData.get_edgecolor\(\) takes 1 positional "
                                        "argument but 2 were given$"):
        mesh.get_edgecolor(None)


def test_a_date_converter_fails_on_category_units_before_the_view_changes():
    fig, ax = plt.subplots()
    ax.plot(['a', 'b', 'c'], [1, 2, 3])
    view = ax.xaxis.get_view_interval().tolist()
    with pytest.warns(UserWarning, match="^This axis already has a converter set"):
        with pytest.raises(TypeError, match="^tz must be string or tzinfo subclass"):
            ax.xaxis.axis_date(tz='')
    assert ax.xaxis.get_view_interval().tolist() == view


def test_scalar_formatters_keep_their_flags_as_given():
    from pltrs import ticker
    formatter = ticker.ScalarFormatter()
    formatter.set_useLocale(1)
    formatter.set_useMathText('False')
    assert (formatter.get_useLocale(), formatter.useMathText) == (1, 'False')
    formatter.set_useMathText(None)
    assert formatter.get_useMathText() is False
