"""matplotlib's own tests of unit conversion (``tests/test_units.py``) that
draw dates and only check that drawing works: one case per figure.

Every test runs in the test suite's default style, ``classic`` plus
``_classic_test_patch``, on classic's 8 x 6 in figure saved at 100 dpi.

* ``empty_arrays``: a scatter of an empty ``datetime64[ns]`` array.
* ``scatter_element0_masked``: a scatter of the days of February 2005 whose
  first value is NaN.
* ``errorbar_mixed_units``: dates of May 2020 against 0 to 9 with error bars
  of half a day.
"""

import datetime

import matplotlib.style
import numpy as np

from spec_support import case

GROUP = "mpl_tests_units"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


def classic(name):
    """Register ``build`` as ``mpl_tests_units/<name>`` in the test suite's
    default style."""
    return case(f"{GROUP}/{name}", figsize=(8, 6), dpi=100, rc=CLASSIC)


@classic("empty_arrays")
def _(fig):
    fig.subplots().scatter(np.array([], dtype="datetime64[ns]"), np.array([]))


@classic("scatter_element0_masked")
def _(fig):
    times = np.arange("2005-02", "2005-03", dtype="datetime64[D]")
    y = np.arange(len(times), dtype=float)
    y[0] = np.nan
    fig.subplots().scatter(times, y)


@classic("errorbar_mixed_units")
def _(fig):
    x = np.arange(10)
    y = [datetime.datetime(2020, 5, i * 2 + 1) for i in x]
    fig.subplots().errorbar(x, y, datetime.timedelta(days=0.5))
