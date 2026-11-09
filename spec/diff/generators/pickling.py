"""Programs pickling and copying what matplotlib pickles and copies.

* **Figures** (``pickling_scenes``): Axes with every kind of artist,
  twinned and shared Axes, colorbars, legends, subfigures, 3-D Axes, the
  ``axes_grid1`` and ``axisartist`` toolkits, polar Axes, images,
  contours, collections, containers, annotations, offset boxes, math
  text, scales and units, layout engines, widgets, callbacks registered
  (kept where matplotlib keeps them, dropped where it drops them), figure
  labels and suptitles: each figure is drawn, pickled and unpickled, and
  the unpickled figure pickled again. Every getter of the figure, of its
  Axes and of their first artists of each kind is then called on the
  original (which pickling must leave as it was) and on the unpickled
  figure, and the scene's own checks run on the unpickled figure; both
  copies are drawn and saved, and stay open for their pictures.
* **Values**: transforms, boxes, paths, colormaps, norms, markers, the
  patch styles, font properties, ``rcParams``, cyclers, locators and
  formatters, date rules, loose artists, each pickled, then copied with
  ``copy.copy`` and ``copy.deepcopy``; each copy changed, to see that the
  original stays as it was (or follows, where matplotlib's shallow copy
  shares).
* **pyplot**: a pickled figure's number, label and manager once
  unpickled (``plt.close`` between), ``copy.deepcopy`` of a figure, and
  what cannot be pickled (a lambda formatter, ``UnitData``).
"""

from __future__ import annotations

from typing import Any, Iterator

from . import common
from .pickling_scenes import SCENES, UNMANAGED

#: The statements every program starts with.
SETUP = common.HEADER + common.DATA + [
    "import copy, io, pickle",
    "def roundtrip(obj):\n    return pickle.loads(pickle.dumps(obj, pickle.HIGHEST_PROTOCOL))",
]

#: Artists of each kind of an Axes whose getters a program calls: the
#: first ones of each list.
KINDS = ("lines", "collections", "patches", "images", "texts", "artists", "tables", "child_axes")
#: How many of each kind.
FIRST = 2

#: The statements pickling ``fig`` into ``fig2``, and ``fig2`` into ``fig3``.
PICKLE = [
    "fig.canvas.draw()",
    "buffer = io.BytesIO()",
    "pickle.dump(fig, buffer, pickle.HIGHEST_PROTOCOL)",
    "buffer.seek(0)",
    "fig2 = pickle.load(buffer)",
    "fig3 = roundtrip(fig2)",
    "(type(fig2).__name__, type(fig2.canvas).__name__, fig2.canvas.figure is fig2, fig2 is fig)",
    "[type(a).__name__ for a in fig2.axes]",
    "[[type(c).__name__ for c in a.get_children()] for a in fig2.axes]",
    "[type(c).__name__ for c in fig2.get_children()]",
    "[[type(c).__name__ for c in a.get_children()] for a in fig3.axes]",
]

#: The statements drawing and saving the copies, after the scene's checks.
SAVE = [
    "fig2.canvas.draw()",
    "fig3.canvas.draw()",
    "fig2.savefig(io.BytesIO(), format='svg')",
    "fig2.savefig(io.BytesIO(), format='pdf')",
    "fig3.savefig(io.BytesIO(), format='eps')",
    "[a.get_xlim() for a in fig3.axes]",
    "[a.get_position().bounds for a in fig3.axes]",
]


def _probed(figure: Any) -> list[tuple[str, Any]]:
    """The objects of ``figure`` whose getters a program calls, each as
    its expression from the figure (``F``)."""
    found = [("F", figure)]
    for i, ax in enumerate(figure.axes):
        found.append((f"F.axes[{i}]", ax))
        for kind in KINDS:
            for j, child in enumerate(list(getattr(ax, kind, []))[:FIRST]):
                found.append((f"F.axes[{i}].{kind}[{j}]", child))
        if ax.get_legend() is not None:
            found.append((f"F.axes[{i}].get_legend()", ax.get_legend()))
    for kind in ("legends", "texts", "images", "artists", "subfigs"):
        for j, child in enumerate(list(getattr(figure, kind, []))[:FIRST]):
            found.append((f"F.{kind}[{j}]", child))
    return found


def _probes(statements: list[str]) -> list[str]:
    """Every getter of the objects of the figure ``statements`` leave in
    ``fig``, called on ``fig`` and on ``fig2``."""
    figure = common.namespace(statements)["fig"]
    calls = []
    for expression, obj in _probed(figure):
        for name in common.getters(obj):
            calls.append(f"{expression}.{name}()")
    return [call.replace("F", variable, 1) for variable in ("fig", "fig2") for call in calls]


#: The values the value programs pickle and copy: name -> (statements
#: leaving the value in ``value``, statements changing ``changed`` (a copy),
#: statements reading ``value`` and ``changed``).
VALUES: dict[str, tuple[list[str], list[str], list[str]]] = {
    "affine": (["value = transforms.Affine2D().rotate(0.5).scale(2, 3)"],
               ["changed.translate(1, 1)"],
               ["value.get_matrix()", "changed.get_matrix()"]),
    "transform_tree": ([
        "identity = transforms.IdentityTransform()",
        "composite = transforms.CompositeGenericTransform(identity, transforms.IdentityTransform())",
        "wrapper = transforms.TransformWrapper(composite)",
        "value = {'composite': composite, 'wrapper': wrapper, "
        "'outer': transforms.CompositeGenericTransform(wrapper, identity)}",
    ], ["changed['wrapper'].set(transforms.Affine2D().scale(2))"],
        ["value['wrapper'].transform([[1.0, 2.0]])", "changed['wrapper'].transform([[1.0, 2.0]])",
         "changed['outer'].transform([[1.0, 2.0]])", "changed['wrapper']._child is changed['composite']",
         "[ref() is changed['outer'] for ref in changed['wrapper']._parents.values()]",
         "(changed['wrapper'].input_dims, changed['wrapper'].output_dims)"]),
    "blended": (["value = transforms.blended_transform_factory(transforms.Affine2D().scale(2), "
                 "transforms.Affine2D().translate(0, 1))"],
                [],
                ["value.transform([[1.0, 1.0]])", "changed.transform([[1.0, 1.0]])", "changed.is_separable"]),
    "bbox": (["value = transforms.Bbox([[0, 1], [2, 3]])"],
             ["changed.x1 = 5", "changed.update_from_data_xy([[10, 10]], ignore=False)"],
             ["value.get_points()", "changed.get_points()", "changed.minpos"]),
    "transformed_bbox": (["value = transforms.TransformedBbox(transforms.Bbox.unit(), "
                          "transforms.Affine2D().scale(3))"],
                         [],
                         ["value.get_points()", "changed.get_points()"]),
    "path": (["value = path.Path([[0, 0], [1, 0], [1, 1], [0, 0]], [1, 2, 2, 79])"],
             ["changed.vertices[1] = [5, 5]"],
             ["value.vertices", "changed.vertices", "changed.codes", "changed.readonly"]),
    "unit_circle": (["value = path.Path.unit_circle()"],
                    ["changed.vertices[0] = [9, 9]"],
                    ["value.vertices[:2]", "changed.vertices[:2]", "changed.readonly"]),
    "text_path": (["from matplotlib.textpath import TextPath", "value = TextPath((0, 0), 'Ab', size=10)"],
                  [],
                  ["value.vertices.shape", "changed.vertices.shape", "changed.get_extents().bounds"]),
    "colormap": (["value = plt.get_cmap('viridis')"],
                 ["changed.set_bad('r')", "changed.set_over('k')"],
                 ["(value.get_bad(), value.get_over(), value.name, value.N)",
                  "(changed.get_bad(), changed.get_over(), changed.name, changed.N)", "changed(0.5)"]),
    "listed_colormap": (["value = colors.ListedColormap(['r', 'g', 'b'], name='rgb').with_extremes(under='k')"],
                        [],
                        ["(value.name, value.N, value.get_under())", "(changed.name, changed.N, changed(0.9))",
                         "changed.reversed().colors"]),
    "segmented_colormap": (["value = colors.LinearSegmentedColormap.from_list('mine', ['k', 'r', 'w'], N=16)"],
                           ["changed.set_under('b')"],
                           ["(value(0.3), value.get_under(), changed.get_under(), changed.N)",
                            "changed.resampled(4)(1.0)"]),
    "multivar_colormap": (["value = colors.MultivarColormap(['Reds', 'Blues'], 'sRGB_add')"],
                          [],
                          ["(value.name, value.n_variates, changed.n_variates, changed.combination_mode)"]),
    "bivar_colormap": (["value = colors.BivarColormapFromImage(np.linspace(0, 1, 27).reshape(3, 3, 3))"],
                       [],
                       ["(changed.N, changed.M, changed.shape)", "changed((0.5, 0.5))"]),
    "norms": (["value = [colors.Normalize(0, 2), colors.LogNorm(1, 10), colors.BoundaryNorm([0, 1, 2], 2), "
               "colors.TwoSlopeNorm(0, -1, 2), colors.CenteredNorm(1, 2), colors.SymLogNorm(1, vmin=-5, vmax=5), "
               "colors.PowerNorm(2, 0, 4), colors.AsinhNorm(0.5, -3, 3), colors.NoNorm(), "
               "colors.FuncNorm((np.square, np.sqrt), 0, 3), "
               "colors.make_norm_from_scale(scale.LogitScale, colors.Normalize)()]"],
              ["changed[0].vmin = -2", "changed[1].autoscale([1, 1000])"],
              ["[type(n).__name__ for n in changed]", "[(n.vmin, n.vmax) for n in value]",
               "[(n.vmin, n.vmax) for n in changed]", "[n(0.5) for n in changed[:-1]]",
               "changed[-1](np.array([0.25]))", "changed[2].boundaries",
               "[type(n.callbacks).__name__ for n in changed]"]),
    "markers": (["from matplotlib import markers",
                 "value = [markers.MarkerStyle('o', fillstyle='left'), markers.MarkerStyle('$x$'), "
                 "markers.MarkerStyle(path.Path.unit_rectangle(), transform=transforms.Affine2D().rotate(1)), "
                 "markers.MarkerStyle('^', capstyle='round', joinstyle='bevel')]"],
                [],
                ["[(m.get_marker() if isinstance(m.get_marker(), str) else type(m.get_marker()).__name__, "
                 "m.get_fillstyle(), m.get_capstyle(), m.get_joinstyle()) for m in changed]",
                 "[m.get_path().vertices.shape for m in changed]",
                 "[m.get_transform().get_matrix() for m in changed]"]),
    "patch_styles": (["value = [patches.BoxStyle('round', pad=0.5), patches.ArrowStyle('->', head_length=0.6), "
                      "patches.ConnectionStyle('arc3', rad=0.3), patches.BoxStyle.Sawtooth(), "
                      "patches.ArrowStyle.Fancy(head_width=2)]"],
                     ["changed[0].pad = 0.9"],
                     ["[type(s).__name__ for s in changed]", "(value[0].pad, changed[0].pad)",
                      "changed[1].head_length", "changed[2].rad"]),
    "join_cap_styles": (["from matplotlib._enums import JoinStyle, CapStyle",
                         "value = [JoinStyle('miter'), CapStyle('round')]"],
                        [],
                        ["changed", "[s is t for s, t in zip(value, changed)]"]),
    "font_properties": (["from matplotlib import font_manager",
                         "value = font_manager.FontProperties(family='serif', size=12, weight='bold', "
                         "style='italic')"],
                        ["changed.set_size(20)"],
                        ["(value.get_size(), changed.get_size(), changed.get_family(), changed.get_weight(), "
                         "changed.get_style())", "value == changed", "hash(value) == hash(roundtrip(value))"]),
    "rcparams": (["value = matplotlib.rcParams"],
                 ["changed['lines.linewidth'] = 7"],
                 ["(value['lines.linewidth'], changed['lines.linewidth'], type(changed).__name__)",
                  "len(changed) == len(value)", "changed is value",
                  "dict(roundtrip(value.find_all('^lines\\\\.')))"]),
    "cycler": (["value = plt.cycler(color='rg') * plt.cycler(lw=[1, 2])"],
               [],
               ["list(changed)", "changed == value", "changed.keys"]),
    "tickers": (["value = [ticker.MaxNLocator(4, steps=[1, 5]), ticker.LogLocator(base=2, subs=(1, 1.5)), "
                 "ticker.MultipleLocator(0.25, offset=0.1), ticker.FixedLocator([1, 2]), "
                 "ticker.FormatStrFormatter('%.2f'), ticker.StrMethodFormatter('{x:.1f}'), ticker.PercentFormatter(50), "
                 "ticker.EngFormatter('Hz'), ticker.FixedFormatter(['a', 'b']), ticker.ScalarFormatter(useMathText=True), "
                 "ticker.LogFormatterSciNotation(base=4)]"],
                ["changed[0].set_params(nbins=9)"],
                ["[type(t).__name__ for t in changed]", "(value[0]._nbins, changed[0]._nbins)",
                 "[t.tick_values(1, 10) for t in changed[:4]]", "[f(0.5, 1) for f in changed[4:6]]",
                 "(changed[6].format_pct(0.5, 1), changed[7](0.5), changed[8](0.5, 1))",
                 "(changed[-2].get_useMathText(), changed[-1]._base, changed[-1].labelOnlyBase)"]),
    "dates": (["from matplotlib import dates as mdates",
               "value = [mdates.rrulewrapper(mdates.DAILY, interval=2), mdates.DateFormatter('%Y-%m'), "
               "mdates.AutoDateLocator(minticks=2), mdates.ConciseDateFormatter(mdates.AutoDateLocator()), "
               "mdates.DayLocator(bymonthday=[1, 15]), mdates.AutoDateFormatter(mdates.AutoDateLocator())]"],
              [],
              ["[type(d).__name__ for d in changed]", "changed[1](19000.5)",
               "changed[2].tick_values(mdates.num2date(19000), mdates.num2date(19100))", "changed[4].tick_values(mdates.num2date(19000), mdates.num2date(19060))",
               "changed[0]._rrule._interval"]),
    "loose_artists": (["value = [lines.Line2D([0, 1], [1, 2], color='r', marker='o'), "
                       "patches.Circle((0, 0), 1, fc='g'), text.Text(0, 0, 'hi', color='b'), "
                       "collections.PathCollection([path.Path.unit_circle()], sizes=[4])]"],
                      ["changed[0].set_color('k')"],
                      ["[type(a).__name__ for a in changed]", "(value[0].get_color(), changed[0].get_color())",
                       "changed[1].get_facecolor()", "changed[2].get_text()", "changed[3].get_sizes()",
                       "[a.figure is None for a in changed]"]),
    "scalar_mappable": (["from matplotlib import cm",
                         "value = cm.ScalarMappable(colors.Normalize(0, 4), 'magma')",
                         "value.set_array([1, 2, 3])"],
                        ["changed.set_clim(0, 10)"],
                        ["(value.get_clim(), changed.get_clim(), changed.get_cmap().name)",
                         "changed.to_rgba(2.0)", "changed.get_array()"]),
    "path_effects": (["from matplotlib import patheffects",
                      "value = [patheffects.withStroke(linewidth=3, foreground='w'), patheffects.SimplePatchShadow(), "
                      "patheffects.Normal(), patheffects.TickedStroke(angle=45)]"],
                     [],
                     ["[type(e).__name__ for e in changed]", "changed[0]._gc", "changed[1]._offset"]),
    "scales": (["fig, ax = plt.subplots()", "value = [scale.LogScale(ax.xaxis, base=2), "
                "scale.SymmetricalLogScale(ax.yaxis, linthresh=2), scale.LogitScale(None), "
                "scale.FuncScale(None, (np.square, np.sqrt)), scale.AsinhScale(None)]"],
               [],
               ["[type(s).__name__ for s in changed]",
                "[s.get_transform().transform([0.5, 4.0]) for s in changed]",
                "[type(s.get_transform()).__name__ for s in changed]"]),
}

#: program -> statements after `SETUP`: pyplot's registry, artists copied,
#: and what cannot be pickled.
PROGRAMS: dict[str, list[str]] = {
    "pyplot_registry": [
        "fig = plt.figure('a label', figsize=(2, 2))",
        "fig.add_subplot().plot(x, y)",
        "other = plt.figure(figsize=(1, 1))",
        "buffer = io.BytesIO()",
        "pickle.dump(fig, buffer, pickle.HIGHEST_PROTOCOL)",
        "plt.close('all')",
        "plt.get_fignums()",
        "fig2 = pickle.loads(buffer.getbuffer())",
        "(plt.get_fignums(), fig2.get_label(), fig2.number, plt.gcf() is fig2)",
        "type(fig2.canvas.manager).__name__",
        "fig2.canvas.manager.num",
        "fig3 = roundtrip(fig2)",
        "(plt.get_fignums(), fig3.number, plt.gcf() is fig3, fig3.get_label())",
        "plt.figure('a label') is fig2",
        "plt.close(fig2)",
        "plt.get_fignums()",
        "deep = copy.deepcopy(fig3)",
        "(plt.get_fignums(), deep.number, deep is fig3, deep.axes[0] is fig3.axes[0])",
        "shallow = copy.copy(fig3)",
        "(shallow is fig3, shallow.axes == fig3.axes, plt.get_fignums())",
        "from matplotlib.figure import Figure",
        "unmanaged = Figure()",
        "unmanaged.add_subplot().plot([1, 2])",
        "loaded = roundtrip(unmanaged)",
        "(plt.get_fignums(), type(loaded.canvas).__name__, loaded.canvas.manager)",
        "ax_alone = roundtrip(fig3.axes[0])",
        "(type(ax_alone).__name__, type(ax_alone.figure).__name__, ax_alone.figure is fig3, plt.get_fignums())",
        "line_alone = roundtrip(fig3.axes[0].lines[0])",
        "(line_alone.axes is not None, line_alone.figure is not None, line_alone.get_xydata()[:2], "
        "plt.get_fignums())",
    ],
    "copied_artists": [
        "fig, ax = plt.subplots(figsize=(3, 2))",
        "line, = ax.plot(x, y, label='line')",
        "rect = ax.add_patch(patches.Rectangle((1, 0), 1, 0.5, fc='y'))",
        "label = ax.text(1, 0.5, 'text')",
        "sc = ax.scatter(x, y)",
        "shallow = copy.copy(line)",
        "shallow.set_color('r')",
        "shallow.set_data([0, 1], [1, 0])",
        "(line.get_color(), shallow.get_color(), line.get_xydata()[:2], shallow.get_xydata())",
        "(shallow.axes is ax, shallow.figure is fig, shallow in ax.lines, shallow.get_label())",
        "deep = copy.deepcopy(line)",
        "(deep.axes is ax, deep.figure is fig, type(deep.figure).__name__, plt.get_fignums())",
        "deep.set_linewidth(5)",
        "(line.get_linewidth(), deep.get_linewidth(), deep.axes.lines[0] is deep)",
        "patch_copy = copy.copy(rect)",
        "patch_copy.set_width(3)",
        "(rect.get_width(), patch_copy.get_width(), patch_copy.get_facecolor())",
        "text_copy = copy.copy(label)",
        "text_copy.set_text('copied')",
        "(label.get_text(), text_copy.get_text(), text_copy in ax.texts)",
        "sc_copy = copy.copy(sc)",
        "sc_copy.set_sizes([100])",
        "(sc.get_sizes(), sc_copy.get_sizes())",
        "legend_handle = copy.copy(line)",
        "legend_handle.update_from(dashed := ax.plot(x, -y, 'r--')[0])",
        "(legend_handle.get_color(), legend_handle.get_linestyle(), line.get_linestyle())",
        "transform_copy = copy.copy(ax.transData)",
        "ax.set_xlim(0, 100)",
        "(transform_copy.transform([[50.0, 0.0]]), ax.transData.transform([[50.0, 0.0]]))",
        "transform_deep = copy.deepcopy(ax.transData)",
        "ax.set_xlim(0, 10)",
        "(transform_deep.transform([[5.0, 0.0]]), ax.transData.transform([[5.0, 0.0]]))",
        "formatter_copy = copy.copy(ax.xaxis.get_major_formatter())",
        "(formatter_copy.axis is ax.xaxis, formatter_copy(5.0, 0))",
        "locator_copy = copy.deepcopy(ax.xaxis.get_major_locator())",
        "(type(locator_copy).__name__, locator_copy.axis is ax.xaxis)",
        "axis_ticks = copy.copy(ax.xaxis.get_major_ticks()[0])",
        "type(axis_ticks).__name__",
        "copied_bbox = copy.deepcopy(ax.bbox)",
        "fig.set_size_inches(5, 4)",
        "(copied_bbox.bounds, ax.bbox.bounds)",
        "copy.copy(ax).get_xlim()",
        "fig.canvas.draw()",
    ],
    "unpicklable": [
        "fig, ax = plt.subplots(figsize=(3, 2))",
        "ax.plot(x, y)",
        "ax.xaxis.set_major_formatter(ticker.FuncFormatter(lambda v, pos: f'[{v}]'))",
        "pickle.dumps(fig)",
        "pickle.dumps(ax)",
        "def named(v, pos):\n    return f'<{v}>'",
        "ax.xaxis.set_major_formatter(named)",
        "roundtrip(fig).axes[0].xaxis.get_major_formatter()(1.0, 0)",
        "ax.yaxis.set_major_formatter(lambda v, pos: 'y')",
        "pickle.dumps(fig)",
        "ax.yaxis.set_major_formatter(ticker.ScalarFormatter())",
        "ax.set_xscale('function', functions=(lambda v: v, lambda v: v))",
        "pickle.dumps(fig)",
        "ax.set_xscale('linear')",
        "roundtrip(fig).axes[0].get_xscale()",
        "from matplotlib import category",
        "fig2, ax2 = plt.subplots()",
        "ax2.bar(['a', 'b'], [1, 2])",
        "pickle.dumps(fig2)",
        "plt.close(fig2)",
        "pickle.dumps(category.UnitData(['a', 'b']))",
        "copy.deepcopy(category.UnitData(['a', 'b']))",
        "copy.copy(category.UnitData(['a', 'b']))._mapping",
        "ax.text(0, 0, 'gen', gid=(i for i in range(2)))",
        "pickle.dumps(fig)",
        "from matplotlib.backends.backend_agg import RendererAgg",
        "type(roundtrip(RendererAgg(10, 20, 30))).__name__",
        "pickle.dumps(fig.canvas)",
        "type(roundtrip(fig.canvas)).__name__",
    ],
}


def _scene_program(build: list[str], checks: list[str]) -> list[str]:
    statements = SETUP + build
    return statements + PICKLE + _probes(statements + ["fig.canvas.draw()"]) + checks + SAVE


def _value_program(make: list[str], change: list[str], read: list[str]) -> list[str]:
    """``value`` pickled, pickled twice, copied and deep-copied, each copy
    changed and read beside the original."""
    statements = SETUP + make + ["type(value).__name__"]
    for how in ("roundtrip(value)", "roundtrip(roundtrip(value))", "copy.copy(value)", "copy.deepcopy(value)"):
        statements += [f"changed = {how}", "type(changed).__name__", *change, *read]
    return statements


def programs() -> Iterator[tuple[str, list[str]]]:
    """The figure programs, the value programs and the others."""
    for name, (build, checks) in {**SCENES, **UNMANAGED}.items():
        yield f"figure_{name}", _scene_program(build, checks)
    for name, (make, change, read) in VALUES.items():
        yield f"value_{name}", _value_program(make, change, read)
    for name, statements in PROGRAMS.items():
        yield name, SETUP + statements
