"""Artists with values in units when the units change, and the gaps the
earlier units work left: which artists convert their values again when an
axis changes unit, error bars computed in the units of the data, date
patches and texts that leave the date ticks alone, and units set on shared
axes.

The converters are registered only while a case is built
(``spec_support.units``); the axes convert values with the converter they
took while being built, so drawing needs no registry.

* ``rectangle_set_units``: ``jpl_units`` distances in kilometres: a line, a
  ``Rectangle`` (it converts its corners whenever it is drawn), an
  ``axvspan``, a bar and a ``fill_between`` (converted once, when made),
  then the x axis in miles and the y axis in metres.
* ``texts_set_units``: a text placed before any data, a line, a text, an
  annotation with both points in data coordinates and one with its text
  offset in points, and a scatter (converted once), then the axes in miles
  and metres.
* ``date_rectangle_ticks``: rectangles with a date corner and a duration
  width on axes with numeric limits, and a text and an annotation at dates
  on other axes with numeric limits: converting their values only sets the
  date converter, so both axes keep their numeric ticks.
* ``errorbar_temperatures``: error bars of temperatures, which are their
  own error sizes, on an axis in degrees Fahrenheit (the bars end at the
  values less and plus the errors converted, not at the converted values
  less and plus the converted errors), and on an axis in degrees Celsius
  that then changes to kelvin (the data line and the caps convert again,
  the bars do not).
* ``shared_set_units``: two axes sharing x, the lower one twinned: a line in
  kilometres on the upper axes, a rectangle, a text and the twin's line on
  the lower ones, then the upper x axis in miles, which every axes sharing
  it follows.
"""

import datetime

from matplotlib.patches import Rectangle

from spec_support import case
from spec_support.units import Temperature, jpl_registered, temperature_registered

GROUP = "units"


def _km():
    import matplotlib.testing.jpl_units as units

    return units.UnitDbl(1.0, "km")


@case(f"{GROUP}/rectangle_set_units")
def _(fig):
    with jpl_registered():
        km = _km()
        ax = fig.subplots()
        ax.plot([0 * km, 10 * km], [0 * km, 6 * km], "k-")
        ax.add_patch(Rectangle((2 * km, 1 * km), 3 * km, 2 * km, alpha=0.5))
        ax.axvspan(6 * km, 7 * km, color="C1", alpha=0.3)
        ax.bar([8 * km], [2 * km], width=1 * km, color="C2")
        ax.fill_between([0 * km, 2 * km], [4 * km, 5 * km], [5 * km, 6 * km], color="C3")
        ax.xaxis.set_units("mile")
        ax.yaxis.set_units("m")


@case(f"{GROUP}/texts_set_units")
def _(fig):
    with jpl_registered():
        km = _km()
        ax = fig.subplots()
        ax.text(1 * km, 5 * km, "before the data")
        ax.plot([0 * km, 10 * km], [0 * km, 6 * km], "k-")
        ax.text(2 * km, 4 * km, "text")
        ax.annotate("data", xy=(5 * km, 3 * km), xytext=(7 * km, 1 * km),
                    arrowprops={"arrowstyle": "->"})
        ax.annotate("offset", xy=(3 * km, 2 * km), xytext=(10, -20),
                    textcoords="offset points", arrowprops={"arrowstyle": "->"})
        ax.scatter([1 * km, 4 * km], [1 * km, 5 * km])
        ax.xaxis.set_units("mile")
        ax.yaxis.set_units("m")


@case(f"{GROUP}/date_rectangle_ticks")
def _(fig):
    day = datetime.timedelta(days=1)
    start = datetime.datetime(2020, 1, 1)
    ax1, ax2 = fig.subplots(2, 1)
    ax1.add_patch(Rectangle((start, 0), 2 * day, 1))
    ax1.add_patch(Rectangle((start + 3 * day, 0.5), day / 2, 0.25, color="C1"))
    ax1.set_xlim(18261, 18266)
    ax1.set_ylim(0, 1.5)
    ax2.set_xlim(18261, 18266)
    ax2.text(start + day, 0.5, "a date")
    ax2.annotate("a point", xy=(start + 3 * day, 0.25), xytext=(start + 2 * day, 0.75),
                 arrowprops={"arrowstyle": "->"})


@case(f"{GROUP}/errorbar_temperatures")
def _(fig):
    with temperature_registered():
        x = [0, 1, 2, 3]
        y = [Temperature(t) for t in (20.0, 25.0, 30.0, 35.0)]
        ax1, ax2 = fig.subplots(1, 2)
        ax1.yaxis.set_units("F")
        ax1.errorbar(x, y, yerr=Temperature(5.0), capsize=4, fmt="o")
        ax2.errorbar(x, y, yerr=[Temperature(e) for e in (1.0, 2.0, 3.0, 4.0)], xerr=0.2,
                     capsize=4, fmt="s")
        ax2.yaxis.set_units("K")


@case(f"{GROUP}/shared_set_units")
def _(fig):
    with jpl_registered():
        km = _km()
        ax1, ax2 = fig.subplots(2, 1, sharex=True)
        ax1.plot([0 * km, 10 * km], [0, 1])
        ax2.add_patch(Rectangle((2 * km, 0.2), 3 * km, 0.5))
        ax2.text(6 * km, 0.5, "shared")
        twin = ax2.twinx()
        twin.plot([1 * km, 9 * km], [5, 3], "C1")
        ax2.set_ylim(0, 1)
        ax1.xaxis.set_units("mile")
