"""
What the impact report on pltrs and matplotlib 3.11 found pltrs doing
otherwise in limits, margins, aspects and tick labels (task DIFFX-C),
reduced to matplotlib's API: twinned Axes refusing to adjust their box,
limits no float holds, limits a logit axis autoscales to NaN, the
rotation of a polar Axes' tick labels, margins added in the scale's own
space with a secondary axis, which Axes of a shared group autoscales
first, and the limit a log axis keeps for an image's side that is not
positive. Every expected value was recorded from matplotlib.
"""

import io

import numpy as np
import pytest
from PIL import Image

import pltrs
import pltrs.pyplot as plt

TWINNED_BOX = ("Adjustable 'box' is not allowed in a twinned Axes; "
               "use 'datalim' instead")

I = np.arange(30.)
X = np.linspace(0.2, 9, 30)
Y = np.sin(I * 0.6) * 3 + I * 0.3
Z = np.add.outer(np.sin(np.arange(9.) * 0.7), np.cos(np.arange(11.) * 0.4)) * 10 + 12


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _pixels(fig):
    buffer = io.BytesIO()
    fig.savefig(buffer, format="png")
    return np.asarray(Image.open(buffer))


def _twinned_box(layout=None, twin="x"):
    fig, ax = plt.subplots(layout=layout)
    ax.plot([0, 1], [0, 2])
    twin = ax.twinx() if twin == "x" else ax.twiny()
    ax.set_aspect("equal", adjustable="box")
    return fig, ax, twin


@pytest.mark.parametrize("call", [
    lambda fig, ax: fig.canvas.draw(),
    lambda fig, ax: fig.savefig(io.BytesIO(), format="svg"),
    lambda fig, ax: fig.savefig(io.BytesIO(), bbox_inches="tight"),
    lambda fig, ax: fig.draw_without_rendering(),
    lambda fig, ax: ax.apply_aspect(),
    lambda fig, ax: ax.get_position(),
    lambda fig, ax: ax.get_tightbbox(fig.canvas.get_renderer()),
])
def test_twinned_axes_refuse_to_adjust_their_box(call):
    fig, ax, _ = _twinned_box()
    with pytest.raises(RuntimeError, match=TWINNED_BOX):
        call(fig, ax)


@pytest.mark.parametrize("layout, twin", [("constrained", "x"), ("tight", "x"), (None, "y")])
def test_twinned_axes_refuse_their_box_with_any_layout_and_twin(layout, twin):
    fig, _, _ = _twinned_box(layout, twin)
    with pytest.raises(RuntimeError, match=TWINNED_BOX):
        fig.canvas.draw()
    # matplotlib's next draw refuses it again.
    with pytest.raises(RuntimeError, match=TWINNED_BOX):
        fig.canvas.draw()


def test_twins_refuse_a_box_whichever_of_them_has_it():
    fig, ax = plt.subplots()
    twin = ax.twinx()
    twin.set_aspect(2, adjustable="box")
    with pytest.raises(RuntimeError, match=TWINNED_BOX):
        fig.canvas.draw()
    fig, ax = plt.subplots()
    ax.twinx()
    ax.set_box_aspect(1)
    assert ax.get_adjustable() == "datalim"
    ax.set_adjustable("box")
    with pytest.raises(RuntimeError, match=TWINNED_BOX):
        fig.canvas.draw()


def test_twinned_axes_adjust_their_data_limits_and_leave_their_window_extent():
    fig, ax, twin = _twinned_box()
    # matplotlib's get_window_extent applies no aspect.
    assert ax.get_window_extent().bounds == pytest.approx((80.0, 52.8, 496.0, 369.6))
    # The twin itself has no aspect to apply.
    twin.apply_aspect()
    ax.set_adjustable("datalim")
    fig.canvas.draw()
    assert ax.get_ylim() == pytest.approx((0.5901612903225806, 1.4098387096774194), rel=1e-12)
    assert twin.get_xlim() == pytest.approx((-0.05, 1.05))


def test_timedelta64_limits_fail_as_matplotlib_takes_them():
    fig, ax = plt.subplots()
    message = "float() argument must be a string or a real number, not 'datetime.timedelta'"
    with pytest.raises(TypeError, match=message.replace("(", r"\(").replace(")", r"\)")):
        ax.set_xlim(np.timedelta64(1, "D"), np.timedelta64(5, "D"))
    assert ax.get_xlim() == (0.0, 1.0)
    with pytest.raises(TypeError, match="float"):
        ax.set_xlim(np.array([1, 3], dtype="timedelta64[s]"))
    with pytest.raises(TypeError, match="float"):
        ax.set_xbound(np.timedelta64(1, "s"), np.timedelta64(3, "s"))
    # Beside a float (the current upper limit), the comparison fails first.
    with pytest.raises(TypeError, match="did not contain a loop"):
        ax.set_ylim(np.timedelta64(1, "s"))
    assert ax.get_ylim() == (0.0, 1.0)


def test_limits_a_logit_axis_autoscales_to_nan_are_refused():
    # A hexbin's smallest positive y is above one, which matplotlib's
    # LogitLocator.nonsingular turns into limits no logit maps.
    fig, ax = plt.subplots()
    ax.hexbin(X, Y, gridsize=4)
    with pytest.raises(ValueError, match="Axis limits cannot be NaN or Inf"):
        ax.set_yscale("logit")
        fig.canvas.draw()
    assert ax.get_ylim() == (0.0, 1.0)
    ax.set_aspect("equal", adjustable="datalim")
    with pytest.raises(ValueError, match="Axis limits cannot be NaN or Inf"):
        ax.apply_aspect()


def test_lines_on_a_logit_axis_still_autoscale():
    fig, ax = plt.subplots()
    ax.plot(X, Y)
    ax.set_yscale("logit")
    ax.set_aspect("equal", adjustable="datalim")
    fig.canvas.draw()
    assert ax.get_xlim() == pytest.approx((-0.24, 9.44))
    assert ax.get_ylim() == pytest.approx((0.0002, 0.9998), abs=1e-4)


def _polar(**tick_params):
    fig = plt.figure()
    # Not a subplot, which autofmt_xdate would move.
    ax = fig.add_axes((0.1, 0.1, 0.8, 0.8), projection="polar")
    ax.plot([0, 1, 2], [1, 2, 3])
    if tick_params:
        ax.tick_params(**tick_params)
    return fig, ax


def test_polar_tick_labels_turn_by_the_tick_parameter_alone():
    plain = _pixels(_polar()[0])
    # autofmt_xdate's rotation, and a rotation given to the labels, are
    # replaced at each draw by the labelrotation tick parameter.
    fig, _ = _polar()
    fig.autofmt_xdate(ha="center")
    assert np.array_equal(_pixels(fig), plain)
    fig, ax = _polar()
    fig.canvas.draw()
    for label in ax.get_xticklabels() + ax.get_yticklabels():
        label.set_rotation(30)
    assert np.array_equal(_pixels(fig), plain)
    turned = _pixels(_polar(axis="x", labelrotation=45)[0])
    assert not np.array_equal(turned, plain)
    fig, ax = _polar(axis="x", labelrotation=45)
    fig.canvas.draw()
    for label in ax.get_xticklabels():
        label.set_rotation(10)
    assert np.array_equal(_pixels(fig), turned)


@pytest.mark.parametrize("scale, limits", [
    ("symlog", (0.6828962729797357, 2.451671996346637)),
    ("asinh", (0.697109101758894, 2.3576055128798488)),
])
def test_a_scale_set_after_a_secondary_axis_adds_margins_in_its_space(scale, limits):
    for secondary in ("secondary_yaxis", "secondary_xaxis"):
        fig, ax = plt.subplots()
        ax.plot([0.75, 2.25], [0, 1])
        getattr(ax, secondary)("right" if secondary == "secondary_yaxis" else "top")
        ax.set_xscale(scale)
        fig.canvas.draw()
        assert ax.get_xlim() == pytest.approx(limits, rel=1e-12)


def test_a_shared_view_takes_the_margins_of_the_first_axes_drawn():
    # The image's Axes applies its aspect first, its sibling's secondary
    # axis after it, so the shared y limits take its margins (0.05).
    fig, (a, b) = plt.subplots(1, 2, sharey=True)
    a.imshow(Z, origin="lower")
    b.plot(X[:10], Y[:10])
    b.secondary_yaxis("right")
    b.margins(0, 0.1)
    fig.canvas.draw()
    assert a.get_ylim() == pytest.approx((-1.0429185178328981, 8.5), rel=1e-12)


def test_a_datalim_aspect_autoscales_the_view_it_shares_first():
    # The tight Axes has no margins; the datalim one reads the shared view
    # first, as matplotlib's draw applies the aspects before the layout.
    fig, (a, b) = plt.subplots(1, 2, sharey=True)
    a.plot(X, Y)
    a.autoscale(tight=True)
    b.plot(X[:10], Y[:10])
    b.set_aspect(0.5, adjustable="datalim")
    fig.canvas.draw()
    assert a.get_ylim() == pytest.approx((-1.1227671843152898, 10.6312466874556), rel=1e-12)
    assert b.get_xlim() == pytest.approx((-0.2269583097073562, 3.3579927924659776),
                                         rel=1e-12)


def test_a_log_axis_keeps_its_autoscaled_limit_for_an_image_side_not_positive():
    fig, ax = plt.subplots()
    ax.plot([1, 2], [1, 2])
    ax.set_yscale("log")
    ax.plot([1, 2], [1, 3])
    with pytest.warns(UserWarning, match="non-positive ylim"):
        ax.imshow(np.ones((9, 11)))
    assert ax.get_ylim() == pytest.approx((8.5, 9.45997245826513), rel=1e-12)
    assert not ax.yaxis_inverted()


def test_polar_axes_lay_out_without_what_their_circle_clips():
    # A line reaching 8.6 radii out is clipped to the polar patch, which
    # matplotlib's tight boxes leave out: the constrained layout places
    # the Axes as for its decorations alone.
    fig = plt.figure(figsize=(4, 3), dpi=60, layout="constrained")
    ax = fig.add_subplot(projection="polar")
    ax.plot(np.arange(24.), np.arange(24.) ** 2)
    ax.set_ylim(0, 50)
    renderer = fig.canvas.get_renderer()
    assert ax.get_tightbbox(renderer).extents == pytest.approx(
        (32.53, 3.97, 208.47, 174.23), abs=0.01)
    fig.canvas.draw()
    assert ax.get_position().bounds == pytest.approx((0.2014, 0.1019, 0.5972, 0.7963),
                                                     abs=1e-4)
