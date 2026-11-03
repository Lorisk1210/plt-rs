"""
The last named gaps of pltrs (Python): overrides the drawing honours
(Python legend handlers, ``fig.artists``, a ``draw`` given to an instance,
table cells subclassing ``Cell.draw``), single ticks of 3-D axes and the
two sides of 2-D tick labels, annotation coordinates evaluated when drawn,
table cells as rectangles, unoriented triangulations, parasites with a
non-affine aux transform and pickling figures with projections and axes
locators of the program's. Each ``scenario_*`` returns what its test
compares; the expected values were recorded from matplotlib 3.11.2
running the same scenarios.
"""

import inspect
import io
import pickle
import sys
import types

import numpy as np

import pltrs.pyplot as plt
from pltrs import _tri, legend_handler, lines, patches, table, text, transforms
from pltrs.axes import Axes
from pltrs.collections import PathCollection
from pltrs.projections import register_projection
from pltrs.projections.polar import PolarAxes
from pltrs.mpl_toolkits.axes_grid1 import parasite_axes

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def _pixels(fig):
    """The figure drawn on its canvas, as an (H, W, 4) uint8 array."""
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba()).copy()


def _count(img, rgb):
    """How many pixels of *img* have the colour *rgb* (0-255 channels,
    within 40)."""
    return int((np.abs(img[..., :3].astype(int) - rgb) <= 40).all(axis=-1).sum())


RED, GREEN, BLUE = (255, 0, 0), (0, 128, 0), (0, 0, 255)


class AnyObject:
    """A legend handle of the program's."""


class AnyObjectHandler:
    """The legend guide's handler drawing a red rectangle."""

    def legend_artist(self, legend, orig_handle, fontsize, handlebox):
        x0, y0 = handlebox.xdescent, handlebox.ydescent
        patch = patches.Rectangle([x0, y0], handlebox.width, handlebox.height,
                                  facecolor="red", edgecolor="black", lw=3,
                                  transform=handlebox.get_transform())
        handlebox.add_artist(patch)
        return patch


class HandlerEllipse(legend_handler.HandlerPatch):
    """The legend guide's handler drawing an ellipse."""

    def create_artists(self, legend, orig_handle, xdescent, ydescent, width,
                       height, fontsize, trans):
        center = 0.5 * width - 0.5 * xdescent, 0.5 * height - 0.5 * ydescent
        p = patches.Ellipse(xy=center, width=width + xdescent,
                            height=height + ydescent)
        self.update_prop(p, orig_handle, legend)
        p.set_transform(trans)
        return [p]


def scenario_legend_handlers():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    leg = ax.legend([AnyObject()], ["first"],
                    handler_map={AnyObject: AnyObjectHandler()})
    first = (type(leg.legend_handles[0]).__name__, _count(_pixels(fig), RED))
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    c = patches.Circle((0.5, 0.5), 0.25, facecolor="green", edgecolor="red",
                       linewidth=3)
    ax.add_patch(c)
    leg = ax.legend([c], ["ellipse"],
                    handler_map={patches.Circle: HandlerEllipse()})
    return first, (type(leg.legend_handles[0]).__name__,
                   _count(_pixels(fig), GREEN))


def scenario_figure_lists():
    fig = plt.figure(figsize=(3, 2), dpi=100)
    fig.artists.append(patches.Rectangle((0.1, 0.1), 0.3, 0.3, color="red",
                                         transform=fig.transFigure))
    fig.patches.append(patches.Rectangle((0.6, 0.1), 0.3, 0.3, color="blue",
                                         transform=fig.transFigure))
    label = text.Text(0.5, 0.8, "x", transform=fig.transFigure)
    label.set_figure(fig)
    fig.texts.append(label)
    img = _pixels(fig)
    return len(fig.artists), len(fig.patches), _count(img, RED), _count(img, BLUE)


def scenario_instance_draw():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=100)
    points = ax.scatter([0, 1], [0, 1], color="blue")
    calls = []
    original = PathCollection.draw

    def draw(self, renderer):
        calls.append(1)
        self.set_offsets([[0.5, 0.5]])
        original(self, renderer)

    points.draw = types.MethodType(draw, points)
    _pixels(fig)
    return len(calls), points.get_offsets().tolist()


def scenario_tick_label_sides():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=100)
    ax.tick_params(labeltop=True)
    for tick in ax.xaxis.get_major_ticks():
        tick.label2.set_color("red")
    img = _pixels(fig)
    return _count(img[: img.shape[0] // 2], RED), _count(img[img.shape[0] // 2:], RED)


def scenario_text_zorder():
    class Front(text.Text):
        zorder = 7

    return (float(Front(0, 0, "a").get_zorder()),
            float(text.Text(0, 0, "b").get_zorder()))


def scenario_annotation_coords():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    found = [
        ax.annotate("a", (0.25, 0.75), xycoords=lambda r: ax.bbox),
        ax.annotate("b", (0, 0), xycoords=lambda r: ax.transData),
        ax.annotate("c", (0.5, 0.5), xycoords="figure fraction",
                    textcoords=text.OffsetFrom(
                        transforms.Bbox.from_bounds(100, 100, 50, 20), (1, 0)),
                    xytext=(10, 5)),
        ax.annotate("d", (0.5, 0.5), xycoords="figure fraction",
                    textcoords=text.OffsetFrom(ax.transAxes, (0, 1),
                                               unit="pixels"),
                    xytext=(3, -20)),
    ]
    ax.set_xlim(-2, 8)
    fig.canvas.draw()
    return [[round(float(v), 3) for v in a.get_window_extent().bounds[:2]]
            for a in found]


def scenario_ticks_3d():
    fig = plt.figure(figsize=(4, 3), dpi=100)
    ax = fig.add_subplot(projection="3d")
    ax.plot([0, 1], [0, 1], [0, 1], color="black")
    ax.set_xticks([0, 0.5, 1])
    labels = ax.xaxis.set_ticklabels(["a", "b", "c"], color="red")
    ax.yaxis.get_major_ticks()[1].label1.set_color("green")
    ax.zaxis.get_major_ticks()[2].tick1line.set_color("blue")
    img = _pixels(fig)
    return (len(labels), _count(img, RED) > 0, _count(img, GREEN) > 0,
            ax.yaxis.get_major_ticks()[1].label1.get_color())


def scenario_table_cells():
    params = sorted(inspect.signature(table.Cell.set).parameters)
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    tab = ax.table(cellText=[["a", "b"]], loc="center")
    tab[0, 0].set(facecolor="red", hatch="//")
    tab[0, 1].set_text_props(color="blue", fontweight="bold")
    tab.add_cell(1, 0, 0.3, 0.1, fill=False, edgecolor="green")
    img = _pixels(fig)
    return (len(params), isinstance(tab[0, 0], patches.Rectangle),
            tab[0, 0].get_hatch(), _count(img, RED), _count(img, BLUE) > 0)


def _cross(cell, renderer):
    """Draw a red line across *cell*, from corner to corner."""
    x0, y0 = cell.get_xy()
    line = lines.Line2D([x0, x0 + cell.get_width()], [y0, y0 + cell.get_height()],
                        transform=cell.get_data_transform(), color="red")
    line.set_figure(cell.get_figure(root=True))
    line.draw(renderer)


class Crossed(table.Cell):
    """A cell crossed by a red line from corner to corner."""

    def draw(self, renderer):
        super().draw(renderer)
        _cross(self, renderer)


def _draw_crossed(cell, renderer):
    """The ``draw`` given to a cell: the cell, crossed."""
    table.Cell.draw(cell, renderer)
    _cross(cell, renderer)


def scenario_cell_draw_override():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    tab = ax.table(cellText=[["x", "y"]], loc="center")
    tab[1, 0] = Crossed((0, 0), 0.4, 0.1, text="crossed")
    return _count(_pixels(fig), RED)


def scenario_cell_given_a_draw():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    tab = ax.table(cellText=[["x", "y"]], loc="center")
    cell = tab[0, 1]
    cell.draw = types.MethodType(_draw_crossed, cell)
    return _count(_pixels(fig), RED)


def scenario_unoriented_triangulation():
    x = np.array([0.0, 1.0, 0.0, 1.0, 2.0])
    y = np.array([0.0, 0.0, 1.0, 1.0, 0.5])
    triangles = np.array([[0, 2, 1], [1, 2, 3], [1, 4, 3]])
    return [_tri.Triangulation(x, y, triangles, (), (), (), correct)
            .get_neighbors().tolist() for correct in (True, False)]


class Squash(transforms.Transform):
    """A transform of the program's squashing y by its square root."""

    input_dims = output_dims = 2

    def transform_non_affine(self, values):
        values = np.asarray(values, dtype=float)
        return np.column_stack([values[:, 0], np.sqrt(np.abs(values[:, 1]))])

    def inverted(self):
        return Unsquash()


class Unsquash(transforms.Transform):
    """The inverse of `Squash`."""

    input_dims = output_dims = 2

    def transform_non_affine(self, values):
        values = np.asarray(values, dtype=float)
        return np.column_stack([values[:, 0], values[:, 1] ** 2])

    def inverted(self):
        return Squash()


def scenario_non_affine_parasite():
    fig = plt.figure(figsize=(4, 3), dpi=100)
    host = fig.add_subplot(axes_class=parasite_axes.HostAxes)
    host.set_xlim(0, 10)
    host.set_ylim(0, 4)
    par = host.get_aux_axes(Squash())
    par.plot(np.linspace(0, 10, 50), np.linspace(0, 16, 50), color="red")
    return _count(_pixels(fig), RED)


class Locator:
    """An axes locator of the program's."""

    def __call__(self, ax, renderer):
        return transforms.Bbox.from_bounds(0.55, 0.55, 0.35, 0.3)


class ShiftedAxes(Axes):
    """A projection of the program's."""

    name = "shifted_gaps"


class SquashedPolar(PolarAxes):
    """A polar projection of the program's."""

    name = "squashed_polar_gaps"


register_projection(ShiftedAxes)
register_projection(SquashedPolar)
# Pickling finds the classes by the name of this module.
sys.modules.setdefault("test_named_gaps", sys.modules[__name__])


def scenario_pickled_projections():
    fig = plt.figure(figsize=(4, 3), dpi=100)
    fig.add_subplot(121, projection="shifted_gaps").plot([0, 1, 2], [0, 1, 4])
    fig.add_subplot(122, projection="squashed_polar_gaps").plot([0, 1, 2], [0.2, 1, 0.4])
    located = fig.add_axes([0.6, 0.6, 0.3, 0.3])
    located.set_axes_locator(Locator())
    located.plot([1, 2], [2, 1], color="red")
    before = _pixels(fig)
    again = pickle.loads(pickle.dumps(fig))
    after = _pixels(again)
    return (bool((before == after).all()),
            [type(ax).__name__ for ax in again.axes],
            [round(float(v), 3) for v in again.axes[2].get_position().bounds])


def scenario_program_renderers():
    from pltrs.backends.backend_svg import RendererSVG  # noqa: PLC0415
    from pltrs.path import Path
    buf = io.StringIO()
    renderer = RendererSVG(144, 72, buf)
    gc = renderer.new_gc()
    gc.set_foreground("blue")
    gc.set_linewidth(3)
    renderer.draw_path(gc, Path([[10, 10], [100, 60]]), transforms.Affine2D())
    gc.restore()
    renderer.finalize()
    svg = buf.getvalue()
    calls = []

    class Counting(RendererSVG):
        def start_filter(self):
            calls.append("start")
            super().start_filter()

        def stop_filter(self, filter_func):
            calls.append("stop")
            super().stop_filter(filter_func)

    fig = plt.figure(figsize=(2, 1))
    line = lines.Line2D([0, 1], [0, 1], transform=fig.transFigure, figure=fig)
    line.set_agg_filter(lambda image, dpi: (image, 0, 0))
    line.draw(Counting(144, 72, io.StringIO()))
    return (svg.rstrip().endswith("</svg>"), "stroke: #0000ff" in svg, calls)


def custom_box(x0, y0, width, height, mutation_size):
    """The box style function of matplotlib's custom box style example."""
    pad = mutation_size * 0.3
    width, height = width + 2 * pad, height + 2 * pad
    x0, y0 = x0 - pad, y0 - pad
    x1, y1 = x0 + width, y0 + height
    from pltrs.path import Path
    return Path([(x0, y0), (x1, y0), (x1, y1), (x0, y1),
                 (x0 - pad, (y0 + y1) / 2), (x0, y0), (x0, y0)], closed=True)


def scenario_program_box_style():
    fig, ax = plt.subplots(figsize=(3, 2), dpi=100)
    ax.text(0.5, 0.5, "box", size=20, ha="center", va="center",
            bbox=dict(boxstyle=custom_box, facecolor="red"))
    return _count(_pixels(fig), RED)


def scenario_axis_3d_methods():
    fig = plt.figure(figsize=(6.4, 4.8), dpi=100)
    ax = fig.add_subplot(projection="3d")
    ax.plot([0.5, 1], [0, 1], [0, 1])
    fig.canvas.draw()
    found = [(axis.get_tick_space(), axis.get_tick_padding(), axis.get_in_layout(),
              axis.get_remove_overlapping_locs(), axis.get_units(),
              axis.get_converter())
             for axis in (ax.xaxis, ax.zaxis)]
    ax.zaxis.set_remove_overlapping_locs(False)
    return found, ax.zaxis.get_remove_overlapping_locs()


def scenario_legend_attribute():
    fig, ax = plt.subplots()
    ax.plot([0, 1], label="a")
    first = ax.legend()
    second = ax.legend(loc="lower right")
    ax.legend_ = first
    restored = ax.get_legend() is first
    ax.legend_ = None
    return restored, second.figure is None or ax.get_legend() is None


def test_python_legend_handlers_draw_their_artists():
    assert scenario_legend_handlers() == (("Rectangle", 134), ("Ellipse", 13133))


def test_figure_lists_draw_what_is_appended():
    assert scenario_figure_lists() == (1, 1, 5551, 5551)


def test_a_draw_given_to_an_instance_is_called():
    assert scenario_instance_draw() == (1, [[0.5, 0.5]])


def test_tick_labels_have_properties_per_side():
    assert scenario_tick_label_sides() == (213, 0)


def test_a_zorder_class_attribute_of_a_text_subclass():
    assert scenario_text_zorder() == (7.0, 3.0)


def test_annotation_coordinates_evaluated_when_drawn():
    assert scenario_annotation_coords() == [
        [127.5, 202.913], [112.0, 29.663], [163.889, 103.608], [53.0, 240.663]]


def test_single_ticks_of_3d_axes():
    assert scenario_ticks_3d() == (3, True, True, "green")


def test_table_cells_are_rectangles():
    assert scenario_table_cells() == (42, True, "//", 1580, True)


def test_cell_draw_overrides_draw_the_table():
    assert scenario_cell_draw_override() == 182


def test_a_draw_given_to_a_cell_draws_the_table():
    assert scenario_cell_given_a_draw() == 222


def test_unoriented_triangulation():
    assert scenario_unoriented_triangulation() == [
        [[-1, 1, -1], [2, -1, 0], [-1, -1, 1]],
        [[-1, 1, -1], [0, -1, -1], [-1, -1, -1]]]


def test_parasite_with_a_non_affine_aux_transform():
    assert scenario_non_affine_parasite() == 585


def test_pickling_projections_and_locators_of_the_program():
    assert scenario_pickled_projections() == (
        True, ["ShiftedAxes", "SquashedPolar", "Axes"], [0.55, 0.55, 0.35, 0.3])


def test_renderers_made_by_the_program():
    assert scenario_program_renderers() == (True, True, ["start", "stop"])


def test_box_style_functions_of_the_program():
    assert scenario_program_box_style() == 2561


def test_methods_of_3d_axes():
    assert scenario_axis_3d_methods() == (
        [(8, 3.5, True, True, None, None)] * 2, False)


def test_legend_attribute_takes_a_legend_of_the_axes():
    assert scenario_legend_attribute() == (True, True)


def test_figure_saves_to_memory_with_cells():
    fig, ax = plt.subplots()
    ax.table(cellText=[["a"]]).add_cell(1, 0, 0.2, 0.1, fill=False)
    fig.savefig(io.BytesIO(), format="svg")
