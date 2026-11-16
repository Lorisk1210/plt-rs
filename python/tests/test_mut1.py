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

import pickle

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, patches, transforms
from pltrs.path import Path

from mutability import runner

#: Whether the tests run on matplotlib itself (through the alias), where
#: no row may differ.
ON_MATPLOTLIB = pltrs.__name__ == "matplotlib"

REFERENCE = runner.recorded()
EXPECTED = {} if ON_MATPLOTLIB else runner.expected()
GROUPS = runner.groups(REFERENCE["surface"])


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
    ours = runner.measure_here(REFERENCE["surface"], names)
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
