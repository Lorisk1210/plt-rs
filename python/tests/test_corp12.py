"""
What the twelfth round over the corpora (``cargo xtask diff --corpus``)
showed pltrs doing otherwise than matplotlib 3.11, reduced to matplotlib's
API: clip boxes of every kind (scikit-learn's ``fig.bbox``), a hidden
suptitle in a tight layout (python-control), axes placed by float32
rectangles (mplsoccer), texts made by a draw callback in a tight print
(python-ternary), legends whose figure a program unset, offset-box trees
with hidden or moved boxes (marsilea and legendkit), arrows between
complex points (python-control's phase plots), 3-D tick labels on their
second side (the what's new of 3.4), legends of colour-mapped collections
made before a draw (handson-ml3) and text boxes made under ``plt.xkcd``.
Every expected value was recorded from matplotlib.
"""

import io
import warnings

import numpy as np
import pytest
from PIL import Image

import pltrs
import pltrs.pyplot as plt
from pltrs.offsetbox import AnchoredOffsetbox, DrawingArea, HPacker, TextArea, VPacker
from pltrs.patches import Circle, FancyArrowPatch, Rectangle
from pltrs.transforms import Bbox, TransformedBbox


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def png(fig, **kwargs):
    """The figure printed to PNG, as an RGB array."""
    buf = io.BytesIO()
    fig.savefig(buf, format="png", **kwargs)
    return np.asarray(Image.open(buf).convert("RGB"))


def red_box(image):
    """The box of the red pixels of *image* and their number."""
    ys, xs = np.nonzero((image[..., 0] > 200) & (image[..., 1] < 60) & (image[..., 2] < 60))
    return [int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max()), int(len(xs))]


@pytest.mark.parametrize("kind, drawn, tight", [
    ("figbbox", [0, 0, 218, 145, 30766], [0, 10, 149, 109, 14214]),
    ("axes_half", [28, 19, 111, 129, 9324], [21, 8, 77, 83, 4332]),
    ("pixels", [30, 56, 119, 125, 6300], [30, 11, 119, 80, 6300]),
    ("box_and_path", [45, 29, 111, 117, 4723], [32, 15, 77, 75, 2216]),
])
def test_every_kind_of_clip_box_clips_drawn_and_printed_tight(kind, drawn, tight):
    fig, ax = plt.subplots(figsize=(3, 2), dpi=73)
    patch = ax.add_patch(Rectangle((-1, -1), 3, 3, color="red"))
    if kind == "figbbox":
        patch.set_clip_box(fig.bbox)
    elif kind == "axes_half":
        patch.set_clip_box(TransformedBbox(Bbox([[0, 0], [0.5, 1]]), ax.transAxes))
    elif kind == "pixels":
        patch.set_clip_box(Bbox([[30, 20], [120, 90]]))
    else:
        # The box and the path both clip.
        patch.set_clip_box(TransformedBbox(Bbox([[0, 0], [0.5, 1]]), ax.transAxes))
        patch.set_clip_path(Circle((0.5, 0.5), 0.4, transform=ax.transAxes))
    assert red_box(png(fig)) == drawn
    assert red_box(png(fig, bbox_inches="tight", dpi=50)) == tight


def test_tight_layout_measures_a_hidden_suptitle_as_the_unit_box():
    fig, ax = plt.subplots()
    fig.suptitle("hidden").set_visible(False)
    fig.tight_layout()
    assert [round(v, 4) for v in ax.get_position().bounds] == [0.073, 0.0804, 0.8864, 0.844]


def test_add_axes_takes_a_float32_rect_through_its_bbox():
    fig = plt.figure()
    rect = tuple(np.float32(v) for v in (0.0287, 0.885, 0.9426, 0.1))
    ax = fig.add_axes(rect)
    # The right and top are sums in float32, as matplotlib's from_bounds.
    assert [float(v) for v in ax.get_position().bounds] == [
        0.028699999675154686, 0.8849999904632568, 0.9426000062376261, 0.10000002384185791]


def test_a_tight_print_takes_in_what_a_draw_callback_made():
    fig, ax = plt.subplots(figsize=(3, 2))
    made = []

    def on_draw(event):
        for text in made:
            text.remove()
        made.clear()
        made.append(ax.text(0.5, 1.6, "made at draw", transform=ax.transAxes))

    fig.canvas.mpl_connect("draw_event", on_draw)
    assert list(png(fig, bbox_inches="tight", dpi=50).shape[:2]) == [151, 146]


def test_a_legend_whose_figure_was_unset_keeps_its_children():
    fig, ax = plt.subplots()
    ax.plot([1, 2], label="a")
    legend = ax.legend(title="T")
    legend.remove()
    # As marsilea detaches the legends it stacks.
    legend._parent_figure = None
    legend.figure = None
    box, frame = legend.get_children()
    assert (type(box).__name__, type(frame).__name__) == ("VPacker", "FancyBboxPatch")
    assert box.figure is fig and frame.figure is fig


def test_an_offset_box_tree_with_a_hidden_box_is_added_and_shown_later():
    fig, ax = plt.subplots(figsize=(3, 2))
    first, hidden, third = TextArea("first"), TextArea("hidden"), TextArea("third")
    hidden.set_visible(False)
    ax.add_artist(AnchoredOffsetbox(
        "center", child=VPacker(children=[first, hidden, third], pad=0, sep=2)))
    renderer = fig.canvas.get_renderer()
    assert [round(v, 2) for v in third._text.get_window_extent(renderer).bounds] == [
        0.0, -3.34, 32.75, 14.34]
    hidden.set_visible(True)
    fig.canvas.draw()
    assert [round(v, 2) for v in hidden._text.get_window_extent(renderer).bounds] == [
        129.25, 91.83, 49.0, 14.34]


class SelfDrawn(DrawingArea):
    """A drawing area drawing its children itself (legendkit's colour
    bars)."""

    def draw(self, renderer):
        self.dpi_transform.clear()
        self.dpi_transform.scale(renderer.points_to_pixels(1.0))
        for child in self._children:
            child.draw(renderer)
        self.stale = False


def test_a_self_drawn_box_moved_out_of_a_removed_box_draws_where_it_went():
    fig, ax = plt.subplots(figsize=(3, 2))
    area = SelfDrawn(30, 40)
    area.add_artist(Rectangle((0, 0), 30, 40, color="red"))
    first = AnchoredOffsetbox("upper left", child=area, frameon=False)
    ax.add_artist(first)
    first.remove()
    stack = AnchoredOffsetbox("center", child=HPacker(
        children=[TextArea("legend"), area], pad=0, sep=5), frameon=False)
    other = fig.add_axes([0, 0, 1, 1])
    other.set_axis_off()
    other.add_artist(stack)
    assert red_box(png(fig, dpi=50)) == [77, 35, 98, 63, 634]


def test_an_arrow_between_complex_points_warns_where_it_maps_them():
    fig, ax = plt.subplots()
    counts = []

    def count(f):
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            result = f()
        counts.append(sum(issubclass(w.category, np.exceptions.ComplexWarning)
                          for w in caught))
        return result

    x = np.array([0, 1], complex)
    y = np.array([0, 1], complex)
    arrow = count(lambda: FancyArrowPatch((x[0], y[0]), (x[1], y[1]), arrowstyle="-|>"))
    count(lambda: ax.add_patch(arrow))
    count(lambda: fig.canvas.draw())
    count(lambda: arrow.get_path())
    # None when made; once when added, drawn and asked for its path.
    assert counts == [0, 1, 1, 1]


def test_3d_tick_labels_on_their_second_side_are_drawn_and_measured():
    with pltrs.rc_context({"xtick.labelbottom": False, "xtick.labeltop": True,
                           "ytick.labelleft": False, "ytick.labelright": True}):
        fig = plt.figure(figsize=(4, 4))
        ax = fig.add_subplot(projection="3d")
        ax.plot([0, 1], [0, 1], [0, 1])
        fig.canvas.draw()
        renderer = fig.canvas.get_renderer()
        assert [round(v, 1) for v in ax.xaxis.get_tightbbox(renderer).bounds] == [
            63.3, 44.1, 184.0, 75.8]
        assert [round(v, 1) for v in ax.get_tightbbox(renderer).bounds] == [
            51.0, 44.0, 331.3, 308.0]


def test_legends_of_mapped_collections_made_before_a_draw():
    fig, ax = plt.subplots()
    X, Y = np.meshgrid(np.linspace(0, 1, 5), np.linspace(0, 1, 4))
    pc = ax.pcolor(X, Y, X * Y, shading="auto")
    mesh = ax.pcolormesh(X, Y, X * Y, label="mesh")
    line, = ax.plot([0, 1], label="line")
    with warnings.catch_warnings(record=True) as given:
        warnings.simplefilter("always")
        legend = ax.legend([pc, mesh, line], ["pc", "mesh", "line"])
    handles = legend.legend_handles
    # The colours the collection has before its first draw.
    assert [round(float(v), 3) for v in handles[0].get_facecolor()] == [
        0.122, 0.467, 0.706, 1.0]
    # No handler draws a QuadMesh: None stands for it.
    assert [type(h).__name__ for h in handles] == ["Rectangle", "NoneType", "Line2D"]
    assert [str(w.message).split("\n")[-1] for w in given] == [
        "See: https://matplotlib.org/stable/users/explain/axes/legend_guide.html"
        "#controlling-the-legend-entries"]
    with warnings.catch_warnings(record=True) as found:
        warnings.simplefilter("always")
        ax.legend()
    assert [str(w.message).split("\n")[-1] for w in found] == [
        "See: https://matplotlib.org/stable/tutorials/intermediate/legend_guide.html"
        "#implementing-a-custom-legend-handler"]


def test_a_text_box_made_under_xkcd_is_sketched():
    with plt.xkcd():
        fig, ax = plt.subplots()
        text = ax.text(0, 0, "a", bbox=dict(boxstyle="round4", lw=2))
    assert text.get_bbox_patch().get_sketch_params() == (1.0, 100.0, 2.0)


def test_figure_hooks_changed_in_place_run_and_are_restored_by_contexts(tmp_path, monkeypatch):
    (tmp_path / "corp12_hook.py").write_text(
        "made = []\n\ndef setup(fig):\n    made.append(fig)\n")
    monkeypatch.syspath_prepend(str(tmp_path))
    plt.rcParams["figure.hooks"] = []
    # As the gallery's mplcvd adds its toolbar to every figure.
    plt.rcParams["figure.hooks"].append("corp12_hook:setup")
    fig = plt.figure()
    import corp12_hook
    assert corp12_hook.made == [fig]
    with pltrs.rc_context({"figure.hooks": ["a:b"]}):
        assert plt.rcParams["figure.hooks"] == ["a:b"]
    assert plt.rcParams["figure.hooks"] == ["corp12_hook:setup"]
    plt.rcParams["figure.hooks"] = ["corp12_no_such_module:setup"]
    with pytest.raises(ModuleNotFoundError):
        plt.figure()
