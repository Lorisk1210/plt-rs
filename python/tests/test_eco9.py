"""
What the libraries of ``cargo xtask ecosystem suite`` need of matplotlib,
beyond `test_eco8`: a scatter's private fields that previews swap and
restore (ultraplot), a 3-D Axes drawn alone after its view changed, an
axis measured before the first draw as matplotlib measures it (where its
Axes stands), an axis joining a sharing group through ``_scale`` keeping
the group's tick helpers, a `Ticker` a program assigns to an axis
(ultraplot's unsharing), the tick helpers of a scale a program registers
as 'linear' (ultraplot's), a tick label's font kept when the rcParams it
was given inside change back, a relative tick label size taken at draw
time, an Axes placed in another positioned in figure fractions, a
colorbar placed by its locator only when drawn (ultraplot's inset
colorbars), a spine and an axis label measured where they stand before a
first draw (cartopy's ``GeoSpine``), a mapping of colour names a program
puts in place of matplotlib's (ultraplot's ``ColorDatabase``), negative contour levels styled after the
colormap's own ``monochrome`` (ultraplot), a 3-D axis' ``_axinfo`` read and
written (qutip, its ``juggled`` kept by the axis), its ``_get_coord_info`` (qutip), unknown keyword
arguments refused even when None (python-control), and a ``filled``
keyword given to the contour methods (getdist). Each test does what a
library does, without the library. The expected values were recorded from
matplotlib 3.11.2 running the same code.
"""

import copy

import numpy as np
import pytest

import pltrs.axis as maxis
import pltrs.colors as mcolors
import pltrs.pyplot as plt
import pltrs.scale as mscale
import pltrs.ticker as mticker
from pltrs.axes import Axes
from pltrs.axes._base import _TransformedBoundsLocator
from pltrs.colors import LinearSegmentedColormap

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def test_a_scatter_takes_offsets_assigned_to_its_field():
    fig, ax = plt.subplots()
    scatter = ax.scatter(np.arange(5.0), np.arange(5.0))
    scatter._offsets = np.array([[0.0, 1.0], [2.0, 3.0]])
    assert scatter.get_offsets().tolist() == [[0.0, 1.0], [2.0, 3.0]]


def test_a_3d_scatter_has_the_fields_of_its_projection():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    scatter = ax.scatter([0, 1, 2], [0, 1, 2], [0, 1, 2], s=[10, 20, 30])
    assert scatter._offset_zordered is None
    assert scatter._z_markers_idx == slice(-1)
    assert scatter._sizes3d.tolist() == [10, 20, 30]
    assert scatter._linewidths3d.tolist() == [1.0]


def test_a_3d_axes_drawn_alone_follows_its_new_view():
    t = np.linspace(0, 6 * np.pi, 200)
    fig = plt.figure(figsize=(3, 3), dpi=40)
    ax = fig.add_subplot(projection="3d")
    ax.plot(np.cos(t), np.sin(t), t)
    fig.canvas.draw()
    ax.view_init(elev=35, azim=55)
    ax.set_animated(True)
    fig.canvas.draw()
    fig.draw_artist(ax)
    alone = np.asarray(fig.canvas.buffer_rgba()).copy()
    ax.set_animated(False)
    fig.canvas.draw()
    assert np.array_equal(alone, np.asarray(fig.canvas.buffer_rgba()))


def test_an_axis_is_measured_where_its_axes_stands_before_a_draw():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    ax.plot([0, 1])
    ax.set_aspect(3)
    renderer = fig.canvas.get_renderer()
    assert ax.xaxis.get_tightbbox(renderer).bounds == pytest.approx(
        (53.1, 9.4, 303.8, 13.9), abs=0.05)
    assert ax.bbox.bounds == pytest.approx((50.0, 33.0, 310.0, 231.0))


def test_a_colorbar_axis_is_measured_before_it_makes_room_for_extensions():
    fig = plt.figure(figsize=(4, 4), dpi=100)
    ax = fig.add_axes([0.1, 0.1, 0.5, 0.5])
    contours = ax.contourf(np.arange(16.0).reshape(4, 4), levels=[2, 5, 8, 11],
                           extend="both")
    cax = fig.add_axes([0.2, 0.8, 0.5, 0.05])
    fig.colorbar(contours, cax=cax, orientation="horizontal")
    assert cax.xaxis.get_tightbbox(fig.canvas.get_renderer()).bounds == pytest.approx(
        (75.5, 296.4, 213.5, 13.9), abs=0.05)


def test_an_axis_joining_a_sharing_group_through_its_scale_keeps_the_groups_helpers():
    fig = plt.figure()
    first = fig.add_subplot(121)
    second = fig.add_subplot(122, sharey=first)
    first.yaxis.set_major_formatter(mticker.NullFormatter())
    panel = Axes(fig, (0.9, 0.1, 0.05, 0.8))
    fig.add_axes(panel)
    # As ultraplot shares a panel's axis: grouped, then given the scale.
    panel._shared_axes["y"].join(panel, second)
    panel.yaxis._scale = second.yaxis._scale
    formatters = [type(ax.yaxis.get_major_formatter()).__name__ for ax in (first, second)]
    assert formatters == ["NullFormatter", "NullFormatter"]


def test_an_axis_ticks_with_the_ticker_a_program_assigns():
    fig, (top, bottom) = plt.subplots(2, sharex=True)
    fig.canvas.draw()
    # As ultraplot's unsharing: out of the group, each axis given copies.
    bottom._shared_axes["x"].remove(bottom)
    for axis in (top.xaxis, bottom.xaxis):
        ticker = maxis.Ticker()
        ticker.locator = copy.copy(axis.major.locator)
        ticker.formatter = copy.copy(axis.major.formatter)
        ticker.locator.set_axis(axis)
        ticker.formatter.set_axis(axis)
        axis.major = ticker
    bottom.xaxis.set_major_formatter(mticker.NullFormatter())
    bottom.set_xlim(0, 10)
    fig.canvas.draw()
    assert bottom.xaxis.major is ticker
    assert ticker.formatter is bottom.xaxis.get_major_formatter()
    assert type(top.xaxis.get_major_formatter()).__name__ == "ScalarFormatter"
    assert [label.get_text() for label in bottom.get_xticklabels()] == [""] * 6
    assert list(bottom.get_xticks()) == [0, 2, 4, 6, 8, 10]
    assert top.get_xlim() == (0, 1)


class _PercentLinearScale(mscale.LinearScale):
    """A scale registered as 'linear', as ultraplot's, with its formatter."""

    def __init__(self):
        super().__init__(None)

    def set_default_locators_and_formatters(self, axis):
        super().set_default_locators_and_formatters(axis)
        axis.set_major_formatter(mticker.PercentFormatter())


def test_a_scale_registered_as_linear_gives_the_axes_their_tick_helpers(monkeypatch):
    for registry in (mscale._scale_mapping, mscale._scale_has_axis_parameter):
        monkeypatch.setitem(registry, "linear", registry["linear"])
    mscale.register_scale(_PercentLinearScale)
    fig = plt.figure()
    flat = fig.add_subplot(221)
    made = plt.Axes(fig, (0.5, 0.5, 0.2, 0.2))
    three_d = fig.add_subplot(222, projection="3d")
    polar = fig.add_subplot(223, projection="polar")
    shared = fig.add_subplot(224, sharey=flat)
    flat.yaxis.set_major_formatter(mticker.NullFormatter())
    names = [type(axis.get_major_formatter()).__name__
             for axis in (flat.xaxis, made.xaxis, three_d.zaxis, polar.xaxis,
                          polar.yaxis, shared.xaxis, shared.yaxis)]
    assert names == ["PercentFormatter", "PercentFormatter", "PercentFormatter",
                     "ThetaFormatter", "PercentFormatter", "PercentFormatter",
                     "NullFormatter"]
    assert type(polar.xaxis.get_major_locator()).__name__ == "ThetaLocator"
    flat.cla()
    assert type(shared.yaxis.get_major_formatter()).__name__ == "PercentFormatter"


def test_negative_contours_follow_the_colormaps_own_monochrome():
    single = LinearSegmentedColormap.from_list("one", ["black", "black"])
    fig, ax = plt.subplots()
    contours = ax.contour(np.arange(16.0).reshape(4, 4) - 8, cmap=single)
    assert single.monochrome is False
    assert all(dashes is None for _, dashes in contours.get_linestyles())


def test_a_3d_axis_info_is_read_and_its_writes_are_drawn():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    info = ax.yaxis._axinfo
    assert (info["i"], info["tickdir"], info["juggled"]) == (1, 0, (0, 1, 2))
    assert info["grid"] == {"color": "#b0b0b0", "linewidth": 0.8, "linestyle": "-"}
    assert info["tick"] == {"inward_factor": 0.2, "outward_factor": 0.1,
                            "linewidth": {True: 0.8, False: 0.6}}
    assert info["label"] == {"va": "center", "ha": "center", "rotation_mode": "anchor"}
    fig.canvas.draw()
    before = np.asarray(fig.canvas.buffer_rgba()).copy()
    info["grid"]["linewidth"] = 4.0  # As qutip's matrix_histogram.
    fig.canvas.draw()
    assert not np.array_equal(before, np.asarray(fig.canvas.buffer_rgba()))


def test_a_3d_axis_gives_its_coordinate_info():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.bar3d([0, 1], [0, 1], [0, 0], 0.5, 0.5, [1, 2])
    ax.view_init(30, 40)
    fig.canvas.draw()
    mins, maxs, bounds_proj, highs = ax.xaxis._get_coord_info()
    assert np.round(mins, 4).tolist() == [-0.1094, -0.1094, -0.0417]
    assert np.round(maxs, 4).tolist() == [1.6094, 1.6094, 2.0417]
    assert np.round(bounds_proj[0], 4).tolist() == [-0.0067, 0.003, -0.9133]
    assert highs.tolist() == [False, False, False]
    centers, deltas = ax.xaxis._calc_centers_deltas(maxs, mins)
    assert np.round(centers, 4).tolist() == [0.75, 0.75, 1.0]
    assert np.round(deltas, 4).tolist() == [0.1375, 0.1375, 0.1667]


@pytest.mark.parametrize("call", [
    lambda ax: ax.plot([0, 1], unknown=None),
    lambda ax: ax.step([0, 1], [0, 1], unknown=None),
    lambda ax: ax.loglog([1, 2], [1, 2], unknown=None),
    lambda ax: ax.fill([0, 1, 1], [0, 0, 1], unknown=None),
])
def test_an_unknown_keyword_is_refused_even_when_none(call):
    fig, ax = plt.subplots()
    with pytest.raises(AttributeError, match="unexpected keyword argument 'unknown'"):
        call(ax)


def test_a_known_property_given_none_takes_its_default():
    fig, ax = plt.subplots()
    line, = ax.plot([0, 1], color=None, linewidth=None)
    assert line.get_linewidth() == 1.5


def test_a_new_super_title_refuses_an_unknown_property_and_is_not_kept():
    fig = plt.figure()
    with pytest.raises(AttributeError, match="'Text' object has no property 'unknown'"):
        fig.suptitle("t", unknown=1)
    assert fig._suptitle is None and not fig.texts
    fig.suptitle("t")
    with pytest.raises(AttributeError, match=r"Text.set\(\) got an unexpected keyword"):
        fig.suptitle("u", unknown=1)


@pytest.mark.parametrize("method, filled", [("contour", False), ("contourf", True),
                                            ("tricontour", False), ("tricontourf", True)])
def test_the_contour_methods_set_filled_themselves(method, filled):
    fig, ax = plt.subplots()
    if method.startswith("tri"):
        x, y = np.array([0, 1, 0, 1, 0.5]), np.array([0, 0, 1, 1, 0.5])
        contours = getattr(ax, method)(x, y, x + y, filled=not filled)
    else:
        z = np.outer(np.linspace(0, 1, 5), np.linspace(0, 1, 5))
        contours = getattr(ax, method)(z, filled=not filled)
    assert contours.filled is filled


def test_a_tick_label_keeps_the_font_it_was_given_inside_an_rc_context():
    fig, ax = plt.subplots()
    with plt.rc_context({"font.family": ["serif"]}):
        ax.tick_params(axis="x", which="major", labelsize="medium")
        labels = ax.xaxis.get_ticklabels()
        for label in labels:
            label.update({"family": ["serif"]})
    ax.xaxis.majorTicks
    assert labels[0].get_fontproperties().get_family() == ["serif"]
    assert labels[1].get_fontproperties().get_family() == ["serif"]


def test_a_relative_tick_label_size_is_taken_when_the_ticks_are_placed():
    fig, ax = plt.subplots(figsize=(4, 3))
    with plt.rc_context({"font.size": 30}):
        ax.tick_params(axis="x", labelsize="medium")
    fig.canvas.draw()
    assert ax.get_xticks() == pytest.approx([0, 0.2, 0.4, 0.6, 0.8, 1])


def test_an_axes_placed_in_another_is_positioned_in_figure_fractions():
    fig, ax = plt.subplots(figsize=(4, 3))
    child = plt.Axes(fig, (0.5, 0.5, 0.2, 0.2))
    ax.add_child_axes(child)
    locator = _TransformedBoundsLocator([0.2, 0.7, 0.4, 0.05], ax.transAxes)
    child.set_axes_locator(locator)
    child.set_position(locator(child, None).bounds)
    assert child.get_position().bounds == pytest.approx((0.28, 0.649, 0.31, 0.0385), abs=5e-5)
    assert child.get_window_extent().bounds == pytest.approx((112, 194.7, 124, 11.6), abs=0.05)


def test_a_colorbar_is_placed_by_its_locator_only_when_drawn():
    fig, ax = plt.subplots(figsize=(4, 3))
    contours = ax.contourf(np.linspace(0, 3, 100).reshape(10, 10), levels=[0, 1, 2, 3],
                           extend="both")
    cax = ax.inset_axes([0.1, 0.8, 0.5, 0.05])
    fig.colorbar(contours, cax=cax, orientation="horizontal", label="label")
    renderer = fig.canvas.get_renderer()
    cax.get_xticklabels()[0].get_window_extent(renderer)
    cax.xaxis.label.get_window_extent(renderer)
    assert cax.get_position().bounds == pytest.approx((0.2025, 0.726, 0.3875, 0.0385), abs=5e-5)
    fig.canvas.draw()
    assert cax.get_position().bounds == pytest.approx((0.2201, 0.726, 0.3523, 0.0385), abs=5e-5)


def test_a_spine_and_an_axis_label_are_measured_where_they_stand_before_a_draw():
    fig, ax = plt.subplots(figsize=(4, 3))
    ax.set_aspect("equal")
    ax.plot([0, 1], [0, 1])
    ax.set_xlabel("x")
    bounds = [extent.bounds for extent in (
        ax.spines["bottom"].get_window_extent(), ax.xaxis.label.get_window_extent(),
        ax.yaxis.offsetText.get_window_extent(), ax.patch.get_window_extent())]
    expected = [(50, 28.1, 310, 4.9), (201, -13.9, 8, 13.9), (50, 0.5, 0, 0), (50, 33, 310, 231)]
    for got, want in zip(bounds, expected):
        assert got == pytest.approx(want, abs=0.05)


class _NamedColors(dict):
    """A mapping of colour names a program puts in place of matplotlib's."""

    def __init__(self, mapping):
        super().__init__(mapping)
        self.cache = {}


def test_a_mapping_of_colour_names_put_in_place_of_matplotlibs_is_read_first(monkeypatch):
    mapping = _NamedColors(mcolors._colors_full_map)
    mapping["plum"] = "#580f41"
    monkeypatch.setattr(mcolors, "_colors_full_map", mapping)
    assert mcolors.to_rgba("plum") == pytest.approx((0.345, 0.059, 0.255, 1), abs=5e-4)
    fig, ax = plt.subplots(figsize=(1, 1), dpi=20)
    ax.set_axis_off()
    ax.add_patch(plt.Rectangle((0, 0), 1, 1, color="plum"))
    fig.canvas.draw()
    assert np.asarray(fig.canvas.buffer_rgba())[10, 10].tolist() == [88, 15, 65, 255]
    monkeypatch.undo()
    assert mcolors.to_rgba("plum") == pytest.approx((0.867, 0.627, 0.867, 1), abs=5e-4)


def test_a_3d_axis_juggled_order_is_written_to_the_axis_and_kept_by_cla():
    fig = plt.figure(figsize=(3, 3), dpi=50)
    ax = fig.add_subplot(projection="3d")
    info = ax.xaxis._axinfo
    assert info["juggled"] == info.get("juggled") == (1, 0, 2)
    fig.canvas.draw()
    before = np.asarray(fig.canvas.buffer_rgba()).copy()
    info["juggled"] = (0, 1, 2)
    assert ax.xaxis._axinfo["juggled"] == (0, 1, 2)
    fig.canvas.draw()
    assert not np.array_equal(before, np.asarray(fig.canvas.buffer_rgba()))
    ax.cla()
    assert ax.xaxis._axinfo["juggled"] == (0, 1, 2)
