"""
What pandas and seaborn need of matplotlib that its own suite does not
reach (``cargo xtask ecosystem`` runs them): each answered as matplotlib
3.11 answers it. The expected values were recorded from matplotlib
3.11.2 running the same code.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import axis, collections, colors, lines, markers, patches, ticker

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def test_sticky_edges_are_lists_autoscaling_reads():
    fig, ax = plt.subplots()
    (line,) = ax.plot([0, 1], [0, 1])
    assert (line.sticky_edges.x, line.sticky_edges.y) == ([], [])
    line.sticky_edges.y[:] = [0]
    line.sticky_edges.x.append(1)
    assert line.sticky_edges == ([1.0], [0.0])
    ax.autoscale_view()
    assert ax.get_xlim() == (-0.05, 1.0) and ax.get_ylim() == (0.0, 1.05)
    bars = ax.bar([0.5], [0.5])
    assert (bars[0].sticky_edges.x, bars[0].sticky_edges.y) == ([], [0])


def test_update_datalim_extends_the_data_limits():
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    ax.update_datalim([(2, 3), (-1, 0.5)], updatey=False)
    ax.autoscale_view()
    assert tuple(ax.dataLim.bounds) == (-1, 0, 3, 1)
    assert ax.get_xlim() == (-1.15, 2.15) and ax.get_ylim() == (-0.05, 1.05)
    ax.update_datalim([(np.nan, np.nan)])
    assert tuple(ax.dataLim.bounds) == (-1, 0, 3, 1)


def test_assigning_none_to_legend_takes_it_away():
    fig, ax = plt.subplots()
    ax.plot([1, 2], label="a")
    ax.legend()
    ax.legend_ = None
    assert ax.get_legend() is None
    assert not [c for c in ax.get_children() if type(c).__name__ == "Legend"]
    ax.legend_ = None


def test_lines_module_lists_the_styles():
    assert len(lines.lineMarkers) == 41 and lines.lineMarkers["o"] == "circle"
    assert sorted(lines.lineStyles)[:3] == ["", " ", "-"]
    assert sorted(lines.drawStyles)[:2] == ["default", "steps"]
    assert lines.fillStyles == ("full", "left", "right", "bottom", "top", "none")


def test_set_ticks_returns_the_ticks():
    fig, ax = plt.subplots()
    ticks = ax.xaxis.set_ticks([0, 0.5])
    assert [type(t).__name__ for t in ticks] == ["XTick", "XTick"]
    minor = ax.yaxis.set_ticks([0.25], minor=True)
    assert [type(t).__name__ for t in minor] == ["YTick"]


def test_a_ticker_made_alone_holds_its_helpers():
    alone = axis.Ticker()
    assert (alone.locator, alone.formatter) == (None, None)
    alone.locator = ticker.MaxNLocator(3)
    alone.formatter = ticker.ScalarFormatter()
    assert type(alone.locator).__name__ == "MaxNLocator"
    assert type(alone.formatter).__name__ == "ScalarFormatter"
    with pytest.raises(TypeError, match="locator must be a subclass"):
        alone.locator = 3


def test_axes_share_property_cycles():
    fig, ax = plt.subplots()
    twin = ax.twinx()
    twin._get_lines = ax._get_lines
    twin._get_patches_for_fill = ax._get_patches_for_fill
    drawn = [ax.plot([1, 2])[0].get_color(), twin.plot([1, 2])[0].get_color(),
             ax.plot([1, 2])[0].get_color(), ax._get_lines.get_next_color()]
    assert [colors.to_hex(c) for c in drawn] == [
        "#1f77b4", "#ff7f0e", "#2ca02c", "#d62728"]
    faces = [ax.fill([0, 1, 1], [0, 0, 1])[0].get_facecolor(),
             twin.fill([0, 1, 1], [0, 0, 1])[0].get_facecolor()]
    assert [colors.to_hex(c) for c in faces] == ["#1f77b4", "#ff7f0e"]
    with pytest.raises(TypeError):
        twin._get_lines = ax._get_patches_for_fill


def test_a_legend_title_of_none_is_no_title():
    fig = plt.figure()
    legend = fig.legend([], [], frameon=False)
    legend.set_title(None)
    fig.canvas.draw()
    assert legend.get_title().get_text() == ""
    assert legend.get_window_extent().width == pytest.approx(11.111111111111086)


def test_legends_show_patches_and_collections_made_alone():
    fig, ax = plt.subplots()
    handles = [patches.Patch(facecolor="red"),
               patches.Ellipse((0, 0), 1, 1, facecolor="blue"),
               collections.PathCollection([markers.MarkerStyle("o").get_path()],
                                          sizes=[20], facecolors=["green"])]
    legend = ax.legend(handles, ["p", "e", "c"])
    fig.canvas.draw()
    drawn = legend.legend_handles
    assert [type(h).__name__ for h in drawn] == ["Rectangle", "Rectangle", "PathCollection"]
    assert [tuple(np.round(h.get_facecolor(), 3).ravel()) for h in drawn] == [
        (1, 0, 0, 1), (0, 0, 1, 1), (0, 0.502, 0, 1)]


def test_ticks_made_later_copy_the_first_tick():
    fig, ax = plt.subplots()
    plt.setp(ax.get_xticklabels(), visible=False)
    for tick in ax.xaxis.get_major_ticks():
        tick.label1.set_color("red")
    ax.set_xlim(0, 100)
    labels = [(t.label1.get_visible(), t.label1.get_color())
              for t in ax.xaxis.get_major_ticks()]
    assert labels == [(False, "red")] * 6


def test_pyplot_imports_offsetbox_and_collections_take_transoffset():
    assert hasattr(pltrs, "offsetbox")
    fig, ax = plt.subplots()
    collection = collections.PathCollection([], transOffset=ax.transData)
    assert collection.get_offset_transform() is ax.transData
