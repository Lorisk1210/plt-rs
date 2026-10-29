"""Plotting values with units on 3-D axes: dates, category names and
``jpl_units`` distances on the axis each lies along.

matplotlib converts no plotting values on 3-D axes by itself (its 3-D
plotting methods hand them to numpy as they are), so these cases convert
them first with ``Axes3D._process_unit_info``, as pltrs' 3-D plotting
methods convert the values they are given: the axes take the converter,
tick helpers and label of the values.

* ``dates_on_z``: a helix and a scatter rising over four days on z.
* ``names_bar3d``: boxes at names on x and y, a grid of regions by year.
* ``names_scatter_text``: a scatter with names on y and a text placed at
  one of them.
* ``names_bar_zdir``: rows of flat bars at names, placed in planes normal
  to y.
* ``jpl_metres_on_z``: a line in kilometres on a z axis in metres, the
  unit set before the line, which takes the converter's label and tick
  formatter.
"""

import datetime

import numpy as np

from spec_support import case
from spec_support.units import jpl_registered

GROUP = "mplot3d_units"


def _ax3d(fig):
    return fig.add_subplot(projection="3d")


def _converted(ax, x, y, z):
    return ax._process_unit_info([("x", x), ("y", y), ("z", z)])


def _quarter_days(count):
    """``count`` instants six hours apart from 2021-03-01."""
    start = datetime.datetime(2021, 3, 1)
    return [start + datetime.timedelta(hours=6 * i) for i in range(count)]


@case(f"{GROUP}/dates_on_z", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    t = np.linspace(0.0, 2 * np.pi, 17)
    days = _quarter_days(17)
    ax.plot(*_converted(ax, np.cos(t), np.sin(t), days))
    ax.scatter(*_converted(ax, 0.5 * np.cos(t), 0.5 * np.sin(t), days), color="tab:orange")


@case(f"{GROUP}/names_bar3d", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    regions = ["north", "south", "east", "west"]
    years = ["2019", "2020", "2021"]
    x = [region for _ in years for region in regions]
    y = [year for year in years for _ in regions]
    heights = [1.0 + ((3 * i) % 7) / 2.0 for i in range(len(x))]
    x, y, _ = _converted(ax, x, y, None)
    ax.bar3d(x, y, 0.0, 0.5, 0.5, heights, color="tab:cyan")


@case(f"{GROUP}/names_scatter_text", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    x = np.linspace(0.0, 1.0, 9)
    names = ["low", "mid", "high"] * 3
    ax.scatter(*_converted(ax, x, names, x**2), c=x, cmap="viridis")
    ax.text(0.5, ax.yaxis.convert_units("mid"), 0.8, "mid point")


@case(f"{GROUP}/names_bar_zdir", figsize=(4, 4))
def _(fig):
    ax = _ax3d(fig)
    months = ["jan", "feb", "mar", "apr", "may"]
    for row, color in enumerate(["tab:red", "tab:green", "tab:blue"]):
        heights = [1.0 + ((row + 2 * i) % 5) for i in range(len(months))]
        (left,) = ax._process_unit_info([("x", months)])
        ax.bar(left, heights, zs=row, zdir="y", color=color, alpha=0.8)


@case(f"{GROUP}/jpl_metres_on_z", figsize=(4, 4))
def _(fig):
    import matplotlib.testing.jpl_units as units

    with jpl_registered():
        km = units.UnitDbl(1.0, "km")
        ax = _ax3d(fig)
        ax.zaxis.set_units("m")
        t = np.linspace(0.0, 1.0, 20)
        ax.plot(*_converted(ax, np.cos(4 * t), np.sin(4 * t), [(1 + v) * km for v in t]))
