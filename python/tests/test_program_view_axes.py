"""
An axis whose class of the program's answers ``get_view_interval`` itself
(the skewt example's) is asked for it as often as matplotlib asks, and
ticked over what it answers. The expected values are real matplotlib
3.11's.
"""

import io

import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import axis as maxis, ticker
from pltrs.axes import Axes
from pltrs.projections import register_projection


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """Every test starts and ends with matplotlib's default rcParams and
    closes its figures."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


CALLS = []


class WideXAxis(maxis.XAxis):
    """An x axis ticked one unit beyond its view on either side."""

    def get_view_interval(self):
        CALLS.append(1)
        low, high = self.axes.viewLim.intervalx
        return low - 1, high + 1


class WideAxes(Axes):
    name = "pltrs_test_wide"

    def _init_axis(self):
        self.xaxis = WideXAxis(self)
        self.spines.top.register_axis(self.xaxis)
        self.spines.bottom.register_axis(self.xaxis)
        self.yaxis = maxis.YAxis(self)
        self.spines.left.register_axis(self.yaxis)
        self.spines.right.register_axis(self.yaxis)


register_projection(WideAxes)


def test_the_view_interval_is_asked_where_matplotlib_asks_it():
    CALLS.clear()
    fig = plt.figure()
    ax = fig.add_subplot(projection="pltrs_test_wide")
    ax.plot([0, 1, 2], [0, 1, 4])
    ax.axvline(1)
    counts = [len(CALLS)]
    ax.xaxis.set_major_locator(ticker.MultipleLocator(0.5))
    fig.canvas.draw()
    counts.append(len(CALLS))
    locs = [float(loc) for loc in ax.xaxis.get_majorticklocs()]
    fig.savefig(io.BytesIO(), format="png")
    counts.append(len(CALLS))
    assert counts == [1, 13, 26]
    expected = [-1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5]
    assert locs == expected
    assert [float(t.get_loc()) for t in ax.xaxis.get_major_ticks()] == expected
