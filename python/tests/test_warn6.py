"""
What the differential tester's hostile programs (sixth round) found pltrs
warning otherwise than matplotlib 3.11: numpy's runtime warnings where
matplotlib's arithmetic meets infinities and NaN, mathtext's messages
about lists of NaN width as often as matplotlib's parser logs them, and
the warnings of image extents, masked points and hidden axes. Each
expected count is recorded from matplotlib.
"""

import collections
import logging
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.collections as mcollections
import pltrs.image as mimage
import pltrs.patches as mpatches
import pltrs.projections
import pltrs.pyplot as plt
from pltrs import mlab, transforms
from pltrs.mpl_toolkits.axisartist import angle_helper, floating_axes, grid_finder


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


class _Logged(logging.Handler):
    """The messages logged meanwhile."""

    def __init__(self):
        super().__init__(logging.WARNING)
        self.messages = []

    def emit(self, record):
        self.messages.append(record.getMessage())


def warned(call):
    """The exception *call* raises (its type and message, or None) and how
    often each warning and logged message (its first line) came."""
    logged = _Logged()
    logging.getLogger().addHandler(logged)
    try:
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            try:
                call()
                raised = None
            except Exception as exc:  # noqa: BLE001 - recorded, as the diff does
                raised = f"{type(exc).__name__}: {exc}"
    finally:
        logging.getLogger().removeHandler(logged)
    counts = collections.Counter(
        str(message).split("\n")[0]
        for message in [warning.message for warning in caught] + logged.messages)
    return raised, dict(counts)


# -- mathtext's Hlist messages --------------------------------------------------------

UNDERFUL_WIDE = "Underful Hlist: Hlist<w=nan h=1.03 d=0.00 s=0.00>["
UNDERFUL_SUPER = "Underful Hlist: Hlist<w=nan h=0.77 d=0.00 s=0.00>["
UNDERFUL_DEGREES = "Underful Hlist: Hlist<w=nan h=1.03 d=0.02 s=0.00>["
NAN_POSITION = "ValueError: cannot convert float NaN to integer"


def test_math_tick_labels_of_nan_size_log_once_per_parse():
    # The renderer parses a label as it measures it; its draw finds the
    # parse kept.
    fig, ax = plt.subplots()
    ax.set_xticks([0.5], ['$x^2$'], fontsize=float('nan'))
    assert warned(fig.canvas.draw) == (NAN_POSITION, {UNDERFUL_WIDE: 1, UNDERFUL_SUPER: 3})
    # Another figure parses with a renderer of its own.
    fig, ax = plt.subplots()
    ax.set_xticks([0.5], ['$x^2$'], fontsize=float('nan'))
    assert warned(fig.canvas.draw) == (NAN_POSITION, {UNDERFUL_WIDE: 1, UNDERFUL_SUPER: 3})


def floating_axes_drawn():
    fig = plt.figure()
    polar = (transforms.Affine2D().scale(np.pi / 180, 1)
             + pltrs.projections.polar.PolarTransform())
    helper = floating_axes.GridHelperCurveLinear(
        polar, extremes=(0, 90, 2, 6), grid_locator1=angle_helper.LocatorDMS(4),
        tick_formatter1=angle_helper.FormatterDMS(),
        grid_locator2=grid_finder.MaxNLocator(4))
    ax = fig.add_subplot(axes_class=floating_axes.FloatingAxes, grid_helper=helper)
    fig.canvas.draw()
    return fig, ax.axis['bottom'].major_ticklabels


@pytest.mark.parametrize("antialiased, counts", [(True, (4, 12)), (False, (5, 15))])
def test_axis_artist_labels_of_nan_size_log_once_per_parse(antialiased, counts):
    # Drawing a label without antialiasing parses it again.
    fig, labels = floating_axes_drawn()
    labels.set_antialiased(antialiased)
    fig.canvas.draw()
    labels.set_fontsize(float('nan'))
    assert warned(fig.canvas.draw) == (NAN_POSITION, {
        "Underful Hlist: Hlist<w=nan h=0.66 d=0.00 s=0.00>[": counts[0],
        UNDERFUL_DEGREES: counts[1]})


def test_axis_artist_labels_of_infinite_spacing_warn_as_their_layout():
    fig, labels = floating_axes_drawn()
    labels.set_linespacing(float('inf'))
    assert warned(fig.canvas.draw) == (NAN_POSITION, {
        "invalid value encountered in scalar subtract": 5,
        "invalid value encountered in scalar add": 1})


def test_text_layout_of_numpy_metrics_warns_plain_text_does_not():
    fig = plt.figure()
    fig.text(0.5, 0.5, '$x$', linespacing=float('inf'), rotation=180,
             rotation_mode='anchor', ha='center')
    assert warned(fig.canvas.draw) == (NAN_POSITION, {
        "invalid value encountered in scalar add": 2,
        "invalid value encountered in scalar subtract": 4})
    fig = plt.figure()
    fig.text(0.5, 0.5, 'x', linespacing=float('inf'), rotation=180,
             rotation_mode='anchor', ha='center')
    assert warned(fig.canvas.draw) == (NAN_POSITION, {})


# -- aspect, scales ---------------------------------------------------------------------

@pytest.mark.parametrize("scale, outcome", [
    ('logit', (None, {"overflow encountered in power": 1})),
    ('log', ("ValueError: Axis limits cannot be NaN or Inf",
             {"overflow encountered in exp": 1})),
])
def test_data_limits_for_an_aspect_overflow_the_inverse_scale(scale, outcome):
    fig, ax = plt.subplots()
    ax.plot([0, 1000], [0.4, 0.6])
    ax.set_yscale(scale)
    ax.set_aspect(1, adjustable='datalim')
    assert warned(ax.apply_aspect) == outcome


@pytest.mark.parametrize("make", [
    lambda ax: ax.specgram(np.sin(np.arange(512) * 0.3), NFFT=64, Fs=2, noverlap=0),
    lambda ax: ax.imshow(np.ones((2, 2)), extent=(0, 1, 0, 1)),
])
def test_extents_a_log_axis_makes_identical_warn(make):
    fig, ax = plt.subplots()
    ax.set_yscale('log')
    ax.set_ylim(1, 10, auto=True)
    assert warned(lambda: make(ax)) == (None, {
        "Attempt to set non-positive ylim on a log-scaled axis will be ignored.": 1,
        "Attempting to set identical low and high ylims makes transformation singular; "
        "automatically expanding.": 1})


def test_hidden_axes_are_not_ticked_with_warnings():
    fig, ax = plt.subplots()
    twin = ax.twiny()
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        ax.set_ylim(-1, 1)
        ax.set_yscale('logit')
        ax.set_animated(True)
    assert not twin.yaxis.get_visible()
    assert warned(fig.canvas.draw) == (None, {})


# -- streamplot, arrows -----------------------------------------------------------------

STREAM = (np.arange(6), np.arange(5), np.ones((5, 6)),
          np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)))


@pytest.mark.parametrize("kwargs, warning, count", [
    ({"arrowsize": 0}, "invalid value encountered in scalar divide", 264),
    ({"arrowsize": float('inf')}, "invalid value encountered in scalar divide", 264),
    ({"integration_max_step_scale": float('inf')},
     "invalid value encountered in scalar multiply", 60),
])
def test_streamplots_warn_as_their_arithmetic(kwargs, warning, count):
    fig, ax = plt.subplots()
    assert warned(lambda: ax.streamplot(*STREAM, **kwargs)) == (None, {warning: count})


def test_streamplot_arrows_of_unusable_size_fail_before_warning():
    fig, ax = plt.subplots()
    assert warned(lambda: ax.streamplot(*STREAM, arrowsize='2')) == (
        "TypeError: can't multiply sequence by non-int of type 'float'", {})


def test_arrow_heads_of_no_size_warn_where_their_path_is_made():
    fig, ax = plt.subplots()
    arrow = mpatches.FancyArrowPatch((0, 0), (1, 1), arrowstyle='-|>', mutation_scale=0)
    divide = {"invalid value encountered in scalar divide": 2}
    assert warned(lambda: ax.add_patch(arrow)) == (None, divide)
    assert warned(arrow.get_path) == (None, divide)
    assert warned(fig.canvas.draw) == (None, divide)


# -- images, collections, histograms, estimates -------------------------------------------

def test_images_magnified_infinitely_read_their_size_as_matplotlib():
    fig, ax = plt.subplots()
    image = ax.imshow(np.arange(12.0).reshape(3, 4))
    fig.canvas.draw()
    assert warned(lambda: image.make_image(fig.canvas.get_renderer(),
                                           magnification=float('inf'))) == (
        NAN_POSITION, {"invalid value encountered in scalar subtract": 3,
                       "invalid value encountered in scalar multiply": 4})


def test_bilinear_nonuniform_images_at_nan_warn_casting_each_channel():
    fig, ax = plt.subplots()
    image = mimage.NonUniformImage(ax, interpolation='bilinear')
    image.set_data([float('nan'), 1, 3], [0, 2, 3], np.arange(9.0).reshape(3, 3))
    ax.add_image(image)
    ax.set_xlim(0, 3)
    ax.set_ylim(0, 3)
    assert warned(fig.canvas.draw) == (None, {"invalid value encountered in cast": 4})


def test_ellipses_at_infinite_angles_warn_in_sin_and_cos():
    fig, ax = plt.subplots()
    ax.add_collection(mcollections.EllipseCollection(
        [1, 2], [1, 1], [0, float('inf')], offsets=[[0, 0], [1, 1]],
        offset_transform=ax.transData))
    assert warned(fig.canvas.draw) == (None, {
        "invalid value encountered in sin": 1, "invalid value encountered in cos": 1})


def test_histogram_bars_on_an_infinite_bottom_convert_their_heights():
    fig, ax = plt.subplots()
    raised, counts = warned(
        lambda: ax.hist([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5], bottom=float('inf')))
    assert raised is None
    assert counts["invalid value encountered in subtract"] == 10


def test_estimates_at_masked_points_follow_matplotlibs_loops():
    kde = mlab.GaussianKDE(np.array([0.0, 1.0, 2.0, 4.0, 7.0, 8.0]))
    few = np.ma.masked_invalid([np.nan, 2.5, 5.0])
    raised, counts = warned(lambda: kde.evaluate(few))
    assert (raised, counts) == (None, {"Warning: converting a masked element to nan.": 1})
    estimate = kde.evaluate(few)
    assert type(estimate) is np.ndarray
    assert np.isnan(estimate[0]) and np.allclose(estimate[1:], [0.09714699, 0.07978634])
    many = np.ma.masked_invalid([np.nan, 2.5, 5.0, 1, 2, 3, 4])
    assert warned(lambda: kde.evaluate(many)) == (None, {})
    estimate = kde.evaluate(many)
    assert isinstance(estimate, np.ma.MaskedArray)
    assert list(np.ma.getmaskarray(estimate)) == [True] + [False] * 6
