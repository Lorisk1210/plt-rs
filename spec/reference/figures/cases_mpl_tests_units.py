"""matplotlib's own tests of unit conversion (``tests/test_units.py``) that
draw: the image tests and the tests that only check that drawing works, one
case per figure.

The tests without a style of their own run in the test suite's default
style, ``classic`` plus ``_classic_test_patch``, on classic's 8 x 6 in
figure saved at 100 dpi; ``mpl20`` is the reference configuration, at the
test's ``savefig_kwarg`` dpi. ``remove_text=True`` becomes
``remove_ticks_and_titles`` at the end of the build, and the test's own
rcParams change becomes the case's ``rc``. Converters are registered only
while a case is built (``spec_support.units``), as the test fixture
restores the registry after each test; ``jpl_units.register()`` registers
the same converters.

* ``empty_arrays``: a scatter of an empty ``datetime64[ns]`` array.
* ``scatter_element0_masked``: a scatter of the days of February 2005 whose
  first value is NaN.
* ``errorbar_mixed_units``: dates of May 2020 against 0 to 9 with error bars
  of half a day.
* ``numpy_facade``: ``Quantity`` hours against miles, a line at 26400 feet
  and one at 120 minutes, the axes then set to seconds and inches.
* ``plot_masked_units``: a masked array in a ``Quantity`` facade, with no
  converter registered for it, plotted against its index.
* ``jpl_bar_units`` and ``jpl_barh_units``: bars at ``jpl_units`` distances
  as long as durations from an epoch.
"""

import datetime

import matplotlib.style
import numpy as np
from matplotlib.testing.decorators import remove_ticks_and_titles

from spec_support import case
from spec_support.units import Quantity, jpl_registered, quantity_registered

GROUP = "mpl_tests_units"

#: The test suite's default style: ``classic`` plus ``_classic_test_patch``.
CLASSIC = {**matplotlib.style.library["classic"],
           **matplotlib.style.library["_classic_test_patch"]}


def classic(name):
    """Register ``build`` as ``mpl_tests_units/<name>`` in the test suite's
    default style."""
    return case(f"{GROUP}/{name}", figsize=(8, 6), dpi=100, rc=CLASSIC)


def mpl20(name, *, dpi=100, remove_text=False, rc=None):
    """Register ``build`` as ``mpl_tests_units/<name>`` in the ``mpl20``
    style at ``dpi``."""
    def register(build):
        def run(fig):
            build(fig)
            if remove_text:
                remove_ticks_and_titles(fig)

        case(f"{GROUP}/{name}", dpi=dpi, rc=rc or {})(run)
        return build

    return register


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


@mpl20("numpy_facade", rc={"axes.formatter.limits": (-7, 7)})
def _(fig):
    with quantity_registered():
        y = Quantity(np.linspace(0, 30), 'miles')
        x = Quantity(np.linspace(0, 5), 'hours')
        ax = fig.subplots()
        fig.subplots_adjust(left=0.15)
        ax.plot(x, y, 'tab:blue')
        ax.axhline(Quantity(26400, 'feet'), color='tab:red')
        ax.axvline(Quantity(120, 'minutes'), color='tab:green')
        ax.yaxis.set_units('inches')
        ax.xaxis.set_units('seconds')


@mpl20("plot_masked_units", remove_text=True)
def _(fig):
    data = np.linspace(-5, 5)
    data_masked = np.ma.array(data, mask=(data > -2) & (data < 2))
    fig.subplots().plot(Quantity(data_masked, 'meters'))


def _jpl_bars(fig, horizontal):
    """``test_jpl_bar_units`` (``horizontal`` false) or
    ``test_jpl_barh_units``."""
    import matplotlib.testing.jpl_units as units

    with jpl_registered():
        day = units.Duration("ET", 24.0 * 60.0 * 60.0)
        x = [0 * units.km, 1 * units.km, 2 * units.km]
        w = [1 * day, 2 * day, 3 * day]
        b = units.Epoch("ET", dt=datetime.datetime(2009, 4, 26))
        ax = fig.subplots()
        if horizontal:
            ax.barh(x, w, left=b)
            ax.set_xlim([b - 1 * day, b + w[-1] + (1.001) * day])
        else:
            ax.bar(x, w, bottom=b)
            ax.set_ylim([b - 1 * day, b + w[-1] + (1.001) * day])


@mpl20("jpl_bar_units", dpi=120)
def _(fig):
    _jpl_bars(fig, horizontal=False)


@mpl20("jpl_barh_units", dpi=120)
def _(fig):
    _jpl_bars(fig, horizontal=True)
