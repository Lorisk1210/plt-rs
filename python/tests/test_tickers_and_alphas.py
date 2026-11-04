"""
Shared axes hold one `Ticker` per tick level, and per-item alphas that do
not fit a collection raise matplotlib's `ValueError` where matplotlib
raises it. The expected values are real matplotlib 3.11's.
"""

import io

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections as mcoll, ticker
from pltrs.path import Path


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """Every test starts and ends with matplotlib's default rcParams and
    closes its figures."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_shared_axes_hold_the_same_tickers():
    fig, (a, b, c) = plt.subplots(3, sharex=True)
    assert a.xaxis.major is b.xaxis.major is c.xaxis.major
    assert a.xaxis.minor is c.xaxis.minor
    assert a.yaxis.major is not b.yaxis.major
    fig2, d = plt.subplots()
    e = fig2.add_subplot(2, 1, 2)
    e.sharex(d)
    assert (e.xaxis.major, e.xaxis.minor) == (d.xaxis.major, d.xaxis.minor)
    f = fig2.add_subplot(2, 2, 1, sharey=d)
    assert f.yaxis.major is d.yaxis.major
    assert f.xaxis.major is not d.xaxis.major
    twin = d.twinx()
    assert twin.xaxis.major is d.xaxis.major
    assert twin.yaxis.major is not d.yaxis.major
    # Clearing keeps them shared; a locator set on one serves them all.
    a.cla()
    b.cla()
    assert a.xaxis.major is b.xaxis.major is c.xaxis.major
    locator = ticker.MaxNLocator(3)
    c.xaxis.set_major_locator(locator)
    assert a.xaxis.get_major_locator() is locator
    assert b.xaxis.major.locator is locator


def test_shared_3d_axes_hold_the_same_tickers():
    fig = plt.figure()
    ax = fig.add_subplot(2, 2, 1, projection="3d")
    other = fig.add_subplot(2, 2, 4, projection="3d", sharez=ax, shareview=ax)
    assert other.zaxis.major is ax.zaxis.major
    assert other.xaxis.major is not ax.xaxis.major


def test_shared_helpers_follow_the_axis_a_clear_reset_last():
    fig, (a, b, c) = plt.subplots(3, sharex=True)
    axes = {a.xaxis: "a", b.xaxis: "b", c.xaxis: "c"}

    def bound(get):
        return [axes.get(get(ax.xaxis).axis) for ax in (a, b, c)]

    assert bound(lambda x: x.get_major_formatter()) == ["a", "a", "a"]
    b.xaxis.set_major_formatter(ticker.FormatStrFormatter("%.1f"))
    assert bound(lambda x: x.get_major_formatter()) == ["b", "b", "b"]
    a.cla()
    assert bound(lambda x: x.get_major_formatter()) == ["c", "c", "c"]
    assert bound(lambda x: x.get_major_locator()) == ["c", "c", "c"]


def _outcome(f):
    try:
        f()
    except ValueError as err:
        return str(err)
    return "ok"


_FLAT = ('This can occur with the deprecated behavior of the "flat" shading '
         'option, in which a row and/or column of the data array is dropped.')


def test_alphas_that_do_not_fit_the_colours_raise_when_set_and_drawn():
    fig, ax = plt.subplots()
    dots = mcoll.PathCollection([Path.unit_circle()],
                                offsets=[(0, 0), (1, 1), (2, 2)],
                                offset_transform=ax.transData,
                                facecolors=["r", "g"])
    ax.add_collection(dots)
    broadcast = "could not broadcast input array from shape (3,) into shape (2,)"
    assert _outcome(lambda: dots.set_alpha([0.1, 0.2, 0.3])) == broadcast
    assert dots.get_alpha().tolist() == [0.1, 0.2, 0.3]
    assert _outcome(fig.canvas.draw) == broadcast


def test_alphas_fit_one_colour_and_no_colour():
    fig, ax = plt.subplots()
    dots = ax.scatter([0, 1, 2], [0, 1, 2])
    assert _outcome(lambda: dots.set_alpha([0.1, 0.2])) == "ok"
    lines = mcoll.LineCollection([[(0, 0), (1, 1)], [(0, 1), (1, 0)]],
                                 alpha=[0.5, 0.5, 0.5])
    ax.add_collection(lines)
    assert _outcome(fig.canvas.draw) == "ok"
    assert (lines.get_edgecolor().shape, lines.get_facecolor().shape) == (
        (3, 4), (0, 4))


@pytest.mark.parametrize("alpha, shape", [(np.ones(5), "(5,)"),
                                          (np.ones((2, 2)), "(2, 2)")])
def test_alphas_that_do_not_fit_the_mapped_values_raise_when_drawn(alpha, shape):
    fig, ax = plt.subplots()
    mesh = ax.pcolormesh(np.arange(12.0).reshape(3, 4))
    assert _outcome(lambda: mesh.set_alpha(alpha)) == "ok"
    assert _outcome(fig.canvas.draw) == (
        f"Data array shape, (3, 4) is incompatible with alpha array shape, "
        f"{shape}. {_FLAT}")


def test_alphas_of_a_mapped_scatter_are_checked_when_saved():
    fig, ax = plt.subplots()
    dots = ax.scatter([0, 1, 2], [0, 1, 2], c=[1, 2, 3])
    dots.set_alpha([0.1, 0.2])
    message = (f"Data array shape, (3,) is incompatible with alpha array "
               f"shape, (2,). {_FLAT}")
    assert _outcome(fig.canvas.draw) == message
    assert _outcome(lambda: fig.savefig(io.BytesIO(), format="svg")) == message
