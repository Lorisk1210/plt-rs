"""matplotlib's own tests of ``Axes`` (``tests/test_axes.py``, the tests
defined before line 5585) that draw without an image comparison and were
not ported by the ``mpl_axes_*`` groups: the ``fig_test`` side of
``test_limits_empty_data``, one case per plotting function.

The test picks the ``default`` style, which needs no rc, on the default
canvas. Plotting empty data first must not change the autoscaled limits
of the dates plotted next; the ``mpl_axes`` probes compare the limits of
both sides.
"""

import numpy as np

from spec_support import case

GROUP = "mpl_tests_axes"


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
