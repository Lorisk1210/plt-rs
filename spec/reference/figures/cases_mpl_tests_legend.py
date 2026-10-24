"""matplotlib's non-image tests from ``tests/test_legend.py`` that only
draw: ``test_legend_face_edgecolor`` (the legend of a ``fill_between``
whose edges take the face colour) and ``test_nanscatter``, whose two
figures are two cases: the legend of a scatter of one NaN point, and that
of three scatters of random points over a grid. The other tests of the
file are probes of the ``mpl_legend`` area.
"""

import numpy as np

from figures._mpl_tests import group_case
from spec_support import rng

GROUP = "mpl_tests_legend"
mpl_test = group_case(GROUP)


@mpl_test("legend_face_edgecolor")
def _(fig):
    ax = fig.subplots()
    ax.fill_between([0, 1, 2], [1, 2, 3], [2, 3, 4],
                    facecolor='r', edgecolor='face', label='Fill')
    ax.legend()


@mpl_test("nanscatter")
def _(fig):
    ax = fig.subplots()
    h = ax.scatter([np.nan], [np.nan], marker="o", facecolor="r", edgecolor="r", s=3)
    ax.legend([h], ["scatter"])


@mpl_test("nanscatter_colors")
def _(fig):
    ax = fig.subplots()
    r = rng(0)
    n = 750
    for color in ['red', 'green', 'blue']:
        xy = r.uniform(2 * n)
        scale = 200.0 * r.uniform(n)
        ax.scatter(xy[:n], xy[n:], c=color, s=scale, label=color,
                   alpha=0.3, edgecolors='none')
    ax.legend()
    ax.grid(True)
