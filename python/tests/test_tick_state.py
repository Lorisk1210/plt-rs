"""
What ticking an axis leaves in its tick helpers (`pltrs._tick_state`):
matplotlib's formatters and locators keep what ``Axis._update_ticks``
computed with them (a formatter's locations, offset, order of magnitude
and format, the frequency an ``AutoDateLocator`` picked), and programs read
it after a draw, also from the helpers pltrs (Rust) ticks with natively.
The expected values are real matplotlib 3.11.2's for the same programs.
"""

import copy
import datetime
import io

import numpy as np
import pytest

import pltrs
import pltrs.dates as mdates
import pltrs.pyplot as plt
from pltrs import ticker
from pltrs.mpl_toolkits import axisartist

MILLION = [999998.0, 1000000.0, 1000002.0, 1000004.0, 1000006.0, 1000008.0,
           1000010.0, 1000012.0]


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """Every test starts and ends with matplotlib's default rcParams and
    closes its figures."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _million():
    """An axes whose x ticks need an offset and y ticks an order of
    magnitude, with their formatters taken before it is drawn."""
    fig, ax = plt.subplots()
    ax.plot([1e6, 1e6 + 10], [1e-7, 3e-7])
    return fig, ax, ax.xaxis.get_major_formatter(), ax.yaxis.get_major_formatter()


def _state(formatter):
    """What a scalar formatter keeps of the ticks it labelled last."""
    return (list(np.asarray(formatter._locs, dtype=float)), formatter.offset,
            formatter._orderOfMagnitude, formatter._format, formatter.get_offset())


DRAWN = (MILLION, 1000000.0, 0, '%1.0f', '+1e6')


@pytest.mark.parametrize("tick", [
    lambda fig: fig.canvas.draw(),
    lambda fig: fig.draw_without_rendering(),
    lambda fig: fig.savefig(io.BytesIO(), format="png"),
    lambda fig: fig.savefig(io.BytesIO(), format="svg"),
    lambda fig: fig.get_tightbbox(),
    lambda fig: fig.axes[0].get_xticklabels(),
], ids=["draw", "draw_without_rendering", "png", "svg", "tightbbox", "ticklabels"])
def test_a_native_formatter_keeps_what_ticking_the_axis_computed(tick):
    fig, ax, x, y = _million()
    assert _state(x) == ([], 0, 0, '', '')
    tick(fig)
    assert _state(x) == DRAWN
    assert ax.xaxis.get_major_formatter() is x


def test_the_state_stays_until_the_axis_is_ticked_again():
    fig, ax, x, y = _million()
    fig.canvas.draw()
    assert _state(y)[2:] == (-7, '%1.2f', '1e\N{MINUS SIGN}7')
    ax.set_xlim(0, 5e5)
    assert _state(x) == DRAWN
    fig.canvas.draw()
    assert _state(x) == ([0.0, 1e5, 2e5, 3e5, 4e5, 5e5], 0, 0, '%1.0f', '')
    assert type(x.offset) is int


def test_the_offset_is_numpy_s_and_copies_keep_the_state():
    fig, ax, x, y = _million()
    fig.canvas.draw()
    assert type(x.offset) is np.float64
    assert _state(copy.copy(x)) == DRAWN


def test_changing_a_formatter_after_a_draw_keeps_what_the_draw_computed():
    fig, ax, x, y = _million()
    fig.canvas.draw()
    x.set_powerlimits((-2, 2))
    assert _state(x) == DRAWN
    fig.canvas.draw()
    assert x.get_offset() == '+1e6'


def test_minor_and_log_formatters():
    fig, ax = plt.subplots()
    ax.plot([0, 5e5], [0, 1])
    ax.minorticks_on()
    fig.canvas.draw()
    minor = ax.xaxis.get_minor_locator()
    assert type(minor) is ticker.AutoMinorLocator and minor.ndivs == 'auto'
    assert ax.xaxis.get_minor_formatter()._locs[:4].tolist() == [
        -20000.0, 20000.0, 40000.0, 60000.0]
    fig, ax = plt.subplots()
    ax.set_yscale("log")
    ax.plot([1, 2], [1, 1000])
    major, minor = ax.yaxis.get_major_formatter(), ax.yaxis.get_minor_formatter()
    fig.draw_without_rendering()
    assert (major._sublabels, minor._sublabels, major._locs) == ({1}, {1}, [])


def test_logit_and_engineering_formatters():
    fig, ax = plt.subplots()
    ax.set_xscale("logit")
    ax.plot([0.01, 0.99], [0, 1])
    fig.canvas.draw()
    major, minor = ax.xaxis.get_major_formatter(), ax.xaxis.get_minor_formatter()
    assert major._locs.tolist() == [0.001, 0.01, 0.1, 0.5, 0.9, 0.99, 0.999]
    assert (major._labelled, minor._labelled, len(minor._locs)) == (set(), set(), 38)
    fig, ax = plt.subplots()
    eng = ticker.EngFormatter(unit="Hz", useOffset=True)
    ax.xaxis.set_major_formatter(eng)
    ax.plot([1e6, 1e6 + 10], [0, 1])
    fig.savefig(io.BytesIO(), format="png")
    assert (eng.offset, eng._format, eng.get_offset()) == (
        1000005.0, '%1.0f', '1 Hz+1.00001 MHz')


def test_date_helpers_keep_what_they_picked():
    fig, ax = plt.subplots()
    t = [datetime.datetime(2020, 1, 1) + datetime.timedelta(hours=h)
         for h in range(0, 24 * 20, 6)]
    ax.plot(t, range(len(t)))
    formatter, locator = ax.xaxis.get_major_formatter(), ax.xaxis.get_major_locator()
    assert (type(formatter), locator._freq) == (mdates.AutoDateFormatter, mdates.YEARLY)
    fig.canvas.draw()
    assert locator._freq == mdates.DAILY
    assert formatter._locs.tolist() == [18262.0, 18266.0, 18270.0, 18274.0, 18278.0,
                                        18282.0]
    assert formatter._formatter.fmt == '%Y-%m-%d'
    with pltrs.rc_context({"date.converter": "concise"}):
        fig, ax = plt.subplots()
        ax.plot(t, range(len(t)))
        concise = ax.xaxis.get_major_formatter()
        fig.canvas.draw()
        assert (concise.get_offset(), concise._offset_string) == ('2020-Jan', '2020-Jan')
        ax.set_xlim(t[0], t[3])
        fig.canvas.draw()
        assert concise.get_offset() == '2020-Jan-01'


def test_rule_locators_keep_the_rule_they_ran():
    fig, ax = plt.subplots()
    ax.plot([datetime.datetime(2020, 1, 1), datetime.datetime(2020, 6, 1)], [0, 1])
    months = mdates.MonthLocator()
    ax.xaxis.set_major_locator(months)
    fig.canvas.draw()
    rule = months.rule._construct
    assert (rule["dtstart"], rule["until"]) == (
        datetime.datetime(2019, 7, 9, 4, 48), datetime.datetime(2020, 11, 23, 19, 12))
    fig, ax = plt.subplots()
    ax.plot([datetime.datetime(2000, 1, 1), datetime.datetime(2020, 6, 1)], [0, 1])
    years = mdates.YearLocator()
    ax.xaxis.set_major_locator(years)
    fig.canvas.draw()
    rule = years.rule._construct
    assert (rule["dtstart"], rule["until"]) == (
        datetime.datetime(1998, 1, 1), datetime.datetime(2021, 1, 1))


def test_the_toolkits_axes():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    ax.plot([1e6, 1e6 + 10], [0, 1])
    x = ax.xaxis.get_major_formatter()
    fig.canvas.draw()
    assert _state(x) == DRAWN
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.plot([1e6, 1e6 + 10], [0, 1], [1e-7, 2e-7])
    x, z = ax.xaxis.get_major_formatter(), ax.zaxis.get_major_formatter()
    fig.canvas.draw()
    assert _state(x) == DRAWN
    assert _state(z)[2:] == (-7, '%1.1f', '1e\N{MINUS SIGN}7')
    ax.set_xlim(0, 10)
    fig.draw_without_rendering()
    assert _state(x) == ([0.0, 2.0, 4.0, 6.0, 8.0, 10.0], 0, 0, '%1.0f', '')


def test_polar_axes_and_colorbars():
    fig, ax = plt.subplots(subplot_kw={"projection": "polar"})
    ax.plot([0, 1], [1e6, 1e6 + 10])
    radial = ax.yaxis.get_major_formatter()
    fig.canvas.draw()
    # The radial locator gives a list of numpy's numbers.
    assert type(radial._locs) is list and type(radial._locs[0]) is np.float64
    assert _state(radial)[2:] == (6, '%1.1f', '1e6')
    assert ax.xaxis.get_major_formatter()._locs[:3].round(6).tolist() == [
        0.0, 0.785398, 1.570796]
    fig, ax = plt.subplots()
    cb = fig.colorbar(ax.imshow([[1e6, 1e6 + 10]]))
    formatter = cb.ax.yaxis.get_major_formatter()
    fig.canvas.draw()
    assert cb.formatter is formatter
    assert _state(formatter) == (
        [1000000.0, 1000002.0, 1000004.0, 1000006.0, 1000008.0, 1000010.0],
        1000000.0, 0, '%1.0f', '+1e6')


def test_edge_integers():
    edge = ticker._Edge_integer(4, 0)
    assert (edge.le(2021), edge.ge(2021), edge.le(2020), edge.ge(2020)) == (
        505.0, 506.0, 505.0, 505.0)
    assert edge.closeto(0.99999999999, 1) and not edge.closeto(0.9, 1)
