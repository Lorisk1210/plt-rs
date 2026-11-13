"""
What the differential tester's hostile programs (second round) found
pltrs doing otherwise than matplotlib 3.11 with 2-D Axes, their axes and
tick helpers, paths, boxes, polar Axes, patch styles, streamplots and
spectra. Each expected value is recorded from matplotlib.
"""

import math
import re
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import ticker
from pltrs.patches import ArrowStyle, BoxStyle, ConnectionStyle
from pltrs.path import Path
from pltrs.transforms import Bbox
from pltrs.units import ConversionError

import build_facts


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def nan_free(values):
    """*values* as floats, NaN as None, for comparisons."""
    return [None if math.isnan(v) else float(v) for v in values]


# -- the tick space -------------------------------------------------------------

def test_an_axes_of_no_size_has_no_tick_space():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        fig.tight_layout(pad=float("nan"))
    assert nan_free(ax.get_position().bounds) == [None] * 4
    with pytest.raises(ValueError, match="^cannot convert float NaN to integer$"):
        ax.xaxis.get_tick_space()
    with pytest.raises(ValueError, match="^cannot convert float NaN to integer$"):
        fig.canvas.draw()


def test_an_axes_of_negative_size_has_a_negative_tick_space():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    ax.set_position([0.5, 0.1, -0.3, 0.5])
    assert (ax.xaxis.get_tick_space(), ax.yaxis.get_tick_space()) == (-5, 8)
    fig.canvas.draw()
    assert list(ax.get_xticks()) == [-1.0, 0.0, 1.0, 2.0]


def test_a_nan_edge_after_a_finite_one_leaves_the_box_no_width():
    fig, ax = plt.subplots()
    ax.set_position([0.1, 0.1, float("nan"), 0.5])
    assert ax.xaxis.get_tick_space() == 0
    fig.canvas.draw()


def test_ticks_widen_the_view_with_pythons_min_and_max():
    fig, ax = plt.subplots()
    ax.plot([0, 6], [0, 1])
    ax.xaxis.set_ticks([float("nan"), 1, 2])
    assert nan_free(ax.xaxis.get_view_interval()) == [None, None]
    ax.xaxis.set_ticks([0, 1, 2])
    assert list(ax.xaxis.get_view_interval()) == [2.0, 0.0]


# -- box aspects ----------------------------------------------------------------

@pytest.mark.parametrize("aspect", [-1.5, 0.0])
def test_box_aspects_that_are_not_positive_fail_when_drawn(aspect):
    fig, ax = plt.subplots()
    ax.set_box_aspect(aspect)
    assert ax.get_box_aspect() == aspect
    assert ax.get_adjustable() == "datalim"
    with pytest.raises(ValueError,
                       match="^'box_aspect' and 'fig_aspect' must be positive$"):
        fig.canvas.draw()


def test_a_failed_contour_takes_its_artists_back():
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError,
                       match="QuadContourSet.set.. got an unexpected keyword"):
        ax.contour(np.outer(np.arange(4), np.arange(5)), nonexistent_kw=1)
    assert len(ax.collections) == 0
    fig.canvas.draw()


# -- patch styles ---------------------------------------------------------------

@pytest.mark.parametrize("make, message", [
    (lambda: BoxStyle.Square(nonexistent_kw=1),
     "BoxStyle.Square.__init__() got an unexpected keyword argument 'nonexistent_kw'"),
    (lambda: BoxStyle.LArrow(None, None, None, None),
     "BoxStyle.RArrow.__init__() takes from 1 to 4 positional arguments but 5 were given"),
    (lambda: ArrowStyle.CurveA(nonexistent_kw=1),
     "ArrowStyle._Curve.__init__() got an unexpected keyword argument 'nonexistent_kw'"),
    (lambda: ArrowStyle.Curve(None),
     "ArrowStyle.Curve.__init__() takes 1 positional argument but 2 were given"),
    (lambda: ConnectionStyle.Arc3(None, None),
     "ConnectionStyle.Arc3.__init__() takes from 1 to 2 positional arguments but 3 were given"),
    (lambda: BoxStyle.DArrow(head_angle='90'),
     "not all arguments converted during string formatting"),
])
def test_style_constructors_fail_as_matplotlibs(make, message):
    with pytest.raises(TypeError) as error:
        make()
    assert str(error.value) == message


def test_arrow_box_styles_check_their_heads():
    with pytest.raises(ValueError, match="^'head_width' must be nonnegative$"):
        BoxStyle.RArrow(head_width=-1.5)
    with pytest.raises(ValueError, match="^'head_angle' must be nonzero$"):
        BoxStyle.RArrow(head_angle=720)
    assert BoxStyle.Round(pad=1).pad == 1


# -- locators and formatters ----------------------------------------------------

def test_locators_compute_with_parameters_that_are_no_numbers():
    multiple = ticker.MultipleLocator(0.5)
    multiple.set_params(base=0.25, offset=(1.0, -1.0))
    with pytest.raises(TypeError, match="^unsupported operand type.s. for -=: 'int' and 'tuple'$"):
        multiple.tick_values(-1, 2)
    assert multiple.view_limits(-1.5, 2.7) == (-1.5, 2.7)
    index = ticker.IndexLocator(1, 0)
    index.set_params(offset=(1, 2))
    with pytest.raises(TypeError, match="^can only concatenate str .not \"tuple\". to str$"):
        index.tick_values('0', 10)
    asinh = ticker.AsinhLocator(1.0)
    asinh.set_params(numticks=5.5)
    with pytest.raises(TypeError, match="^'float' object cannot be interpreted as an integer$"):
        asinh.tick_values(-1, 100)


def test_locators_compare_limits_that_are_no_numbers_as_matplotlibs():
    with pytest.raises(TypeError, match="^'<=' not supported between instances of 'str' and 'float'$"):
        ticker.LogLocator().tick_values('1', 1000)
    with pytest.raises(TypeError, match="^'>' not supported between instances of 'str' and 'int'$"):
        ticker.LogitLocator().nonsingular('0', 1)
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        assert ticker.LogitLocator().nonsingular(0, float('nan')) == (1e-07, 0.9999999)
    symlog = ticker.SymmetricalLogLocator(linthresh=1, base=10)
    with pytest.raises(TypeError, match="^'<' not supported between instances of 'int' and 'str'$"):
        symlog.view_limits('-50', 50)
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        with pytest.raises(ValueError):
            symlog.tick_values(float('inf'), 100)
    floor_divide = "invalid value encountered in scalar floor_divide"
    assert ((floor_divide in [str(w.message) for w in caught])
            == build_facts.FLOOR_DIVIDING_INFINITY_WARNS)


def test_a_negative_count_of_symlog_ticks_is_matplotlibs():
    symlog = ticker.SymmetricalLogLocator(linthresh=1, base=10)
    symlog.set_params(subs=[1, 5], numticks=-1)
    assert list(symlog.tick_values(-100, 100)) == [
        -100.0, -500.0, -10.0, -50.0, -1.0, -5.0, 0.0, 1.0, 5.0, 10.0, 50.0, 100.0, 500.0]


def test_a_locator_of_unusable_parameters_fails_drawing_as_matplotlibs():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    locator = ticker.MultipleLocator(0.5)
    ax.xaxis.set_major_locator(locator)
    locator.set_params(offset=(1.0, -1.0))
    with pytest.raises(ValueError, match="truth value of an array"):
        fig.canvas.draw()
    locator.set_params(offset=0.1)
    fig.canvas.draw()
    assert np.allclose(ax.get_xticks(), [-0.4, 0.1, 0.6, 1.1])


def test_percent_formatter_keeps_what_matplotlib_keeps():
    fig, ax = plt.subplots()
    formatter = ticker.PercentFormatter(xmax=5)
    formatter.set_axis(ax.xaxis)
    with pytest.raises(TypeError, match="^unsupported operand type.s. for /: 'str' and 'float'$"):
        formatter('0.5')
    formatter.symbol = 3
    formatter.set_locs([0, 1])
    with pytest.raises(TypeError, match=r'^can only concatenate str \(not "int"\) to str$'):
        formatter(0.5)
    with pytest.raises(TypeError, match=r'^can only concatenate str \(not "int"\) to str$'):
        formatter.format_data_short(0.5)
    formatter.symbol = '%'
    formatter.set_locs(None)
    assert formatter(0.5) == '10.0%'
    with pytest.raises(TypeError) as error:
        formatter.set_locs([0, 1], nonexistent_kw=1)
    assert str(error.value) == (
        "Formatter.set_locs() got an unexpected keyword argument 'nonexistent_kw'")


def test_formatters_take_their_first_steps_with_what_they_are_given():
    with pytest.raises(TypeError, match="^'<=' not supported between instances of 'str' and 'int'$"):
        ticker.LogitFormatter()('0.5')
    assert ticker.LogitFormatter()(0.5, pos='x') == r'$\mathdefault{\frac{1}{2}}$'
    with pytest.raises(TypeError, match="^'<' not supported between instances of 'str' and 'float'$"):
        ticker.LogitFormatter().format_data_short('0.25')
    fig, ax = plt.subplots()
    scalar = ax.xaxis.get_major_formatter()
    with pytest.raises(TypeError, match="^object of type 'NoneType' has no len..$"):
        scalar.set_locs(None)


# -- polar Axes -----------------------------------------------------------------

def test_polar_setters_take_values_as_numpy_does():
    fig, ax = plt.subplots(subplot_kw={'projection': 'polar'})
    with pytest.raises(TypeError, match="ufunc 'deg2rad' not supported"):
        ax.set_thetamin('45')
    ax.set_theta_offset(None)
    assert math.isnan(ax.get_theta_offset())
    with pytest.raises(KeyError):
        ax.set_theta_zero_location(None)
    with pytest.raises(ValueError, match="^setting an array element with a sequence"):
        ax.set_theta_zero_location('N', offset=(1.0, -1.0))
    ax.set_theta_zero_location('W', offset=10)
    assert ax.get_theta_offset() == pytest.approx(np.pi + np.deg2rad(10))


def test_polar_radial_grids_convert_with_the_angles_units():
    fig, ax = plt.subplots(subplot_kw={'projection': 'polar'})
    with pytest.raises(ConversionError, match="^Failed to convert value.s. to axis units: 'ab'$"):
        ax.set_rgrids('ab')
    assert type(ax.xaxis.get_converter()).__name__ == "StrCategoryConverter"
    lines, labels = ax.set_rgrids([0.5, 1.0], angle=30)
    # Degrees through radians and back, as matplotlib's transform holds them.
    assert (len(lines), ax.get_rlabel_position()) == (2, 29.999999999999996)


def test_theta_ticks_turn_by_their_location_as_matplotlibs():
    fig, ax = plt.subplots(subplot_kw={'projection': 'polar'})
    tick = ax.xaxis.get_major_ticks()[0]
    with pytest.raises(TypeError, match="^can't multiply sequence by non-int of type 'numpy.float64'$"):
        tick.update_position('0.5')
    with pytest.raises(ValueError, match=f"^{build_facts.raised(lambda: math.cos(math.inf))}$"):
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            tick.update_position(float('inf'))


def test_a_theta_view_of_no_number_fails_as_pythons_int():
    fig, ax = plt.subplots(subplot_kw={'projection': 'polar'})
    ax.set_thetamin(float('nan'))
    with pytest.raises(ValueError, match="^cannot convert float NaN to integer$"):
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            fig.canvas.draw()


# -- paths and boxes ------------------------------------------------------------

def test_unit_shapes_of_counts_that_are_no_positive_integers():
    assert len(Path.unit_regular_polygon(-1).vertices) == 0
    with pytest.raises(ZeroDivisionError):
        Path.unit_regular_polygon(0)
    assert len(Path.unit_regular_polygon(5.5).vertices) == 7
    with pytest.raises(TypeError, match="^unsupported operand type.s. for .: 'float' and 'NoneType'$"):
        Path.circle(radius=None)
    with pytest.raises(ValueError, match="^operands could not be broadcast together"):
        Path.circle(center=(0.3, 0.4, 0.4))


def test_box_predicates_read_what_matplotlib_reads():
    bbox = Bbox.from_bounds(0.1, 0.2, 0.5, 0.4)
    with pytest.raises(TypeError, match="^'<=' not supported between instances of 'float' and 'NoneType'$"):
        bbox.containsx(None)
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'extents'"):
        bbox.overlaps(None)
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'xmin'"):
        Bbox.intersection(None, Bbox.unit())
    with pytest.raises(TypeError, match="^cannot unpack non-iterable NoneType object$"):
        bbox.anchored(None, Bbox.unit())
    with pytest.raises(AttributeError, match="'int' object has no attribute 'size'"):
        bbox.shrunk_to_aspect(2, container=3)
    with pytest.raises(ValueError, match=r"^bbox array must have shape \(N, 2, 2\), got \(1, 1, 1\)$"):
        bbox.count_overlaps([None])
    assert bbox.overlaps(Bbox.unit()) and not bbox.fully_containsx(0.1)


# -- streamplots and spectra ----------------------------------------------------

def test_streamplot_checks_its_grid_and_arguments_as_matplotlibs():
    fig, ax = plt.subplots()
    x, y, u = np.arange(6), np.arange(5), np.ones((5, 6))
    with pytest.raises(ValueError, match="^'x' values must be equally spaced$"):
        ax.streamplot(np.array([0, 1, 2, 3, 4, np.inf]), y, u, u)
    message = build_facts.raised(lambda: 30 * np.array(["6.0", "6.0"]))
    with pytest.raises(TypeError, match=f"^{re.escape(message)}$"):
        ax.streamplot(x, y, u, u, density='6.0')
    with pytest.raises(AttributeError, match="^'list' object has no attribute 'shape'$"):
        ax.streamplot(x, y, u.tolist(), u)
    with pytest.raises(TypeError, match="^unsupported operand type.s. for /=: 'str' and 'float'$"):
        ax.streamplot(x, y, u, u, maxlength='2')
    with pytest.raises(IndexError, match="^too many indices for array"):
        ax.streamplot(x, y, u, u, start_points=[])


def test_spectra_fail_as_matplotlibs_mlab():
    fig, ax = plt.subplots()
    signal = np.sin(np.arange(256) * 0.3)
    with pytest.raises(TypeError, match="^len.. of unsized object$"):
        ax.psd(None, NFFT=64)
    with pytest.raises(ValueError, match="^Coherence is calculated by averaging"):
        ax.cohere(signal[:1], signal, NFFT=64)
    with pytest.raises(ZeroDivisionError):
        ax.magnitude_spectrum(signal, Fs=0)
    with pytest.raises(TypeError, match="^not all arguments converted during string formatting$"):
        ax.angle_spectrum(signal, pad_to='128')
    with pytest.raises(ValueError, match="^noverlap must be less than NFFT$"):
        ax.phase_spectrum(signal[:0])


def test_spectra_warn_as_matplotlibs():
    fig, ax = plt.subplots()
    signal = np.sin(np.arange(256) * 0.3)
    pxy, freqs = ax.csd(signal, None, NFFT=64)
    pxx, _ = ax.psd(signal, NFFT=64)
    assert np.allclose(pxy.real, pxx)
    with pytest.warns(RuntimeWarning, match="divide by zero encountered in log10"):
        ax.psd(np.zeros(64), NFFT=64)
    with pytest.warns(UserWarning, match=r"^Only one segment is calculated since parameter "
                                         r"NFFT \(=64\) >= signal length \(=0\).$"):
        with pytest.raises(ValueError):
            ax.specgram(signal[:0], NFFT=64, Fs=2)


def test_native_arithmetic_warnings_follow_numpys_error_state():
    fig, ax = plt.subplots()
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        with np.errstate(divide="ignore"):
            ax.psd(np.zeros(64), NFFT=64)
    with np.errstate(divide="raise"):
        with pytest.raises(FloatingPointError, match="^divide by zero encountered in log10$"):
            ax.psd(np.zeros(64), NFFT=64)


def test_radial_axes_wrap_the_locators_of_integer_ticks():
    from pltrs.projections.polar import RadialLocator
    fig, ax = plt.subplots(subplot_kw={'projection': 'polar'})
    ax.set_rticks([0, 1, 2, 3])
    locator = ax.yaxis.get_major_locator()
    assert isinstance(locator, RadialLocator)
    assert isinstance(locator.base, ticker.FixedLocator)
    assert list(locator.base.locs) == [0, 1, 2, 3]
