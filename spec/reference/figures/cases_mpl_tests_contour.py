"""matplotlib's non-image tests from ``tests/test_contour.py`` that only
check that a figure draws: ``test_subfigure_clabel``, the labelled
contours of two Gaussians in each of two subfigures. The other tests of
the file are probes of the ``mpl_contour``, ``contour_behaviour`` and
``contour`` areas or cases of ``mpl_polar``.
"""

import numpy as np

from figures._mpl_tests import group_case

GROUP = "mpl_tests_contour"
mpl_test = group_case(GROUP)


@mpl_test("subfigure_clabel")
def _(fig):
    delta = 0.025
    x = np.arange(-3.0, 3.0, delta)
    y = np.arange(-2.0, 2.0, delta)
    X, Y = np.meshgrid(x, y)
    Z1 = np.exp(-(X**2) - Y**2)
    Z2 = np.exp(-((X - 1) ** 2) - (Y - 1) ** 2)
    Z = (Z1 - Z2) * 2
    for f in fig.subfigures(nrows=1, ncols=2):
        ax = f.subplots()
        cs = ax.contour(X, Y, Z)
        ax.clabel(cs, inline=True, fontsize=10)
        ax.set_title("Simplest default with labels")
