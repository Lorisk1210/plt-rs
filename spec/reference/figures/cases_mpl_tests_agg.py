"""matplotlib's own tests of the Agg canvas (``tests/test_agg.py``) that draw
a figure and only check that drawing it works: one case per figure.

Every test runs in the test suite's default style, ``classic`` plus
``_classic_test_patch``, on classic's 8 x 6 in figure saved at 100 dpi.

* ``large_single_path_collection``: a ``PathCollection`` of one rectangle
  much larger than the view (x limits ``1e-3`` to 1), which Agg must not
  draw as markers.
* ``marker_with_nan``: ``semilogx`` of 0 to 999 and ``fill_between`` of 0.8
  and 1.2 times it, whose first point is lost to the log scale.
* ``long_path``: 100 000 points alternating between -1 and 1.
* ``chunksize`` and ``chunksize_105``: the sine of 0 to 199, drawn whole and
  with ``agg.path.chunksize = 105``.
* ``non_tuple_rgbaface``: a 3-D scatter of three points with a ``Stroke``
  path effect 4 points wide.
"""

import matplotlib.style
import numpy as np
from matplotlib import collections, patheffects
from matplotlib.path import Path

from spec_support import case

GROUP = "mpl_tests_agg"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


def classic(name, *, rc=None):
    """Register ``build`` as ``mpl_tests_agg/<name>`` in the test suite's
    default style."""
    return case(f"{GROUP}/{name}", figsize=(8, 6), dpi=100, rc={**CLASSIC, **(rc or {})})


@classic("large_single_path_collection")
def _(fig):
    ax = fig.subplots()
    collection = collections.PathCollection(
        [Path([[-10, 5], [10, 5], [10, -5], [-10, -5], [-10, 5]])])
    ax.add_artist(collection)
    ax.set_xlim(10**-3, 1)


@classic("marker_with_nan")
def _(fig):
    ax = fig.subplots(1)
    data = np.arange(1000)
    ax.semilogx(data)
    ax.fill_between(data, data * 0.8, data * 1.2)


@classic("long_path")
def _(fig):
    ax = fig.subplots()
    points = np.ones(100_000)
    points[::2] *= -1
    ax.plot(points)


def _sine(fig):
    x = range(200)
    fig.subplots().plot(x, np.sin(x))


classic("chunksize")(_sine)
classic("chunksize_105", rc={"agg.path.chunksize": 105})(_sine)


@classic("non_tuple_rgbaface")
def _(fig):
    fig.add_subplot(projection="3d").scatter(
        [0, 1, 2], [0, 1, 2], path_effects=[patheffects.Stroke(linewidth=4)])
