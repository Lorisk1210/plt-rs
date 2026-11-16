"""
Rows written by hand: what programs do with the objects artists hand
out. First the idioms a sweep of 600 user scripts changed in place (the
half violin among them), each with the change such a script makes; then
the objects behind the artists (colormaps, norms, tick helpers, boxes,
transforms), the lists of Axes, figures, containers, contour sets,
legends and tables, and the lists of rcParams.
"""

import numpy as np

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, colors, patches, ticker, transforms
from pltrs.path import Path

from . import scenes
from .measure import Row, Scene

#: The rows of this module.
ROWS = []


def row(name, make, hand, mutate, read=None, same=None, frame=True):
    """
    Add the row *name*: *make* builds its subject in an Axes (and may
    return ``(subject, {name: kept})``), *hand*, *mutate*, *read* and
    *same* are `.Row`'s. The view is fixed after building unless *frame*
    is false.
    """
    def build():
        fig, ax = scenes.figure_and_axes()
        made = make(ax)
        keeps = type(made) is tuple and len(made) == 2 and isinstance(made[1], dict)
        made, kept = made if keeps else (made, {})
        if frame:
            scenes.frame(ax)
        return Scene(fig, made, ax=ax, **kept)
    ROWS.append(Row(name, build, hand, mutate, read, same))


def setitem(key, value):
    """A change in place: ``target[key] = value``."""
    return lambda target: target.__setitem__(key, value)


def _scatter(ax):
    return ax.scatter([1, 2, 3], [1, 2, 3], s=[40, 60, 80], c=[0.1, 0.5, 0.9],
                      linewidths=[1, 2, 3], edgecolors="k")


def _line(ax):
    return ax.plot(np.array([0.0, 1, 2]), np.array([0.0, 1, 0]))[0]


def _polygon(ax):
    return ax.add_patch(patches.Polygon([[0, 0], [1, 0], [1, 1]]))


def _path_patch(ax):
    return ax.add_patch(patches.PathPatch(Path([(0, 0), (1, 0), (1, 1), (0, 0)], [1, 2, 2, 79])))


def _field():
    grid = np.linspace(-1, 1, 10)
    return grid, grid, np.add.outer(grid, grid)


# The idioms of the scripts.

ALL = slice(None)

row("fill_between_paths_vertices", lambda ax: ax.fill_between([0, 1, 2], [0, 1, 0], [1, 2, 1]),
    lambda s: s.obj.get_paths()[0].vertices, setitem((ALL, 0), 0.5))


def _half(vertices):
    vertices[:, 0] = np.clip(vertices[:, 0], 1.0, None)


row("violin_body_vertices", lambda ax: ax.violinplot([[1, 2, 2, 3, 3, 3, 4]])["bodies"][0],
    lambda s: s.obj.get_paths()[0].vertices, _half)
row("scatter_paths_vertices", _scatter, lambda s: s.obj.get_paths()[0].vertices,
    lambda v: v.__imul__(3))
row("contourf_paths_vertices", lambda ax: ax.contourf(*_field(), levels=3),
    lambda s: s.obj.get_paths()[1].vertices, lambda v: v.__iadd__(0.3))
row("scatter_offsets", _scatter, lambda s: s.obj.get_offsets(), setitem((0, 0), 3))
row("scatter_sizes", _scatter, lambda s: s.obj.get_sizes(), setitem(0, 900))
row("scatter_facecolor", _scatter, lambda s: s.obj.get_facecolor(), setitem((ALL, 0), 1.0))
row("scatter_edgecolor", _scatter, lambda s: s.obj.get_edgecolor(), setitem((ALL, 1), 1.0))
row("scatter_linewidths", _scatter, lambda s: s.obj.get_linewidths(), setitem(0, 8))
row("scatter_array", _scatter, lambda s: s.obj.get_array(), setitem(0, 0.9))
row("linecollection_segments",
    lambda ax: ax.add_collection(collections.LineCollection([[(0, 0), (1, 1)], [(0, 1), (1, 0)]])),
    lambda s: s.obj.get_segments()[0], setitem((1, 1), 0))
row("quadmesh_array", lambda ax: ax.pcolormesh(np.arange(12.0).reshape(3, 4)),
    lambda s: s.obj.get_array(), setitem((0, 0), 11))
row("quadmesh_coordinates", lambda ax: ax.pcolormesh(np.arange(12.0).reshape(3, 4)),
    lambda s: s.obj.get_coordinates(), setitem((0, 0, 0), 2))
row("line_xdata_orig", _line, lambda s: s.obj.get_xdata(orig=True), setitem(1, 2),
    read=lambda s: s.obj.get_xdata())
row("line_ydata", _line, lambda s: s.obj.get_ydata(), setitem(1, 0.5))
row("line_xdata_not_orig", _line, lambda s: s.obj.get_xdata(orig=False), setitem(1, 2))
row("line_data", _line, lambda s: s.obj.get_data()[1], setitem(1, 0.2))
row("line_xydata", _line, lambda s: s.obj.get_xydata(), setitem((1, 1), 0.3))
row("line_path_vertices", _line, lambda s: s.obj.get_path().vertices, setitem((1, 1), 0.3))
row("polygon_xy", _polygon, lambda s: s.obj.get_xy(), setitem((2, 0), 0))
row("polygon_xy_attribute", _polygon, lambda s: s.obj.xy, setitem((2, 0), 0))
row("polygon_path_vertices", _polygon, lambda s: s.obj.get_path().vertices, setitem((2, 0), 0))
row("pathpatch_path_vertices", _path_patch, lambda s: s.obj.get_path().vertices,
    setitem((2, 0), 0))
row("pathpatch_path_codes", _path_patch, lambda s: s.obj.get_path().codes, setitem(2, 1))
row("polygon_verts", _polygon, lambda s: s.obj.get_verts(), setitem((0, 0), 999))
row("bar_path_vertices", lambda ax: ax.bar([0, 1], [1, 2])[0],
    lambda s: s.obj.get_path().vertices, lambda v: v.__imul__(2))
row("image_array", lambda ax: ax.imshow(np.arange(9.0).reshape(3, 3)),
    lambda s: s.obj.get_array(), setitem((0, 0), 8))
row("image_extent", lambda ax: ax.imshow(np.arange(9.0).reshape(3, 3)),
    lambda s: s.obj.get_extent(), setitem(0, 1.0))


def _plotted(ax):
    ax.plot([0, 1], [0, 1])
    return ax


row("axes_position_points", _plotted, lambda s: s.obj.get_position().get_points(),
    setitem((0, 0), 0.4))
row("axes_datalim_points", _plotted, lambda s: s.obj.dataLim.get_points(), setitem((1, 1), 5))
row("axes_viewlim_points", _plotted, lambda s: s.obj.viewLim.get_points(), setitem((1, 1), 5))
row("axes_bbox_points", _plotted, lambda s: s.obj.bbox.get_points(), setitem((0, 0), 100))


def _shifted(ax):
    shift = transforms.Affine2D().translate(0.5, 0)
    line, = ax.plot([0, 1], [0, 1], transform=shift + ax.transData)
    return line, {"shift": shift}


row("affine_matrix", _shifted, lambda s: s.shift.get_matrix(), setitem((0, 2), 0.9))
row("affine_translate", _shifted, lambda s: s.shift, lambda shift: shift.translate(0.4, 0),
    read=lambda s: s.shift.get_matrix())
row("affine_clear_rotate", _shifted, lambda s: s.shift,
    lambda shift: shift.clear().rotate_deg(30), read=lambda s: s.shift.get_matrix())
row("affine_mtx", _shifted, lambda s: s.shift._mtx, setitem((0, 2), 0.9))


def _two_lines(ax):
    ax.plot([0, 1])
    ax.plot([1, 0])
    return ax


row("list_ax_lines", _two_lines, lambda s: s.obj.lines, lambda lines: lines.__delitem__(0),
    read=lambda s: len(s.obj.lines))
row("list_ax_patches", lambda ax: (ax.bar([0, 1], [1, 2]), ax)[1], lambda s: s.obj.patches,
    lambda found: found.pop(), read=lambda s: len(s.obj.patches))
row("list_children", _plotted, lambda s: s.obj.get_children(), lambda found: found.clear(),
    read=lambda s: len(s.obj.get_children()) > 0)


def _legend(ax):
    ax.plot([0, 1], label="a")
    ax.plot([1, 0], label="b")
    return ax.legend()


row("list_legend_texts", _legend, lambda s: s.obj.get_texts(), lambda found: found.pop(),
    read=lambda s: len(s.obj.get_texts()))
row("list_xticklabels", _plotted, lambda s: s.obj.get_xticklabels(),
    lambda found: found.clear(), read=lambda s: len(s.obj.get_xticklabels()))
row("list_legend_handles", _legend, lambda s: s.obj.legend_handles, lambda found: found.pop(),
    read=lambda s: len(s.obj.legend_handles))


def _red(value):
    if isinstance(value, np.ndarray):
        value[1] = 1.0


row("line_marker_facecolor_array", lambda ax: ax.plot([0, 1], "o", mfc=np.array([1.0, 0, 0]))[0],
    lambda s: s.obj.get_markerfacecolor(), _red, read=lambda s: str(s.obj.get_markerfacecolor()))
row("errorbar_barlines_segments",
    lambda ax: ax.errorbar([0, 1], [0, 1], yerr=0.2).lines[2][0],
    lambda s: s.obj.get_segments()[0], setitem((1, 1), 2))
row("circle_center", lambda ax: ax.add_patch(patches.Circle((0.5, 0.5), 0.2)),
    lambda s: s.obj.get_center(), lambda center: None, read=lambda s: list(s.obj.get_center()))
row("stem_segments", lambda ax: ax.stem([0, 1, 2], [1, 2, 1]).stemlines,
    lambda s: s.obj.get_segments()[0], setitem((1, 1), 0))
row("contour_allsegs", lambda ax: ax.contour(*_field(), levels=3),
    lambda s: s.obj.allsegs[1][0], lambda segment: segment.__iadd__(0.5))

# Colormaps and norms.


def _mapped(cmap=None, norm=None, data=None):
    """A builder of an image of *data* mapped by *cmap* and *norm*, kept."""
    def make(ax):
        values = np.arange(12.0).reshape(3, 4) if data is None else data()
        used = plt.get_cmap("viridis") if cmap is None else cmap()
        image = ax.imshow(values, cmap=used, norm=None if norm is None else norm())
        return image, {"cmap": used}
    return make


def _lut(scene):
    cmap = scene.obj.get_cmap()
    cmap(0.5)
    return cmap._lut


row("cmap_lut", _mapped(), _lut, setitem((ALL, 0), 1.0))
row("cmap_set_bad", _mapped(data=lambda: np.ma.masked_less(np.arange(12.0).reshape(3, 4), 4)),
    lambda s: s.obj.get_cmap(), lambda cmap: cmap.set_bad("red"),
    read=lambda s: list(s.obj.get_cmap().get_bad()))
row("cmap_given_is_kept", _mapped(), lambda s: s.obj.get_cmap(), lambda cmap: cmap.set_over("k"),
    read=lambda s: list(s.obj.get_cmap().get_over()), same=lambda s: s.obj.get_cmap() is s.cmap)
row("norm_vmin", _mapped(), lambda s: s.obj.norm, lambda norm: setattr(norm, "vmin", 5.0),
    read=lambda s: [s.obj.norm.vmin, s.obj.norm.vmax, *s.obj.get_clim()])
row("norm_clip", _mapped(norm=lambda: colors.Normalize(2, 6)), lambda s: s.obj.norm,
    lambda norm: setattr(norm, "clip", True), read=lambda s: s.obj.norm.clip)
row("boundary_norm_boundaries",
    _mapped(norm=lambda: colors.BoundaryNorm(np.array([0.0, 3.0, 6.0, 12.0]), 256)),
    lambda s: s.obj.norm.boundaries, setitem(1, 9.0))
row("two_slope_vcenter", _mapped(norm=lambda: colors.TwoSlopeNorm(4.0, 0.0, 11.0)),
    lambda s: s.obj.norm, lambda norm: setattr(norm, "vcenter", 8.0),
    read=lambda s: s.obj.norm.vcenter)
row("listed_colormap_colors",
    _mapped(cmap=lambda: colors.ListedColormap([[1.0, 0, 0], [0, 1.0, 0], [0, 0, 1.0]])),
    lambda s: s.obj.get_cmap().colors, setitem(0, [0.0, 0.0, 0.0]))

# Tick helpers.


def _ticked(locator=None, formatter=None):
    """A builder of a plotted Axes whose x axis has the helpers given."""
    def make(ax):
        ax.plot([0, 10], [0, 1])
        if locator is not None:
            ax.xaxis.set_major_locator(locator())
        if formatter is not None:
            ax.xaxis.set_major_formatter(formatter())
        return ax
    return make


def _labels(scene):
    scene.fig.canvas.draw()
    return [label.get_text() for label in scene.obj.get_xticklabels()]


LOCATOR = (lambda s: s.obj.xaxis.get_major_locator())
FORMATTER = (lambda s: s.obj.xaxis.get_major_formatter())

row("locator_set_params", _ticked(), LOCATOR, lambda locator: locator.set_params(nbins=2),
    read=_labels, frame=False)
row("max_n_locator_nbins", _ticked(lambda: ticker.MaxNLocator(8)), LOCATOR,
    lambda locator: setattr(locator, "_nbins", 2), read=_labels, frame=False)
row("multiple_locator_set_params", _ticked(lambda: ticker.MultipleLocator(2)), LOCATOR,
    lambda locator: locator.set_params(base=5), read=_labels, frame=False)
row("fixed_locator_locs", _ticked(lambda: ticker.FixedLocator(np.array([1.0, 4.0, 8.0]))),
    lambda s: s.obj.xaxis.get_major_locator().locs, setitem(1, 6.0), read=_labels, frame=False)
row("fixed_formatter_seq",
    _ticked(lambda: ticker.FixedLocator([1, 4, 8]), lambda: ticker.FixedFormatter(["a", "b", "c"])),
    lambda s: s.obj.xaxis.get_major_formatter().seq, setitem(1, "changed"), read=_labels,
    frame=False)
row("format_str_formatter_fmt", _ticked(formatter=lambda: ticker.FormatStrFormatter("%d")),
    FORMATTER, lambda formatter: setattr(formatter, "fmt", "%.2f"), read=_labels, frame=False)
row("scalar_formatter_set_useoffset", _ticked(), FORMATTER,
    lambda formatter: formatter.set_powerlimits((0, 0)), read=_labels, frame=False)
row("ticklocs", _ticked(), lambda s: s.obj.xaxis.get_ticklocs(), setitem(0, 3.0), frame=False)

# Boxes.

row("viewlim_x0", _plotted, lambda s: s.obj.viewLim, lambda box: setattr(box, "x0", -3.0),
    read=lambda s: s.obj.get_xlim())
row("viewlim_intervalx", _plotted, lambda s: s.obj.viewLim.intervalx, setitem(1, 9.0),
    read=lambda s: s.obj.get_xlim())
row("datalim_intervaly", _plotted, lambda s: s.obj.dataLim.intervaly, setitem(1, 9.0),
    read=lambda s: s.obj.dataLim.get_points())
row("figure_bbox_inches_points", _plotted, lambda s: s.fig.bbox_inches.get_points(),
    setitem((1, 0), 4.0), read=lambda s: list(s.fig.get_size_inches()))

# The lists of Axes and figures.


def _full(ax):
    ax.plot([0, 1], label="a")
    ax.bar([0, 1], [1, 2])
    ax.scatter([0, 1], [1, 0])
    ax.imshow(np.arange(4.0).reshape(2, 2), extent=(0, 1, 0, 1))
    ax.text(1, 1, "text")
    ax.table([["a"]], loc="center")
    ax.add_artist(patches.Circle((2, 2), 0.5))
    ax.inset_axes([0.6, 0.6, 0.3, 0.3])
    ax.figure.text(0.1, 0.9, "note")
    ax.figure.legend()
    ax.figure.add_artist(patches.Circle((0.5, 0.5), 0.1))
    ax.figure.figimage(np.arange(4.0).reshape(2, 2))
    return ax


def _popped(name, label, of=lambda s: s.obj):
    """The row *label* popping the last item of the list *name* of an
    Axes (or of what *of* gives of the scene)."""
    row(label, _full, lambda s: getattr(of(s), name), lambda found: found.pop(),
        read=lambda s: len(getattr(of(s), name)))


for _name in ("lines", "patches", "collections", "images", "texts", "artists", "tables",
              "containers", "child_axes", "_children"):
    _popped(_name, f"list_{_name}_pop")
for _name in ("axes", "texts", "images", "artists", "legends"):
    _popped(_name, f"list_figure_{_name}_pop", of=lambda s: s.fig)


def _figure_text(found):
    found.append(pltrs.text.Text(0.5, 0.5, "appended"))
    found[-1].set_figure(plt.gcf())


def _figure_patch(found):
    fig = plt.gcf()
    found.append(patches.Rectangle((0.2, 0.2), 0.5, 0.5, transform=fig.transFigure, figure=fig))


def _figure_lines(found):
    fig = plt.gcf()
    found.extend([pltrs.lines.Line2D([0, 1], [0, 1], transform=fig.transFigure, figure=fig)])


row("list_figure_texts_append", _full, lambda s: s.fig.texts, _figure_text,
    read=lambda s: len(s.fig.texts))
row("list_figure_patches_append", _full, lambda s: s.fig.patches, _figure_patch,
    read=lambda s: len(s.fig.patches))
row("list_figure_lines_extend", _full, lambda s: s.fig.lines, _figure_lines,
    read=lambda s: len(s.fig.lines))
row("list_get_lines", _two_lines, lambda s: s.obj.get_lines(), lambda found: found.pop(),
    read=lambda s: len(s.obj.get_lines()))
row("list_get_axes", _full, lambda s: s.fig.get_axes(), lambda found: found.pop(),
    read=lambda s: len(s.fig.get_axes()))


def _sticky(scene):
    scene.obj.axes.autoscale_view()
    return scene.obj.axes.get_xlim()


row("sticky_edges_x", lambda ax: ax.plot([0, 1], [0, 1])[0], lambda s: s.obj.sticky_edges.x,
    lambda edges: edges.append(1.0), read=_sticky, frame=False)

# Containers, contour sets, legends, tables.

row("bar_container_patches", lambda ax: ax.bar([0, 1], [1, 2]), lambda s: s.obj.patches,
    lambda found: found.pop(), read=lambda s: len(s.obj.patches))
row("bar_container_datavalues", lambda ax: ax.bar([0, 1], [1, 2]), lambda s: s.obj.datavalues,
    setitem(0, 9), read=lambda s: [label.get_text() for label in s.ax.bar_label(s.obj)])
row("errorbar_container_children", lambda ax: ax.errorbar([0, 1], [0, 1], yerr=0.2),
    lambda s: s.obj.get_children(), lambda found: found.pop(),
    read=lambda s: len(s.obj.get_children()))
row("stem_container_stemlines", lambda ax: ax.stem([0, 1, 2], [1, 2, 1]),
    lambda s: s.obj.stemlines, lambda stems: stems.set_color("r"),
    read=lambda s: s.obj.stemlines.get_color())


def _contour(ax):
    return ax.contour(*_field(), levels=3)


def _labeled(ax):
    contour = _contour(ax)
    contour.clabel()
    return contour


row("contour_levels", _contour, lambda s: s.obj.levels, setitem(1, -0.9))
row("contour_layers", _contour, lambda s: s.obj.layers, setitem(1, -0.9))
row("contour_allkinds", _contour, lambda s: s.obj.allkinds[1][0], setitem(-1, 1))
row("contour_label_texts", _labeled, lambda s: s.obj.labelTexts, lambda found: found.pop(),
    read=lambda s: len(s.obj.labelTexts))
row("contour_label_cvalues", _labeled, lambda s: s.obj.labelCValues, setitem(0, 0.5))
row("contour_label_text_color", _labeled, lambda s: s.obj.labelTexts[0],
    lambda label: label.set_color("r"), read=lambda s: s.obj.labelTexts[0].get_color())
row("legend_texts_attribute", _legend, lambda s: s.obj.texts, lambda found: found.pop(),
    read=lambda s: len(s.obj.texts))
row("legend_text_color", _legend, lambda s: s.obj.get_texts()[0],
    lambda label: label.set_color("r"), read=lambda s: s.obj.get_texts()[0].get_color())
row("legend_handle_color", _legend, lambda s: s.obj.legend_handles[0],
    lambda handle: handle.set_color("r"), read=lambda s: s.obj.legend_handles[0].get_color())
row("legend_frame_facecolor", _legend, lambda s: s.obj.get_frame(),
    lambda frame: frame.set_facecolor("r"), read=lambda s: list(s.obj.get_frame().get_facecolor()))


def _table(ax):
    return ax.table([["a", "b"], ["c", "d"]], loc="center")


row("table_cells", _table, lambda s: s.obj._cells, lambda cells: cells.pop((0, 0)),
    read=lambda s: sorted(s.obj.get_celld()))
row("table_celld", _table, lambda s: s.obj.get_celld(), lambda cells: cells.pop((1, 1)),
    read=lambda s: sorted(s.obj.get_celld()))
row("table_cell_text", _table, lambda s: s.obj.get_celld()[0, 0].get_text(),
    lambda cell: cell.set_text("changed"), read=lambda s: s.obj[0, 0].get_text().get_text())

# Texts.


def _text(ax):
    return ax.text(1, 1, "text", bbox=dict(fc="y"))


row("text_fontproperties", _text, lambda s: s.obj.get_fontproperties(),
    lambda font: font.set_size(30), read=lambda s: s.obj.get_fontsize())
row("text_bbox_patch", _text, lambda s: s.obj.get_bbox_patch(),
    lambda box: box.set_facecolor("r"), read=lambda s: list(s.obj.get_bbox_patch().get_facecolor()))

# The lists of rcParams.


def _rc(key):
    """A reader of the rcParam *key* of a scene."""
    return lambda s: pltrs.rcParams[key]


def _titled(ax):
    ax.set_title("title")
    return ax


row("rc_font_sans_serif", _titled, _rc("font.sans-serif"),
    lambda names: names.insert(0, "DejaVu Serif"))
row("rc_font_family", _titled, _rc("font.family"), setitem(0, "serif"))
row("rc_figure_figsize", _titled, _rc("figure.figsize"), setitem(0, 3.0),
    read=lambda s: list(plt.figure().get_size_inches()))
row("rc_dashed_pattern", lambda ax: ax.plot([0, 3], [0, 3], "--")[0], _rc("lines.dashed_pattern"),
    setitem(0, 1.0))
