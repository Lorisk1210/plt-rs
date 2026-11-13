"""
matplotlib's private names that other libraries read, write and call
(the census ``spec/diff/private_census.json``, measured by ``cargo xtask
diff --private``), family by family, reduced to matplotlib's API: the
plain fields a public getter returns as they are (read, and written where
libraries write them), an artist's ``_transform``, ``_stale``,
``_remove_method``, ``_parent_figure`` and ``_set_gc_clip``, the private
helpers of an axis with matplotlib's signatures, the property cycles
called as matplotlib's ``_process_plot_var_args``, a colormap's lookup
table, a legend's boxes, a table's location, a text's rotation and
renderer, a spine's position, a tick's label rotation, the groups of
aligned labels, a patch's hatch colour, and module names. Every expected
value was recorded from matplotlib 3.11.
"""

import inspect
import pickle

import numpy as np
import pytest

import pltrs
import pltrs.axes._base as maxes_base
import pltrs.collections as mcollections
import pltrs.patches as mpatches
import pltrs.pyplot as plt
import pltrs.text as mtext
from pltrs.artist import _XYPair
from pltrs.backend_bases import GraphicsContextBase
from pltrs.lines import Line2D


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_plain_fields_read_as_the_getters_give_them():
    fig, ax = plt.subplots()
    text = ax.text(0, 0, "hi", color="r", url="u", gid="g")
    line, = ax.plot([1, 2], [3, 4], drawstyle="steps-mid", color="g")
    patch = ax.add_patch(mpatches.Rectangle((0, 0), 1, 2, ec="b", hatch="/"))
    assert (text._text, text._color, text._usetex, text._url, text._gid) == ("hi", "r", False, "u", "g")
    assert (line._drawstyle, line._color, line._linestyle) == ("steps-mid", "g", "-")
    assert patch._edgecolor == (0.0, 0.0, 1.0, 1)
    assert (patch._width, patch._height, patch._hatch) == (1, 2, "/")
    assert (ax._frameon, ax._axisbelow, ax._adjustable) == (True, "line", "box")
    assert ax.xaxis._converter is None
    assert text._clipon is False and line._clipon is True


def test_plain_fields_libraries_assign_take_effect():
    fig, ax = plt.subplots()
    text = ax.text(0, 0, "a")
    text._text = "b"
    text._color = "r"
    line = ax.axline((0, 0), slope=1)
    line._slope = 2
    assert (text.get_text(), text.get_color(), line.get_slope()) == ("b", "r", 2)


def test_transform_field_follows_get_transform():
    fig, ax = plt.subplots()
    ax.plot([1, 2], [1, 2])
    fig.canvas.draw()
    tick = ax.xaxis.get_major_ticks()[0]
    assert type(tick.tick1line._transform).__name__ == "BlendedGenericTransform"
    assert tick.tick1line._transform is tick.tick1line.get_transform()
    assert tick._transform is None
    tick.get_transform()
    assert type(tick._transform).__name__ == "IdentityTransform"


def test_artist_state_fields():
    fig, ax = plt.subplots()
    text = ax.text(0, 0, "a")
    assert text._stale is True and text._parent_figure is fig
    text._stale = False
    assert text.stale is False
    assert fig._remove_method is None and ax.patch._remove_method is None
    text._remove_method(text)
    assert len(ax.texts) == 0
    rect = ax.axvspan(0, 1)
    rect._alpha = 0.3
    assert rect.get_alpha() == 0.3


def test_set_gc_clip_gives_the_clip():
    fig, ax = plt.subplots()
    line, = ax.plot([1, 2])
    gc = GraphicsContextBase()
    line._set_gc_clip(gc)
    assert gc.get_clip_rectangle() is line.get_clip_box()
    line.set_clip_on(False)
    line._set_gc_clip(gc)
    assert gc.get_clip_rectangle() is None


def test_axis_helpers_have_matplotlibs_signatures():
    ax = plt.figure().add_subplot()
    assert str(inspect.signature(ax.xaxis._copy_tick_props)) == "(src, dest)"
    assert str(inspect.signature(ax.xaxis._init)) == "()"
    assert str(inspect.signature(ax.yaxis._set_tick_locations)) == "(ticks, *, minor=False)"
    assert ax.xaxis._get_axis_name() == "x"


def test_property_cycles_make_artists_as_matplotlib():
    fig, ax = plt.subplots()
    lines = list(ax._get_lines(ax, [1, 2, 3], [4, 5, 6], "r--", label="a"))
    assert [(type(l).__name__, l.get_color(), l.get_linestyle(), l.get_label()) for l in lines] \
        == [("Line2D", "r", "--", "a")]
    two = list(ax._get_lines(ax, np.arange(3), np.ones((3, 2))))
    assert [l.get_color() for l in two] == [(0.12156862745098039, 0.4666666666666667, 0.7058823529411765),
                                           (1.0, 0.4980392156862745, 0.054901960784313725)]
    assert ax.plot([1, 2])[0].get_color() == (0.17254901960784313, 0.6274509803921569, 0.17254901960784313)
    polygon, = ax._get_patches_for_fill(ax, [0, 1, 1], [0, 0, 1])
    assert type(polygon).__name__ == "Polygon"
    assert str(inspect.signature(ax._get_lines)) == "(axes, *args, data=None, return_kwargs=False, **kwargs)"
    assert maxes_base._process_plot_var_args is type(ax._get_lines)


def test_colormap_lookup_table_of_a_drawn_image():
    fig, ax = plt.subplots()
    image = ax.imshow([[0, 1]], cmap="viridis")
    fig.canvas.draw()
    assert image.cmap._lut.shape == (259, 4)
    assert image.cmap._isinit is True
    assert pltrs._replacer({"a": [1]}, "a") == [1]


def test_legend_boxes():
    fig, ax = plt.subplots()
    ax.plot([1, 2], label="a")
    legend = ax.legend(title="T")
    fig.canvas.draw()
    assert type(legend._legend_title_box).__name__ == "TextArea"
    assert legend._legend_title_box.get_children()[0].get_text() == "T"
    assert [type(c).__name__ for c in legend._legend_handle_box.get_children()] == ["VPacker"]
    assert legend._alignment == "center"


def test_table_location_and_offset():
    fig, ax = plt.subplots()
    table = ax.table([["a"]], loc="center")
    assert table._loc == 9
    cell = table[0, 0]
    x = cell.get_x()
    table._offset(0.25, 0)
    assert cell.get_x() == pytest.approx(x + 0.25)


def test_text_rotation_and_renderer():
    fig, ax = plt.subplots()
    text = ax.text(0, 0, "a", rotation=30)
    assert text._rotation == 30.0 and text._renderer is None
    fig.canvas.draw()
    assert type(text._renderer).__name__ == "RendererAgg"
    text._rotation = 90
    assert text.get_rotation() == 90.0


def test_spine_position_and_tick_label_rotation():
    fig, ax = plt.subplots()
    spine = ax.spines["left"]
    assert spine._position == ("outward", 0.0)
    ax.cla()
    assert spine._position is None
    spine.set_position(("axes", 0.2))
    assert spine._position == ("axes", 0.2)
    rotation = ax.xaxis.get_major_ticks()[0]._labelrotation
    assert rotation == ("default", 0) and type(rotation[1]) is int


def test_aligned_label_groups_join():
    fig, (a, b) = plt.subplots(1, 2)
    a.set_xlabel("x")
    b.set_xlabel("x")
    b.tick_params(axis="x", labelsize=30)
    groups = fig._align_label_groups
    assert sorted(groups) == ["title", "x", "y"]
    assert groups["x"].get_siblings(a) == [a]
    groups["x"].join(a, b)
    assert groups["x"].joined(a, b)
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    ya = a.xaxis.label.get_window_extent(renderer).y0
    yb = b.xaxis.label.get_window_extent(renderer).y0
    assert ya == pytest.approx(yb)


def test_patch_hatch_colour_and_connection_fields():
    fig, ax = plt.subplots()
    edge = ax.add_patch(mpatches.Rectangle((0, 0), 1, 1, hatch="/"))
    red = ax.add_patch(mpatches.Rectangle((0, 0), 1, 1, hatch="/", hatchcolor="r"))
    assert edge._hatch_color == "edge"
    assert red._hatch_color == (1.0, 0.0, 0.0, 1)
    connection = mpatches.ConnectionPatch((0, 0), (1, 1), "data")
    assert (connection._posA_posB, connection._path_original) == ([(0, 0), (1, 1)], None)
    line = Line2D([0, 1], [0, 2])
    assert line._path is None
    line.get_path()
    assert line._path.vertices.tolist() == [[0.0, 0.0], [1.0, 2.0]]
    plotted, = plt.figure().add_subplot().plot([0, 1], [0, 2])
    assert plotted._path.vertices.tolist() == [[0.0, 0.0], [1.0, 2.0]]


def test_fields_listed_by_dir_and_carried_by_pickles():
    fig, ax = plt.subplots()
    text = ax.text(0, 0, "a", color="r")
    assert {"_color", "_usetex", "_text"} <= set(dir(text))
    text._color = "b"
    copy = pickle.loads(pickle.dumps(fig)).axes[0].texts[0]
    assert (copy._color, copy.get_color()) == ("b", "b")


def test_names_matplotlib_s_stubs_declare():
    rc = pltrs.RcParams()
    rc._update_raw({"lines.linewidth": 3})
    rc._update_raw(pltrs.rcParams)
    assert rc["lines.linewidth"] == pltrs.rcParams["lines.linewidth"]
    bare = pltrs.RcParams()
    bare._ensure_has_backend()
    assert "backend" in bare
    pair = _XYPair([1], [2])
    assert (pair.x, pair.y) == ([1], [2])
    assert str(inspect.signature(_XYPair)) == "(x, y)"
    assert str(inspect.signature(mcollections._MeshData.__init__)) == "(self, coordinates, *, shading='flat')"
    curve = mpatches.ArrowStyle._Curve
    assert list(inspect.signature(curve).parameters) == [
        "head_length", "head_width", "widthA", "widthB", "lengthA", "lengthB",
        "angleA", "angleB", "scaleA", "scaleB"]
    assert list(inspect.signature(mpatches.ArrowStyle.BracketA).parameters) == ["widthA", "lengthA", "angleA"]
    fig, ax = plt.subplots()
    note = ax.annotate("a", (0.5, 0.5))
    mtext._AnnotationBase.__init__(note, (0.2, 0.3), "axes fraction", True)
    assert (note.xy, note.xycoords, note.get_annotation_clip()) == ((0.2, 0.3), "axes fraction", True)
    ax._axis_map = {"x": ax.xaxis, "y": ax.yaxis}
    assert ax._axis_map == {"x": ax.xaxis, "y": ax.yaxis}
