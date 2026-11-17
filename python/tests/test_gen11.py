"""
What GEN11 found pltrs calling otherwise than matplotlib 3.11 in a
program's own code: renderers of the program's (texts handed over with
`draw_text` and measured by the renderer, collections probed for their
hatch colours, the questions about images, groups, the gid in the
context), methods replaced on `RendererAgg`, print methods run dry
before a tight print, `axes_locator` callables after a layout, offset
callables, and a table cell's text group. Every expected value was
recorded from matplotlib.
"""

import io
import re

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import offsetbox
from pltrs.backend_bases import FigureCanvasBase, GraphicsContextBase, RendererBase
from pltrs.backends.backend_agg import RendererAgg
from pltrs.backends.backend_mixed import MixedModeRenderer
from pltrs.transforms import Bbox


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


class _Recording(RendererBase):
    """A text backend: its own text metrics, recording what it draws."""

    def __init__(self, dpi, width, height):
        super().__init__()
        self.dpi, self.size, self.calls = dpi, (width, height), []

    def get_canvas_width_height(self):
        return self.size

    def points_to_pixels(self, points):
        return points * self.dpi / 72

    def flipy(self):
        return True

    def get_text_width_height_descent(self, s, prop, ismath):
        self.calls.append(("measure", s))
        return len(s) * prop.get_size_in_points() * 0.6, prop.get_size_in_points(), 2.0

    def draw_path(self, gc, path, transform, rgbFace=None):
        self.calls.append(("path", gc.get_gid()))

    def draw_text(self, gc, x, y, s, prop, angle, ismath=False, mtext=None):
        self.calls.append(("text", round(x, 2), round(y, 2), s, prop.get_size_in_points(),
                           angle, ismath, type(mtext).__name__))

    def draw_image(self, gc, x, y, im):
        self.calls.append(("image", gc.get_gid()))

    def draw_path_collection(self, gc, master_transform, paths, all_transforms, offsets,
                             offset_trans, facecolors, edgecolors, linewidths, linestyles,
                             antialiaseds, urls, offset_position):
        # Written before matplotlib 3.10: no hatch colours.
        self.calls.append(("collection", len(paths)))


def _drawn(fig, renderer=None):
    renderer = renderer or _Recording(fig.dpi, *fig.bbox.size)
    fig.draw(renderer)
    return renderer.calls


def test_a_text_reaches_a_renderer_of_the_program_through_draw_text():
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.set_axis_off()
    ax.text(0.1, 0.5, "plain")
    ax.text(0.5, 0.2, "one\ntwo", rotation=30, fontsize=12)
    ax.text(0.6, 0.8, r"$x^2$")
    texts = [call for call in _drawn(fig) if call[0] != "path"]
    assert texts == [
        ("measure", "plain"), ("text", 60.75, 101.0, "plain", 10.0, 0.0, False, "Text"),
        ("measure", "one"), ("measure", "two"),
        ("text", 160.92, 130.64, "one", 12.0, 30.0, False, "NoneType"),
        ("text", 170.92, 147.96, "two", 12.0, 30.0, False, "NoneType"),
        ("measure", "$x^2$"), ("text", 177.0, 54.8, "$x^2$", 10.0, 0.0, True, "Text")]


def test_tick_labels_reach_the_renderer_with_their_tick_label():
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.plot([0, 1], [0, 1])
    texts = [call for call in _drawn(fig) if call[0] == "text"]
    # Laid out by the renderer's own metrics.
    assert texts[0] == ("text", 36.07, 198.27, "0.00", 10.0, 0.0, False, "Text")
    assert len(texts) == 10


def test_a_renderer_without_hatch_colours_draws_collections():
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.set_axis_off()
    ax.scatter([0, 1, 2], [0, 1, 0], s=[10, 20, 30])
    assert [call for call in _drawn(fig) if call[0] == "collection"] == [("collection", 1)]


def test_only_images_hand_their_gid_in_the_context():
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.set_axis_off()
    ax.plot([0, 1], gid="line")
    ax.imshow(np.arange(4.).reshape(2, 2), gid="image", extent=(0, 1, 0, 1))
    calls = _drawn(fig)
    assert ("path", None) in calls and ("path", "line") not in calls
    assert ("image", "image") in calls


def test_a_mixed_mode_renderer_hands_texts_to_its_vector_renderer():
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.set_axis_off()
    ax.text(0.1, 0.5, "mixed")
    vector = _Recording(72, *fig.get_size_inches() * 72)
    fig.draw(MixedModeRenderer(fig, *fig.get_size_inches(), 72, vector))
    assert [call for call in vector.calls if call[0] == "text"] == [
        ("text", 60.75, 45.0, "mixed", 10.0, 0.0, False, "Text")]


def test_a_renderer_that_raised_is_called_no_more():
    class Failing(_Recording):
        def draw_path(self, gc, path, transform, rgbFace=None):
            raise ValueError("refused")

        def close_group(self, s):
            self.calls.append(("close", s))

    fig, ax = plt.subplots(figsize=(3, 2))
    renderer = Failing(fig.dpi, *fig.bbox.size)
    with pytest.raises(ValueError, match="refused"):
        fig.draw(renderer)
    assert ("close", "figure") not in renderer.calls


def test_methods_replaced_on_the_agg_renderer_draw_as_matplotlibs():
    fig, ax = plt.subplots(figsize=(3, 3), dpi=40)
    ax.plot([0, 1], [0, 1])
    ax.set_title("t")
    counts = {}
    originals = {name: getattr(RendererAgg, name)
                 for name in ("draw_path", "draw_text", "draw_markers")}

    def counting(name):
        def method(self, *args, **kwargs):
            counts[name] = counts.get(name, 0) + 1
            return originals[name](self, *args, **kwargs)
        return method

    for name in originals:
        setattr(RendererAgg, name, counting(name))
    try:
        fig.canvas.draw()
    finally:
        for name, method in originals.items():
            setattr(RendererAgg, name, method)
    # matplotlib binds draw_markers on each renderer: replacing it on the
    # class draws nothing through it.
    assert counts == {"draw_path": 7, "draw_text": 12}
    plain = np.asarray(fig.canvas.buffer_rgba()).copy()
    fig.canvas.draw()
    assert np.array_equal(plain, np.asarray(fig.canvas.buffer_rgba()))


class _PrintCanvas(FigureCanvasBase):
    """A canvas of the program's whose print method draws with a renderer
    of its own."""

    printed = []

    def print_rec(self, filename, **kwargs):
        type(self).printed.append(sorted(kwargs))
        self.figure.draw(_Recording(self.figure.dpi, *self.figure.bbox.size))


@pytest.mark.parametrize("kwargs, printed", [
    ({}, [["bbox_inches_restore", "edgecolor", "facecolor", "orientation"]]),
    ({"bbox_inches": "tight"},
     [["orientation"], ["bbox_inches_restore", "edgecolor", "facecolor", "orientation"]]),
])
def test_a_print_method_of_the_program_runs_dry_before_a_tight_print(kwargs, printed):
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.plot([0, 1])
    _PrintCanvas(fig)
    _PrintCanvas.printed = []
    fig.savefig(io.BytesIO(), format="rec", **kwargs)
    assert _PrintCanvas.printed == printed


def test_a_print_method_never_drawing_fails_a_tight_print():
    class Silent(FigureCanvasBase):
        def print_rec(self, filename, **kwargs):
            pass

    fig, ax = plt.subplots()
    Silent(fig)
    with pytest.raises(RuntimeError, match="did not call Figure.draw"):
        fig.savefig(io.BytesIO(), format="rec", bbox_inches="tight")


def test_an_axes_locator_places_its_axes_after_the_layout():
    calls = []

    def relative(ax, renderer):
        p = parent.get_position()
        calls.append(1)
        return Bbox.from_bounds(p.x0 + 0.6 * p.width, p.y0 + 0.6 * p.height,
                                0.35 * p.width, 0.35 * p.height)

    fig, parent = plt.subplots(figsize=(5, 4), layout="constrained")
    parent.set_title("parent")
    inset = fig.add_axes([0, 0, 1, 1])
    inset.set_axes_locator(relative)
    fig.canvas.draw()
    assert inset.get_position().bounds == pytest.approx((0.6105, 0.59, 0.3143, 0.3037), abs=1e-4)
    fig.set_size_inches(8, 3)
    fig.canvas.draw()
    assert inset.get_position().bounds == pytest.approx((0.6066, 0.5867, 0.3277, 0.2882), abs=1e-4)
    assert len(calls) == 4


def test_an_offset_callable_is_asked_where_matplotlib_asks_it():
    fig, ax = plt.subplots(figsize=(4, 3))
    box = offsetbox.VPacker(children=[offsetbox.TextArea("corner"), offsetbox.TextArea("box")],
                            align="right", pad=3, sep=1)
    calls = []

    def place(width, height, xdescent, ydescent, renderer):
        calls.append(1)
        return ax.bbox.x1 - width - 5 + xdescent, ax.bbox.y0 + 5 + ydescent

    box.set_offset(place)
    box.set_figure(fig)
    fig.artists.append(box)
    assert calls == []
    renderer = fig.canvas.get_renderer()
    assert fig.get_tightbbox(renderer).bounds == pytest.approx((0.18, 0.09, 3.53, 2.6), abs=0.01)
    calls.clear()
    fig.canvas.draw()
    assert len(calls) == 1


def test_a_table_cell_draws_its_text_in_a_group():
    fig, ax = plt.subplots()
    ax.set_axis_off()
    ax.table([["a", "b"]], loc="center")
    out = io.StringIO()
    with pltrs.rc_context({"svg.hashsalt": "x"}):
        fig.savefig(out, format="svg")
    assert re.findall(r'<g id="(\w+?)_\d+"', out.getvalue()) == [
        "figure", "patch", "axes", "table", "patch", "text", "patch", "text"]


def test_a_package_class_whose_module_left_sys_modules_is_not_the_programs(monkeypatch):
    # pytester's in-process runs take the modules they imported out of
    # sys.modules again, and a renderer class made during one lives on: it
    # measures as the package's renderers do, not through itself.
    import sys

    from pltrs import _program_classes, _text_metrics
    from pltrs.backends import backend_agg

    class Later(backend_agg.RendererAgg):
        pass

    monkeypatch.delitem(sys.modules, backend_agg.__name__)
    assert not _program_classes.takes_from_program(Later, "get_text_width_height_descent")
    assert not _text_metrics._measures_itself(Later)
