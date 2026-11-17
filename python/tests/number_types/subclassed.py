"""
C, the package's own classes subclassed: what a program's override of a
locator, formatter, norm or artist method receives where matplotlib's
code calls it on ``self``, on dates, 3-D and colour bar axes, and the
callables some plotting methods take.
"""

import datetime

import numpy as np

import pltrs.pyplot as plt
from pltrs import colors, dates, ticker


def locators(rows):
    """Overrides of package locators' methods."""
    seen = rows.seen("C locator subclasses")

    class Steps(ticker.MaxNLocator):
        def tick_values(self, vmin, vmax):
            seen.note("MaxNLocator.tick_values", vmin, vmax)
            return super().tick_values(vmin, vmax)

        def view_limits(self, dmin, dmax):
            seen.note("MaxNLocator.view_limits", dmin, dmax)
            return super().view_limits(dmin, dmax)

    class Logs(ticker.LogLocator):
        def tick_values(self, vmin, vmax):
            seen.note("LogLocator.tick_values", vmin, vmax)
            return super().tick_values(vmin, vmax)

        def nonsingular(self, vmin, vmax):
            seen.note("LogLocator.nonsingular", vmin, vmax)
            return super().nonsingular(vmin, vmax)

    class Auto(ticker.AutoLocator):
        def __call__(self):
            seen.note("AutoLocator.__call__ view interval", *self.axis.get_view_interval())
            return super().__call__()

    fig, (ax, lg) = plt.subplots(1, 2)
    ax.plot([1, 2, 3], [4, 5, 6])
    ax.xaxis.set_major_locator(Steps(4))
    ax.yaxis.set_major_locator(Auto())
    ax.set_autoscalex_on(True)
    lg.plot([1, 10, 100], [1, 10, 100])
    lg.set_yscale("log")
    lg.yaxis.set_major_locator(Logs())
    fig.canvas.draw()
    ax.autoscale_view()
    fig.canvas.draw()
    seen.close()


def formatters(rows):
    """Overrides of package formatters' methods, on plain, date, 3-D and
    colour bar axes."""
    seen = rows.seen("C formatter subclasses")

    class Scalar(ticker.ScalarFormatter):
        def __call__(self, x, pos=None):
            seen.note("ScalarFormatter.__call__", x, pos)
            return super().__call__(x, pos)

        def set_locs(self, locs):
            seen.note("ScalarFormatter.set_locs", locs)
            super().set_locs(locs)

        def format_data_short(self, value):
            seen.note("ScalarFormatter.format_data_short", value)
            return super().format_data_short(value)

    class Dates(dates.AutoDateFormatter):
        def __call__(self, x, pos=None):
            seen.note("AutoDateFormatter.__call__", x, pos)
            return super().__call__(x, pos)

    fig, (ax, dx) = plt.subplots(1, 2)
    ax.plot([1, 2, 3], [4, 5, 6])
    ax.xaxis.set_major_formatter(Scalar())
    day = datetime.datetime(2020, 1, 1)
    dx.plot([day, day + datetime.timedelta(days=5)], [1, 2])
    dx.xaxis.set_major_formatter(Dates(dx.xaxis.get_major_locator()))
    fig.canvas.draw()
    ax.format_coord(1.5, 4.5)
    three = plt.figure().add_subplot(projection="3d")
    three.plot([1, 2], [3, 4], [5, 6])
    three.zaxis.set_major_formatter(ticker.FuncFormatter(lambda x, pos: seen.note("3-D z FuncFormatter", x, pos) or ""))
    three.figure.canvas.draw()
    cf, cax = plt.subplots()
    cb = cf.colorbar(cax.imshow(np.arange(9).reshape(3, 3)))
    cb.formatter = ticker.FuncFormatter(lambda x, pos: seen.note("colorbar of ints FuncFormatter", x, pos) or "")
    cf.canvas.draw()
    seen.close()


def mapping(rows):
    """Norm subclasses on scatters and meshes of ints, and the reducing
    function of `hexbin`."""
    seen = rows.seen("C norm subclasses")

    class Linear(colors.Normalize):
        def __call__(self, value, clip=None):
            seen.note("Normalize.__call__", value)
            return super().__call__(value, clip)

    class Logarithmic(colors.LogNorm):
        def __call__(self, value, clip=None):
            seen.note("LogNorm.__call__", value)
            return super().__call__(value, clip)

    fig, ax = plt.subplots(figsize=(2, 2), dpi=30)
    ax.scatter([1, 2], [1, 2], c=np.array([1, 2]), norm=Linear())
    ax.pcolormesh(np.arange(1, 5).reshape(2, 2), norm=Logarithmic())
    ax.hexbin([1, 2, 3], [1, 2, 3], C=[1, 2, 3], gridsize=2,
              reduce_C_function=lambda values: seen.note("hexbin reduce_C_function", values) or 0)
    fig.canvas.draw()
    seen.close()


SCENES = (locators, formatters, mapping)
