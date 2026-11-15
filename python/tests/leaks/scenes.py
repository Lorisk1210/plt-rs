"""
The scenes the leak harness makes and drops: each a few statements of
matplotlib's API, written for pltrs (``import pltrs.pyplot as plt``),
which run under matplotlib through the harness's alias.

A `Scene` is either *dropped* (its statements make figures, the harness
draws and saves every figure the scene left, then drops every reference,
the scene's names cleared as a script rebinding them leaves its module,
and checks with weak references that the figures, their Axes and their
artists are gone) or *kept* (its statements make a figure once, and its
``update`` runs on that figure at every iteration, as an animation, a
notebook cell run again or a web page redrawn does: what the updates
remove must be gone, and the process must not grow).

The harness's own scenes are below; the differential tester's are added
from ``spec/diff/generators`` (the interaction, pickling and overrides
scenes, the artists' subjects), as written there, their imports naming
pltrs.
"""

import json
import pathlib
import re
import sys
from dataclasses import dataclass, field

#: The repository's root.
ROOT = pathlib.Path(__file__).resolve().parents[3]

#: What every scene's namespace holds before its statements run.
HEADER = [
    "import io, pickle, copy, weakref, gc",
    "import numpy as np",
    "import pltrs",
    "import pltrs.pyplot as plt",
    "from pltrs import (animation, artist, axes, axis, colors, collections, contour, dates, "
    "figure, gridspec, image, layout_engine, legend, legend_handler, lines, offsetbox, patches, "
    "patheffects, path, projections, scale, spines, table, text, ticker, transforms, units, "
    "widgets)",
    "from pltrs.backend_bases import MouseEvent, KeyEvent, PickEvent",
    "from pltrs.backends.backend_agg import FigureCanvasAgg",
    "from pltrs.backends.backend_pdf import FigureCanvasPdf",
    "from pltrs.backends.backend_svg import FigureCanvasSVG",
    "x = np.linspace(0, 2 * np.pi, 7)",
    "y = np.sin(x)",
    "z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))",
    "values = np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])",
    "grid_x, grid_y = np.meshgrid(np.linspace(-2, 2, 12), np.linspace(-2, 2, 10))",
    "field = np.exp(-grid_x ** 2 - grid_y ** 2)",
]


@dataclass(frozen=True)
class Scene:
    """A scene: statements making figures, and for a kept scene the
    statements run on them at each iteration."""

    #: The statements making the scene (run once per iteration of a
    #: dropped scene, once in all for a kept one).
    make: tuple
    #: The statements run at each iteration of a kept scene; empty for a
    #: dropped one.
    update: tuple = ()
    #: The formats the dropped scene's figures are saved in.
    formats: tuple = ("png",)
    #: Whether it needs TeX on ``PATH``.
    usetex: bool = False
    #: Notes on the scene (why it is here).
    note: str = ""
    #: The names the update leaves which must be gone at the next
    #: iteration (weak references are taken to them).
    removed: tuple = field(default=())

    @property
    def kept(self):
        """Whether the scene's figure lives across iterations."""
        return bool(self.update)


def _axes(*statements, **more):
    """A dropped scene of one Axes."""
    return Scene(("fig, ax = plt.subplots(figsize=(4, 3))", *statements), **more)


def _axes3d(*statements, **more):
    """A dropped scene of one 3-D Axes."""
    return Scene(("fig = plt.figure(figsize=(4, 3))", "ax = fig.add_subplot(projection='3d')",
                  *statements), **more)


#: Every plotting method of `Axes`.
PLOTS = {
    "plot": _axes("ax.plot(x, y, 'o-', label='sin')"),
    "scatter": _axes("ax.scatter(x, y, c=values, s=values * 10, cmap='viridis')"),
    "bar": _axes("ax.bar(x, values, yerr=0.3, label='bars')"),
    "barh": _axes("ax.barh(x, values, hatch='//')"),
    "hist": _axes("ax.hist(np.sin(np.arange(200) * 0.37), bins=12)"),
    "step": _axes("ax.step(x, y, where='mid')"),
    "stem": _axes("ax.stem(x, y)"),
    "fill_between": _axes("ax.fill_between(x, y, y + 1, where=y > 0, interpolate=True)"),
    "errorbar": _axes("ax.errorbar(x, y, yerr=0.2, xerr=0.1, capsize=3)"),
    "pie": _axes("ax.pie(values, labels=list('abcdefg'), autopct='%1.1f%%', shadow=True)"),
    "stackplot": _axes("ax.stackplot(x, y + 1, values, labels=['a', 'b'])"),
    "stairs": _axes("ax.stairs(values, np.arange(8), fill=True)"),
    "eventplot": _axes("ax.eventplot([x, x * 2], colors=['C0', 'C1'])"),
    "hlines_vlines": _axes("ax.hlines(y, 0, x)", "ax.vlines(x, 0, y)"),
    "spans_lines": _axes("ax.axhline(0.5)", "ax.axvspan(1, 2, alpha=0.3)",
                         "ax.axline((0, 0), slope=0.5)"),
    "text_annotate": _axes("ax.text(1, 0.5, 'text', bbox=dict(boxstyle='round'))",
                           "ax.annotate('peak', xy=(1.5, 1), xytext=(3, 0.5), "
                           "arrowprops=dict(arrowstyle='->', connectionstyle='arc3,rad=0.3'))"),
    "imshow": _axes("ax.imshow(field, interpolation='bilinear')"),
    "matshow": _axes("ax.matshow(z)"),
    "pcolormesh": _axes("ax.pcolormesh(grid_x, grid_y, field, shading='auto')"),
    "pcolor": _axes("ax.pcolor(z)"),
    "pcolorfast": _axes("ax.pcolorfast(z)"),
    "contour": _axes("cs = ax.contour(grid_x, grid_y, field, levels=5)", "ax.clabel(cs)"),
    "contourf": _axes("ax.contourf(grid_x, grid_y, field, levels=6, hatches=['/', None])"),
    "spy": _axes("ax.spy(np.eye(6))"),
    "hexbin": _axes("ax.hexbin(np.sin(np.arange(300)), np.cos(np.arange(300) * 1.3), gridsize=8)"),
    "hist2d": _axes("ax.hist2d(np.sin(np.arange(300)), np.cos(np.arange(300) * 1.3), bins=6)"),
    "quiver": _axes("q = ax.quiver(grid_x, grid_y, grid_y, -grid_x)",
                    "ax.quiverkey(q, 0.8, 0.9, 1, 'key')"),
    "barbs": _axes("ax.barbs(grid_x[::3, ::3], grid_y[::3, ::3], grid_y[::3, ::3] * 20, "
                   "grid_x[::3, ::3] * 20)"),
    "boxplot": _axes("ax.boxplot([y, values], patch_artist=True)"),
    "violinplot": _axes("ax.violinplot([np.sin(np.arange(50)), np.cos(np.arange(50))])"),
    "fill": _axes("ax.fill(x, y, 'r', alpha=0.5)"),
    "loglog": _axes("ax.loglog(np.logspace(0, 3, 9), np.logspace(1, 2, 9))"),
    "broken_barh": _axes("ax.broken_barh([(1, 2), (4, 1)], (0, 1))"),
    "tricontour": _axes("ax.tricontour(np.cos(x), np.sin(x) + x / 10, values, levels=3)"),
    "tripcolor": _axes("ax.tripcolor(np.cos(x), np.sin(x) + x / 10, values)"),
    "triplot": _axes("ax.triplot(np.cos(x), np.sin(x) + x / 10)"),
    "specgram": _axes("ax.specgram(np.sin(np.arange(1024) * 0.3), NFFT=64, noverlap=32)"),
    "psd": _axes("ax.psd(np.sin(np.arange(512) * 0.3), NFFT=64)"),
    "acorr": _axes("ax.acorr(np.sin(np.arange(100) * 0.3), maxlags=10)"),
    "streamplot": _axes("ax.streamplot(np.linspace(-2, 2, 12), np.linspace(-2, 2, 10), grid_y, "
                        "-grid_x, density=0.5)"),
    "ecdf": _axes("ax.ecdf(values)"),
    "table": _axes("ax.table(cellText=[['1', '2'], ['3', '4']], colLabels=['a', 'b'], "
                   "loc='bottom')"),
    "bar_label": _axes("bars = ax.bar(x, values)", "ax.bar_label(bars, fmt='%.1f')"),
    "grouped_bar": _axes("ax.grouped_bar([values[:3], values[3:6]], tick_labels=['a', 'b', 'c'])"),
}

#: Every plotting method of `Axes3D`.
PLOTS_3D = {
    "3d_plot": _axes3d("ax.plot(x, y, values)"),
    "3d_scatter": _axes3d("ax.scatter(x, y, values, c=values)"),
    "3d_surface": _axes3d("ax.plot_surface(grid_x, grid_y, field, cmap='viridis')"),
    "3d_wireframe": _axes3d("ax.plot_wireframe(grid_x, grid_y, field)"),
    "3d_trisurf": _axes3d("ax.plot_trisurf(np.cos(x), np.sin(x) + x / 10, values)"),
    "3d_contour": _axes3d("ax.contour(grid_x, grid_y, field)", "ax.contourf(grid_x, grid_y, field, "
                          "offset=-1)"),
    "3d_bar3d": _axes3d("ax.bar3d(x, y, 0, 0.3, 0.3, values)"),
    "3d_quiver": _axes3d("ax.quiver(x, y, values, y, x, values, length=0.2)"),
    "3d_voxels": _axes3d("ax.voxels(np.indices((3, 3, 3)).sum(axis=0) < 4)"),
    "3d_text": _axes3d("ax.text(0, 0, 0, 'origin', 'x')"),
    "3d_stem": _axes3d("ax.stem(x, y, values)"),
    "3d_fill_between": _axes3d("ax.fill_between(x, y, 0, x, y, 1)"),
    "3d_errorbar": _axes3d("ax.errorbar(x, y, values, zerr=0.5)"),
}

#: Everything around the plots: decorations, layouts, projections, text,
#: images, toolkits.
DECORATIONS = {
    "legend": _axes("ax.plot(x, y, label='a')", "ax.plot(x, -y, label='b')",
                    "ax.legend(loc='upper right', title='legend', shadow=True)"),
    "legend_draggable": _axes("ax.plot(x, y, label='a')", "ax.legend().set_draggable(True)"),
    "figure_legend": _axes("ax.plot(x, y, label='a')", "fig.legend()"),
    "colorbar": _axes("im = ax.imshow(field)", "fig.colorbar(im, ax=ax, extend='both')"),
    "colorbar_mappable": _axes("fig.colorbar(plt.cm.ScalarMappable(cmap='magma'), ax=ax)"),
    "colorbar_removed": _axes("im = ax.imshow(field)", "fig.colorbar(im, ax=ax).remove()"),
    "annotation_arrows": _axes("ax.annotate('a', xy=(0.2, 0.2), xytext=(0.7, 0.8), "
                               "arrowprops=dict(facecolor='black', shrink=0.05))",
                               "ax.add_patch(patches.ConnectionPatch((0.1, 0.1), (0.9, 0.5), "
                               "'data', arrowstyle='->'))"),
    "inset": _axes("ax.imshow(field)", "inset = ax.inset_axes([0.6, 0.6, 0.35, 0.35])",
                   "inset.imshow(field)", "inset.set_xlim(2, 5)", "inset.set_ylim(5, 2)",
                   "ax.indicate_inset_zoom(inset)"),
    "twins": _axes("ax.plot(x, y)", "ax.twinx().plot(x, values, 'r')",
                   "ax.twiny().plot(values, y, 'g')"),
    "secondary_axes": _axes("ax.plot(x, y)",
                            "ax.secondary_xaxis('top', functions=(lambda v: v * 2, lambda v: v / 2))",
                            "ax.secondary_yaxis('right')"),
    "shared": Scene(("fig, axs = plt.subplots(2, 2, sharex=True, sharey=True)",
                     "[a.plot(x, y * i) for i, a in enumerate(axs.flat)]")),
    "shared_across_figures": Scene(("fig, ax = plt.subplots()", "fig2, ax2 = plt.subplots()",
                                    "ax2.sharex(ax)", "ax.plot(x, y)", "ax2.plot(x, values)")),
    "subfigures": Scene(("fig = plt.figure(layout='constrained')",
                         "left, right = fig.subfigures(1, 2)", "left.subplots(2, 1)[0].plot(x, y)",
                         "right.add_subplot().imshow(z)", "right.suptitle('right')")),
    "layout_constrained": Scene(("fig, axs = plt.subplots(2, 2, layout='constrained')",
                                 "fig.colorbar(axs[0, 0].imshow(z), ax=axs)",
                                 "fig.suptitle('constrained')")),
    "layout_compressed": Scene(("fig, axs = plt.subplots(2, 2, layout='compressed')",
                                "[a.imshow(z) for a in axs.flat]")),
    "layout_tight": Scene(("fig, axs = plt.subplots(2, 2, layout='tight')",
                           "[a.set_title('t') for a in axs.flat]")),
    "tight_layout_call": Scene(("fig, axs = plt.subplots(1, 2)", "fig.tight_layout()")),
    "subplot_mosaic": Scene(("fig, axd = plt.subplot_mosaic('AB;CC', layout='constrained')",
                             "axd['C'].plot(x, y)")),
    "polar": Scene(("fig = plt.figure()", "ax = fig.add_subplot(projection='polar')",
                    "ax.plot(x, values)", "ax.set_rlim(0, 10)")),
    "aitoff": Scene(("fig = plt.figure()", "ax = fig.add_subplot(projection='aitoff')",
                     "ax.plot(x / 4 - 0.8, y / 2)", "ax.grid(True)")),
    "hammer_mollweide_lambert": Scene((
        "fig = plt.figure()", "fig.add_subplot(1, 3, 1, projection='hammer').plot(x / 4 - 0.8, y / 2)",
        "fig.add_subplot(1, 3, 2, projection='mollweide').grid(True)",
        "fig.add_subplot(1, 3, 3, projection='lambert').plot(x / 4 - 0.8, y / 2)")),
    "mathtext": _axes("ax.set_title(r'$\\frac{a}{b} \\sqrt{x^2} \\int_0^\\infty \\alpha$')",
                      "ax.set_xlabel(r'$\\mathrm{time}\\ \\mathbf{t}$')"),
    "usetex": _axes("ax.set_title(r'$\\frac{a}{b}$ tex', usetex=True)", usetex=True),
    "text_fontproperties": _axes("t = ax.text(0.5, 0.5, 'text')", "t.get_fontproperties()",
                                 note="ECO6: a text whose font properties were read kept its "
                                      "figure alive"),
    "image_kinds": _axes(
        "ax.imshow(field, extent=(0, 1, 0, 1))",
        "nu = image.NonUniformImage(ax, extent=(1, 2, 0, 1))",
        "nu.set_data(np.linspace(0, 1, 6), np.linspace(0, 1, 5), z)", "ax.add_image(nu)",
        "pc = image.PcolorImage(ax, np.arange(7), np.arange(6), z)", "ax.add_image(pc)",
        "ax.add_artist(image.BboxImage(ax.bbox, data=z))", "fig.figimage(z, 10, 10)",
        "ax.set_xlim(0, 7)", "ax.set_ylim(0, 6)"),
    "pyplot_imshow_removed": Scene(("plt.figure()", "im = plt.imshow(z)", "im.remove()",
                                    "plt.plot(x, y)"),
                                   note="ECO7: a removed image stayed the current image"),
    "offsetboxes": _axes(
        "ax.add_artist(offsetbox.AnchoredText('anchored', loc='upper left'))",
        "ax.add_artist(offsetbox.AnnotationBbox(offsetbox.OffsetImage(z, zoom=3), (0.5, 0.5)))",
        "da = offsetbox.DrawingArea(30, 20)", "da.add_artist(patches.Circle((10, 10), 5))",
        "ax.add_artist(offsetbox.AnchoredOffsetbox('lower right', child=da))"),
    "path_effects": _axes("ax.plot(x, y, lw=3, path_effects=[patheffects.withStroke(linewidth=6, "
                          "foreground='w'), patheffects.SimpleLineShadow()])",
                          "ax.text(1, 0, 'shadow', path_effects=[patheffects.withSimplePatchShadow()])"),
    "units_dates": _axes("days = np.arange('2020-01-01', '2020-01-08', dtype='datetime64[D]')",
                         "ax.plot(days, values)",
                         "ax.xaxis.set_major_formatter(dates.ConciseDateFormatter("
                         "ax.xaxis.get_major_locator()))"),
    "units_categories": _axes("ax.bar(['a', 'b', 'c'], [1, 2, 3])", "ax.plot(['b', 'c'], [3, 1])"),
    "scales": Scene(("fig, axs = plt.subplots(2, 2)", "axs[0, 0].set_yscale('log')",
                     "axs[0, 1].set_xscale('symlog')", "axs[1, 0].set_xscale('logit')",
                     "axs[1, 1].set_yscale('function', functions=(np.sqrt, np.square))",
                     "[a.plot(np.linspace(0.1, 0.9, 5), np.linspace(1, 9, 5)) for a in axs.flat]")),
    "sankey": Scene(("from pltrs.sankey import Sankey", "fig, ax = plt.subplots()",
                     "Sankey(ax=ax, flows=[1, -0.5, -0.5], labels=['in', 'a', 'b']).finish()")),
    "axes_grid1": Scene(("from pltrs.mpl_toolkits.axes_grid1 import ImageGrid, make_axes_locatable",
                         "fig = plt.figure()",
                         "grid = ImageGrid(fig, 211, nrows_ncols=(1, 2), cbar_mode='single')",
                         "[g.imshow(z) for g in grid]",
                         "ax = fig.add_subplot(212)", "im = ax.imshow(z)",
                         "fig.colorbar(im, cax=make_axes_locatable(ax).append_axes('right', '5%'))")),
    "axisartist": Scene(("from pltrs.mpl_toolkits import axisartist", "fig = plt.figure()",
                         "ax = fig.add_subplot(axes_class=axisartist.Axes)", "ax.plot(x, y)",
                         "ax.axis['top'].set_visible(False)")),
    "parasite_axes": Scene(("from pltrs.mpl_toolkits.axes_grid1 import host_subplot",
                            "plt.figure()", "host = host_subplot(111)", "par = host.twinx()",
                            "host.plot(x, y)", "par.plot(x, values)")),
}

#: What programs do with figures besides drawing them: pickles, saves of
#: every backend, canvases swapped, events, animations, widgets, clears
#: and closes.
LIVES = {
    "pickled": _axes("ax.plot(x, y)", "ax.legend(['l'])",
                     "fig2 = pickle.loads(pickle.dumps(fig))", "fig2.canvas.draw()"),
    "deepcopied": _axes("ax.plot(x, y)", "fig2 = copy.deepcopy(fig)"),
    "saves_every_format": _axes("ax.plot(x, y)", "ax.set_title('$x^2$')",
                                formats=("png", "pdf", "svg", "ps", "eps", "raw", "jpg", "tiff",
                                         "webp")),
    "saves_pgf": _axes("ax.plot(x, y)", "ax.set_title('$x^2$')", formats=("pgf",), usetex=True),
    "save_tight": _axes("ax.plot(x, y)", "fig.savefig(io.BytesIO(), bbox_inches='tight')"),
    "canvas_swapped": _axes("ax.plot(x, y)", "FigureCanvasAgg(fig).draw()",
                            "FigureCanvasPdf(fig).print_pdf(io.BytesIO())",
                            "FigureCanvasSVG(fig).print_svg(io.BytesIO())"),
    "figure_without_pyplot": Scene(("fig = figure.Figure()", "ax = fig.add_subplot()",
                                    "ax.plot(x, y)", "FigureCanvasAgg(fig)")),
    "figure_without_pyplot_or_canvas": Scene(("fig = figure.Figure()", "ax = fig.subplots()",
                                              "ax.plot(x, y)")),
    "events_lambda": _axes("ax.plot(x, y, picker=True)",
                           "fig.canvas.mpl_connect('button_press_event', lambda e: e.inaxes)",
                           "fig.canvas.mpl_connect('pick_event', lambda e: e.artist)",
                           "fig.canvas.draw()",
                           "MouseEvent('button_press_event', fig.canvas, 200, 150, 1)._process()"),
    "events_bound_method": _axes(
        "class Watcher:\n    def __init__(self, fig):\n        self.fig = fig\n        "
        "self.cid = fig.canvas.mpl_connect('motion_notify_event', self.moved)\n    "
        "def moved(self, event):\n        self.last = event",
        "watcher = Watcher(fig)", "fig.canvas.draw()",
        "MouseEvent('motion_notify_event', fig.canvas, 100, 100)._process()"),
    "axes_callbacks": _axes("ax.callbacks.connect('xlim_changed', lambda a: a.get_xlim())",
                            "ax.set_xlim(0, 3)"),
    "keys_navigation": _axes("ax.plot(x, y)", "fig.canvas.draw()",
                             "KeyEvent('key_press_event', fig.canvas, 'l', 100, 100)._process()",
                             "KeyEvent('key_press_event', fig.canvas, 'g', 100, 100)._process()"),
    "pick": _axes("line, = ax.plot(x, y, picker=5)", "fig.canvas.draw()",
                  "fig.canvas.mpl_connect('pick_event', lambda e: e.artist)",
                  "ax.pick(MouseEvent('button_press_event', fig.canvas, *ax.transData.transform("
                  "(x[2], y[2])), 1))"),
    "animation_func": _axes(
        "line, = ax.plot(x, y)",
        "anim = animation.FuncAnimation(fig, lambda i: (line.set_ydata(y * i),) and (line,), "
        "frames=4, blit=False, repeat=False)",
        "anim._init_draw()", "[anim._draw_next_frame(i, False) for i in range(3)]"),
    "animation_saved_pillow": _axes(
        "line, = ax.plot(x, y)",
        "anim = animation.FuncAnimation(fig, lambda i: line.set_ydata(y * i), frames=3)",
        "anim.save('anim.gif', writer=animation.PillowWriter(fps=5))"),
    "animation_artists": _axes(
        "frames = [[ax.plot(x, y * i)[0]] for i in range(3)]",
        "anim = animation.ArtistAnimation(fig, frames)", "anim._init_draw()",
        "[anim._draw_next_frame(f, False) for f in frames]"),
    "widgets": Scene((
        "fig = plt.figure()",
        "slider = widgets.Slider(fig.add_axes([0.1, 0.9, 0.8, 0.05]), 'v', 0, 1)",
        "slider.on_changed(lambda v: v)", "slider.set_val(0.3)",
        "button = widgets.Button(fig.add_axes([0.1, 0.8, 0.2, 0.05]), 'b')",
        "button.on_clicked(lambda e: e)",
        "check = widgets.CheckButtons(fig.add_axes([0.1, 0.6, 0.2, 0.15]), ['a', 'b'])",
        "radio = widgets.RadioButtons(fig.add_axes([0.4, 0.6, 0.2, 0.15]), ['a', 'b'])",
        "box = widgets.TextBox(fig.add_axes([0.7, 0.8, 0.2, 0.05]), 't', initial='x')",
        "ax = fig.add_axes([0.1, 0.1, 0.8, 0.4])", "ax.plot(x, y)",
        "rect = widgets.RectangleSelector(ax, lambda a, b: None, interactive=True)",
        "span = widgets.SpanSelector(ax, lambda a, b: None, 'horizontal')",
        "lasso = widgets.LassoSelector(ax, lambda v: None)",
        "cursor = widgets.Cursor(ax, useblit=False)",
        "multi = widgets.MultiCursor(None, [ax], useblit=False)")),
    "clf": _axes("ax.plot(x, y)", "fig.canvas.draw()", "fig.clf()", "fig.add_subplot().plot(y)"),
    "cla": _axes("ax.plot(x, y)", "ax.legend(['l'])", "fig.canvas.draw()", "ax.cla()",
                 "ax.plot(values)"),
    "axes_removed": _axes("ax.plot(x, y)", "fig.canvas.draw()", "ax.remove()",
                          "fig.add_subplot().plot(y)"),
    "closed_by_figure": Scene(("fig = plt.figure()", "fig.add_subplot().plot(x, y)",
                               "plt.close(fig)")),
    "closed_by_number": Scene(("fig = plt.figure(7)", "fig.add_subplot().plot(x, y)", "plt.close(7)")),
    "gcf_gca": Scene(("plt.plot(x, y)", "plt.title('t')", "plt.colorbar(plt.scatter(x, y, c=y))",
                      "fig = plt.gcf()")),
    "rc_context": Scene(("with plt.rc_context({'lines.linewidth': 3}):\n    "
                         "fig, ax = plt.subplots()\n    ax.plot(x, y)",)),
    "style_context": Scene(("with plt.style.context('ggplot'):\n    "
                            "fig, ax = plt.subplots()\n    ax.plot(x, y)",)),
    "funcformatter_locator": _axes(
        "ax.plot(x, y)",
        "ax.xaxis.set_major_formatter(ticker.FuncFormatter(lambda v, p: f'{v:.1f}'))",
        "ax.yaxis.set_major_locator(ticker.MaxNLocator(3))",
        "ax.xaxis.set_minor_locator(ticker.AutoMinorLocator())"),
    "formatter_referring_figure": _axes(
        "ax.plot(x, y)",
        "ax.xaxis.set_major_formatter(ticker.FuncFormatter(lambda v, p: f'{v}{len(fig.axes)}'))"),
    "agg_filter_lambda": _axes("line, = ax.plot(x, y)",
                               "line.set_agg_filter(lambda img, dpi: (img, 0, 0))"),
    "axes_locator_lambda": _axes(
        "ax.set_axes_locator(lambda a, r: transforms.Bbox.from_bounds(0.1, 0.1, 0.8, 0.8))"),
    "transform_program": _axes(
        "class Shift(transforms.Transform):\n    input_dims = output_dims = 2\n    "
        "def transform_non_affine(self, v):\n        return v + 0.1",
        "ax.plot(x, y, transform=Shift() + ax.transData)"),
    "scale_keeping_axis": _axes(
        "class KeepingScale(scale.LinearScale):\n    name = 'keeping'\n    "
        "def __init__(self, axis):\n        super().__init__(axis)\n        self.kept = axis",
        "scale.register_scale(KeepingScale)", "ax.set_xscale('keeping')", "ax.plot(x, y)",
        note="a scale of the program's keeping its axis, as matplotlib's own pattern does"),
    "transform_keeping_axes": _axes(
        "class Keeping(transforms.Transform):\n    input_dims = output_dims = 2\n    "
        "def __init__(self, ax):\n        super().__init__()\n        self.ax = ax\n    "
        "def transform_non_affine(self, v):\n        return v",
        "line, = ax.plot(x, y)", "line.set_transform(Keeping(ax) + ax.transData)",
        "ax.add_patch(patches.Circle((1, 0), 0.5, transform=Keeping(ax) + ax.transData))",
        note="a transform of the program's keeping its Axes, given to artists"),
    "transform_keyword_keeping_axes": _axes(
        "class Keeping(transforms.Transform):\n    input_dims = output_dims = 2\n    "
        "def __init__(self, ax):\n        super().__init__()\n        self.ax = ax\n    "
        "def transform_non_affine(self, v):\n        return v",
        "ax.plot(x, y, transform=Keeping(ax) + ax.transData)",
        note="the same, given as the plotting method's keyword"),
    "path_effect_keyword_keeping_axes": _axes(
        "class Effect(patheffects.AbstractPathEffect):\n    def __init__(self, ax):\n"
        "        super().__init__()\n        self.ax = ax\n    "
        "def draw_path(self, renderer, gc, tpath, affine, rgbFace=None):\n        "
        "renderer.draw_path(gc, tpath, affine, rgbFace)",
        "ax.plot(x, y, path_effects=[Effect(ax)])"),
    "norm_program": _axes(
        "class Norm(colors.Normalize):\n    def __call__(self, v, clip=None):\n        "
        "return super().__call__(v, clip)",
        "ax.imshow(z, norm=Norm())"),
    "scale_registered": _axes(
        "class Ident(transforms.Transform):\n    input_dims = output_dims = 1\n    "
        "def transform_non_affine(self, v):\n        return v\n    def inverted(self):\n        "
        "return Ident()",
        "class Plain(scale.ScaleBase):\n    name = 'leakplain'\n    def get_transform(self):\n"
        "        return Ident()\n    def set_default_locators_and_formatters(self, axis):\n        "
        "axis.set_major_locator(ticker.AutoLocator())\n        "
        "axis.set_major_formatter(ticker.ScalarFormatter())",
        "scale.register_scale(Plain)", "ax.set_xscale('leakplain')", "ax.plot(x, y)"),
    "units_registered": _axes(
        "class Thing:\n    def __init__(self, v):\n        self.v = v",
        "class Conv(units.ConversionInterface):\n    @staticmethod\n    def convert(v, u, a):\n"
        "        return np.asarray([t.v for t in np.atleast_1d(v)], dtype=float)",
        "units.registry[Thing] = Conv()", "ax.plot([Thing(1), Thing(2)], [1, 2])",
        "del units.registry[Thing]"),
    "private_names": _axes(
        "line, = ax.plot(x, y)", "t = ax.text(0.5, 0.5, 't')",
        "line._drawstyle, t._color, ax._get_lines, ax._children, fig._suptitle",
        "fig._suptitle = fig.text(0.5, 0.95, 'sup')", "fig.suptitle('again')",
        "ax._get_lines(ax, x, y)", "t._x = 0.3", "ax.legend(['a'])._legend_box",
        note="PRIV1's private names: read, assigned and called"),
    "assigned_attributes": _axes(
        "w = ax.add_patch(patches.Wedge((0.5, 0.5), 0.3, 10, 100))", "w.theta2 = 200",
        "im = ax.imshow(z)", "im.origin = 'lower'", "ax.axison = True",
        "a = ax.annotate('a', (0.2, 0.2), xytext=(0.5, 0.5), arrowprops={})",
        "a.arrow_patch", "ax.callbacks = type(ax.callbacks)()",
        note="TYPES2/3's assignable attributes"),
}

#: Kept scenes: one figure updated at every iteration, as long-running
#: programs update theirs.
KEPT = {
    "kept_set_data": Scene(("fig, ax = plt.subplots()", "line, = ax.plot(x, y)"),
                           ("line.set_ydata(y * np.cos(i))", "fig.canvas.draw()")),
    "kept_clear_replot": Scene(("fig, ax = plt.subplots()",),
                               ("ax.clear()", "line, = ax.plot(x, y * i)", "ax.legend(['l'])",
                                "fig.canvas.draw()"), removed=("line",)),
    "kept_remove_readd": Scene(("fig, ax = plt.subplots()", "line, = ax.plot(x, y)"),
                               ("line.remove()", "line, = ax.plot(x, y * i)",
                                "fig.canvas.draw()"), removed=("line",)),
    "kept_images_replaced": Scene(("fig, ax = plt.subplots()", "im = ax.imshow(field)"),
                                  ("im.remove()", "im = ax.imshow(field * i)",
                                   "fig.canvas.draw()"), removed=("im",)),
    "kept_pyplot_imshow": Scene(("fig = plt.figure()",),
                                ("im = plt.imshow(field)", "im.remove()", "fig.canvas.draw()"),
                                removed=("im",),
                                note="ECO7: pyplot's current image kept removed images"),
    "kept_colorbar_replaced": Scene(("fig, ax = plt.subplots()", "im = ax.imshow(field)",
                                     "cb = fig.colorbar(im)"),
                                    ("cb.remove()", "cb = fig.colorbar(im)", "fig.canvas.draw()"),
                                    removed=("cb",)),
    "kept_legend_remade": Scene(("fig, ax = plt.subplots()", "ax.plot(x, y, label='a')"),
                                ("leg = ax.legend(title=str(i))", "fig.canvas.draw()"),
                                removed=("leg",)),
    "kept_texts": Scene(("fig, ax = plt.subplots()", "t = ax.text(0.5, 0.5, '')"),
                        ("t.set_text(f'frame {i % 50}')", "ax.set_title(f'$x_{{{i % 30}}}$')",
                         "fig.canvas.draw()")),
    "kept_savefig": Scene(("fig, ax = plt.subplots()", "ax.plot(x, y)"),
                          ("fig.savefig(io.BytesIO(), format=('png', 'pdf', 'svg')[i % 3])",)),
    "kept_clf": Scene(("fig = plt.figure()",),
                      ("fig.clf()", "ax = fig.add_subplot()", "ax.plot(x, y * i)",
                       "fig.canvas.draw()"), removed=("ax",)),
    "kept_twins_removed": Scene(("fig, ax = plt.subplots()",),
                                ("twin = ax.twinx()", "twin.plot(x, y)", "fig.canvas.draw()",
                                 "twin.remove()"), removed=("twin",)),
    "kept_connect_disconnect": Scene(("fig, ax = plt.subplots()",),
                                     ("cid = fig.canvas.mpl_connect('draw_event', lambda e: i)",
                                      "fig.canvas.draw()", "fig.canvas.mpl_disconnect(cid)")),
    "kept_pickled": Scene(("fig, ax = plt.subplots()", "ax.plot(x, y)"),
                          ("pickle.loads(pickle.dumps(fig))",)),
    "kept_events": Scene(("fig, ax = plt.subplots()", "ax.plot(x, y)", "fig.canvas.draw()"),
                         ("MouseEvent('motion_notify_event', fig.canvas, 100 + i % 50, 100)"
                          "._process()",
                          "MouseEvent('scroll_event', fig.canvas, 100, 100, 'up', step=1)"
                          "._process()")),
    "kept_blit": Scene(("fig, ax = plt.subplots()", "line, = ax.plot(x, y, animated=True)",
                        "fig.canvas.draw()", "bg = fig.canvas.copy_from_bbox(ax.bbox)"),
                       ("fig.canvas.restore_region(bg)", "line.set_ydata(y * np.sin(i))",
                        "ax.draw_artist(line)", "fig.canvas.blit(ax.bbox)")),
    "kept_scatter_offsets": Scene(("fig, ax = plt.subplots()", "sc = ax.scatter(x, y)"),
                                  ("sc.set_offsets(np.c_[x, y * np.cos(i)])",
                                   "sc.set_array(values * i)", "fig.canvas.draw()")),
    "kept_annotations": Scene(("fig, ax = plt.subplots()",),
                              ("a = ax.annotate(str(i), (0.5, 0.5), xytext=(0.2, 0.2), "
                               "arrowprops=dict(arrowstyle='->'))", "fig.canvas.draw()",
                               "a.remove()"), removed=("a",)),
}


#: The scenes that leak under pltrs (or fail) for a cause not fixed yet:
#: what holds what, by scene. The harness reports them as known; the
#: smoke test lets them be.
KNOWN = {
    **dict.fromkeys(
        ("transform_keyword_keeping_axes", "path_effect_keyword_keeping_axes"),
        "pltrs (Rust) holds the program's object given as a plotting method's keyword "
        "(``transform=``, ``path_effects=``) strongly, and the object keeps its Axes: a "
        "cycle through native state, which Python's collector cannot see (objects a Python "
        "object of the figure holds are held weakly, `_program_classes.Held`; nothing holds "
        "these)"),
    **dict.fromkeys(
        ("override_transforms_Transform_inverted", "override_colors_Normalize_inverse"),
        "the last transform or norm of the program's mapped stays held by pltrs (Rust)'s "
        "process-wide memo of the program's answers (one object, not its figure: nothing "
        "grows)"),
    "kept_blit": "pltrs refuses to restore a region saved before the canvas was drawn "
                 "again (`CanvasError::StaleRegion`); matplotlib restores it",
}


def _to_pltrs(statement):
    """A statement of matplotlib's names, naming pltrs's."""
    return (statement.replace("mpl_toolkits", "pltrs.mpl_toolkits")
            .replace("matplotlib", "pltrs"))


def _generators():
    """The differential tester's scene modules (``spec/diff/generators``)."""
    sys.path.insert(0, str(ROOT / "spec" / "diff"))
    try:
        from generators import artists, interaction_scenes, overrides_scenes, pickling_scenes
    finally:
        sys.path.pop(0)
    return artists, interaction_scenes, overrides_scenes, pickling_scenes


#: A member each overridden class's subclass overrides, by the base's name
#: as the census writes it: the first member of the census that the
#: class defines (its subclass calls the base's).
def _overridden_members():
    census = json.loads((ROOT / "spec" / "diff" / "overrides_census.json").read_text("utf-8"))
    members = {}
    for row in census["rows"]:
        if row["kind"] == "method" and not row["member"].startswith("__"):
            members.setdefault(row["class"], row["member"])
    return members


def _subclass(base, member):
    """The source of ``Sub``, a subclass of *base* overriding *member*
    with a method calling the base's."""
    return (f"class Sub({base}):\n    def {member}(self, *args, **kwargs):\n        "
            f"return super().{member}(*args, **kwargs)")


def _generated():
    """The differential tester's scenes, by name."""
    artists, interaction_scenes, overrides_scenes, pickling_scenes = _generators()
    found = {}
    for name, statements in interaction_scenes.SCENES.items():
        found[f"interaction_{name}"] = Scene(tuple(map(_to_pltrs, statements)))
    for name, (statements, _) in {**pickling_scenes.SCENES,
                                  **pickling_scenes.UNMANAGED}.items():
        found[f"pickling_{name}"] = Scene(tuple(map(_to_pltrs, statements)))
    for name, statements in artists.SUBJECTS:
        found[f"artist_{name}"] = Scene(tuple(map(_to_pltrs, statements)))
    members = _overridden_members()
    for base, scene in overrides_scenes.SCENES.items():
        member = members.get(base)
        if member is None:
            continue
        module, _, cls = base.rpartition(".")
        statements = [overrides_scenes.IMPORTS, *scene.imports,
                      f"from {module} import {cls} as Base"]
        if scene.impl:
            statements += [scene.impl.strip(), "Base = Impl"]
        statements.append(_subclass("Base", member))
        if scene.hosts:
            statements.append(scene.hosts.strip())
        statements += [*scene.make, *scene.steps]
        name = re.sub(r"\W", "_", base.removeprefix("matplotlib."))
        found[f"override_{name}_{member}"] = Scene(tuple(map(_to_pltrs, statements)))
    return found


def own_scenes():
    """The harness's own scenes, by name."""
    return {**PLOTS, **PLOTS_3D, **DECORATIONS, **LIVES, **KEPT}


def all_scenes():
    """Every scene, by name: the harness's own, then the generated ones."""
    return {**own_scenes(), **_generated()}
