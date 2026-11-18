"""
What a getter hands out is the artist's own, as in matplotlib (task MUT1):
programs change in place the arrays, lists and paths that getters return
(``body.get_paths()[0].vertices[:, 0] = ...``, the half violin) and the
objects they gave an artist, and the artist is drawn, measured and pickled
with the change, while its data limits stay until ``relim``.

The sweep of `mutability` changes in place every object reached from every
public getter and attribute of every kind of artist, and holds each row to
matplotlib's recorded outcome (``mutability/reference.json``); the rows
still to do are listed in ``mutability/expected.json``, and a row that
stops differing fails until it is taken off that list. The tests after it
pin the idioms themselves and what the mechanism costs an artist that
hands nothing out. Every expected value was recorded from matplotlib
3.11; the file passes under matplotlib through the alias of
``python/tests/leaks/alias``.
"""

import gc
import pickle

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, colors, patches, transforms
from pltrs.path import Path

from mutability import runner, scenes, setters, threed

#: Whether the tests run on matplotlib itself (through the alias), where
#: no row may differ.
ON_MATPLOTLIB = pltrs.__name__ == "matplotlib"

REFERENCE = runner.recorded()
EXPECTED = {} if ON_MATPLOTLIB else runner.expected()
FOUND = runner.found_in(REFERENCE)
GROUPS = runner.groups(FOUND)


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


@pytest.mark.parametrize("group", sorted(GROUPS))
def test_changes_in_place_follow_matplotlib(group):
    names = GROUPS[group]
    ours = runner.measure_here(FOUND, names)
    differing = runner.differing(ours, REFERENCE["outcomes"])
    expected = {name: EXPECTED[name] for name in names if name in EXPECTED}
    unexpected = {name: keys for name, keys in differing.items()
                  if expected.get(name) != keys}
    assert not unexpected, (
        "rows differing from matplotlib otherwise than expected.json says "
        f"(row: the keys that differ): {unexpected}")
    done = sorted(set(expected) - set(differing))
    assert not done, (
        f"rows that no longer differ from matplotlib: {done}; take them off "
        "mutability/expected.json (python -m mutability --expect)")


def pixels(fig):
    """The pixels of *fig*, drawn now."""
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba()).copy()


def small_axes():
    """A small figure and its Axes, with a fixed view."""
    fig, ax = plt.subplots(figsize=(3, 3), dpi=40)
    ax.set_xlim(-1, 4)
    ax.set_ylim(-1, 4)
    return fig, ax


def test_a_violin_is_halved_by_clipping_its_vertices():
    # USR1's ``box_violin/violin_split_by_hand``, reduced: the idiom of
    # countless scripts.
    fig, ax = plt.subplots(figsize=(3, 3), dpi=40)
    body = ax.violinplot([[1, 2, 2, 3, 3, 3, 4]], showextrema=False)["bodies"][0]
    body.set_alpha(1)
    ax.axis("off")
    ax.set_position([0, 0, 1, 1])
    ax.set_xlim(0.5, 1.5)
    limits = ax.dataLim.get_points().copy()

    def painted():
        """How many pixels the body covers left and right of the middle."""
        drawn = (pixels(fig)[:, :, :3] != 255).any(axis=-1)
        return int(drawn[:, :60].sum()), int(drawn[:, 60:].sum())

    left, right = painted()
    assert left > 500 and right > 500
    vertices = body.get_paths()[0].vertices
    vertices[:, 0] = np.clip(vertices[:, 0], 1.0, np.inf)
    assert painted() == (0, right)
    assert body.get_paths()[0].vertices is vertices
    assert vertices[:, 0].min() == 1.0
    # The data limits stay, as in matplotlib: nothing autoscales.
    assert np.array_equal(ax.dataLim.get_points(), limits)
    assert not fig.stale


def test_a_scatter_follows_its_offsets_sizes_and_colours():
    fig, ax = small_axes()
    points = ax.scatter([1, 2, 3], [1, 2, 3], s=[40, 60, 80], color=["r", "g", "b"])
    before = pixels(fig)
    offsets, sizes, faces = points.get_offsets(), points.get_sizes(), points.get_facecolor()
    assert points.get_offsets() is offsets and points.get_sizes() is sizes
    assert points.get_facecolor() is faces
    offsets[0] = (3, 1)
    moved = pixels(fig)
    assert (moved != before).any()
    sizes[1] = 900
    grown = pixels(fig)
    assert (grown != moved).any()
    faces[2] = (0, 0, 0, 1)
    assert (pixels(fig) != grown).any()
    assert points.get_offsets()[0].tolist() == [3, 1]
    assert points.get_sizes().tolist() == [40, 900, 80]
    assert points.get_facecolor()[2].tolist() == [0, 0, 0, 1]
    # The limits follow only once they are recomputed.
    assert ax.dataLim.get_points().tolist() == [[1, 1], [3, 3]]
    ax.relim()
    assert ax.dataLim.get_points().tolist() == [[2, 1], [3, 3]]
    # A setter binds a new array: the old one is nobody's.
    points.set_offsets([[0, 0], [1, 1], [2, 2]])
    replaced = pixels(fig)
    offsets[:] = 3
    assert points.get_offsets() is not offsets
    assert (pixels(fig) == replaced).all()


def test_a_line_shares_one_buffer_between_its_data_and_its_path():
    fig, ax = small_axes()
    line, = ax.plot(np.array([0.0, 1, 2]), np.array([0.0, 1, 0]))
    before = pixels(fig)
    xy = line.get_xydata()
    assert line.get_xydata() is xy and line.get_path().vertices is xy
    assert line.get_xdata(orig=False) is line.get_xdata(orig=False)
    assert np.shares_memory(line.get_ydata(orig=False), xy)
    line.get_path().vertices[1, 1] = 3
    assert xy[1].tolist() == [1, 3]
    assert line.get_ydata(orig=False).tolist() == [0, 3, 0]
    assert (pixels(fig) != before).any()
    # The data given are copies, which the change leaves alone.
    assert line.get_ydata().tolist() == [0, 1, 0]
    assert ax.dataLim.get_points().tolist() == [[0, 0], [2, 1]]
    ax.relim()
    assert ax.dataLim.get_points().tolist() == [[0, 0], [2, 3]]
    line.set_ydata([1, 1, 1])
    assert line.get_xydata() is not xy


def test_patches_follow_their_paths_and_shared_paths_refuse_changes():
    fig, ax = small_axes()
    triangle = np.array([[0.0, 0.0], [3.0, 0.0], [1.5, 3.0], [0.0, 0.0]])
    polygon = ax.add_patch(patches.Polygon(triangle))
    path = Path(triangle / 3 + 2, [1, 2, 2, 79])
    patch = ax.add_patch(patches.PathPatch(path))
    wedge = ax.add_patch(patches.Wedge((1.5, 1.5), 1.2, 30, 120))
    before = pixels(fig)
    # matplotlib makes a polygon's path of the array it is given, and a
    # path patch draws the path it is given.
    assert polygon.get_xy() is triangle and polygon.get_path().vertices is triangle
    assert patch.get_path() is path
    triangle[2] = (0, 3)
    after_polygon = pixels(fig)
    assert (after_polygon != before).any()
    path.vertices[2, 0] = 2
    after_patch = pixels(fig)
    assert (after_patch != after_polygon).any()
    outline = wedge.get_path()
    assert wedge.get_path() is outline
    outline.vertices *= 0.5
    after_wedge = pixels(fig)
    assert (after_wedge != after_patch).any()
    # A setter of the geometry makes the path anew.
    wedge.set_radius(1.0)
    assert wedge.get_path() is not outline
    assert wedge.get_path().get_extents().y1 == pytest.approx(2.5, abs=1e-3)
    # The unit circle is one path for every circle, and read-only.
    circle = ax.add_patch(patches.Circle((1, 1), 0.5))
    with pytest.raises(ValueError, match="read-only"):
        circle.get_path().vertices[0, 0] = 2


def test_a_collection_keeps_the_paths_it_is_given():
    fig, ax = small_axes()
    triangle = np.array([[0.0, 0.0], [3.0, 0.0], [1.5, 3.0], [0.0, 0.0]])
    paths = [Path(triangle), Path(triangle / 3 + 2)]
    collection = ax.add_collection(collections.PathCollection(
        paths, facecolors="c", transform=ax.transData))
    before = pixels(fig)
    assert collection.get_paths() is paths
    paths.pop()
    one = pixels(fig)
    assert (one != before).any()
    triangle[2] = (0, 3)
    assert (pixels(fig) != one).any()
    assert len(collection.get_paths()) == 1
    read_back = pickle.loads(pickle.dumps(collection))
    assert read_back.get_paths()[0].vertices[2].tolist() == [0, 3]


def test_an_image_follows_its_array_and_its_extent():
    fig, ax = small_axes()
    image = ax.imshow(np.arange(9.0).reshape(3, 3), extent=(0, 3, 0, 3))
    ax.set_xlim(-1, 4)
    ax.set_ylim(-1, 4)
    before = pixels(fig)
    data, extent = image.get_array(), image.get_extent()
    assert image.get_array() is data and image.get_extent() is extent
    data[0, 0] = 8
    recoloured = pixels(fig)
    assert (recoloured != before).any()
    # The limits of the colours stay, as matplotlib's norm keeps them.
    assert image.get_clim() == (0, 8)
    extent[1] = 2.0
    assert (pixels(fig) != recoloured).any()
    assert image.get_extent() == [0, 2.0, 0, 3]


def test_artists_follow_the_transform_they_were_given():
    fig, ax = small_axes()
    turn = transforms.Affine2D()
    patch = ax.add_patch(patches.Rectangle((0, 0), 2, 1))
    patch.set_transform(turn + ax.transData)
    points = ax.scatter([1, 2], [1, 1], transform=turn + ax.transData)
    clipped = ax.add_patch(patches.Rectangle((0, 2), 3, 1.5, color="k"))
    clipped.set_clip_path(Path([(0, 0), (3, 0), (3, 4), (0, 4), (0, 0)]),
                          turn + ax.transData)
    before = pixels(fig)
    corner = patch.get_window_extent().get_points()[1].tolist()
    # The animation idiom: the transform is changed, not set again.
    turn.clear().rotate_deg(30)
    turned = pixels(fig)
    assert (turned != before).any()
    # The rectangle from (0, 0) to (2, 1), turned by 30 degrees: its box
    # ends at (2 cos 30, 2 sin 30 + cos 30) in data coordinates.
    assert corner == pytest.approx(ax.transData.transform((2, 1)).tolist())
    assert patch.get_window_extent().get_points()[1].tolist() == pytest.approx(
        ax.transData.transform((3 ** 0.5, 1 + 3 ** 0.5 / 2)).tolist())
    assert points.get_offset_transform().transform((1, 0)).tolist() == pytest.approx(
        ax.transData.transform((3 ** 0.5 / 2, 0.5)).tolist())
    turn.translate(0.5, 0)
    assert (pixels(fig) != turned).any()


def test_the_lists_of_rcparams_are_the_parameters():
    family = pltrs.rcParams["font.sans-serif"]
    assert pltrs.rcParams["font.sans-serif"] is family
    size = pltrs.rcParams["figure.figsize"]
    size[0] = 3.0
    assert plt.figure().get_size_inches().tolist() == [3.0, 4.8]
    fig, ax = plt.subplots(figsize=(3, 1), dpi=40)
    ax.set_title("Title")
    before = pixels(fig)
    family.insert(0, "DejaVu Serif")
    assert (pixels(fig) != before).any()
    assert pltrs.rcParams["font.sans-serif"][0] == "DejaVu Serif"
    # Set anew, the parameter is another list.
    pltrs.rcParams["figure.figsize"] = [5, 4]
    assert pltrs.rcParams["figure.figsize"] is not size
    assert pltrs.rcParams["figure.figsize"] == [5.0, 4.0]
    # A context puts the very lists back, with what was changed in them.
    kept = pltrs.rcParams["figure.figsize"]
    with pltrs.rc_context():
        kept[1] = 2.0
    assert pltrs.rcParams["figure.figsize"] is kept and kept == [5.0, 2.0]
    assert plt.figure().get_size_inches().tolist() == [5.0, 2.0]


def test_a_statement_changing_a_list_of_rcparams_changes_the_parameter():
    # The list need not be held: the idiom is one statement.
    pltrs.rcParams["font.sans-serif"].insert(0, "DejaVu Serif")
    assert pltrs.rcParams["font.sans-serif"][0] == "DejaVu Serif"
    fig, ax = plt.subplots(figsize=(3, 1), dpi=40)
    ax.set_title("Title")
    before = pixels(fig)
    pltrs.rcParams["font.sans-serif"].insert(0, "DejaVu Sans Mono")
    assert (pixels(fig) != before).any()
    pltrs.rcParams["figure.figsize"][0] = 2.0
    assert plt.figure().get_size_inches().tolist() == [2.0, 4.8]


def test_lists_of_rcparams_are_looked_at_only_while_a_program_holds_them():
    if ON_MATPLOTLIB:
        pytest.skip("how pltrs keeps the lists it hands out")
    # (A list an earlier test read is held until the collector has run, if
    # what holds it lies in a cycle.)
    gc.collect()
    rc = pltrs.rcParams
    rc._get("lines.linewidth")
    assert rc._lists == {}
    # Copies of all the parameters by item keep none of their lists.
    as_dict = dict(rc)
    as_items = dict(rc.items())
    as_text = repr(rc)
    by_key = {key: rc[key] for key in rc}
    assert rc._lists == {}
    assert as_dict == as_items == by_key and "font.sans-serif" in as_text
    # A list read by its key is kept while the program holds it ...
    family = rc["font.sans-serif"]
    styles = {key: rc[key] for key in ("font.family", "font.serif")}
    rc._get("lines.linewidth")
    assert sorted(rc._lists) == ["font.family", "font.sans-serif", "font.serif"]
    del styles
    rc._get("lines.linewidth")
    assert list(rc._lists) == ["font.sans-serif"]
    # ... and forgotten once it does not, with what it changed in it.
    family.append("Inter")
    del family
    assert rc._get("font.sans-serif")[-1] == "Inter"
    assert rc._lists == {}


def test_a_list_read_before_its_parameter_was_set_anew_is_nobodys():
    family = pltrs.rcParams["font.family"]
    pltrs.rcParams["font.family"] = ["serif"]
    # The end of a context puts back the parameters as they were at its
    # start, not a list that was the parameter once.
    with pltrs.rc_context():
        pass
    assert pltrs.rcParams["font.family"] == ["serif"]
    family.append("monospace")
    with pltrs.rc_context({"lines.linewidth": 3}):
        assert pltrs.rcParams["font.family"] == ["serif"]
    assert pltrs.rcParams["font.family"] == ["serif"]
    # A list changed inside a context is the parameter again after it,
    # with the change, as matplotlib puts the very object back.
    serif = pltrs.rcParams["font.serif"]
    with pltrs.rc_context():
        serif.insert(0, "Inter")
    assert pltrs.rcParams["font.serif"] is serif and serif[0] == "Inter"


def test_a_figure_pickles_with_artists_put_in_its_lists():
    fig, ax = small_axes()
    fig.patches.append(patches.Rectangle(
        (0.2, 0.2), 0.5, 0.5, transform=fig.transFigure, figure=fig))
    fig.lines.extend([pltrs.lines.Line2D(
        [0, 1], [0, 1], transform=fig.transFigure, figure=fig)])
    read_back = pickle.loads(pickle.dumps(fig))
    assert (len(read_back.patches), len(read_back.lines)) == (1, 1)
    assert (pixels(read_back) == pixels(fig)).all()


def test_a_cell_taken_out_of_the_dict_of_cells_leaves_its_table():
    fig, ax = small_axes()
    table = ax.table([["a", "b"], ["c", "d"]], loc="center")
    before = pixels(fig)
    cells = table.get_celld()
    assert table.get_celld() is cells
    cells.pop((0, 0))
    assert sorted(table.get_celld()) == [(0, 1), (1, 0), (1, 1)]
    assert (pixels(fig) != before).any()
    assert sorted(pickle.loads(pickle.dumps(table)).get_celld()) == [(0, 1), (1, 0), (1, 1)]
    del cells[0, 1]
    assert sorted(table.get_celld()) == [(1, 0), (1, 1)]


@pytest.mark.skipif(ON_MATPLOTLIB, reason="what pltrs' mechanism costs")
def test_what_the_package_reads_itself_is_kept_by_nobody():
    from pltrs import _own_state

    fig, ax = plt.subplots()
    # `boxplot` reads the data of the lines it made, a legend handler the
    # widths and colours of the collection it stands for, `getp` every
    # getter: none of them is the program asking for the artist's state.
    boxes = ax.boxplot([[1.0, 2.0, 3.0, 9.0], [2.0, 3.0, 4.0]])
    segments = collections.LineCollection([[(0, 0), (1, 1)]], label="segments")
    ax.add_collection(segments)
    ax.legend()
    line = boxes["medians"][0]
    plt.getp(line)
    fig.canvas.draw()
    for artist in (segments, *(found for lines in boxes.values() for found in lines)):
        assert _own_state.RECORDS not in artist.__dict__
    assert _own_state.OWNERS not in fig.__dict__
    # The program asking is another matter.
    assert line.get_ydata() is line.get_ydata()
    assert list(line.__dict__[_own_state.RECORDS]) == ["xy"]


@pytest.mark.skipif(ON_MATPLOTLIB, reason="what pltrs' mechanism costs")
def test_an_artist_that_hands_nothing_out_pays_nothing():
    from pltrs import _own_state

    lists = list(pltrs.rcParams._lists)
    fig, axs = plt.subplots(1, 2)
    ax = axs[0]
    line, = ax.plot([0, 1, 2], [0, 1, 0], "--", label="line")
    points = ax.scatter([0, 1], [1, 0], c=[1, 2])
    bars = ax.bar([0, 1], [1, 2])
    image = axs[1].imshow(np.arange(4.0).reshape(2, 2))
    fig.colorbar(image)
    ax.legend()
    ax.set_title("title")
    fig.canvas.draw()
    fig.savefig(__import__("io").BytesIO(), format="svg")
    for artist in (fig, ax, line, points, image, *bars):
        assert _own_state.RECORDS not in artist.__dict__
    assert _own_state.OWNERS not in fig.__dict__
    # Nor does the figure ask anything else: the tables of its colormaps
    # were read by the package alone.
    assert _own_state.FOLLOWED not in fig.__dict__
    assert id(image.cmap) not in colors._luts_read
    assert id(points.cmap) not in colors._luts_read
    # No list of the parameters was handed to the program either (the
    # tests run before in this process may still hold some).
    assert set(pltrs.rcParams._lists) <= set(lists)
    # Handing one array out makes one record, which the figure tells.
    points.get_offsets()
    assert list(points.__dict__[_own_state.RECORDS]) == ["offsets"]
    assert list(fig.__dict__[_own_state.OWNERS]) == [points]
    # A pickle leaves the records out, and holds what the program changed.
    points.get_offsets()[0] = (5, 5)
    read_back = pickle.loads(pickle.dumps(points))
    assert _own_state.RECORDS not in read_back.__dict__
    assert read_back.get_offsets()[0].tolist() == [5, 5]


def test_the_private_name_of_the_mapped_array_is_the_array():
    fig, ax = plt.subplots()
    points = ax.scatter([0, 1, 2], [0, 1, 0], c=[1.0, 2.0, 3.0])
    image = ax.imshow(np.arange(6.0).reshape(2, 3))
    before = pixels(fig)
    assert points._A is points._A and points._A is points.get_array()
    points._A[0] = 3.0
    after = pixels(fig)
    assert (after != before).any()
    image._A[0, 0] = 5.0
    assert (pixels(fig) != after).any()
    # Assigning replaces the array, with no check, as matplotlib's attribute.
    values = np.array([3.0, 2.0, 1.0])
    points._A = values
    assert points.get_array() is values


def test_the_lists_of_artists_are_one_list_each():
    fig, ax = plt.subplots()
    ax.plot([0, 1])
    inset = ax.inset_axes([0.5, 0.5, 0.4, 0.4])
    text = fig.text(0.5, 0.5, "text")
    fig.legend(handles=ax.lines, labels=["line"])
    assert fig.texts is fig.texts and fig.artists is fig.artists
    assert fig.legends is fig.legends
    assert ax._children is ax._children and ax.child_axes is ax.child_axes
    # What the figure and the Axes add later is in the list read before.
    texts, children = fig.texts, ax._children
    other = fig.text(0.1, 0.1, "other")
    line, = ax.plot([1, 0])
    assert texts == [text, other] and children[-1] is line
    # An artist taken out of its list is drawn no more.
    before = pixels(fig)
    children.pop()
    texts.remove(other)
    after = pixels(fig)
    assert (after != before).any()
    assert ax.lines[:] == [ax._children[0]] and fig.texts == [text]
    ax.child_axes.pop()
    assert (pixels(fig) != after).any()
    assert inset not in ax.child_axes


def test_a_legend_and_a_contour_set_keep_their_lists():
    fig, ax = plt.subplots()
    ax.plot([0, 1], label="a")
    ax.plot([1, 0], label="b")
    legend = ax.legend()
    assert legend.texts is legend.texts and legend.legend_handles is legend.legend_handles
    legend.texts.pop()
    assert len(legend.texts) == 1 and len(legend.get_texts()) == 1
    x, y = np.meshgrid(np.linspace(-1, 1, 9), np.linspace(-1, 1, 9))
    contours = ax.contour(x, y, x * y, levels=[-0.5, 0.0, 0.5])
    assert contours.levels is contours.levels
    contours.levels[0] = -0.25
    assert contours.levels[0] == -0.25
    assert pickle.loads(pickle.dumps(contours)).levels[0] == -0.25
    labels = contours.labelTexts
    made = ax.clabel(contours)
    assert contours.labelTexts is labels and len(labels) == len(made) > 0


def test_fixed_ticks_follow_the_array_they_were_given():
    fig, ax = plt.subplots()
    ax.plot([0, 10], [0, 10])
    ticks = np.array([1.0, 4.0, 8.0])
    ax.set_xticks(ticks)
    fig.canvas.draw()
    assert [label.get_text() for label in ax.get_xticklabels()] == ["1", "4", "8"]
    ticks[1] = 6.0
    fig.canvas.draw()
    assert [label.get_text() for label in ax.get_xticklabels()] == ["1", "6", "8"]
    # The locator's own array, read from it, likewise.
    locs = ax.xaxis.get_major_locator().locs
    assert locs is ticks
    locs[2] = 9.0
    fig.canvas.draw()
    assert ax.get_xticks().tolist() == [1.0, 6.0, 9.0]


def test_limit_boxes_follow_their_arrays():
    fig, ax = plt.subplots()
    ax.plot([0, 1, 2], [0, 1, 0])
    fig.canvas.draw()
    points = ax.viewLim.get_points()
    points[:, 0] = 0.5, 1.5
    assert ax.get_xlim() == (0.5, 1.5)
    assert ax.viewLim.get_points() is points
    # The data limits' own box moves at once; nothing is autoscaled.
    ax.dataLim.get_points()[1, 1] = 5.0
    assert ax.dataLim.y1 == 5.0 and ax.get_ylim()[1] < 2


def test_a_3d_axes_follows_its_boxes_and_its_aspect():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    ax.plot([0, 1], [0, 1], [0, 1])
    before = pixels(fig)
    aspect = ax.get_box_aspect()
    assert ax.get_box_aspect() is aspect
    aspect[2] *= 0.5
    after = pixels(fig)
    assert (after != before).any() and ax.get_box_aspect() is aspect
    box = ax.xy_viewLim
    box.get_points()[:, 0] = -1.0, 2.0
    assert ax.xy_viewLim is box
    assert ax.get_xlim3d() == (-1.0, 2.0)
    assert (pixels(fig) != after).any()


def test_a_figures_images_and_an_axis_ticks_are_lists_of_their_own():
    fig, ax = plt.subplots()
    ax.plot([0, 1, 2], [0, 1, 0])
    image = fig.figimage(np.arange(12.0).reshape(3, 4), xo=20, yo=20)
    assert fig.images is fig.images and fig.images == [image]
    before = pixels(fig)
    fig.images.pop()
    assert fig.images == [] and (pixels(fig) != before).any()
    # A tick taken out of an axis' list stays out until the axis makes its
    # ticks again, at the next draw.
    ticks = ax.xaxis.majorTicks
    count = len(ticks)
    ticks.pop()
    assert ax.xaxis.majorTicks is ticks and len(ax.xaxis.majorTicks) == count - 1
    fig.canvas.draw()
    assert ax.xaxis.majorTicks is ticks and len(ticks) == count
    minor = ax.xaxis.minorTicks
    minor.clear()
    assert ax.xaxis.minorTicks == []


def test_a_mesh_follows_its_coordinates_and_a_colormap_its_table():
    fig, ax = plt.subplots()
    x, y = np.meshgrid(np.arange(4.0), np.arange(3.0))
    mesh = ax.pcolormesh(x, y, np.arange(6.0).reshape(2, 3), cmap="viridis")
    before = pixels(fig)
    coordinates = mesh.get_coordinates()
    assert mesh.get_coordinates() is coordinates
    coordinates[1, 1] += 0.3
    moved = pixels(fig)
    assert (moved != before).any() and mesh.get_coordinates() is coordinates
    # A colormap's table changed after the mesh took the colormap.
    table = mesh.get_cmap()._lut
    table[:, 0] = 1.0
    assert (pixels(fig) != moved).any()


def test_the_first_draw_sets_a_collections_colours_again():
    # matplotlib's first draw of a collection sets its colours from what
    # the setters were given: a change made in the arrays before it is
    # read back until then, and never drawn.
    fig, ax = plt.subplots()
    filled = ax.fill_between([0, 1], [1, 1], color="tab:blue")
    faces = filled.get_facecolor()
    faces[:, 0] = 1.0
    assert filled.get_facecolor()[0, 0] == 1.0
    fig.canvas.draw()
    assert filled.get_facecolor() is not faces
    assert filled.get_facecolor()[0].round(3).tolist() == [0.122, 0.467, 0.706, 1.0]
    # After a draw the array is the collection's own, and drawn.
    faces = filled.get_facecolor()
    before = pixels(fig)
    faces[:, 0] = 1.0
    assert (pixels(fig) != before).any() and filled.get_facecolor() is faces


def native_calls(function):
    """The native functions *function* calls, as ``module.name``."""
    import sys

    calls = []

    def profile(frame, event, called):
        module = getattr(called, "__module__", None) or ""
        if event == "c_call" and "._native" in module:
            calls.append(f"{module.rsplit('.', 1)[-1]}.{called.__name__}")
    sys.setprofile(profile)
    try:
        function()
    finally:
        sys.setprofile(None)
    return calls


def hand_overs(artist, function):
    """How many times *artist* is asked for the objects it keeps, to hand
    pltrs (Rust) what changed in them, while *function* runs."""
    from pltrs import _own_state

    told = []
    sync = _own_state.sync

    def counting(asked):
        if asked.__dict__.get(_own_state.RECORDS):
            told.append(asked)
        return sync(asked)
    _own_state.sync = counting
    try:
        function()
    finally:
        _own_state.sync = sync
    return told.count(artist)


@pytest.mark.skipif(ON_MATPLOTLIB, reason="what pltrs' mechanism costs")
def test_every_function_passing_by_has_rows_of_calls_that_tell_nobody():
    from pltrs import _native, _own_state

    listed = {f"{module}.{name}": what for module, name, what in _native.artist.passing_by()}
    assert all(listed.values()), "each function says what it reads or writes"
    for function in listed:
        module, name = function.split(".")
        assert callable(getattr(getattr(_native, module), name)), function
    # The rows of the sweep are made of the calls of this table: a function
    # passed by without a call has none.
    assert set(setters.CALLS) == set(listed)
    for function, calls in setters.CALLS.items():
        assert calls, function
        for subject, name, call in calls:
            _, artist = scenes.build(subject)
            assert function in native_calls(lambda: call(artist)), (function, subject, name)
            # On an artist that keeps an object of its own, the call hands
            # nothing over: every native call it makes is passed by.
            _, artist = scenes.build(subject)
            held = artist.get_linewidth() if hasattr(artist, "get_paths") else (
                artist.get_xydata() if hasattr(artist, "get_xydata") else artist.get_path())
            assert _own_state.RECORDS in artist.__dict__ and held is not None
            assert hand_overs(artist, lambda: call(artist)) == 0, (function, subject, name)
            plt.close("all")
    rows = {row.name for row in setters.rows(REFERENCE["surface"])}
    assert all(any(f" then {name}" in row for row in rows)
               for calls in setters.CALLS.values() for _, name, _ in calls)
    assert rows <= set(REFERENCE["outcomes"])


def test_a_setter_between_a_change_and_a_draw_leaves_the_change():
    # A scatter read once, then restyled: the offsets changed in place are
    # drawn with the new width, whichever came first.
    fig, ax = small_axes()
    points = ax.scatter([0.0, 1.0, 2.0], [0.0, 1.0, 2.0], s=200)
    before = pixels(fig)
    offsets = points.get_offsets()
    offsets[:, 1] = [2.0, 1.0, 0.0]
    points.set_linewidth(4.0)
    points.set_zorder(5)
    assert points.get_offsets() is offsets
    assert points.get_linewidth().tolist() == [4.0]
    changed = pixels(fig)
    assert (changed != before).any()
    offsets[:, 1] = [0.0, 1.0, 2.0]
    points.set_linewidth(1.0)
    restored = pixels(fig)
    assert (restored != changed).any()
    # A setter of the state itself binds a new object: the old one is the
    # artist's no more, whatever the program changed in it.
    offsets[:, 0] = 3.0
    points.set_offsets([[0.5, 0.5]])
    assert points.get_offsets().tolist() == [[0.5, 0.5]]
    pixels(fig)
    assert points.get_offsets().tolist() == [[0.5, 0.5]]
    widths = points.get_linewidth()
    widths[0] = 9.0
    points.set_linewidth(2.0)
    pixels(fig)
    assert points.get_linewidth().tolist() == [2.0]


@pytest.mark.skipif(ON_MATPLOTLIB, reason="what pltrs' mechanism costs")
def test_setting_styles_of_a_scatter_whose_offsets_are_held_compares_nothing():
    from pltrs import _own_state

    fig, ax = small_axes()
    points = ax.scatter(np.arange(50.0), np.arange(50.0))
    offsets = points.get_offsets()

    def restyle():
        for width in (1.0, 2.0, 3.0):
            points.set_linewidth(width)
            points.set_zorder(width)
            points.set_label(str(width))
            points.set_linestyle("--")
            points.set_antialiased(False)
    assert native_calls(lambda: points.set_linewidth(2.5)) == ["collections.set_linewidths"]
    assert hand_overs(points, restyle) == 0
    # What reads the scatter hands the offsets over first.
    offsets[0] = (7.0, 7.0)
    assert hand_overs(points, lambda: points.get_window_extent()) > 0
    assert _own_state.kept(points, "offsets") is offsets


@pytest.mark.skipif(ON_MATPLOTLIB, reason="what pltrs' mechanism costs")
def test_an_array_nothing_but_its_record_holds_is_compared_no_more():
    import sys
    import weakref

    from pltrs import _own_state

    fig, ax = small_axes()
    points = ax.scatter([0.0, 1.0, 2.0], [0.0, 1.0, 2.0], s=200)
    before = pixels(fig)
    record = lambda: points.__dict__[_own_state.RECORDS]["offsets"]  # noqa: E731
    compared = []
    state = points.get_offsets() is not None and record().state
    spied = state.same
    assert spied is not None
    # (The spy counts, and holds no array.)
    state.same = lambda artist, obj: (compared.append(1), spied(artist, obj))[1]
    try:
        # The baseline is measured, in the call shape the hand-over counts
        # in: an array of the test's own held by a record alone.
        # (One answer per statement, outside the assertion: pytest's
        # assertions keep what they compare, the array too.)
        probe = _own_state._Record(None, np.zeros(1), None, None, None)
        counted = _own_state._references(probe._obj)
        assert counted == _own_state._UNHELD == _own_state._count_unheld()
        only = _own_state._only_recorded(probe)
        assert only
        held = probe._obj
        only = _own_state._only_recorded(probe)
        assert not only
        del held
        # Held by the program: compared at every hand-over.
        offsets = points.get_offsets()
        compared.clear()
        points.get_window_extent()
        points.get_window_extent()
        assert len(compared) >= 2 and not record().settled
        # Changed, then let go of: the change is handed over, once, and
        # the array is compared no more.
        offsets[:, 1] = [2.0, 1.0, 0.0]
        del offsets
        changed = pixels(fig)
        assert (changed != before).any()
        assert record().settled
        compared.clear()
        points.get_window_extent()
        points.set_alpha(None)
        pixels(fig)
        assert compared == []
        # Handed out again, it is the same array (nothing is read, and
        # nothing compared), compared again while the program has it.
        offsets = points.get_offsets()
        assert compared == []
        assert offsets[:, 1].tolist() == [2.0, 1.0, 0.0] and not record().settled
        points.get_window_extent()
        assert compared
        offsets[:, 1] = [0.0, 1.0, 2.0]
        assert (pixels(fig) == before).all()
        # A view holds its array, and so does a buffer taken of it.
        column = offsets[:, 1]
        del offsets
        points.get_window_extent()
        assert not record().settled
        column[:] = [2.0, 1.0, 0.0]
        assert (pixels(fig) == changed).all()
        buffer = memoryview(np.ma.getdata(points.get_offsets()))
        del column
        points.get_window_extent()
        assert not record().settled
        buffer[0, 0] = 1.5
        assert (pixels(fig) != changed).any() and points.get_offsets()[0, 0] == 1.5
        buffer[0, 0] = 0.0
        del buffer
        # A weak reference can reach the array without holding it.
        weak = weakref.ref(points.get_offsets())
        points.get_window_extent()
        assert not record().settled
        weak()[0, 0] = 0.0
        assert (pixels(fig) == changed).all()
        del weak
        points.get_window_extent()
        assert record().settled
        # What may be reached another way is always compared: a list by
        # its items, a path by its vertices.
        points.get_paths()
        points.get_window_extent()
        assert not points.__dict__[_own_state.RECORDS]["paths"].settled
    finally:
        state.same = spied
    # Where reference counts tell nothing (an interpreter without its
    # GIL), nothing is ever taken for unheld.
    gil = getattr(sys, "_is_gil_enabled", None)
    sys._is_gil_enabled = lambda: False
    try:
        assert _own_state._count_unheld() == 0
        assert pltrs._count_unheld() == 0
    finally:
        if gil is None:
            del sys._is_gil_enabled
        else:
            sys._is_gil_enabled = gil
    assert _own_state._count_unheld() == _own_state._UNHELD > 0
    assert pltrs._count_unheld() == pltrs._UNHELD > 0


@pytest.mark.skipif(ON_MATPLOTLIB, reason="what pltrs' mechanism costs")
def test_offsets_and_sizes_are_compared_with_what_pltrs_holds():
    from pltrs import _own_state

    fig, ax = small_axes()
    points = ax.scatter([0.0, 1.0, 2.0], [0.0, 1.0, 2.0], s=200)
    records = lambda: points.__dict__[_own_state.RECORDS]  # noqa: E731
    before = pixels(fig)
    offsets = points.get_offsets()
    # No copy of the array is kept beside it (a scatter's offsets are a
    # masked array, compared with its mask).
    assert records()["offsets"].seen is _own_state.NATIVELY
    offsets[1] = (1.0, 3.0)
    assert points.get_offsets() is offsets
    moved = pixels(fig)
    assert (moved != before).any()
    offsets[1] = np.ma.masked
    assert (pixels(fig) != moved).any() and points.get_offsets() is offsets
    assert records()["offsets"].seen is _own_state.NATIVELY
    # Sizes given as an array of floats are the collection's own, compared
    # the same way; other arrays by what they held.
    sizes = np.array([100.0, 200.0, 300.0])
    points.set_sizes(sizes)
    assert points.get_sizes() is sizes
    assert records()["sizes"].seen is _own_state.NATIVELY
    before = pixels(fig)
    sizes[:] = 900.0
    assert (pixels(fig) != before).any() and points.get_sizes() is sizes
    whole = np.array([100, 200, 300])
    points.set_sizes(whole)
    assert records()["sizes"].seen is not _own_state.NATIVELY
    before = pixels(fig)
    whole[:] = 900
    assert (pixels(fig) != before).any() and points.get_sizes() is whole
    # Offsets set anew are another array; the old one, changed or not, is
    # nobody's.
    offsets[0] = (3.0, 3.0)
    points.set_offsets(np.array([[0.5, 0.5], [1.5, 1.5]]))
    pixels(fig)
    assert points.get_offsets().tolist() == [[0.5, 0.5], [1.5, 1.5]]
    held = points.get_offsets()
    held[0] = (2.5, 2.5)
    points._offsets = np.array([[1.0, 1.0]])
    pixels(fig)
    assert points.get_offsets().tolist() == [[1.0, 1.0]]
    # The offsets of a 3-D scatter, which each projection sets, are
    # compared by what they held.
    ax3d = plt.figure().add_subplot(projection="3d")
    cloud = ax3d.scatter([0.0, 1.0], [0.0, 1.0], [0.0, 1.0])
    cloud.get_offsets()
    assert cloud.__dict__[_own_state.RECORDS]["offsets"].seen is not _own_state.NATIVELY


def test_a_lines_columns_are_views_of_its_numbers_while_they_are_held():
    fig, ax = small_axes()
    line, = ax.plot([0.0, 1.0, 2.0], [0.0, 1.0, 0.0])
    before = pixels(fig)
    x, y = line._x, line._y
    assert line._x is x and line._y is y and x.base is line._xy
    y[1] = 3.0
    assert line._xy[1].tolist() == [1.0, 3.0]
    assert (pixels(fig) != before).any()
    del x, y
    # New views of the same array, once the program has none.
    assert line._y.tolist() == [0.0, 3.0, 0.0] and line._y.base is line._xy
    assert pickle.loads(pickle.dumps(fig)).axes[0].lines[0]._y.tolist() == [0.0, 3.0, 0.0]


def axes3d():
    """A small figure and its 3-D Axes, with a fixed view."""
    fig = plt.figure(figsize=(3, 3), dpi=40)
    ax = fig.add_subplot(projection="3d")
    return fig, ax


def test_line_collections_a_3d_axes_makes_keep_their_segments():
    # (USR2: `quiver` and `plot_wireframe` returned collections without.)
    fig, ax = axes3d()
    arrows = ax.quiver([0.0], [0.0], [0.0], [1.0], [1.0], [1.0], length=1, arrow_length_ratio=0.3)
    xs, ys = np.meshgrid([0.0, 1.0], [0.0, 1.0])
    wires = ax.plot_wireframe(xs, ys, xs + ys)
    threed.frame(ax)
    segments = arrows._segments3d
    assert type(segments) is list and len(segments) == 3
    assert np.round(segments[0], 3).tolist() == [[1.0, 1.0, 1.0], [0.0, 0.0, 0.0]]
    assert arrows._segments3d is segments
    assert len(wires._segments3d) == 4
    assert np.asarray(wires._segments3d[0]).tolist() == [[0.0, 0.0, 0.0], [1.0, 0.0, 1.0]]
    # Changed in place, or assigned, they are drawn.
    before = pixels(fig)
    segments[0][0] = (3.0, 3.0, 3.0)
    moved = pixels(fig)
    assert (moved != before).any() and arrows._segments3d is segments
    wires._segments3d = [np.array([[0.0, 0.0, 0.0], [3.0, 3.0, 3.0]])]
    assert (pixels(fig) != moved).any() and len(wires._segments3d) == 1
    read_back = pickle.loads(pickle.dumps(fig)).axes[0].collections[0]
    assert np.asarray(read_back._segments3d[0]).tolist() == np.asarray(segments[0]).tolist()


def test_3d_polygons_patches_and_texts_keep_the_fields_of_their_geometry():
    from pltrs.mpl_toolkits.mplot3d import art3d

    fig, ax = axes3d()
    xs, ys = np.meshgrid(np.linspace(-1.0, 2.0, 4), np.linspace(-1.0, 1.0, 3))
    surface = ax.plot_surface(xs, ys, xs + ys)
    circle = ax.add_patch(patches.Circle((1.0, 1.0), 0.8))
    art3d.patch_2d_to_3d(circle, z=1.0, zdir="x")
    label = ax.text(1.0, 1.0, 1.0, "deep", zdir="x")
    threed.frame(ax)
    faces = surface._faces
    assert faces.shape == (6, 4, 3) and surface._faces is faces
    assert surface._invalid_vertices is False and surface._codes3d is None
    assert surface._zsortfunc is np.average and surface._closed is True
    assert surface._axlim_clip is False and circle._axlim_clip is False
    outline = circle._segment3d
    assert type(outline) is list and type(outline[0]) is tuple and circle._segment3d is outline
    assert label._z == 1.0 and label._dir_vec.tolist() == [1, 0, 0]
    assert not hasattr(surface, "_facecolors2d")
    before = pixels(fig)
    # What the projection left: the colours in the order drawn, the path
    # of the patch.
    assert surface._facecolors2d.shape == (6, 4) and surface._facecolors2d is surface._facecolors2d
    assert len(circle._path2d.vertices) == len(outline)
    faces[:, :, 2] += 1.0
    lifted = pixels(fig)
    assert (lifted != before).any() and surface._faces is faces
    outline[:] = [(1.0, y, z + 1.0) for _, y, z in outline]
    label._dir_vec = np.array([0, 1, 0])
    assert (pixels(fig) != lifted).any()
    assert circle._segment3d is outline and label._dir_vec.tolist() == [0, 1, 0]


def test_a_scatter_reads_back_what_its_projection_left():
    fig, ax = axes3d()
    points = ax.scatter([0.0, 1.0, 2.0], [0.0, 1.0, 2.0], [2.0, 1.0, 0.0], s=200)
    threed.frame(ax)
    assert points._vzs is None and points._z_markers_idx == slice(-1)
    assert points._offset_zordered is None and not hasattr(points, "_data_scale")
    assert points._depthshade_minalpha == pltrs.rcParams["axes3d.depthshade_minalpha"]
    pixels(fig)
    depths = points._vzs
    assert depths.shape == (3,) and points._vzs is depths
    assert sorted(points._z_markers_idx.tolist()) == [0, 1, 2]
    assert points._z_markers_idx.tolist() == np.ma.argsort(depths)[::-1].tolist()
    assert points._offset_zordered.shape == (3, 2) and points._data_scale > 0
    # Colours are new arrays at every call, shaded with depth.
    assert points.get_facecolor() is not points.get_facecolor()
    assert points.get_facecolor().shape == (3, 4)
    pixels(fig)
    assert points._vzs is not depths


def test_what_a_projection_leaves_follows_the_draws_of_the_axes():
    # matplotlib's 3-D Axes projects its visible collections and patches
    # before it draws any: also an animated one, which it does not draw.
    from pltrs.mpl_toolkits.mplot3d import art3d
    fig, ax = axes3d()
    faces = ax.add_collection3d(art3d.Poly3DCollection(
        [[(0, 0, 0), (1, 0, 0), (1, 1, 1)], [(0, 1, 0), (1, 1, 0), (0, 0, 1)]],
        facecolors=["r", "g"]))
    threed.frame(ax)
    faces.set_animated(True)
    assert faces.get_facecolor().shape == (2, 4)
    faces.set_facecolor("none")
    # The colours of the last projection, until the next.
    assert faces.get_facecolor().shape == (2, 4)
    pixels(fig)
    assert faces.get_facecolor().shape == (0, 4)
    assert faces.get_edgecolor().shape == (0, 4)


def test_a_collection_converted_after_a_3d_axes_added_it_is_drawn_in_3d():
    # (USR2's ``art3d_conversions``: ``ax.add_collection(lines)``, then
    # ``line_collection_2d_to_3d(lines)``, was drawn flat.)
    from pltrs.mpl_toolkits.mplot3d import art3d

    def drawn(added_first):
        fig, ax = axes3d()
        made = (
            (collections.LineCollection([[(0, 0), (3, 3)], [(0, 3), (3, 0)]], linewidths=4),
             art3d.line_collection_2d_to_3d, 1.0),
            (collections.PolyCollection([[(0.5, 0.5), (2.5, 0.5), (1.5, 3)]], facecolors="C1"),
             art3d.poly_collection_2d_to_3d, 2.0),
            (collections.PatchCollection([patches.Circle((2.5, 2.5), 0.6)], facecolors="C2"),
             art3d.patch_collection_2d_to_3d, 3.0))
        for collection, convert, z in made:
            if added_first:
                ax.add_collection(collection)
                convert(collection, zs=z)
            else:
                convert(collection, zs=z)
                ax.add_collection(collection)
        threed.frame(ax)
        return ax, pixels(fig)
    ax, after = drawn(True)
    _, before = drawn(False)
    assert [type(collection).__name__ for collection in ax.collections] == [
        "Line3DCollection", "Poly3DCollection", "Patch3DCollection"]
    assert np.array_equal(after, before)
    assert (after[:, :, :3] != 255).any()


def test_a_3d_path_patch_given_no_path_says_what_it_reads_first():
    from pltrs.mpl_toolkits.mplot3d import art3d
    patch = art3d.PathPatch3D(Path([(0, 0), (1, 0), (1, 1)]), zs=(0, 0, 0))
    with pytest.raises(AttributeError, match="'vertices'"):
        patch.set_3d_properties(None, 1)


def program_arrow():
    """qiskit's arrow of its Bloch spheres (`mutability.threed`)."""
    return threed._arrow_class()


def test_a_3d_patch_another_class_draws_keeps_its_segment_and_its_head():
    from pltrs.mpl_toolkits.mplot3d import art3d

    def drawn(style):
        fig, ax = axes3d()
        arrow = ax.add_artist(program_arrow()(
            [0.0, 2.5], [0.0, 2.5], [0.0, 3.0], mutation_scale=40, lw=3, arrowstyle=style,
            color="red"))
        threed.frame(ax)
        return arrow, pixels(fig)
    arrow, headed = drawn("-|>")
    _, bare = drawn("-")
    assert isinstance(arrow, art3d.Patch3D) and isinstance(arrow, patches.FancyArrowPatch)
    assert [tuple(map(float, point)) for point in arrow._segment3d] == [
        (0.0, 0.0, 0.0), (2.5, 2.5, 3.0)]
    assert len(arrow.seen) == 1 and arrow._path2d.vertices.shape == (2, 2)
    assert threed._red(bare) > 20, "the shaft is drawn"
    assert threed._red(headed) > 1.2 * threed._red(bare), "and so is the head"


def test_a_subclass_projecting_a_3d_collection_is_called_at_every_draw():
    from pltrs.mpl_toolkits.mplot3d import art3d

    seen = []

    class Lines(art3d.Line3DCollection):
        def do_3d_projection(self):
            seen.append(len(self._segments3d))
            return super().do_3d_projection()

    fig, ax = axes3d()
    lines = Lines([np.array([[0.0, 0.0, 0.0], [1.0, 2.0, 1.0]])], linewidths=4)
    ax.add_collection3d(lines)
    threed.frame(ax)
    before = pixels(fig)
    assert seen == [1]
    lines.set_segments([np.array([[0.0, 0.0, 0.0], [3.0, 3.0, 3.0]]),
                        np.array([[3.0, 0.0, 0.0], [0.0, 3.0, 3.0]])])
    assert (pixels(fig) != before).any()
    assert seen == [1, 2]


def test_relim_of_a_3d_axes_leaves_the_projected_lines_in_its_data_limits():
    fig, ax = axes3d()
    ax.plot([0.0, 1.0, 2.0], [0.0, 1.0, 2.0], [0.0, 1.0, 0.0])
    ax.scatter([0.0, 3.0], [0.0, 3.0], [0.0, 3.0])
    threed.frame(ax)
    assert ax.dataLim.get_points().tolist() == [[0.0, 0.0], [2.0, 2.0]]
    pixels(fig)
    ax.relim()
    # matplotlib's 2-D relim, inherited: the lines as last drawn, which is
    # their projection, and nothing of the collections.
    (x0, y0), (x1, y1) = ax.dataLim.get_points().tolist()
    assert -0.2 < x0 < x1 < 0.2 and -0.2 < y0 < y1 < 0.2


def test_a_bare_patch_has_no_edge_while_it_is_filled():
    from pltrs.mpl_toolkits.mplot3d import art3d
    from pltrs.path import Path as PathClass

    patch = patches.Patch(facecolor="red")
    assert patch.get_edgecolor() == (0.0, 0.0, 0.0, 0.0)
    patch.set_fill(False)
    assert patch.get_edgecolor() == (0.0, 0.0, 0.0, 1.0)
    patch.set_fill(True)
    assert patch.get_edgecolor() == (0.0, 0.0, 0.0, 0.0)
    outline = PathClass(np.array([[0.0, 0.0], [2.0, 0.5], [1.0, 2.5], [0.0, 0.0]]))
    assert art3d.PathPatch3D(outline, zs=1.0).get_edgecolor() == (0.0, 0.0, 0.0, 0.0)
    assert art3d.PathPatch3D(outline, zs=1.0, edgecolor="blue").get_edgecolor() == (
        0.0, 0.0, 1.0, 1.0)


def test_values_given_are_the_values_handed_back():
    fig, ax = small_axes()
    for every in ((3, 7), slice(2, 60, 9), float("0.125"), (0.05, 0.2), [0, 10, 50],
                  np.arange(101) % 13 == 0):
        line, = ax.plot(np.arange(101.0), marker="o", markevery=every)
        assert line.get_markevery() is every
        line.set_markevery(every)
        assert line.get_markevery() is every
    # After `update_from` two artists share the clip box of the first.
    first, = ax.plot([0, 1], [0, 1])
    second, = ax.plot([0, 1], [1, 0])
    assert first.get_clip_box() is not second.get_clip_box()
    second.update_from(first)
    assert second.get_clip_box() is first.get_clip_box()
    assert pickle.loads(pickle.dumps(fig)).axes[0].lines[-1].get_clip_box() is not None
    # A reset of the tick parameters makes new ticks.
    fig.canvas.draw()
    before = ax.xaxis.get_major_ticks()[1]
    ax.tick_params(axis="x", width=2)
    assert ax.xaxis.get_major_ticks()[1] is before
    ax.tick_params(reset=True, axis="x")
    fig.canvas.draw()
    assert ax.xaxis.get_major_ticks()[1] is not before


def test_what_the_first_draw_reads_follows_a_change_made_before_it():
    def drawn(change):
        fig, ax = small_axes()
        line, = ax.plot([0, 3], [0, 3], lw=30)
        clip = Path(np.array([[0.5, 0.5], [2.5, 0.5], [2.5, 2.5], [0.5, 2.5], [0.5, 0.5]]))
        line.set_clip_path(clip, ax.transData)
        points = ax.scatter([0.5, 2.5], [2.5, 0.5], s=400)
        colours = np.array([[1.0, 0.0, 0.0, 1.0], [0.0, 0.0, 1.0, 1.0]])
        points.set_facecolor(colours)
        if change == "before":
            clip.vertices[:] *= 0.5
            colours[:, 1] = 1.0
        first = pixels(fig)
        if change == "after":
            clip.vertices[:] *= 0.5
            colours[:, 1] = 1.0
        return first, pixels(fig)
    plain, _ = drawn(None)
    changed, _ = drawn("before")
    assert (changed != plain).any()
    # matplotlib reads both at the first draw, and no more.
    first, second = drawn("after")
    assert (first == plain).all() and (second == plain).all()
