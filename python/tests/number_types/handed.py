"""
C, what pltrs computes with: the arguments a program's own classes and
functions receive where matplotlib calls them while it lays out and draws
(locators, formatters, norms, scales, transforms, converters, the
formatting functions of colour bars, contour labels, bar labels and pies,
path effects and image filters).
"""

import numpy as np

import pltrs.pyplot as plt
from pltrs import colors, patheffects, scale, ticker, transforms, units


def program_locator(seen):
    """A locator of the program's noting its arguments into *seen*."""

    class Locator(ticker.Locator):
        def __call__(self):
            vmin, vmax = self.axis.get_view_interval()
            seen.note("Locator.axis.get_view_interval", vmin, vmax)
            return self.tick_values(vmin, vmax)

        def tick_values(self, vmin, vmax):
            seen.note("Locator.tick_values", vmin, vmax)
            return [vmin, vmax]

        def view_limits(self, vmin, vmax):
            seen.note("Locator.view_limits", vmin, vmax)
            return vmin, vmax

        def nonsingular(self, vmin, vmax):
            seen.note("Locator.nonsingular", vmin, vmax)
            return vmin, vmax

    return Locator()


def tickers(rows):
    """A locator and formatters of the program's on a plain, a log and a
    polar Axes and on a colour bar."""
    seen = rows.seen("C ticker")

    class Formatter(ticker.Formatter):
        def __call__(self, x, pos=None):
            seen.note("Formatter.__call__", x, pos)
            return f"{x}"

        def format_ticks(self, values):
            seen.note("Formatter.format_ticks", values)
            return super().format_ticks(values)

        def format_data(self, value):
            seen.note("Formatter.format_data", value)
            return str(value)

        def format_data_short(self, value):
            seen.note("Formatter.format_data_short", value)
            return str(value)

        def set_locs(self, locs):
            seen.note("Formatter.set_locs", locs)
            super().set_locs(locs)

    fig, ax = plt.subplots()
    ax.plot([1, 2, 3], [1, 4, 9])
    ax.xaxis.set_major_locator(program_locator(seen))
    ax.xaxis.set_major_formatter(Formatter())
    ax.yaxis.set_major_formatter(ticker.FuncFormatter(
        lambda x, pos: seen.note("FuncFormatter", x, pos) or ""))
    ax.yaxis.set_minor_formatter(
        lambda x, pos: seen.note("set_minor_formatter(function)", x, pos) or "")
    ax.yaxis.set_minor_locator(ticker.AutoMinorLocator())
    fig.canvas.draw()
    ax.format_coord(1.5, 2.5)
    seen.close()

    on_log = rows.seen("C ticker on a log axis")
    lg = plt.figure().add_subplot()
    lg.plot([1, 10, 100], [1, 2, 3])
    lg.set_xscale("log")
    lg.xaxis.set_major_formatter(ticker.FuncFormatter(lambda x, pos: on_log.note("FuncFormatter", x, pos) or ""))
    lg.xaxis.set_minor_formatter(ticker.FuncFormatter(lambda x, pos: on_log.note("minor FuncFormatter", x, pos) or ""))
    lg.figure.canvas.draw()
    on_log.close()

    polar = rows.seen("C ticker on a polar axis")
    pa = plt.figure().add_subplot(projection="polar")
    pa.plot([0, 1, 2], [1, 2, 3])
    pa.xaxis.set_major_formatter(ticker.FuncFormatter(lambda x, pos: polar.note("theta FuncFormatter", x, pos) or ""))
    pa.yaxis.set_major_formatter(ticker.FuncFormatter(lambda x, pos: polar.note("r FuncFormatter", x, pos) or ""))
    pa.figure.canvas.draw()
    polar.close()

    bar = rows.seen("C ticker on a colour bar")
    cf, cax = plt.subplots()
    image = cax.imshow(np.arange(9).reshape(3, 3))
    cb = cf.colorbar(image, format=ticker.FuncFormatter(lambda x, pos: bar.note("FuncFormatter", x, pos) or ""))
    cb.ax.yaxis.set_major_locator(program_locator(bar))
    cf.canvas.draw()
    bar.close()


def norms(rows):
    """Norms of the program's on images, scatters, contours and colour bars."""
    seen = rows.seen("C norm")

    class Norm(colors.Normalize):
        def __call__(self, value, clip=None):
            seen.note("Normalize.__call__", value)
            return super().__call__(value, clip)

        def inverse(self, value):
            seen.note("Normalize.inverse", value)
            return super().inverse(value)

        def autoscale_None(self, A):
            seen.note("Normalize.autoscale_None", A)
            return super().autoscale_None(A)

    fig, ax = plt.subplots()
    image = ax.imshow(np.arange(9).reshape(3, 3), norm=Norm())
    fig.colorbar(image)
    ax.scatter([1, 2], [1, 2], c=[1, 2], norm=Norm())
    ax.contourf(np.arange(9.0).reshape(3, 3), norm=Norm())
    fig.canvas.draw()
    seen.close()

    single = rows.seen("C norm of one image")
    for label, data in [("float32", np.float32(np.arange(4).reshape(2, 2))),
                        ("float64", np.arange(4.0).reshape(2, 2)),
                        ("int16", np.int16(np.arange(4).reshape(2, 2))),
                        ("uint8", np.uint8(np.arange(4).reshape(2, 2))),
                        ("masked", np.ma.masked_array(np.arange(4.0).reshape(2, 2), [[0, 1], [0, 0]]))]:
        class Noted(colors.Normalize):
            def __call__(self, value, clip=None, label=label):
                single.note(f"imshow({label}) Normalize.__call__", value)
                return super().__call__(value, clip)
        f, a = plt.subplots(figsize=(1, 1), dpi=20)
        a.imshow(data, norm=Noted())
        f.canvas.draw()
        f2, a2 = plt.subplots(figsize=(1, 1), dpi=20)
        a2.pcolormesh(data, norm=Noted())
        f2.canvas.draw()
    single.close()

    funcs = rows.seen("C FuncNorm")
    fn = colors.FuncNorm((lambda x: funcs.note("FuncNorm forward", x) or x,
                          lambda x: funcs.note("FuncNorm inverse", x) or x), 0, 8)
    fig7, ax7 = plt.subplots()
    ax7.imshow(np.arange(9).reshape(3, 3), norm=fn)
    fig7.canvas.draw()
    funcs.close()


def scales(rows):
    """A scale and transform of the program's, and the functions of
    ``FuncScale``, a secondary axis and ``FuncTransform``-like classes."""
    seen = rows.seen("C scale and transform")

    class Forward(transforms.Transform):
        input_dims = output_dims = 1

        def transform_non_affine(self, values):
            seen.note("Transform.transform_non_affine", values)
            return values

        def inverted(self):
            return Forward()

    class Scale(scale.ScaleBase):
        name = "num1probe"

        def get_transform(self):
            return Forward()

        def set_default_locators_and_formatters(self, axis):
            axis.set_major_locator(ticker.AutoLocator())
            axis.set_major_formatter(ticker.ScalarFormatter())

        def limit_range_for_scale(self, vmin, vmax, minpos):
            seen.note("Scale.limit_range_for_scale", vmin, vmax, minpos)
            return vmin, vmax

    scale.register_scale(Scale)
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3], [1, 2, 3])
    ax.set_xscale("num1probe")
    fig.canvas.draw()
    fs = plt.figure().add_subplot()
    fs.plot([1, 4], [1, 2])
    fs.set_xscale("function", functions=(lambda x: seen.note("FuncScale forward", x) or x,
                                         lambda x: seen.note("FuncScale inverse", x) or x))
    fs.get_figure().canvas.draw()
    fs.secondary_xaxis("top", functions=(lambda x: seen.note("secondary forward", x) or x,
                                         lambda x: seen.note("secondary inverse", x) or x))
    fs.get_figure().canvas.draw()
    seen.close()

    two = rows.seen("C transform of points")

    class Shift(transforms.Transform):
        input_dims = output_dims = 2

        def transform_non_affine(self, values):
            two.note("transform_non_affine", values)
            return values + 1

        def transform_path_non_affine(self, path):
            two.note("transform_path_non_affine vertices", path.vertices)
            return super().transform_path_non_affine(path)

        def inverted(self):
            return Shift()

    fig, ax = plt.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    ax.plot([1, 2, 3], [1, 2, 3], transform=Shift() + ax.transData)
    ax.scatter([1, 2], [1, 2], transform=Shift() + ax.transData)
    ax.add_patch(plt.Rectangle((1, 1), 2, 2, transform=Shift() + ax.transData))
    ax.text(1, 1, "t", transform=Shift() + ax.transData)
    ax.plot(np.float32([1, 2]), np.float32([3, 4]), transform=Shift() + ax.transData)
    fig.canvas.draw()
    two.close()


def converters(rows):
    """A converter of the program's, for a class of numbers it registers."""
    seen = rows.seen("C units")

    class Quantity(float):
        pass

    class Converter(units.ConversionInterface):
        @staticmethod
        def convert(value, unit, axis):
            seen.note("ConversionInterface.convert", value)
            return np.asarray(value, float) if np.iterable(value) else float(value)

        @staticmethod
        def axisinfo(unit, axis):
            seen.note("ConversionInterface.axisinfo", unit)
            return units.AxisInfo()

        @staticmethod
        def default_units(x, axis):
            seen.note("ConversionInterface.default_units", x)
            return "u"

    units.registry[Quantity] = Converter()
    try:
        fig, ax = plt.subplots()
        ax.plot([Quantity(1), Quantity(2)], [1, 2])
        ax.set_xlim(Quantity(0), Quantity(3))
        fig.canvas.draw()
        f2, a2 = plt.subplots()
        a2.bar([Quantity(1), Quantity(2)], [1, 2])
        a2.scatter([Quantity(1)], [1])
        a2.axvline(Quantity(1))
        a2.text(Quantity(1), 1, "t")
        a2.fill_between([Quantity(1), Quantity(2)], [1, 2])
        f2.canvas.draw()
    finally:
        del units.registry[Quantity]
    seen.close()


def formatting(rows):
    """The functions that format colour bar ticks, contour labels, bar
    labels and pie wedges."""
    seen = rows.seen("C colorbar and clabel")
    figc, axc = plt.subplots()
    imc = axc.imshow(np.arange(4.0).reshape(2, 2))
    figc.colorbar(imc, format=lambda x, pos: seen.note("colorbar format", x, pos) or f"{x}")
    axc.clabel(axc.contour(np.arange(9.0).reshape(3, 3), levels=[2, 4]),
               fmt=lambda v: seen.note("clabel fmt", v) or f"{v}")
    axc.clabel(axc.contour(np.arange(9).reshape(3, 3), levels=[3, 5]),
               fmt=lambda v: seen.note("clabel fmt of int data", v) or f"{v}")
    figc.canvas.draw()
    seen.close()

    labels = rows.seen("C bar_label and pie")
    fig, ax = plt.subplots()
    ax.bar_label(ax.bar([1, 2], [3, 4]), fmt=lambda v: labels.note("bar_label fmt ints", v) or f"{v}")
    ax.bar_label(ax.bar([1, 2], [3.5, 4.5]), fmt=lambda v: labels.note("bar_label fmt floats", v) or f"{v}")
    ax.bar_label(ax.barh([1, 2], [3, 4]), fmt=lambda v: labels.note("bar_label fmt barh", v) or f"{v}")
    ax.pie([1, 2, 3], autopct=lambda p: labels.note("pie autopct", p) or f"{p}")
    ax.pie(np.float32([1, 2]), autopct=lambda p: labels.note("pie autopct float32", p) or f"{p}")
    fig.canvas.draw()
    labels.close()


def drawing(rows):
    """Path effects and image filters of the program's."""
    seen = rows.seen("C path effects and filters")

    class Effect(patheffects.AbstractPathEffect):
        def draw_path(self, renderer, gc, tpath, affine, rgbFace=None):
            seen.note("draw_path tpath.vertices, affine matrix, rgbFace",
                      tpath.vertices, affine.get_matrix(), rgbFace)
            renderer.draw_path(gc, tpath, affine, rgbFace)

    fig, ax = plt.subplots(figsize=(2, 2), dpi=40)
    ax.plot([1, 2], [3, 4], path_effects=[Effect()])
    ax.add_patch(plt.Rectangle((1, 3), 1, 1, fc=(1, 0, 0), path_effects=[Effect()]))
    ax.text(1.5, 3.5, "t", path_effects=[Effect()])

    def agg_filter(image, dpi):
        seen.note("agg_filter image, dpi", image, dpi)
        return image, 0, 0

    ax.plot([1, 2], [4, 3], agg_filter=agg_filter)
    fig.canvas.draw()
    seen.close()


SCENES = (tickers, norms, scales, converters, formatting, drawing)
