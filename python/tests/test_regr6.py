"""
What the sixth round's merges changed in programs users wrote (the
corpus run on main after them): each expected value is recorded from
matplotlib 3.11.
"""

import io
import logging

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.cm import ScalarMappable
from pltrs.mpl_toolkits.axes_grid1.inset_locator import inset_axes
from build_facts import DEFAULT_INT


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


@pytest.mark.parametrize("orientation", ["horizontal", "vertical"])
def test_a_colorbar_with_ticks_in_an_inset_axes_in_use(orientation):
    # python-graph-gallery's circular barplot: the Axes, whose transforms
    # the inset's placement had read, asked for the scale of the long axis
    # before it knew its colorbar ("cannot unpack non-iterable XAxis
    # object").
    fig, ax = plt.subplots()
    cmap = pltrs.colormaps["viridis"]
    cax = inset_axes(ax, width="100%", height="100%", loc="center",
                     bbox_to_anchor=(0.3, 0.1, 0.35, 0.01),
                     bbox_transform=fig.transFigure)
    norm = pltrs.colors.BoundaryNorm([0, 100, 150, 200, 250, 300], cmap.N)
    cb = fig.colorbar(ScalarMappable(norm=norm, cmap=cmap), cax=cax,
                      orientation=orientation, ticks=[100, 150, 200, 250])
    long_axis = cb.ax.xaxis if orientation == "horizontal" else cb.ax.yaxis
    assert type(cb.locator).__name__ == "FixedLocator"
    ticks = cb.get_ticks()
    assert list(ticks) == [100, 150, 200, 250]
    assert ticks.dtype == DEFAULT_INT
    assert long_axis.get_scale() == "function"
    assert type(long_axis._scale).__name__ == "FuncScale"
    assert cb.ax._colorbar is cb
    fig.canvas.draw()
    assert [label.get_text() for label in long_axis.get_ticklabels()] == [
        "100", "150", "200", "250"]


def test_a_colorbar_that_cannot_be_made_leaves_its_axes_unlinked():
    fig, (ax, cax) = plt.subplots(1, 2)
    with pytest.raises(ValueError):
        fig.colorbar(ScalarMappable(), cax=cax, orientation="diagonal")
    assert not hasattr(cax, "_colorbar")


def _missing_glyph_messages(caplog):
    """How many times mathtext logged a symbol no font has, since the last
    call."""
    count = sum("does not have a glyph" in record.getMessage()
                for record in caplog.records)
    caplog.clear()
    return count


def test_a_mathtext_marker_is_parsed_when_its_style_is_made(caplog):
    # matplotlib-3-cookbook, chapter 7: a marker's formula is typeset by
    # TextPath's parser, which keeps it, so the symbol no font has is
    # logged once, not again when a second style is made with it or when
    # the markers are drawn (pltrs logged it at each draw).
    marker = "$\U0001f9ea$"
    with caplog.at_level(logging.WARNING):
        plt.scatter(2, 2, marker=marker, s=500)
        assert _missing_glyph_messages(caplog) == 1
        plt.gcf().canvas.draw()
        assert _missing_glyph_messages(caplog) == 0
        plt.close("all")
        fig, ax = plt.subplots(figsize=(10, 8))
        x = np.arange(1, 2.6, 0.1)
        ax.plot(x, x, marker=marker, markersize=15)
        assert _missing_glyph_messages(caplog) == 0
        fig.canvas.draw()
        assert _missing_glyph_messages(caplog) == 0
        fig.savefig(io.BytesIO(), format="png", bbox_inches="tight")
        assert _missing_glyph_messages(caplog) == 0
