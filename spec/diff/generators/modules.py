"""matplotlib's modules beyond the artists: formatters, locators, scales,
norms, colormaps, colours, transforms, paths, text, legends, colorbars,
layout, 3-D axes and the toolkits; then the state drawing updates (polar
backgrounds, cleared legends) and values read back as given, and the parts
of box plots and the arrays of ``pcolor``.

Each area is one program. Where matplotlib has a family of classes
(formatters, locators, norms, colormaps, scales), the program is made from
the family's members as matplotlib lists them, with constructor arguments
from a table.
"""

from __future__ import annotations

from typing import Iterator

from . import common

AXIS_SETUP = ["fig, ax = plt.subplots()", "ax.plot([0.001, 0.5, 2.5, 1000])", "fig.canvas.draw()"]

#: class -> constructor arguments, one program variant each
FORMATTERS: dict[str, list[str]] = {
    "NullFormatter": [""],
    "FixedFormatter": ["['a', 'b', 'c']"],
    "FuncFormatter": ["lambda v, pos: f'<{v:.1f}|{pos}>'"],
    "FormatStrFormatter": ["'%.2f'", "'%d units'"],
    "StrMethodFormatter": ["'{x:.3g}'", "'{x} at {pos}'"],
    "ScalarFormatter": ["", "useOffset=False", "useMathText=True"],
    "LogFormatter": ["", "base=2", "labelOnlyBase=True"],
    "LogFormatterExponent": [""],
    "LogFormatterMathtext": [""],
    "LogFormatterSciNotation": [""],
    "LogitFormatter": ["", "use_overline=True"],
    "EngFormatter": ["", "unit='Hz', places=1", "sep=''"],
    "PercentFormatter": ["", "xmax=1, decimals=1", "symbol=None"],
}

LOCATORS: dict[str, list[str]] = {
    "IndexLocator": ["2, 0.5"],
    "FixedLocator": ["[0, 1, 5, 10]", "np.arange(20), nbins=4"],
    "NullLocator": [""],
    "LinearLocator": ["", "numticks=4"],
    "LogLocator": ["", "base=2, subs=(1.0, 1.5)", "subs='auto', numticks=5"],
    "AutoLocator": [""],
    "MultipleLocator": ["0.25", "base=2, offset=0.5"],
    "MaxNLocator": ["", "nbins=3", "nbins=4, integer=True, steps=[1, 5]", "prune='both'", "symmetric=True"],
    "AutoMinorLocator": ["", "4"],
    "SymmetricalLogLocator": ["linthresh=1, base=10"],
    "AsinhLocator": ["linear_width=1"],
    "LogitLocator": ["", "minor=True"],
}

#: (vmin, vmax) ranges the locators place ticks in
RANGES = ["0, 1", "-3.7, 12.2", "1e-3, 1e5", "0.01, 0.99", "-1000, 1000", "5, 5", "1e12, 1e12 + 3", "-0.0001, 0.0001"]
#: values the formatters format
TICKS = "[0, 0.5, 1, 2.5, 10, 100, 1e5, -3, 1e-4, 0.999]"


def _ticker() -> Iterator[str]:
    yield from AXIS_SETUP
    for cls, variants in FORMATTERS.items():
        for arguments in variants:
            yield f"f = ticker.{cls}({arguments})"
            yield "f.set_axis(ax.xaxis)"
            yield f"f.format_ticks({TICKS})"
            yield "f.get_offset()"
            yield "[f(v) for v in [0, 0.25, 1, 3.14159, 1e6, -2, 1e-7]]"
            yield "[f(v, i) for i, v in enumerate([0, 0.25, 1, 3.14159])]"
            yield "f.format_data(1234.5678)"
            yield "f.format_data_short(1234.5678)"
            yield "f.locs"
    for cls, variants in LOCATORS.items():
        for arguments in variants:
            yield f"loc = ticker.{cls}({arguments})"
            for limits in RANGES:
                yield f"loc.tick_values({limits})"
            yield "loc.set_axis(ax.xaxis)"
            yield "loc()"
            yield "loc.view_limits(0.3, 7.7)"
            yield "loc.nonsingular(1, 1)"
            yield "loc.nonsingular(-np.inf, np.inf)"
    yield "ax.xaxis.get_major_formatter()"
    yield "ax.xaxis.get_major_locator()"
    yield "ax.xaxis.get_minor_locator()"
    yield "ax.xaxis.get_major_formatter().get_useOffset()"
    yield "ax.xaxis.get_major_formatter().get_useMathText()"
    yield "ax.xaxis.get_major_formatter().orderOfMagnitude"
    yield "ax.xaxis.get_major_formatter().offset"
    yield "ax.xaxis.get_major_locator()._nbins"
    yield "ticker.scale_range(0, 100)"
    yield "ticker.MaxNLocator.default_params"


SCALES = ["linear", "log", "symlog", "logit", "asinh", "function", "functionlog"]


def _scales() -> Iterator[str]:
    yield "scale.get_scale_names()"
    for name in SCALES:
        extra = ", functions=(np.sqrt, np.square)" if name.startswith("function") else ""
        data = "[0.01, 0.5, 0.99]" if name == "logit" else "[0.001, 1, 50, 1000]"
        yield "fig, ax = plt.subplots()"
        yield f"ax.plot({data}, {data})"
        yield f"ax.set_xscale('{name}'{extra})"
        yield "ax.get_xscale()"
        yield "s = ax.xaxis._scale"
        yield "type(s).__name__"
        yield "t = ax.xaxis.get_transform()"
        yield "type(t).__name__"
        yield "t.transform(np.array([0.01, 0.5, 0.9]))"
        yield "t.inverted().transform(np.array([-1.0, 0.0, 0.5]))"
        yield "type(t.inverted()).__name__"
        yield "s.limit_range_for_scale(-1, 2, 1e-300)"
        yield "ax.xaxis.get_major_locator()"
        yield "ax.xaxis.get_minor_locator()"
        yield "ax.xaxis.get_major_formatter()"
        yield "ax.xaxis.get_minor_formatter()"
        yield "fig.canvas.draw()"
        yield "ax.get_xlim()"
        yield "ax.get_xticks()"
        yield "[t.get_text() for t in ax.get_xticklabels()]"
        yield "ax.get_xticks(minor=True)"


NORMS = {
    "Normalize": ["", "vmin=0, vmax=10", "0, 1, clip=True"],
    "LogNorm": ["", "1, 1000"],
    "SymLogNorm": ["linthresh=1", "1, linscale=2, vmin=-100, vmax=100, base=10"],
    "PowerNorm": ["gamma=0.5", "2, vmin=0, vmax=5"],
    "BoundaryNorm": ["[0, 1, 3, 9], 5", "[0, 1, 3, 9], 2, extend='both'"],
    "TwoSlopeNorm": ["vcenter=2", "0, -1, 10"],
    "CenteredNorm": ["", "vcenter=1, halfrange=3"],
    "NoNorm": [""],
    "AsinhNorm": ["", "linear_width=0.5, vmin=-5, vmax=5"],
    "FuncNorm": ["(np.sqrt, np.square), vmin=0, vmax=9"],
}


def _norms() -> Iterator[str]:
    data = "np.array([0.5, 1.0, 2.0, 4.0, 8.0, np.nan, -1.0])"
    for cls, variants in NORMS.items():
        for arguments in variants:
            yield f"n = colors.{cls}({arguments})"
            yield "n.vmin, n.vmax, n.clip"
            yield "n.scaled()"
            yield f"n({data})"
            yield "n(3.0)"
            yield "n(np.ma.masked_array([1.0, 2.0, 3.0], [0, 1, 0]))"
            yield "n.vmin, n.vmax"
            yield "n.inverse(np.array([0.0, 0.25, 1.0]))"
            yield "n.autoscale([1, 5, 10])"
            yield "n.vmin, n.vmax"
            yield "n.autoscale_None([0.1, 20])"
            yield "n.vmin, n.vmax"
    yield "colors.Normalize(1, 2) == colors.Normalize(1, 2)"
    yield "colors.make_norm_from_scale"


def _colormaps() -> Iterator[str]:
    import matplotlib

    data = "np.array([0.0, 0.1, 0.5, 0.99, 1.0, -1.0, 2.0, np.nan])"
    yield "sorted(matplotlib.colormaps)"
    for name in sorted(matplotlib.colormaps):
        yield f"cm = matplotlib.colormaps['{name}']"
        yield "type(cm).__name__, cm.name, cm.N"
        yield f"cm({data})"
        yield f"cm({data}, bytes=True)"
        yield f"cm({data}, alpha=0.5)"
        yield "cm(np.arange(0, 300, 37))"
        yield "cm.get_bad(), cm.get_under(), cm.get_over()"
        yield "cm.reversed().name"
        yield "cm.is_gray()"
        yield "cm.colorbar_extend"
    yield "cm = matplotlib.colormaps['viridis'].with_extremes(bad='r', under='g', over='b')"
    yield "cm(np.array([-1.0, 2.0, np.nan]))"
    yield "cm.resampled(4)(np.linspace(0, 1, 4))"
    yield "colors.ListedColormap(['r', 'g', 'b'])(np.array([0.0, 0.5, 1.0]))"
    yield "colors.ListedColormap(['r', 'g', 'b']).colors"
    yield "colors.LinearSegmentedColormap.from_list('mine', ['r', 'w', 'b'], N=5)(np.linspace(0, 1, 5))"
    yield "colors.LinearSegmentedColormap.from_list('mine', [(0, 'r'), (0.3, 'w'), (1, 'b')])._segmentdata"
    yield "sorted(matplotlib.color_sequences)"
    yield "matplotlib.color_sequences['tab10'][:3]"
    yield "matplotlib.colormaps.get_cmap('jet').name"
    yield "matplotlib.colormaps.get_cmap(None).name"
    yield "matplotlib.cm.ScalarMappable().get_cmap().name"
    yield "colors.Colormap('x').N"


COLOR_SPECS = ["'r'", "'red'", "'xkcd:sky blue'", "'tab:orange'", "'C3'", "'#abc'", "'#abcd'", "'#a1b2c3'",
               "'#a1b2c380'", "'0.5'", "(0.1, 0.2, 0.3)", "(0.1, 0.2, 0.3, 0.4)", "('r', 0.5)", "'none'",
               "'Red'", "'C10'", "[1, 0, 0]", "np.array([0.0, 1.0, 0.0])", "'#xyz'", "1.5", "'grey'", "'k'"]


def _colors() -> Iterator[str]:
    for spec in COLOR_SPECS:
        yield f"colors.to_rgba({spec})"
        yield f"colors.to_hex({spec})"
        yield f"colors.to_rgb({spec})"
        yield f"colors.is_color_like({spec})"
        yield f"colors.to_rgba({spec}, alpha=0.25)"
    yield "colors.to_hex((0.1, 0.2, 0.3, 0.4), keep_alpha=True)"
    yield "colors.to_rgba_array(['r', 'g', (0, 0, 1, 0.5)])"
    yield "colors.to_rgba_array('none')"
    yield "colors.to_rgba_array([])"
    yield "colors.to_rgba_array(np.zeros((2, 3)))"
    yield "colors.to_rgba_array(['r', 'b'], alpha=[0.2, 0.4])"
    yield "colors.same_color('r', (1, 0, 0))"
    yield "colors.same_color(['r', 'g'], ['red', 'green'])"
    yield "colors.rgb_to_hsv([0.2, 0.4, 0.6])"
    yield "colors.hsv_to_rgb(np.array([[0.5, 0.5, 0.5], [0.1, 1, 1]]))"
    yield "ls = colors.LightSource(azdeg=315, altdeg=45)"
    yield "ls.hillshade(z)"
    yield "ls.shade(z, matplotlib.colormaps['gray']).shape"
    yield "ls.direction"
    yield "len(colors.CSS4_COLORS), len(colors.XKCD_COLORS), len(colors.TABLEAU_COLORS), len(colors.BASE_COLORS)"
    yield "colors.CSS4_COLORS['rebeccapurple']"
    yield "colors.get_named_colors_mapping()['C0'] if 'C0' in colors.get_named_colors_mapping() else None"


def _transforms() -> Iterator[str]:
    yield "fig, ax = plt.subplots()"
    yield "ax.plot([0, 10], [0, 5])"
    yield "a = transforms.Affine2D().rotate_deg(30).scale(2, 3).translate(1, 2)"
    yield "a.get_matrix()"
    yield "a.transform([[0, 0], [1, 1]])"
    yield "a.inverted().get_matrix()"
    yield "type(a.inverted()).__name__"
    yield "b = a + transforms.Affine2D().skew_deg(10, 0)"
    yield "type(b).__name__"
    yield "b.get_matrix()"
    yield "type(a + ax.transData).__name__"
    yield "type(ax.transData).__name__"
    yield "type(ax.transAxes).__name__"
    yield "type(ax.transScale).__name__"
    yield "type(ax.transLimits).__name__"
    yield "type(fig.transFigure).__name__"
    yield "type(fig.dpi_scale_trans).__name__"
    yield "type(ax.get_xaxis_transform()).__name__"
    yield "type(ax.get_yaxis_transform(which='tick2')).__name__"
    yield "type(ax.transData.inverted()).__name__"
    yield "ax.transData.is_affine, ax.transData.is_separable, ax.transData.has_inverse"
    yield "ax.transData.input_dims, ax.transData.output_dims"
    yield "ax.transData.transform([[0, 0], [10, 5]])"
    yield "ax.transData.inverted().transform([[80, 58], [576, 422]])"
    yield "ax.transAxes.transform((0.5, 0.5))"
    yield "ax.transData.get_affine().get_matrix()"
    yield "ax.transData.depth"
    yield "ax.transData.contains_branch(ax.transAxes)"
    yield "ax.transData.contains_branch(fig.transFigure)"
    yield "(ax.transData - ax.transAxes).get_matrix()"
    yield "type(ax.transData - ax.transAxes).__name__"
    yield "ax.transData == ax.transData"
    yield "ax.transData is ax.transData"
    yield "ax.transAxes is ax.transAxes"
    yield "ax.bbox is ax.bbox"
    yield "fig.bbox is fig.bbox"
    yield "type(ax.bbox).__name__"
    yield "type(fig.bbox).__name__"
    yield "type(ax.viewLim).__name__, type(ax.dataLim).__name__"
    yield "ax.viewLim.bounds"
    yield "ax.bbox.bounds"
    yield "fig.bbox.bounds"
    yield "fig.bbox_inches.bounds"
    yield "bl = transforms.blended_transform_factory(ax.transData, ax.transAxes)"
    yield "type(bl).__name__"
    yield "bl.transform([[1, 0.5]])"
    yield "type(transforms.blended_transform_factory(transforms.Affine2D(), transforms.Affine2D())).__name__"
    yield "type(transforms.composite_transform_factory(transforms.IdentityTransform(), ax.transAxes)).__name__"
    yield "o = transforms.offset_copy(ax.transData, fig, x=5, y=5, units='points')"
    yield "type(o).__name__"
    yield "o.transform([[0, 0]])"
    yield "st = transforms.ScaledTranslation(1, 2, fig.dpi_scale_trans)"
    yield "st.get_matrix()"
    yield "bb = transforms.Bbox([[0, 0], [1, 2]])"
    yield "bb.bounds, bb.extents, bb.width, bb.height, bb.size, bb.min, bb.max, bb.p0, bb.p1"
    yield "bb.x0, bb.y0, bb.x1, bb.y1, bb.xmin, bb.ymax, bb.intervalx, bb.intervaly"
    yield "transforms.Bbox.union([bb, transforms.Bbox.from_bounds(1, 1, 3, 3)]).bounds"
    yield "transforms.Bbox.intersection(bb, transforms.Bbox.from_bounds(0.5, 0.5, 3, 3)).bounds"
    yield "transforms.Bbox.intersection(bb, transforms.Bbox.from_bounds(5, 5, 1, 1))"
    yield "bb.transformed(a).bounds"
    yield "bb.padded(0.5).bounds"
    yield "bb.shrunk(0.5, 0.25).bounds"
    yield "bb.expanded(2, 3).bounds"
    yield "bb.translated(1, 1).bounds"
    yield "bb.anchored('C', transforms.Bbox.from_bounds(0, 0, 4, 4)).bounds"
    yield "bb.splitx(0.5)"
    yield "bb.contains(0.5, 0.5), bb.overlaps(transforms.Bbox.unit()), bb.fully_contains(1, 2)"
    yield "bb.count_contains([[0.5, 0.5], [3, 3]])"
    yield "transforms.Bbox.null().bounds"
    yield "transforms.Bbox.unit().bounds"
    yield "transforms.TransformedBbox(bb, a).bounds"
    yield "type(transforms.TransformedBbox(bb, a)).__name__"
    yield "transforms.BboxTransform(bb, transforms.Bbox.unit()).get_matrix()"
    yield "transforms.BboxTransformTo(bb).get_matrix()"
    yield "transforms.BboxTransformFrom(bb).get_matrix()"
    yield "transforms.IdentityTransform().get_matrix()"
    yield "transforms.nonsingular(0, 0)"
    yield "transforms.nonsingular(1, 1e-20)"
    yield "transforms.interval_contains((0, 1), 0.5)"
    yield "str(transforms.Affine2D())"
    yield "repr(transforms.Bbox.unit())"
    yield "str(ax.transData).splitlines()[0]"


def _paths() -> Iterator[str]:
    yield "p = path.Path([(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)], closed=True)"
    yield "p.vertices, p.codes"
    yield "p.contains_point((0.5, 0.5))"
    yield "p.contains_points([[0.5, 0.5], [2, 2], [1, 0.5]])"
    yield "p.contains_path(path.Path([(0.2, 0.2), (0.4, 0.4)]))"
    yield "p.get_extents().bounds"
    yield "p.get_extents(transforms.Affine2D().scale(2)).bounds"
    yield "p.intersects_path(path.Path([(-1, 0.5), (2, 0.5)]))"
    yield "p.intersects_bbox(transforms.Bbox.from_bounds(0.5, 0.5, 2, 2))"
    yield "p.interpolated(3).vertices"
    yield "p.to_polygons()"
    yield "p.to_polygons(closed_only=False)"
    yield "p.transformed(transforms.Affine2D().rotate_deg(90)).vertices"
    yield "p.cleaned().vertices, p.cleaned().codes"
    yield "list(p.iter_segments())"
    yield "list(p.iter_bezier())[:2]"
    yield "len(p), p.should_simplify, p.simplify_threshold, p.has_nonfinite, p.readonly"
    yield "path.Path.unit_circle().vertices.shape"
    yield "path.Path.unit_rectangle().vertices"
    yield "path.Path.unit_regular_polygon(5).vertices"
    yield "path.Path.unit_regular_star(5).vertices"
    yield "path.Path.arc(0, 90).vertices"
    yield "path.Path.arc(0, 90, n=2).codes"
    yield "path.Path.wedge(10, 100).codes"
    yield "path.Path.circle((1, 1), 2).vertices[:3]"
    yield "path.Path.make_compound_path(p, path.Path.unit_rectangle()).codes"
    yield "path.Path.hatch('/', 4).vertices.shape"
    yield "path.Path([(0, 0), (np.nan, 1), (2, 2)]).has_nonfinite"
    yield "c = path.Path([(0, 0), (1, 1), (2, 0)], [1, 3, 3])"
    yield "c.get_extents().bounds"
    yield "c.to_polygons()"
    yield "c.interpolated(2).vertices"
    yield "path.get_path_collection_extents(transforms.IdentityTransform(), [p], [transforms.Affine2D()], "\
          "[[0, 0], [5, 5]], transforms.IdentityTransform()).bounds"
    yield "path.Path(np.zeros((0, 2))).get_extents().bounds"
    yield "path.Path([(0, 0)], [path.Path.MOVETO]).codes"
    yield "path.Path.code_type"
    yield "path.Path.NUM_VERTICES_FOR_CODE"
    yield "p == p"
    yield "repr(path.Path([(0, 0), (1, 1)]))"
    yield "p.deepcopy().vertices"


def _text() -> Iterator[str]:
    yield "fig, ax = plt.subplots()"
    for spec in ["'plain'", "'two\\nlines'", "'$\\\\frac{1}{2}$'", "'mixed $x^2$ text'", "''", "' '",
                 "'rotated', rotation=45", "'big', fontsize=30", "'bold', weight='bold'",
                 "'anchor', rotation=30, rotation_mode='anchor'", "'wrap me ' * 20, wrap=True",
                 "'mono', family='monospace'", "'serif', family='serif'", "'centered', ha='center', va='center'",
                 "'boxed', bbox=dict(pad=5)", "'α β γ'", "'with\\ttab'"]:
        yield f"t = ax.text(0.3, 0.4, {spec})"
        yield "t.get_window_extent().bounds"
        yield "t.get_tightbbox().bounds"
        yield "t.get_prop_tup()"
        yield "t.get_fontname()"
        yield "t.get_fontproperties().get_size_in_points()"
        yield "t.get_unitless_position()"
        yield "t.get_transform_rotates_text()"
    yield "fig.canvas.draw()"
    yield "r = fig.canvas.get_renderer()"
    yield "r.get_text_width_height_descent('Hello', matplotlib.font_manager.FontProperties(), False)"
    yield "r.get_text_width_height_descent('$x^2$', matplotlib.font_manager.FontProperties(), True)"
    yield "r.points_to_pixels(10)"
    yield "r.get_canvas_width_height()"
    yield "from matplotlib.textpath import TextPath"
    yield "tp = TextPath((0, 0), 'Ag', size=10)"
    yield "tp.vertices.shape, tp.get_extents().bounds"
    yield "TextPath((0, 0), '$x$', size=12).get_extents().bounds"
    yield "from matplotlib.font_manager import FontProperties, findfont, fontManager"
    yield "fp = FontProperties(family='serif', size=12, weight='bold')"
    yield "fp.get_family(), fp.get_size(), fp.get_weight(), fp.get_style(), fp.get_stretch(), fp.get_variant()"
    yield "fp.get_size_in_points(), fp.get_fontconfig_pattern()"
    yield "str(fp)"
    yield "import os"
    yield "os.path.basename(findfont(FontProperties(family='DejaVu Sans')))"
    yield "os.path.basename(findfont('monospace'))"
    yield "from matplotlib import mathtext"
    yield "mathtext.MathTextParser('path').parse('$x^2$').width"
    yield "matplotlib.text.Text(0, 0, 'loose').get_text()"
    yield "matplotlib.text.Text(0, 0, 'loose').get_window_extent(fig.canvas.get_renderer()).bounds"


def _legends() -> Iterator[str]:
    for args in ["", "loc='upper left'", "loc=3", "ncols=2, title='t', frameon=False",
                 "bbox_to_anchor=(1, 1), loc='upper left'", "fontsize='small', markerscale=2",
                 "handles=[lines.Line2D([], [], color='r', label='proxy')]", "labelcolor='linecolor'",
                 "reverse=True, shadow=True, fancybox=False"]:
        yield "fig, ax = plt.subplots()"
        yield "ax.plot([1, 2], label='line')"
        yield "ax.scatter([1, 2], [2, 1], label='points')"
        yield "ax.bar([1], [1], label='bar')"
        yield "ax.fill_between([0, 1], [0, 1], label='fill')"
        yield "ax.errorbar([1], [1], yerr=0.1, label='err')"
        yield f"leg = ax.legend({args})"
        yield "[t.get_text() for t in leg.get_texts()]"
        yield "leg.legend_handles"
        yield "leg.get_lines()"
        yield "leg.get_patches()"
        yield "leg.get_title().get_text()"
        yield "leg.get_frame()"
        yield "leg.get_frame_on()"
        yield "leg._loc"
        yield "leg.get_children()"
        yield "leg.get_bbox_to_anchor().bounds"
        yield "leg.numpoints, leg.markerscale, leg.scatterpoints, leg.borderpad, leg.labelspacing"
        yield "leg.handlelength, leg.handletextpad, leg.columnspacing, leg.shadow, leg.isaxes, leg.prop.get_size()"
        yield "fig.canvas.draw()"
        yield "leg.get_window_extent().bounds"
        yield "leg.get_tightbbox().bounds"
        yield "[h.get_window_extent().bounds for h in leg.get_texts()]"
    yield "ax.get_legend_handles_labels()"
    yield "matplotlib.legend.Legend.codes"
    yield "fig.legends"


def _drawn_state() -> Iterator[str]:
    """State matplotlib updates as it draws: the background of polar Axes
    (fitted to the view), a cleared Axes' legend, and values read back as
    they were given (alphas, label pads)."""
    yield "fig = plt.figure()"
    for index, limits in enumerate(["", "ax.set_thetamin(10); ax.set_thetamax(200)",
                                    "ax.set_rorigin(-1); ax.set_rlim(1, 3)",
                                    "ax.set_theta_direction(-1); ax.set_theta_offset(1)"]):
        yield f"ax = fig.add_subplot(2, 2, {index + 1}, projection='polar')"
        yield "ax.plot(x, values)"
        if limits:
            yield limits
        yield "patch = ax.patch"
        yield "patch.center, patch.r, patch.theta1, patch.theta2, patch.width"
        yield "len(patch.get_path().vertices)"
        yield "fig.canvas.draw()"
        yield "patch.center, patch.r, patch.theta1, patch.theta2, patch.width"
        yield "patch.get_path().vertices[:4], len(patch.get_path().vertices)"
        yield "ax.patch is patch, patch.get_window_extent().bounds"
        yield "ax.set_rmax(20)"
        yield "patch.width"
        yield "fig.canvas.draw()"
        yield "patch.r, patch.width"
    yield "fig, ax = plt.subplots()"
    yield "ax.plot([1, 2], label='line')"
    yield "leg = ax.legend()"
    yield "ax.clear()"
    yield "leg.axes is ax, ax.get_legend(), leg.figure is fig, leg.get_texts()[0].get_text()"
    yield "ax.plot([2, 1], label='again')"
    yield "other = ax.legend()"
    yield "other.remove()"
    yield "other.axes, other.figure, leg.axes is ax"
    yield "patch = ax.add_patch(patches.Rectangle((0, 0), 1, 1))"
    yield "patch.set_alpha(1)"
    yield "patch.get_alpha(), patch.get_facecolor(), patch.get_edgecolor()"
    yield "patch.set_alpha(0.25)"
    yield "patch.get_alpha(), patch.get_facecolor()"
    yield "line, = ax.plot([0, 1])"
    yield "line.set_alpha(1)"
    yield "line.get_alpha()"
    yield "ax.set_xlabel('x', labelpad=12)"
    yield "ax.xaxis.labelpad"
    yield "ax.yaxis.labelpad = 3"
    yield "ax.yaxis.labelpad"
    yield "fig.canvas.draw()"
    yield "ax.xaxis.labelpad, ax.yaxis.labelpad"


def _box_parts() -> Iterator[str]:
    """Box-plot parts given the colour None (the next colours of the
    property cycle), filled boxes' faces, and the arrays of ``pcolor``."""
    for props in ["medianprops={'color': None}", "whiskerprops={'color': None, 'linestyle': ':'}",
                  "boxprops={'color': None}, capprops={'color': None}",
                  "flierprops={'color': None}, meanprops={'color': None}, showmeans=True",
                  "boxprops={'color': None}, patch_artist=True",
                  "patch_artist=True, boxprops={'facecolor': 'r'}"]:
        yield "fig, ax = plt.subplots()"
        yield "ax.plot([0, 1])"
        yield f"parts = ax.boxplot([values, values * 2], {props})"
        yield ("{key: [a.get_facecolor() if isinstance(a, patches.Patch) else a.get_color() for a in arts] "
               "for key, arts in parts.items()}")
        yield "ax.plot([1, 0])[0].get_color()"
    for data in ["np.ma.masked_less(z, 0)", "np.arange(30).reshape(5, 6)", "np.where(z > 1, np.nan, z)",
                 "np.arange(7), np.arange(6), np.ma.masked_greater(z, 1)"]:
        yield "fig, ax = plt.subplots()"
        yield f"mesh = ax.pcolor({data})"
        yield "mesh.get_array()"
        yield "len(mesh.get_paths()), mesh.get_facecolor().shape"
        yield "fig.canvas.draw()"
        yield "mesh.get_array(), mesh.get_facecolor().shape"
        yield "mesh.set_array(np.ones((5, 6)))"
        yield "len(mesh.get_paths()), mesh.get_array().shape"


def _colorbars() -> Iterator[str]:
    for args in ["im", "im, orientation='horizontal'", "im, extend='both'", "im, ticks=[0, 0.5, 1]",
                 "im, shrink=0.5, aspect=10, pad=0.1", "im, location='left'", "cs", "csf",
                 "im, format='%.2f'", "matplotlib.cm.ScalarMappable(cmap='jet'), ax=ax",
                 "im, spacing='proportional', boundaries=[-2, 0, 1, 3]"]:
        yield "fig, ax = plt.subplots()"
        yield "im = ax.imshow(z)"
        yield "cs = ax.contour(z, levels=4)"
        yield "csf = ax.contourf(z, levels=[-2, -1, 0, 1, 2])"
        yield f"cb = fig.colorbar({args})"
        yield "cb.orientation, cb.extend, cb.spacing, cb.drawedges, cb.filled"
        yield "cb.vmin, cb.vmax"
        yield "cb.values"
        yield "cb.boundaries"
        yield "cb.get_ticks()"
        yield "cb.get_ticks(minor=True)"
        yield "cb.norm"
        yield "cb.cmap.name"
        yield "cb.ax.get_position().bounds"
        yield "cb.ax.get_xlim(), cb.ax.get_ylim()"
        yield "cb.solids"
        yield "cb.lines"
        yield "cb.outline"
        yield "cb.dividers"
        yield "cb.formatter"
        yield "cb.locator"
        yield "cb.long_axis"
        yield "fig.canvas.draw()"
        yield "cb.ax.get_position().bounds"
        yield "ax.get_position().bounds"
        yield "cb.get_ticks()"
        yield "[t.get_text() for t in cb.ax.get_yticklabels()]"
        yield "[t.get_text() for t in cb.ax.get_xticklabels()]"


def _layout() -> Iterator[str]:
    for layout in ["None", "'tight'", "'constrained'", "'compressed'"]:
        yield f"fig, axs = plt.subplots(2, 2, layout={layout})"
        yield "axs[0, 0].set_title('title')"
        yield "axs[1, 1].set_xlabel('xlabel')"
        yield "fig.colorbar(axs[0, 1].imshow(z), ax=axs[0, 1])"
        yield "fig.suptitle('suptitle')"
        yield "fig.get_layout_engine()"
        yield "fig.get_constrained_layout(), fig.get_tight_layout()"
        yield "fig.canvas.draw()"
        yield "[ax.get_position().bounds for ax in fig.axes]"
        yield "[ax.get_tightbbox().bounds for ax in fig.axes]"
        yield "fig.get_tightbbox().bounds"
        yield "fig.subplotpars.left, fig.subplotpars.right, fig.subplotpars.wspace"
    yield "fig = plt.figure()"
    yield "gs = fig.add_gridspec(3, 3, width_ratios=[1, 2, 1], hspace=0.4)"
    yield "gs.get_geometry(), gs.get_width_ratios(), gs.get_height_ratios()"
    yield "ax1 = fig.add_subplot(gs[0, :])"
    yield "ax2 = fig.add_subplot(gs[1:, 1])"
    yield "ax1.get_subplotspec().get_geometry(), ax1.get_subplotspec().rowspan, ax1.get_subplotspec().colspan"
    yield "gs.get_grid_positions(fig)"
    yield "ax2.get_position().bounds"
    yield "fig.subplots_adjust(left=0.2, top=0.8)"
    yield "ax2.get_position().bounds"
    yield "fig.tight_layout()"
    yield "ax2.get_position().bounds"
    yield "sub = gs[2, 2].subgridspec(2, 1)"
    yield "fig.add_subplot(sub[0]).get_position().bounds"
    yield "fig.subplot_mosaic([['a', 'b'], ['c', 'c']], width_ratios=[2, 1])"
    yield "fig, ax = plt.subplots(figsize=(3, 2), dpi=50)"
    yield "fig.get_size_inches(), fig.dpi, fig.bbox.bounds"
    yield "fig.set_size_inches(5, 4)"
    yield "fig.bbox.bounds, ax.bbox.bounds"
    yield "ax.set_aspect('equal')"
    yield "ax.get_position().bounds"
    yield "ax.get_position(original=True).bounds"
    yield "ax.apply_aspect()"
    yield "ax.get_position().bounds"
    yield "fig.align_labels()"
    yield "plt.figure(figsize=(4, 4)).subfigures(2, 2, wspace=0.1)[0][1].bbox.bounds"


def _three_d() -> Iterator[str]:
    yield "from mpl_toolkits.mplot3d import Axes3D, axes3d, art3d, proj3d"
    yield "fig = plt.figure()"
    yield "ax = fig.add_subplot(projection='3d')"
    yield "type(ax).__name__"
    yield "ax.name"
    yield "X, Y, Z = axes3d.get_test_data(0.5)"
    yield "X.shape, Z.dtype, float(Z.sum())"
    yield "ax.plot_wireframe(X, Y, Z, rstride=2, cstride=2)"
    yield "ax.get_xlim(), ax.get_ylim(), ax.get_zlim()"
    yield "ax.elev, ax.azim, ax.roll, ax.dist if hasattr(ax, 'dist') else None"
    yield "ax.view_init(30, 60)"
    yield "ax.elev, ax.azim"
    yield "ax.get_proj()"
    yield "ax.get_zticks()"
    yield "ax.get_w_lims()"
    yield "ax.format_coord(0.01, 0.01)"
    yield "ax.get_box_aspect()"
    yield "ax.set_box_aspect((1, 2, 1))"
    yield "ax.get_box_aspect()"
    yield "ax.get_proj()"
    yield "fig.canvas.draw()"
    yield "[t.get_text() for t in ax.get_zticklabels()]"
    yield "ax.zaxis.get_ticklocs()"
    yield "ax.get_zlabel()"
    yield "ax.set_zlabel('zz')"
    yield "ax.zaxis.label.get_text()"
    yield "s = ax.scatter([0, 1], [0, 1], [0, 1], c=[1, 2])"
    yield "type(s).__name__"
    yield "s.get_offsets()"
    yield "s._offsets3d"
    yield "b = ax.bar3d([0, 1], [0, 1], [0, 0], 0.5, 0.5, [1, 2])"
    yield "type(b).__name__"
    yield "q = ax.quiver([0], [0], [0], [1], [1], [1])"
    yield "type(q).__name__"
    yield "t = ax.text(0, 0, 0, 'three')"
    yield "type(t).__name__"
    yield "proj3d.proj_transform(1, 1, 1, ax.get_proj())"
    yield "art3d.Line3D([0, 1], [0, 1], [0, 1]).get_data_3d()"
    yield "c = ax.contour(X, Y, Z, zdir='z', offset=-100)"
    yield "type(c).__name__"
    yield "fig.canvas.draw()"
    yield "ax.get_xlim(), ax.get_zlim()"


def _toolkits() -> Iterator[str]:
    yield "from mpl_toolkits.axes_grid1 import make_axes_locatable, ImageGrid, AxesGrid, host_subplot"
    yield "from mpl_toolkits.axes_grid1.inset_locator import inset_axes, zoomed_inset_axes, mark_inset"
    yield "from mpl_toolkits.axes_grid1.anchored_artists import AnchoredSizeBar"
    yield "from mpl_toolkits.axes_grid1 import axes_size"
    yield "fig, ax = plt.subplots()"
    yield "div = make_axes_locatable(ax)"
    yield "type(div).__name__"
    yield "cax = div.append_axes('right', size='5%', pad=0.05)"
    yield "type(cax).__name__"
    yield "fig.colorbar(ax.imshow(z), cax=cax)"
    yield "fig.canvas.draw()"
    yield "ax.get_position().bounds, cax.get_position().bounds"
    yield "fig = plt.figure()"
    yield "grid = ImageGrid(fig, 111, nrows_ncols=(2, 2), axes_pad=0.1, cbar_mode='single')"
    yield "len(grid), grid.get_geometry(), type(grid[0]).__name__"
    yield "[g.imshow(z) for g in grid]"
    yield "grid.cbar_axes[0].colorbar(grid[0].images[0])"
    yield "fig.canvas.draw()"
    yield "[g.get_position().bounds for g in grid]"
    yield "fig, ax = plt.subplots()"
    yield "ins = inset_axes(ax, width='30%', height='40%', loc='lower left')"
    yield "type(ins).__name__"
    yield "zoom = zoomed_inset_axes(ax, 2, loc='upper right')"
    yield "zoom.set_xlim(0, 0.2)"
    yield "zoom.set_ylim(0, 0.2)"
    yield "mark_inset(ax, zoom, loc1=2, loc2=4, fc='none')"
    yield "bar = AnchoredSizeBar(ax.transData, 0.2, '0.2', 'lower center')"
    yield "ax.add_artist(bar)"
    yield "fig.canvas.draw()"
    yield "ins.get_position().bounds, zoom.get_position().bounds"
    yield "axes_size.Fixed(2).get_size(None)"
    yield "axes_size.Scaled(2).get_size(None)"
    yield "axes_size.from_any('5%', fraction_ref=axes_size.AxesX(ax)).get_size(fig.canvas.get_renderer())"
    yield "fig = plt.figure()"
    yield "host = host_subplot(111)"
    yield "par = host.twinx()"
    yield "type(host).__name__, type(par).__name__"
    yield "import mpl_toolkits.axisartist as AA"
    yield "fig = plt.figure()"
    yield "aa = fig.add_subplot(axes_class=AA.Axes)"
    yield "type(aa).__name__"
    yield "aa.axis['top'].set_visible(False)"
    yield "aa.axis['bottom'].label.set_text('bottom')"
    yield "sorted(aa.axis.keys())"
    yield "fig.canvas.draw()"
    yield "aa.get_position().bounds"


AREAS = {
    "ticker": _ticker,
    "scales": _scales,
    "norms": _norms,
    "colormaps": _colormaps,
    "colors": _colors,
    "transforms": _transforms,
    "paths": _paths,
    "text": _text,
    "legends": _legends,
    "colorbars": _colorbars,
    "layout": _layout,
    "three_d": _three_d,
    "toolkits": _toolkits,
    "drawn_state": _drawn_state,
    "box_parts": _box_parts,
}


def programs() -> Iterator[tuple[str, list[str]]]:
    """One program per area."""
    for name, statements in AREAS.items():
        yield name, common.HEADER + common.DATA + list(statements())
