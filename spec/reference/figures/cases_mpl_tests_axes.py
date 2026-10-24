"""matplotlib's own tests of ``Axes`` (``tests/test_axes.py``) that draw
without an image comparison and were not ported by the ``mpl_axes_*``
groups.

* ``limits_empty_data_*``: the ``fig_test`` side of
  ``test_limits_empty_data``, one case per plotting function. The test
  picks the ``default`` style, which needs no rc, on the default canvas.
  Plotting empty data first must not change the autoscaled limits of the
  dates plotted next; the ``mpl_axes`` probes compare the limits of both
  sides.
* ``indicate_inset_inverted_*``: ``test_indicate_inset_inverted``, one case
  per inversion of the x and y axes, checks that the connectors run to
  the right corners.
* ``move_offsetlabel_*``: ``test_move_offsetlabel``, the y offset text
  moving with the ticks to the right, then the x ticks moving to the top.
* ``animated_artists_not_drawn``: ``test_animated_artists_not_drawn_by_
  default``, an animated image and line that a draw leaves out.

The tests without a style of their own run in the test suite's
``classic`` style with ``_classic_test_patch`` on classic's 8x6 in canvas.
numpy's random data is replaced by ``spec_support.rng`` streams with the
tests' line numbers as seeds.
"""

import matplotlib.style
import numpy as np

from spec_support import case, rng

GROUP = "mpl_tests_axes"
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


def classic_case(name):
    """A case under the test suite's classic style on classic's canvas."""
    return case(f"{GROUP}/{name}", figsize=(8, 6), rc=CLASSIC)


def _limits_empty_data(plot_fun):
    """``test_limits_empty_data``'s ``fig_test``: ``plot_fun([], [])``, then
    ``plot_fun`` of the days of 2010 against their index in ``C0``."""

    def build(fig):
        x = np.arange("2010-01-01", "2011-01-01", dtype="datetime64[D]")
        ax = fig.subplots()
        getattr(ax, plot_fun)([], [])
        getattr(ax, plot_fun)(x, range(len(x)), color="C0")

    return build


for _plot_fun in ["scatter", "plot", "fill_between"]:
    case(f"{GROUP}/limits_empty_data_{_plot_fun}")(_limits_empty_data(_plot_fun))


def _indicate_inset_inverted(x_inverted, y_inverted):
    """An inset indicator from the first subplot to the second, with the
    first subplot's axes inverted as asked."""

    def build(fig):
        ax1, ax2 = fig.subplots(1, 2)
        x = np.arange(10)
        ax1.plot(x, x, "o")
        if x_inverted:
            ax1.invert_xaxis()
        if y_inverted:
            ax1.invert_yaxis()
        ax1.indicate_inset([2, 2, 5, 4], ax2)

    return build


for _x in [False, True]:
    for _y in [False, True]:
        classic_case(f"indicate_inset_inverted_{'x' if _x else 'nox'}_{'y' if _y else 'noy'}")(
            _indicate_inset_inverted(_x, _y))


@classic_case("move_offsetlabel_y")
def _move_offsetlabel_y(fig):
    ax = fig.subplots()
    ax.plot(rng(7244).uniform(10) * 1e-22)
    ax.yaxis.tick_right()


@classic_case("move_offsetlabel_x")
def _move_offsetlabel_x(fig):
    ax = fig.subplots()
    ax.plot(rng(7244).uniform(10) * 1e-22)
    ax.xaxis.tick_top()


@classic_case("animated_artists_not_drawn")
def _animated_artists_not_drawn(fig):
    ax1, ax2 = fig.subplots(ncols=2)
    imdata = rng(10416).uniform(400).reshape(20, 20)
    ax1.imshow(imdata, animated=True)
    ax2.plot(imdata[0], animated=True)
