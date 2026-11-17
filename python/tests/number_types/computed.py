"""
R, values matplotlib computes: limits and positions, boxes and
transforms, tickers, colours, dates, ``cbook`` and ``mlab``, paths and
Bézier curves, text extents from each renderer, all from ints (and numpy
scalars and arrays of other types) where a program gives them.
"""

import datetime
import io

import numpy as np

import pltrs.pyplot as plt
from pltrs import bezier, cbook, colors, dates, mlab, ticker, transforms
from pltrs.font_manager import FontProperties
from pltrs.path import Path

#: A day as the dates module's tests give it.
DAY = datetime.datetime(2020, 1, 2, 3, 4, 5)


def limits(rows):
    """Limits, positions and sizes after plotting ints."""
    fig, ax = plt.subplots(figsize=(4, 3), dpi=72)
    ax.plot([1, 2, 3], [4, 5, 6])
    for name in ("get_xlim", "get_ylim", "get_xbound", "get_position", "get_anchor"):
        rows.row("R Axes ints", f"{name}()", getattr(ax, name))
    rows.row("R Axes ints", "viewLim", lambda: ax.viewLim)
    rows.row("R Axes ints", "dataLim", lambda: ax.dataLim)
    rows.row("R Axes ints", "viewLim.x0, width", lambda: (ax.viewLim.x0, ax.viewLim.width))
    rows.row("R Axes ints", "dataLim.extents", lambda: ax.dataLim.extents)
    rows.row("R Axes ints", "get_position(original=True).bounds",
             lambda: ax.get_position(original=True).bounds)
    rows.row("R Axes ints", "bbox.bounds", lambda: ax.bbox.bounds)
    rows.row("R Figure", "get_size_inches()", fig.get_size_inches)
    rows.row("R Figure", "dpi_scale_trans matrix", lambda: fig.dpi_scale_trans.get_matrix())
    fig.canvas.draw()
    rows.row("R Axes ints drawn", "get_window_extent()", ax.get_window_extent)
    rows.row("R Axes ints drawn", "get_tightbbox()", ax.get_tightbbox)
    rows.row("R Axes ints drawn", "xaxis.get_tick_padding()", ax.xaxis.get_tick_padding)
    rows.row("R Axes ints drawn", "xaxis.get_ticklocs()", ax.xaxis.get_ticklocs)
    rows.row("R Axes ints drawn", "xaxis.get_minorticklocs()", ax.xaxis.get_minorticklocs)
    rows.row("R Axes ints drawn", "xaxis.label.get_position()", ax.xaxis.label.get_position)
    rows.row("R Axes ints drawn", "title.get_position()", ax.title.get_position)
    rows.row("R Axes ints drawn", "xaxis.get_ticklabels()[0].get_position()",
             lambda: ax.xaxis.get_ticklabels()[0].get_position())
    ix = plt.figure().add_subplot()
    ix.plot(np.arange(3), np.arange(3))
    rows.row("R Axes np.int64 data", "get_xlim()", ix.get_xlim)
    ix.set_xlim(np.int64(1), np.int64(2))
    rows.row("R Axes np.int64 limits", "get_xlim()", ix.get_xlim)
    rows.row("R Axes np.float32 limits", "set_xlim returns",
             lambda: ix.set_xlim(np.float32(1), np.float32(2)))
    rows.row("R Axes", "axis() after ints", lambda: ix.axis([0, 1, 2, 3]))
    rows.row("R Axes", "get_xlim() after axis(ints)", ix.get_xlim)
    lg = plt.figure().add_subplot()
    lg.plot([1, 10, 100], [1, 2, 3])
    lg.set_xscale("log")
    rows.row("R Axes log", "get_xlim()", lg.get_xlim)
    rows.row("R Axes log", "set_xlim(1, 100) returns", lambda: lg.set_xlim(1, 100))
    lg.figure.canvas.draw()
    rows.row("R Axes log", "get_xticks()", lg.get_xticks)


def boxes(rows):
    """Bbox and Affine2D methods given ints."""
    box = transforms.Bbox.from_bounds(1, 2, 3, 4)
    for name in ("bounds", "extents", "width", "height", "x0", "y1", "intervalx", "p0",
                 "size", "min", "max", "xmin"):
        rows.row("R Bbox.from_bounds ints", name, lambda n=name: getattr(box, n))
    rows.row("R Bbox ints", "get_points()", box.get_points)
    rows.row("R Bbox([[ints]])", "get_points()", lambda: transforms.Bbox([[0, 0], [1, 2]]).get_points())
    rows.row("R Bbox.from_extents ints", "get_points()",
             lambda: transforms.Bbox.from_extents(0, 0, 1, 2).get_points())
    rows.row("R Bbox ints", "padded(1)", lambda: box.padded(1))
    rows.row("R Bbox ints", "expanded(2, 2)", lambda: box.expanded(2, 2))
    rows.row("R Bbox ints", "shrunk(1, 1)", lambda: box.shrunk(1, 1))
    rows.row("R Bbox ints", "translated(1, 1)", lambda: box.translated(1, 1))
    rows.row("R Bbox", "union", lambda: transforms.Bbox.union([box, transforms.Bbox.unit()]))
    rows.row("R Bbox", "intersection", lambda: transforms.Bbox.intersection(box, transforms.Bbox.unit()))
    rows.row("R Bbox", "contains(2, 3)", lambda: box.contains(2, 3))
    rows.row("R Bbox", "overlaps", lambda: box.overlaps(transforms.Bbox.unit()))
    rows.row("R Bbox", "count_contains", lambda: box.count_contains([(2, 3), (0, 0)]))
    rows.row("R Bbox", "count_overlaps", lambda: box.count_overlaps([transforms.Bbox.unit()]))
    rows.row("R Bbox", "fully_contains", lambda: box.fully_contains(2, 3))
    rows.row("R Bbox", "corners()", box.corners)
    rows.row("R Bbox", "splitx(0.5)", lambda: box.splitx(0.5))
    rows.row("R Bbox", "anchored('C', unit)", lambda: box.anchored("C", transforms.Bbox.unit()))
    rows.row("R Bbox", "transformed(Affine2D().scale(2))",
             lambda: box.transformed(transforms.Affine2D().scale(2)))
    rows.row("R Bbox", "update_from_data_xy(ints) then points",
             lambda: (lambda b: (b.update_from_data_xy(np.array([[1, 2], [3, 4]])), b.get_points())[1])(
                 transforms.Bbox.null()))
    rows.row("R Bbox", "null() bounds", lambda: transforms.Bbox.null().bounds)
    affine = transforms.Affine2D().rotate_deg(90).scale(2, 3).translate(1, 2)
    rows.row("R Affine2D", "get_matrix()", affine.get_matrix)
    rows.row("R Affine2D", "to_values()", affine.to_values)
    rows.row("R Affine2D", "transform(int array)", lambda: affine.transform(np.array([[1, 2]])))
    rows.row("R Affine2D", "transform(int list)", lambda: affine.transform([[1, 2]]))
    rows.row("R Affine2D", "transform_point((1, 2))", lambda: affine.transform_point((1, 2)))
    rows.row("R Affine2D", "transform(float32 array)",
             lambda: affine.transform(np.float32([[1, 2]])))
    rows.row("R Affine2D", "inverted().get_matrix()", lambda: affine.inverted().get_matrix())
    rows.row("R Affine2D(int matrix)", "get_matrix()",
             lambda: transforms.Affine2D(np.array([[1, 0, 0], [0, 1, 0], [0, 0, 1]])).get_matrix())
    rows.row("R Affine2D.from_values ints", "get_matrix()",
             lambda: transforms.Affine2D.from_values(1, 0, 0, 1, 2, 3).get_matrix())
    rows.row("R Affine2D", "transform_path(int path).vertices",
             lambda: affine.transform_path(Path([[0, 0], [1, 1]])).vertices)
    rows.row("R IdentityTransform", "transform(int array)",
             lambda: transforms.IdentityTransform().transform(np.array([[1, 2]])))
    rows.row("R IdentityTransform", "transform_path(int path).vertices",
             lambda: transforms.IdentityTransform().transform_path(Path([[0, 0], [1, 1]])).vertices)
    rows.row("R BboxTransform", "transform(int array)",
             lambda: transforms.BboxTransform(box, transforms.Bbox.unit()).transform(np.array([[1, 2]])))
    blended = transforms.blended_transform_factory(transforms.Affine2D().scale(2), transforms.IdentityTransform())
    rows.row("R blended", "transform(int array)", lambda: blended.transform(np.array([[1, 2]])))
    rows.row("R blended", "get_matrix()", blended.get_matrix)
    rows.row("R offset_copy", "get_matrix()",
             lambda: transforms.offset_copy(transforms.IdentityTransform(), plt.figure(), 1, 2,
                                            units="dots").get_matrix())
    fig, ax = plt.subplots()
    ax.set_xlim(0, 10)
    ax.set_ylim(0, 10)
    rows.row("R transData", "transform(int array)", lambda: ax.transData.transform(np.array([[1, 2]])))
    rows.row("R transData", "transform((1, 2))", lambda: ax.transData.transform((1, 2)))
    rows.row("R transData", "transform_point((1, 2))", lambda: ax.transData.transform_point((1, 2)))
    rows.row("R transData", "inverted().transform((100, 100))",
             lambda: ax.transData.inverted().transform((100, 100)))
    rows.row("R transData", "transform_path(int path).vertices",
             lambda: ax.transData.transform_path(Path(np.array([[0, 0], [1, 1]]))).vertices)
    rows.row("R transAxes", "transform(ints)", lambda: ax.transAxes.transform([[0, 1]]))
    rows.row("R transData", "get_matrix()", ax.transData.get_matrix)
    ax.set_xscale("log")
    rows.row("R transData log", "transform(int array)", lambda: ax.transData.transform(np.array([[1, 2]])))
    rows.row("R transScale log", "transform_non_affine(ints)",
             lambda: ax.transScale.transform_non_affine(np.array([[1, 2]])))
    polar = plt.figure().add_subplot(projection="polar")
    rows.row("R transData polar", "transform(int array)",
             lambda: polar.transData.transform(np.array([[1, 2]])))
    rows.row("R transProjection polar", "transform(ints)",
             lambda: polar.transProjection.transform(np.array([[1, 2]])))


def tickers(rows):
    """Locators and formatters called on ints."""
    for label, make in [("MaxNLocator", ticker.MaxNLocator),
                        ("MaxNLocator(integer)", lambda: ticker.MaxNLocator(integer=True)),
                        ("AutoLocator", ticker.AutoLocator),
                        ("MultipleLocator(1)", lambda: ticker.MultipleLocator(1)),
                        ("MultipleLocator(2.5)", lambda: ticker.MultipleLocator(2.5)),
                        ("LinearLocator", ticker.LinearLocator),
                        ("FixedLocator(ints)", lambda: ticker.FixedLocator([1, 2, 3])),
                        ("IndexLocator", lambda: ticker.IndexLocator(1, 0)),
                        ("LogLocator", ticker.LogLocator),
                        ("LogLocator(subs)", lambda: ticker.LogLocator(subs=(1, 2, 5))),
                        ("SymmetricalLogLocator", lambda: ticker.SymmetricalLogLocator(base=10, linthresh=1)),
                        ("AsinhLocator", lambda: ticker.AsinhLocator(1)),
                        ("LogitLocator", ticker.LogitLocator),
                        ("NullLocator", ticker.NullLocator)]:
        low, high = (0.01, 0.99) if label == "LogitLocator" else (1, 100) if "Log" in label else (0, 10)
        rows.row(f"R {label}", f"tick_values({low}, {high})",
                 lambda m=make, a=low, b=high: m().tick_values(a, b))
        rows.row(f"R {label}", f"tick_values(np.int64)",
                 lambda m=make, a=low, b=high: m().tick_values(np.int64(max(a, 1)), np.int64(b)))
        rows.row(f"R {label}", f"view_limits({low}, {high})",
                 lambda m=make, a=low, b=high: m().view_limits(a, b))
        rows.row(f"R {label}", "nonsingular(1, 1)", lambda m=make: m().nonsingular(1, 1))
        rows.row(f"R {label}", f"nonsingular({low}, {high})",
                 lambda m=make, a=low, b=high: m().nonsingular(a, b))
    rows.row("R MaxNLocator", "bin_boundaries(0, 10)",
             lambda: ticker.MaxNLocator()._raw_ticks(0, 10))
    fig, ax = plt.subplots()
    ax.plot([0, 10], [1, 100])
    ax.xaxis.set_major_locator(ticker.MaxNLocator(5))
    fig.canvas.draw()
    rows.row("R MaxNLocator on an axis", "__call__()", ax.xaxis.get_major_locator())
    minor = ticker.AutoMinorLocator(2)
    ax.xaxis.set_minor_locator(minor)
    rows.row("R AutoMinorLocator on an axis", "minor __call__()", minor)
    rows.row("R ScalarFormatter on an axis", "__call__(2)", lambda: ax.xaxis.get_major_formatter()(2))
    rows.row("R ScalarFormatter on an axis", "format_ticks(ints)",
             lambda: ax.xaxis.get_major_formatter().format_ticks([0, 5, 10]))
    rows.row("R ScalarFormatter on an axis", "locs", lambda: ax.xaxis.get_major_formatter().locs)
    rows.row("R ScalarFormatter on an axis", "orderOfMagnitude",
             lambda: ax.xaxis.get_major_formatter().orderOfMagnitude)
    rows.row("R ScalarFormatter on an axis", "format_data_short(2)",
             lambda: ax.xaxis.get_major_formatter().format_data_short(2))
    rows.row("R LogFormatter", "__call__(100)", lambda: ticker.LogFormatter()(100))
    rows.row("R EngFormatter", "format_eng(1000)", lambda: ticker.EngFormatter().format_eng(1000))
    rows.row("R ticker", "scale_range(0, 10)", lambda: ticker.scale_range(0, 10))
    rows.row("R ticker", "_Edge_integer(1, 0).le(3)", lambda: ticker._Edge_integer(1, 0).le(3))
    rows.row("R Axis", "get_major_locator()() ints", lambda: ax.yaxis.get_major_locator()())

    class ListLocator(ticker.Locator):
        def __call__(self):
            return [1, 2, 3]

    listed = plt.figure().add_subplot()
    listed.xaxis.set_major_locator(ListLocator())
    rows.row("R Axes with a locator returning a list", "get_xticks()", listed.get_xticks)
    rows.row("R Axes with a locator returning a list", "xaxis.get_majorticklocs()",
             listed.xaxis.get_majorticklocs)
    rows.row("R Axis", "get_view_interval()", ax.yaxis.get_view_interval)


def colours(rows):
    """Colours, norms and colormaps given every kind of number."""
    rows.row("R colors", "to_rgba((1, 0, 0), 1)", lambda: colors.to_rgba((1, 0, 0), 1))
    rows.row("R colors", "to_rgba('r', 0.5)", lambda: colors.to_rgba("r", 0.5))
    rows.row("R colors", "to_rgba('#ff000080')", lambda: colors.to_rgba("#ff000080"))
    rows.row("R colors", "to_rgba(np.array([1, 0, 0]))", lambda: colors.to_rgba(np.array([1, 0, 0])))
    rows.row("R colors", "to_rgba(np.float32 triple)", lambda: colors.to_rgba(np.float32([1, 0, 0])))
    rows.row("R colors", "to_rgba(('r', 1))", lambda: colors.to_rgba(("r", 1)))
    rows.row("R colors", "to_rgba('0.5')", lambda: colors.to_rgba("0.5"))
    rows.row("R colors", "to_rgba('C1')", lambda: colors.to_rgba("C1"))
    rows.row("R colors", "to_rgba_array(['r', 'g'])", lambda: colors.to_rgba_array(["r", "g"]))
    rows.row("R colors", "to_rgba_array(int array)", lambda: colors.to_rgba_array(np.array([[1, 0, 0]])))
    rows.row("R colors", "to_rgba_array(float32 array)",
             lambda: colors.to_rgba_array(np.float32([[1, 0, 0, 1]])))
    rows.row("R colors", "to_rgba_array('r', alpha=1)", lambda: colors.to_rgba_array("r", alpha=1))
    rows.row("R colors", "to_rgb('r')", lambda: colors.to_rgb("r"))
    rows.row("R colors", "rgb_to_hsv(ints)", lambda: colors.rgb_to_hsv([1, 0, 0]))
    rows.row("R colors", "hsv_to_rgb(ints)", lambda: colors.hsv_to_rgb([0, 1, 1]))
    rows.row("R colors", "same_color", lambda: colors.same_color("r", (1, 0, 0)))
    data = {"float": 5.0, "int": 5, "np.int64": np.int64(5), "np.float32": np.float32(5),
            "int array": np.array([2, 5]), "int32 array": np.array([2, 5], np.int32),
            "float32 array": np.float32([2, 5]), "masked array": np.ma.masked_array([2, 5], [0, 1]),
            "masked float32": np.ma.masked_array(np.float32([2, 5]), [0, 1]),
            "list of ints": [2, 5], "uint8 array": np.uint8([2, 5])}
    norms = {"Normalize(0, 10)": lambda: colors.Normalize(0, 10),
             "Normalize(0.0, 10.0)": lambda: colors.Normalize(0.0, 10.0),
             "Normalize()": colors.Normalize,
             "LogNorm(1, 100)": lambda: colors.LogNorm(1, 100),
             "SymLogNorm(1)": lambda: colors.SymLogNorm(1, vmin=-10, vmax=10),
             "PowerNorm(2, 0, 10)": lambda: colors.PowerNorm(2, 0, 10),
             "CenteredNorm()": colors.CenteredNorm,
             "BoundaryNorm": lambda: colors.BoundaryNorm([0, 3, 6, 10], 3),
             "NoNorm": colors.NoNorm}
    for label, make in norms.items():
        for kind, value in data.items():
            rows.row(f"R {label}", f"call({kind})", lambda m=make, v=value: m()(v))
        rows.row(f"R {label}", "inverse(0.5)", lambda m=make: (lambda n: (n(5), n.inverse(0.5))[1])(m()))
        rows.row(f"R {label}", "inverse(float array)",
                 lambda m=make: (lambda n: (n(5), n.inverse(np.array([0.25, 0.5])))[1])(m()))
        rows.row(f"R {label}", "vmin, vmax after call(ints)",
                 lambda m=make: (lambda n: (n([2, 5]), (n.vmin, n.vmax))[1])(m()))
    cmap = plt.get_cmap("viridis")
    for kind, value in [("float", 0.5), ("int", 3), ("np.int64", np.int64(3)),
                        ("np.float32", np.float32(0.5)), ("float array", np.array([0.25, 0.5])),
                        ("int array", np.array([1, 3])), ("float32 array", np.float32([0.25])),
                        ("masked array", np.ma.masked_array([0.2, 0.4], [0, 1])),
                        ("list", [0.1, 0.9]), ("2-D", np.zeros((2, 2)))]:
        rows.row("R Colormap", f"cmap({kind})", lambda v=value: cmap(v))
        rows.row("R Colormap", f"cmap({kind}, alpha=1)", lambda v=value: cmap(v, alpha=1))
        rows.row("R Colormap", f"cmap({kind}, bytes=True)", lambda v=value: cmap(v, bytes=True))
    listed = colors.ListedColormap([[1, 0, 0], [0, 1, 0]])
    rows.row("R ListedColormap(ints)", "colors", lambda: listed.colors)
    rows.row("R ListedColormap(ints)", "__call__(0)", lambda: listed(0))
    rows.row("R ListedColormap(ints)", "N", lambda: listed.N)
    rows.row("R LinearSegmentedColormap.from_list", "__call__(0.5)",
             lambda: colors.LinearSegmentedColormap.from_list("x", ["r", "b"])(0.5))
    rows.row("R Colormap", "get_bad()", cmap.get_bad)
    rows.row("R Colormap", "resampled(4)(1)", lambda: cmap.resampled(4)(1))
    rows.row("R ScalarMappable ints", "to_rgba(int array)",
             lambda: plt.cm.ScalarMappable(colors.Normalize(0, 4)).to_rgba(np.array([1, 2])))
    rows.row("R ScalarMappable ints", "get_clim() after set_array(ints) autoscale",
             lambda: (lambda m: (m.set_array(np.array([1, 3])), m.autoscale(), m.get_clim())[2])(
                 plt.cm.ScalarMappable()))
    rows.row("R LightSource", "shade(ints) dtype",
             lambda: colors.LightSource().shade(np.arange(9).reshape(3, 3), cmap))


def dates_(rows):
    """dates functions given every kind of date."""
    rows.row("R dates", "date2num(datetime)", lambda: dates.date2num(DAY))
    rows.row("R dates", "date2num(date)", lambda: dates.date2num(DAY.date()))
    rows.row("R dates", "date2num([datetime])", lambda: dates.date2num([DAY]))
    rows.row("R dates", "date2num(np.datetime64)", lambda: dates.date2num(np.datetime64("2020-01-02")))
    rows.row("R dates", "date2num(datetime64 array)",
             lambda: dates.date2num(np.array(["2020-01-02"], "datetime64[D]")))
    rows.row("R dates", "num2date(int)", lambda: dates.num2date(18000).day)
    rows.row("R dates", "num2date([ints]) length", lambda: len(dates.num2date([18000, 18001])))
    rows.row("R dates", "drange", lambda: dates.drange(DAY, DAY + datetime.timedelta(days=2),
                                                       datetime.timedelta(days=1)))
    rows.row("R dates", "datestr2num", lambda: dates.datestr2num("2020-01-02"))
    rows.row("R dates", "num2timedelta(1).days", lambda: dates.num2timedelta(1).days)
    rows.row("R dates", "get_epoch()", dates.get_epoch)
    locator = dates.AutoDateLocator()
    rows.row("R AutoDateLocator", "tick_values", lambda: locator.tick_values(DAY, DAY + datetime.timedelta(days=30)))
    rows.row("R AutoDateLocator", "nonsingular(ints)", lambda: locator.nonsingular(18000, 18000))
    rows.row("R DayLocator", "tick_values", lambda: dates.DayLocator().tick_values(DAY, DAY + datetime.timedelta(days=3)))
    fig, ax = plt.subplots()
    ax.plot([DAY, DAY + datetime.timedelta(days=3)], [1, 2])
    rows.row("R Axes dates", "get_xlim()", ax.get_xlim)
    fig.canvas.draw()
    rows.row("R Axes dates", "get_xticks()", ax.get_xticks)
    rows.row("R Axes dates", "line get_xdata(orig=False)", lambda: ax.lines[0].get_xdata(orig=False))


def helpers(rows):
    """cbook and mlab functions given ints."""
    stats = cbook.boxplot_stats([1, 2, 3, 4, 10])[0]
    for key in ("med", "q1", "q3", "whislo", "whishi", "mean", "fliers", "iqr", "cilo"):
        rows.row("R cbook.boxplot_stats ints", key, lambda k=key: stats[k])
    rows.row("R cbook.boxplot_stats float32", "med, fliers",
             lambda: (lambda s: (s["med"], s["fliers"]))(cbook.boxplot_stats(np.float32([1, 2, 3, 4, 10]))[0]))
    vstats = cbook.violin_stats([[1, 2, 3, 4]], lambda d, coords: np.ones_like(coords))[0]
    for key in ("coords", "vals", "mean", "median", "min", "max"):
        rows.row("R cbook.violin_stats ints", key, lambda k=key: vstats[k])
    rows.row("R cbook", "simple_linear_interpolation(ints, 2)",
             lambda: cbook.simple_linear_interpolation(np.array([[0], [2]]), 2))
    rows.row("R cbook", "index_of(ints)", lambda: cbook.index_of([3, 4]))
    rows.row("R cbook", "safe_masked_invalid(ints)", lambda: cbook.safe_masked_invalid(np.array([1, 2])))
    rows.row("R cbook", "contiguous_regions", lambda: cbook.contiguous_regions([0, 1, 1, 0]))
    rows.row("R cbook", "delete_masked_points(ints)",
             lambda: cbook.delete_masked_points(np.array([1, 2]), np.array([3, 4])))
    rows.row("R cbook", "_reshape_2D(ints)", lambda: cbook._reshape_2D([[1, 2], [3, 4]], "x"))
    rows.row("R cbook", "pts_to_prestep(ints)", lambda: cbook.pts_to_prestep([1, 2], [3, 4]))
    rows.row("R cbook", "pts_to_midstep(ints)", lambda: cbook.pts_to_midstep([1, 2], [3, 4]))
    rows.row("R cbook", "_unpack_to_numpy(ints)", lambda: cbook._unpack_to_numpy([1, 2]))
    rows.row("R cbook", "_safe_first_finite(ints)", lambda: cbook._safe_first_finite([1, 2]))
    rows.row("R cbook", "ls_mapper", lambda: cbook.ls_mapper["-"])
    signal = np.arange(16) % 5
    rows.row("R mlab", "detrend_mean(ints)", lambda: mlab.detrend_mean(signal))
    rows.row("R mlab", "detrend_linear(ints)", lambda: mlab.detrend_linear(signal))
    rows.row("R mlab", "detrend_none(ints)", lambda: mlab.detrend_none(signal))
    rows.row("R mlab", "window_hanning(ints)", lambda: mlab.window_hanning(signal))
    rows.row("R mlab", "window_none(ints)", lambda: mlab.window_none(signal))
    rows.row("R mlab", "psd(ints)", lambda: mlab.psd(signal, NFFT=8))
    rows.row("R mlab", "csd(ints)", lambda: mlab.csd(signal, signal, NFFT=8))
    rows.row("R mlab", "specgram(ints)", lambda: mlab.specgram(signal, NFFT=8, noverlap=4))
    rows.row("R mlab", "magnitude_spectrum(ints)", lambda: mlab.magnitude_spectrum(signal))
    rows.row("R mlab", "cohere(ints)", lambda: mlab.cohere(signal, signal[::-1], NFFT=8))
    rows.row("R mlab", "GaussianKDE(ints)(ints)", lambda: mlab.GaussianKDE([1, 2, 3, 4])([1, 2]))
    rows.row("R mlab", "GaussianKDE(ints).covariance_factor()",
             lambda: mlab.GaussianKDE([1, 2, 3, 4]).covariance_factor())


def curves(rows):
    """Paths and Bézier functions given ints."""
    square = Path([[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]], closed=True)
    rows.row("R Path(ints)", "vertices", lambda: square.vertices)
    rows.row("R Path(ints)", "codes", lambda: square.codes)
    rows.row("R Path(ints)", "get_extents()", square.get_extents)
    rows.row("R Path(ints)", "contains_point((0.5, 0.5))", lambda: square.contains_point((0.5, 0.5)))
    rows.row("R Path(ints)", "contains_points(ints)", lambda: square.contains_points([[0, 0], [2, 2]]))
    rows.row("R Path(ints)", "contains_path", lambda: square.contains_path(Path([[0.2, 0.2], [0.4, 0.4]])))
    rows.row("R Path(ints)", "interpolated(2).vertices", lambda: square.interpolated(2).vertices)
    rows.row("R Path(ints)", "to_polygons()", square.to_polygons)
    rows.row("R Path(ints)", "cleaned().vertices", lambda: square.cleaned().vertices)
    rows.row("R Path(ints)", "transformed(scale 2).vertices",
             lambda: square.transformed(transforms.Affine2D().scale(2)).vertices)
    rows.row("R Path(ints)", "intersects_bbox", lambda: square.intersects_bbox(transforms.Bbox.unit()))
    rows.row("R Path(ints)", "intersects_path", lambda: square.intersects_path(Path([[0, 0], [2, 2]])))
    rows.row("R Path(ints)", "iter_segments", lambda: next(square.iter_segments())[0])
    rows.row("R Path(ints)", "iter_bezier", lambda: next(square.iter_bezier())[0].control_points)
    rows.row("R Path(ints)", "get_extents() of a curve",
             lambda: Path([[0, 0], [1, 2], [2, 0]], [1, 3, 3]).get_extents())
    rows.row("R Path(ints)", "copy().vertices", lambda: square.copy().vertices)
    rows.row("R Path(ints)", "__deepcopy__().vertices", lambda: square.deepcopy().vertices)
    rows.row("R Path(ints)", "make_compound_path", lambda: Path.make_compound_path(square, square).vertices)
    rows.row("R Path", "arc(0, 90).vertices", lambda: Path.arc(0, 90).vertices)
    rows.row("R Path", "circle((1, 1), 2).vertices", lambda: Path.circle((1, 1), 2).vertices)
    rows.row("R Path", "unit_regular_polygon(4).vertices", lambda: Path.unit_regular_polygon(4).vertices)
    rows.row("R path", "get_path_collection_extents",
             lambda: __import__("pltrs.path", fromlist=["x"]).get_path_collection_extents(
                 transforms.IdentityTransform(), [square], [], [[1, 2]], transforms.IdentityTransform()))
    rows.row("R bezier", "split_bezier_intersecting_with_closedpath(int points)",
             lambda: bezier.split_bezier_intersecting_with_closedpath(
                 [(0, 0), (1, 2), (2, 0)], lambda xy: xy[0] < 1)[0])
    rows.row("R bezier", "BezierSegment(ints).control_points",
             lambda: bezier.BezierSegment(np.array([[0, 0], [1, 2]])).control_points)
    segment = bezier.BezierSegment(np.array([[0, 0], [1, 2], [2, 0]]))
    rows.row("R BezierSegment(ints)", "point_at_t(0.5)", lambda: segment.point_at_t(0.5))
    rows.row("R BezierSegment(ints)", "__call__([0, 1])", lambda: segment([0, 1]))
    rows.row("R BezierSegment(ints)", "__call__(0)", lambda: segment(0))
    rows.row("R BezierSegment(ints)", "dimension, degree", lambda: (segment.dimension, segment.degree))
    rows.row("R BezierSegment(ints)", "polynomial_coefficients", lambda: segment.polynomial_coefficients)
    rows.row("R BezierSegment(ints)", "axis_aligned_extrema()", segment.axis_aligned_extrema)
    rows.row("R bezier", "split_de_casteljau(ints)",
             lambda: bezier.split_de_casteljau([(0, 0), (1, 2), (2, 0)], 0.5))
    rows.row("R bezier", "get_intersection(ints)", lambda: bezier.get_intersection(0, 0, 1, 1, 0, 1, 1, 0))
    rows.row("R bezier", "get_normal_points(ints)", lambda: bezier.get_normal_points(0, 0, 1, 0, 1))
    rows.row("R bezier", "get_cos_sin(ints)", lambda: bezier.get_cos_sin(0, 0, 1, 1))
    rows.row("R bezier", "check_if_parallel(ints)", lambda: bezier.check_if_parallel(1, 0, 2, 0))
    rows.row("R bezier", "get_parallels(ints)",
             lambda: bezier.get_parallels([(0, 0), (1, 1), (2, 0)], 1))
    rows.row("R bezier", "find_control_points(ints)", lambda: bezier.find_control_points(0, 0, 1, 1, 2, 0))
    rows.row("R bezier", "make_wedged_bezier2(ints)",
             lambda: bezier.make_wedged_bezier2([(0, 0), (1, 1), (2, 0)], 1))
    rows.row("R bezier", "inside_circle(0, 0, 1)((0, 0))", lambda: bezier.inside_circle(0, 0, 1)((0, 0)))
    rows.row("R bezier", "find_bezier_t_intersecting_with_closedpath",
             lambda: bezier.find_bezier_t_intersecting_with_closedpath(
                 segment.point_at_t, lambda xy: xy[0] < 1))
    rows.row("R bezier", "split_path_inout(int path)",
             lambda: [p.vertices for p in bezier.split_path_inout(
                 Path([(0, 0), (1, 1), (2, 0)], [1, 3, 3]), lambda xy: xy[0] < 1)])


def extents(rows):
    """Text extents from each renderer and from mathtext."""
    from pltrs.backends.backend_agg import RendererAgg
    from pltrs.backends.backend_pdf import PdfFile, RendererPdf
    from pltrs.backends.backend_ps import RendererPS
    from pltrs.backends.backend_svg import RendererSVG
    from pltrs.mathtext import MathTextParser
    from pltrs.textpath import TextPath, TextToPath

    prop = FontProperties(size=12)
    renderers = {"RendererAgg": RendererAgg(400, 300, 72),
                 "RendererPdf": RendererPdf(PdfFile(io.BytesIO()), 72, 300, 400),
                 "RendererPS": RendererPS(300, 400, io.StringIO()),
                 "RendererSVG": RendererSVG(300, 400, io.StringIO())}
    for name, renderer in renderers.items():
        rows.row(f"R {name}", "get_text_width_height_descent(mathtext)",
                 lambda r=renderer: r.get_text_width_height_descent(r"$x^2$", prop, ismath=True))
        rows.row(f"R {name}", "get_text_width_height_descent(text)",
                 lambda r=renderer: r.get_text_width_height_descent("Hello", prop, ismath=False))
        rows.row(f"R {name}", "get_canvas_width_height()", renderer.get_canvas_width_height)
        rows.row(f"R {name}", "points_to_pixels(10)", lambda r=renderer: r.points_to_pixels(10))
    rows.row("R MathTextParser('path')", "parse()", lambda: MathTextParser("path").parse(r"$x^2$", 72, prop)[:3])
    rows.row("R MathTextParser('agg')", "parse() width, height, depth",
             lambda: MathTextParser("agg").parse(r"$x^2$", 72, prop)[:3])
    rows.row("R TextToPath", "get_text_width_height_descent",
             lambda: TextToPath().get_text_width_height_descent("Hello", prop, ismath=False))
    rows.row("R TextToPath", "get_text_width_height_descent(mathtext)",
             lambda: TextToPath().get_text_width_height_descent(r"$x^2$", prop, ismath=True))
    rows.row("R TextPath(ints)", "vertices", lambda: TextPath((1, 2), "A", size=10).vertices)
    rows.row("R TextPath(ints)", "get_size()", lambda: TextPath((1, 2), "A", size=10).get_size())
    fig = plt.figure(figsize=(3, 2), dpi=72)
    text = fig.text(0.1, 0.5, "Ag\nxy")
    renderer = fig.canvas.get_renderer()
    rows.row("R Text._get_layout", "corner of the layout", lambda: text._get_layout(renderer)[2][0])
    rows.row("R Text._get_layout", "box", lambda: text._get_layout(renderer)[0])
    rows.row("R Text._get_layout", "lines' (w, h, d)", lambda: text._get_layout(renderer)[1][0])
    rows.row("R Text", "get_window_extent().bounds", lambda: text.get_window_extent(renderer).bounds)
    rows.row("R Text math", "get_window_extent().bounds",
             lambda: fig.text(0.1, 0.1, r"$x^2$").get_window_extent(renderer).bounds)


SCENES = (limits, boxes, tickers, colours, dates_, helpers, curves, extents)
