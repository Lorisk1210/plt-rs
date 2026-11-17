"""
The axis' parts as matplotlib's code leaves them (AXIS1): tick parameters
reaching the ticks handed out, the spines of polar and geographic Axes
read back as arcs and circles, a secondary axis keeping its scale's
functions, minor divisions settled once, axisartist under a tight layout
and its axis line, the 3-D axis label's rotation and box and the panes'
visibility, ticks placed on a removed Axes, what a draw through a
renderer leaves on an axis, and deprecation warnings shown once per line.
Every expected value was recorded from matplotlib 3.11.
"""

import io
import types
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
import pltrs.text as mtext
from pltrs import ticker
from pltrs.mpl_toolkits import axisartist
from pltrs.mpl_toolkits.axisartist import GridHelperCurveLinear
from pltrs.mpl_toolkits.axisartist.axislines import AxesZero
from pltrs.spines import Spine


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def rounded(values, digits=1):
    """*values* (a box's bounds or an array), rounded, as lists."""
    return np.round(np.asarray(getattr(values, "bounds", values), float), digits).tolist()


@pytest.mark.parametrize("drawn", [False, True])
def test_tick_parameters_reach_the_ticks_handed_out(drawn):
    fig, ax = plt.subplots(figsize=(4, 3), dpi=50)
    if drawn:
        fig.canvas.draw()
    ticks = ax.xaxis.get_major_ticks()
    ticks[0].label1.set_color("r")
    ticks[0].tick1line.set_markersize(20)
    ax.xaxis.set_tick_params(length=12, width=2)
    assert (ticks[0].label1.get_color(), ticks[0].tick1line.get_markersize()) == ("r", 12.0)
    ax.xaxis.set_tick_params(labelcolor="g", labelsize=7)
    assert (ticks[1].label1.get_color(), ticks[1].label1.get_fontsize()) == ("g", 7.0)
    ax.xaxis.set_ticks_position("none")
    assert ax.xaxis.get_ticks_position() == "unknown"
    assert ticks[0].tick1line.get_visible() is False
    ax.xaxis.set_tick_params(reset=True, length=5)
    made = ax.xaxis.get_major_ticks()
    # matplotlib makes the ticks anew; those held keep what they had.
    assert made[0] is not ticks[0]
    assert (ticks[0].tick1line.get_markersize(), made[0].tick1line.get_markersize()) == (12.0, 5.0)
    assert (made[0].label1.get_color(), ticks[0].label1.get_color()) == ("black", "g")


def test_tick_marks_have_their_tick_zorder_and_polar_labels_anchor():
    fig, ax = plt.subplots()
    ax.minorticks_on()
    assert ax.xaxis.get_major_ticks()[0].tick1line.get_zorder() == 2.01
    assert ax.xaxis.get_minor_ticks()[0].tick1line.get_zorder() == 2
    ax.xaxis.set_tick_params(zorder=7)
    assert ax.xaxis.get_major_ticks()[0].tick1line.get_zorder() == 7
    polar = plt.figure().add_subplot(projection="polar")
    assert polar.xaxis.get_major_ticks()[0].label1.get_rotation_mode() == "anchor"


@pytest.mark.parametrize("setup, counts, ends", [
    ("full", {"end": 2, "inner": 49, "polar": 49, "start": 2},
     [[134.6, 74.2], [19.1, 74.2], [134.6, 74.2]]),
    ("range", {"end": 2, "inner": 25, "polar": 25, "start": 2},
     [[134.6, 100.5], [37.8, 113.3], [50.6, 16.5]]),
])
def test_polar_spines_read_back_as_arcs(setup, counts, ends):
    fig = plt.figure(figsize=(3, 3), dpi=50)
    ax = fig.add_subplot(projection="polar")
    if setup == "range":
        ax.set_thetamin(30)
        ax.set_thetamax(240)
    ax.plot([0, 1], [0, 1])
    fig.canvas.draw()
    assert {k: len(s.get_path().vertices) for k, s in sorted(ax.spines.items())} == counts
    spine = ax.spines["polar"]
    assert spine._patch_type == "arc"
    drawn = spine.get_transform().transform(spine.get_path().vertices)
    assert rounded(drawn[[0, len(drawn) // 2, -1]]) == ends


def test_inner_arc_spans_the_radial_origin():
    fig = plt.figure(figsize=(3, 3), dpi=50)
    ax = fig.add_subplot(projection="polar")
    ax.set_thetamin(30)
    ax.set_thetamax(240)
    ax.set_rorigin(-0.5)
    ax.set_rmin(0.2)
    ax.plot([0, 1], [0, 1])
    fig.canvas.draw()
    inner = ax.spines["inner"]
    assert rounded(inner.get_patch_transform().get_matrix(), 3)[0] == [0.161, 0.0, 0.5]
    assert rounded(inner.get_window_extent()) == [61.5, 52.6, 37.0, 37.0]


def test_shaped_spines_keep_their_unit_paths_in_axes_coordinates():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
    ax.set_xlim(0, 2)
    ax.set_ylim(0, 2)
    ax.spines["c"] = Spine.circular_spine(ax, (1, 1), 0.5)
    ax.spines["a"] = Spine.arc_spine(ax, "top", (1, 1), 0.7, 10, 100)
    circle = ax.spines["c"]
    assert len(circle.get_path().vertices) == 26
    assert rounded(circle.get_transform().transform(circle.get_path().vertices)[0]) == [135.0, 74.2]
    fig.canvas.draw()
    assert len(ax.spines["a"].get_path().vertices) == 13
    assert rounded(circle.get_window_extent()) == [76.9, 74.2, 116.2, 115.5]


@pytest.mark.parametrize("projection", ["mollweide", "hammer", "aitoff", "lambert"])
def test_a_geographic_axes_has_one_circle_spine(projection):
    fig = plt.figure(figsize=(3, 2), dpi=50)
    ax = fig.add_subplot(projection=projection)
    assert list(ax.spines.keys()) == ["geo"]
    spine = ax.spines["geo"]
    assert (spine.spine_type, spine._patch_type, spine.axis is ax.yaxis) == ("circle", "circle", True)
    assert len(spine.get_path().vertices) == 26
    spine.set_color("r")
    spine.set_linewidth(3)
    fig.canvas.draw()
    pixels = np.asarray(fig.canvas.buffer_rgba())[:, :, :3].astype(int)
    assert ((pixels[:, :, 0] > 200) & (pixels[:, :, 1] < 80)).sum() > 300


def test_a_secondary_axis_keeps_the_scale_of_its_first_functions():
    # matplotlib's `set_functions` remakes the scale only once the parent's
    # scale changes: its draws go on calling the functions before.
    calls = []

    def old(p):
        calls.append(len(np.atleast_1d(p)))
        return 1 / p

    fig, ax = plt.subplots(figsize=(6, 4), dpi=60, layout="constrained")
    ax.set_xlim(1, 100)
    sec = ax.secondary_xaxis("top", functions=(lambda f: 1 / f, old))
    fig.canvas.draw()
    sec.set_functions((lambda f: f * 2, lambda f: f / 2))
    calls.clear()
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", RuntimeWarning)
        fig.canvas.draw()
    assert len(calls) > 0
    assert rounded(sec.get_xlim(), 4) == [2.0, 200.0]


def test_minor_divisions_are_settled_by_the_first_ticks():
    with plt.rc_context({"xtick.minor.ndivs": 3, "xtick.minor.visible": True}):
        fig, ax = plt.subplots()
        ax.set_xlim(0, 10)
        ax.xaxis.set_minor_locator(ticker.AutoMinorLocator())
        fig.canvas.draw()
    assert ax.xaxis.get_minor_locator().ndivs == 3
    fig.canvas.draw()
    assert len(ax.xaxis.get_minorticklocs()) == 10


def test_axisartist_tight_layout_measures_again_for_its_rectangle():
    fig, axs = plt.subplots(2, 2, figsize=(5, 4), dpi=80, layout="tight",
                            subplot_kw={"axes_class": axisartist.Axes})
    for k, ax in enumerate(axs.flat):
        ax.plot(np.arange(5), np.arange(5) ** (k + 1))
        ax.axis["left"].label.set_text(f"y{k}")
        ax.axis["bottom"].label.set_text(f"x{k}")
    fig.canvas.draw()
    assert rounded(axs.flat[0].get_position(), 3) == [0.084, 0.598, 0.371, 0.364]


def test_an_axis_line_is_a_patch_of_matplotlibs_classes():
    fig = plt.figure(figsize=(4, 3), dpi=50)
    ax = fig.add_subplot(axes_class=AxesZero)
    ax.plot([0, 1], [0, 1])
    assert type(ax.axis["left"].line).__name__ == "PathPatch"
    assert ax.axis["left"].line.get_path().vertices.tolist() == [[0.0, 0.0], [0.0, 1.0]]
    ax.axis["xzero"].set_axisline_style("-|>", size=1.5)
    assert type(ax.axis["xzero"].line).__name__ == "FilledArrow"


def test_a_curvilinear_grid_records_the_limits_it_was_laid_out_for():
    helper = GridHelperCurveLinear((lambda x, y: (x + 0.5 * y, y),
                                    lambda x, y: (x - 0.5 * y, y)))
    fig = plt.figure(figsize=(4, 3), dpi=50)
    ax = fig.add_subplot(axes_class=axisartist.Axes, grid_helper=helper)
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    ax.axis["t"] = ax.new_floating_axis(0, 3)
    assert helper._old_limits == (0.0, 10.0, 0.0, 10.0)
    fig.canvas.draw()
    assert len(helper.get_gridlines()) == 19


@pytest.mark.parametrize("rotate, text, rotation", [
    (False, "vertical label", 90.0),
    (None, "ab", 45.0),
    (None, "vertical label", 87.38),
])
def test_a_3d_axis_label_keeps_its_rotation_where_it_is_not_turned(rotate, text, rotation):
    fig = plt.figure(figsize=(3, 3), dpi=50)
    ax = fig.add_subplot(projection="3d")
    ax.set_zlabel(text)
    if rotate is not None:
        ax.zaxis.set_rotate_label(rotate)
    label = ax.zaxis.label
    label.set_rotation(90 if rotate is False else 45)
    assert (label.get_ha(), label.get_va(), label.get_rotation_mode()) == ("center", "top", "default")
    fig.canvas.draw()
    label = ax.zaxis.label
    assert round(label.get_rotation(), 2) == rotation
    assert (label.get_ha(), label.get_va()) == ("center", "center")


def test_3d_label_boxes_and_hidden_panes_are_drawn():
    def pixels(fig):
        fig.canvas.draw()
        return np.asarray(fig.canvas.buffer_rgba()).copy()

    fig = plt.figure(figsize=(3, 3), dpi=50)
    ax = fig.add_subplot(projection="3d")
    ax.set_zlabel("vertical label")
    plain = pixels(fig)
    ax.zaxis.label.set_bbox(dict(facecolor="y"))
    boxed = pixels(fig)
    yellow = (boxed[:, :, 0] > 150) & (boxed[:, :, 1] > 150) & (boxed[:, :, 2] < 80)
    assert yellow.sum() == 9 and not np.array_equal(plain, boxed)
    ax.xaxis.pane.set_visible(False)
    assert np.asarray(pixels(fig))[60, 40, 0] == 255


def test_ticks_of_a_removed_axes_are_placed_where_it_stood():
    calls = []

    def counted(self, x):
        calls.append(x)
        mtext.Text.set_x(self, x)

    fig, (ax, _) = plt.subplots(1, 2, figsize=(4, 2), dpi=50)
    ax.plot([0, 1])
    fig.canvas.draw()
    for label in ax.get_xticklabels():
        label.set_x = types.MethodType(counted, label)
    ax.remove()
    calls.clear()
    ax.get_xticklabels()
    assert len(calls) == 5


@pytest.mark.parametrize("draw", [
    lambda fig, ax: fig.draw(fig.canvas.get_renderer()),
    lambda fig, ax: ax.draw(fig.canvas.get_renderer()),
    lambda fig, ax: fig.savefig(io.BytesIO(), bbox_inches="tight"),
    lambda fig, ax: ax.get_tightbbox(fig.canvas.get_renderer()),
], ids=["figure", "axes", "tight-save", "tightbbox"])
def test_an_axis_holds_its_offset_after_every_way_of_drawing(draw):
    fig, ax = plt.subplots(figsize=(6, 5))
    x = np.linspace(0, 10, 100)
    ax.plot(x, np.sin(x) * 1e6 + 5e6)
    draw(fig, ax)
    assert ax.yaxis.get_offset_text().get_text() == "1e6"
    assert ax.yaxis.get_major_formatter().get_offset() == "1e6"


def test_a_deprecation_warning_is_shown_once_per_line():
    shown = []
    fig, ax = plt.subplots()
    fig.canvas.draw()
    formatter = ax.yaxis.get_major_formatter()
    with warnings.catch_warnings():
        warnings.simplefilter("default")
        warnings.showwarning = lambda message, *args, **kwargs: shown.append(str(message))
        for _ in range(3):
            fig.canvas.draw()
            formatter.locs  # noqa: B018 (read for its warning)
            ax.get_xticklabels()
            fig.savefig(io.BytesIO(), bbox_inches="tight")
    assert len(shown) == 1
