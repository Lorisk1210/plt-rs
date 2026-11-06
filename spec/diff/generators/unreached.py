"""The public names of matplotlib's core modules the ``api`` programs leave
unreached, each called where it has what it needs.

The ``api`` generator calls every callable with values per parameter on
its subject; some need an object a subject does not make (a renderer of a
drawn figure, a list of images, a font of a DVI file), a subclass (the
abstract `Transform`), or a state the earlier calls of their program
spoiled (a figure whose resolution a setter made negative). Each program
here is one area: its statements make what the names need, then call them
with representative and edge arguments.
"""

from __future__ import annotations

from typing import Iterator

from . import common

#: The statements of every program: the shared data, a figure and Axes
#: with a line, and the renderer of their drawn canvas.
SETUP = common.HEADER + common.DATA + [
    "import io",
    "from matplotlib import image, colorbar, legend, legend_handler, patheffects, table, tri, spines, mlab, "
    "texmanager, backend_bases, font_manager",
    "fig, ax = plt.subplots()",
    "line, = ax.plot(x, y, label='l')",
    "fig.canvas.draw()",
    "renderer = fig.canvas.get_renderer()",
    "square = path.Path([(0, 0), (10, 0), (10, 10), (0, 10), (0, 0)], closed=True)",
]

#: program name -> statements after `SETUP`
PROGRAMS: dict[str, list[str]] = {
    "figure_draw": [
        "sub = fig.subfigures(1, 2)[1]",
        "sub.add_subplot().plot(y, x)",
        "fig.draw(renderer)",
        "sub.draw(renderer)",
        "fig.draw(fig.canvas.get_renderer())",
        "fig.draw_without_rendering()",
    ],
    "image_composite": [
        "a = ax.imshow(z)",
        "b = ax.imshow(z[::-1], alpha=0.5, extent=(0, 3, 0, 2))",
        "fig.canvas.draw()",
        "image.composite_images([a, b], renderer)",
        "image.composite_images([a, b], renderer, magnification=2.0)",
        "image.composite_images([a], renderer)",
        "image.composite_images([], renderer)",
    ],
    "colorbar_axes": [
        "fig2, (left, right) = plt.subplots(1, 2)",
        "cax, kw = colorbar.make_axes([left, right], location='right', fraction=0.1)",
        "kw",
        "cax.get_position().bounds",
        "cax2, kw2 = colorbar.make_axes_gridspec(left, location='bottom', shrink=0.8)",
        "kw2",
        "cax2.get_position().bounds",
        "cax3, kw3 = colorbar.make_axes(right, orientation='horizontal', aspect=10, pad=0.2)",
        "kw3",
        "colorbar.make_axes(right, location='nope')",
        "colorbar.make_axes_gridspec(right, location='left', orientation='horizontal')",
    ],
    "transform_base": [
        "class Plain(transforms.Transform):\n    input_dims = output_dims = 2",
        "t = Plain()",
        "t.transform_non_affine([[1.0, 2.0], [3.0, 4.0]])",
        "t.transform_non_affine(np.array([[np.nan, 1.0]]))",
        "t.transform([[1.0, 2.0]])",
        "t.inverted()",
        "transforms.Transform.inverted(transforms.Affine2D().scale(2))",
        "transforms.Transform.transform_non_affine(transforms.Affine2D().scale(2), [[1.0, 1.0]])",
    ],
    "path_compound": [
        "path.Path.make_compound_path(path.Path.unit_rectangle(), path.Path.unit_circle())",
        "path.Path.make_compound_path(square, path.Path([(0, 0), (1, 1)]))",
        "path.Path.make_compound_path()",
        "path.Path.make_compound_path(path.Path(np.empty((0, 2))))",
        "path.Path.unit_circle_righthalf()",
        "path.Path.unit_circle_righthalf() is path.Path.unit_circle_righthalf()",
    ],
    "legend_handler_map": [
        "original = dict(legend.Legend.get_default_handler_map())",
        "legend.Legend.update_default_handler_map({lines.Line2D: legend_handler.HandlerLine2D(numpoints=3)})",
        "sorted(k.__name__ for k in legend.Legend.get_default_handler_map())",
        "legend.Legend.get_legend_handler(legend.Legend.get_default_handler_map(), line)",
        "legend.Legend.get_legend_handler({}, line)",
        "legend.Legend.get_legend_handler({line: legend_handler.HandlerLine2D()}, line)",
        "ax.legend().legend_handles",
        "legend.Legend.set_default_handler_map(original)",
        "len(legend.Legend.get_default_handler_map()) == len(original)",
    ],
    "manager_loop": [
        "backend_bases.FigureManagerBase.start_main_loop()",
        "backend_bases.FigureManagerBase.pyplot_show(block=False)",
        "from matplotlib.backends import backend_agg",
        "backend_agg._BackendAgg.FigureManager",
        "backend_agg._BackendAgg.draw_if_interactive()",
        "backend_agg._BackendAgg.show(block=False)",
    ],
    "patheffects_draw": [
        "gc = renderer.new_gc()",
        "patheffects.PathPatchEffect(offset=(2, -2), facecolor='y').draw_path(renderer, gc, square, "
        "transforms.Affine2D(), (1, 0, 0, 1))",
        "patheffects.SimpleLineShadow(offset=(1, -1), shadow_color='g').draw_path(renderer, gc, square, "
        "transforms.Affine2D(), None)",
        "patheffects.TickedStroke(angle=60, length=2).draw_path(renderer, gc, square, transforms.Affine2D(), "
        "(0, 0, 1, 1))",
        "patheffects.TickedStroke(spacing=3).draw_path(renderer, gc, path.Path([(0, 0), (20, 5)]), "
        "transforms.Affine2D(), None)",
        "patheffects.withStroke(linewidth=4, foreground='w').draw_path(renderer, gc, square, "
        "transforms.Affine2D(), (1, 0, 0, 1))",
        "renderer.buffer_rgba().tobytes() == renderer.buffer_rgba().tobytes()",
    ],
    "table_items": [
        "tab = table.table(ax, cellText=[['a', 'b'], ['c', 'd']], loc='top')",
        "table.table(ax, cellText=[['e']], rowLabels=['r'], colLabels=['C'], loc='center left')",
        "tab[2, 0] = table.Cell((0, 0), 0.1, 0.1, text='new')",
        "tab[2, 0].get_text().get_text()",
        "sorted(tab.get_celld())",
        "tab[0, 0] = table.Cell((0, 0), 0.2, 0.2)",
        "tab['x'] = table.Cell((0, 0), 0.1, 0.1)",
        "table.table(ax)",
    ],
    "tri_functions": [
        "xs, ys, zs = [0, 1, 0, 1, 0.5], [0, 0, 1, 1, 0.5], [1, 2, 3, 4, 0]",
        "tri.tricontour(ax, xs, ys, zs).levels",
        "tri.tricontourf(ax, xs, ys, zs, levels=3).levels",
        "tri.tripcolor(ax, xs, ys, zs).get_array()",
        "tri.tripcolor(ax, xs, ys, facecolors=[1, 2, 3, 4]).get_array()",
        "[l.get_xydata().shape for l in tri.triplot(ax, xs, ys, 'r.-')]",
        "tri.tricontour(ax, xs, ys)",
    ],
    "spine_factories": [
        "spines.Spine.arc_spine(ax, 'top', (0.5, 0.5), 0.4, 0, 90).get_path().vertices.shape",
        "spines.Spine.circular_spine(ax, (0.5, 0.5), 0.4).get_path().vertices.shape",
        "ax.spines['left'] = spines.Spine.linear_spine(ax, 'left')",
        "ax.spines['extra'] = spines.Spine.linear_spine(ax, 'right')",
        "sorted(ax.spines.keys())",
    ],
    "kde_factor": [
        "kde = mlab.GaussianKDE(values)",
        "kde.covariance_factor()",
        "mlab.GaussianKDE(values, bw_method='silverman').covariance_factor()",
        "mlab.GaussianKDE(values, bw_method=0.3).covariance_factor()",
        "mlab.GaussianKDE(values, bw_method=lambda k: 0.5).covariance_factor()",
    ],
    "tex_preambles": [
        "texmanager.TexManager.get_font_preamble()",
        "texmanager.TexManager.get_custom_preamble()",
        "plt.rcParams['text.latex.preamble'] = r'\\usepackage{amsmath}'",
        "texmanager.TexManager.get_custom_preamble()",
        "plt.rcParams['font.family'] = 'serif'",
        "texmanager.TexManager.get_font_preamble()",
    ],
    "pdf_reference": [
        "from matplotlib.backends import backend_pdf",
        "out = io.BytesIO()",
        "backend_pdf.Reference(7).write({'Type': backend_pdf.Name('Page')}, out)",
        "backend_pdf.Reference(8).write([1, 2.5, None, True], out)",
        "out.getvalue()",
        "backend_pdf.Reference(9).write(float('nan'), out)",
    ],
    "svg_writer_flush": [
        "from matplotlib.backends import backend_svg",
        "svg_out = io.StringIO()",
        "w = backend_svg.XMLWriter(svg_out)",
        "w.start('g', id='a')",
        "w.flush()",
        "svg_out.getvalue()",
        "w.end('g')",
        "w.flush()",
        "svg_out.getvalue()",
    ],
    "deprecated_reads": [
        "formatter = ax.yaxis.get_major_formatter()",
        "formatter.locs",
        "formatter.orderOfMagnitude",
        "formatter.format",
        "ax.xaxis.converter",
        "from matplotlib import dates",
        "concise = dates.ConciseDateFormatter(dates.AutoDateLocator())",
        "concise.offset_string",
        "scale.InvertedSymmetricalLogTransform(10, 1, 1).invlinthresh",
        "ins = ax.inset_axes([0.6, 0.6, 0.3, 0.3])",
        "ins.set_xlim(1, 2)",
        "indicator = ax.indicate_inset_zoom(ins)",
        "indicator[0]",
        "patches.BoxStyle.register('square2', patches.BoxStyle.Square)",
        "'square2' in patches.BoxStyle.get_styles()",
    ],
}


def programs() -> Iterator[tuple[str, list[str]]]:
    """One program per area, ending as the ``api`` programs end: every
    pyplot figure drawn and saved in each vector format."""
    from .api import ENDING
    for name, statements in PROGRAMS.items():
        yield name, SETUP + statements + ENDING
