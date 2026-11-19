"""
What the impact report of 2026-11-19 found pltrs drawing or answering
otherwise than matplotlib 3.11 where an artist reads another object late:
- a `Shadow` follows the outline its patch has when it draws (a donut
  pie's shadows are rings), and takes nothing else of the patch late;
- a Gouraud-shaded `tripcolor` draws its triangulation's points,
  triangles and mask as they are when it draws (set, assigned or changed
  in place), the flat one those of when it was made;
- a patch in no figure (the box behind a text, an annotation's arrow, a
  loose patch) is hit-tested through its transform; an annotation's arrow
  is in its annotation's figure, and the arrow and a text's box are fresh
  after their owner's draw;
- ``get_hatch()`` answers the hatch given, ``''`` and numpy strings too.
Every expected value was recorded from matplotlib.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
import pltrs.tri as mtri
from pltrs.backend_bases import MouseEvent
from pltrs.collections import TriMesh
from pltrs.offsetbox import AnnotationBbox, TextArea
from pltrs.patches import Circle, Polygon, Rectangle, Shadow, Wedge
from pltrs.text import Annotation
from pltrs.transforms import Affine2D

#: matplotlib's figure background and the colour of a shadow of a patch
#: in the first colour of the cycle, over it.
WHITE = (255, 255, 255)
SHADE = (131, 145, 154)
BLUE = (31, 119, 180)


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def canvas(xlim=(-2, 2), ylim=(-2, 2)):
    """A figure of 144 pixels at 72 dots per inch, filled by one Axes
    without decorations: a shadow's offset (in points times dpi / 72,
    in data units) is the number given."""
    fig = plt.figure(figsize=(2, 2), dpi=72)
    ax = fig.add_axes([0, 0, 1, 1])
    ax.set_axis_off()
    ax.set_xlim(*xlim)
    ax.set_ylim(*ylim)
    return fig, ax


def pixels(fig, ax, points):
    """The RGB colours Agg draws at the data *points* of *ax*."""
    fig.canvas.draw()
    image = np.asarray(fig.canvas.buffer_rgba())[..., :3]
    return [tuple(int(v) for v in image[image.shape[0] - 1 - int(y), int(x)])
            for x, y in ax.transData.transform(points)]


def test_a_shadow_draws_the_outline_its_patch_has_when_it_draws():
    fig, ax = canvas()
    wedge = ax.add_patch(Wedge((0, 0), 1, 0, 90))
    ax.add_patch(Shadow(wedge, 0.2, -0.2))
    wedge.set_width(0.3)
    rectangle = ax.add_patch(Rectangle((-1.5, -1.5), 1, 1))
    ax.add_patch(Shadow(rectangle, 0.2, -0.2))
    rectangle.set_width(0.2)
    circle = ax.add_patch(Circle((1, -1), .3))
    ax.add_patch(Shadow(circle, 0.2, -0.2))
    circle.set_center((-1, 1))
    # The wedge's hole and the wedge's ring; the narrowed rectangle's
    # shadow and where the wide one's was; the moved circle's and where it
    # was.
    assert pixels(fig, ax, [(0.5, 0.15), (0.95, 0.1), (-1.2, -1.6), (-0.6, -1.0),
                            (-0.7, 0.7), (1.2, -1.2)]) == [
        WHITE, BLUE, SHADE, WHITE, SHADE, WHITE]


def test_a_shadow_follows_its_patch_changed_in_place():
    fig, ax = canvas()
    triangle = ax.add_patch(Polygon([[-1, -1], [0, -1], [-1, 0]]))
    ax.add_patch(Shadow(triangle, 0.2, -0.2))
    triangle.get_path().vertices[1] = [1.5, -1]
    assert pixels(fig, ax, [(1.4, -1.15), (-0.3, -1.15)]) == [SHADE, SHADE]


def shadowed_square():
    """A unit square at the origin and its shadow, drawn 0.2 data units
    right and down."""
    fig, ax = canvas()
    square = ax.add_patch(Rectangle((0, 0), 1, 1))
    shadow = ax.add_patch(Shadow(square, 0.2, -0.2))
    return fig, ax, square, shadow


#: Below the square's right half, in its shadow; below where a square
#: moved left and down would shadow; above the square; beyond the
#: shadow's right edge.
LATE_POINTS = [(1.1, -0.1), (-1.4, -1.6), (0.5, 1.5), (1.5, -0.1)]


def test_a_shadow_keeps_the_transform_its_patch_had_when_it_was_made():
    fig, ax, square, _ = shadowed_square()
    square.set_transform(Affine2D().translate(-1.5, -1.5) + ax.transData)
    assert pixels(fig, ax, LATE_POINTS) == [SHADE, WHITE, WHITE, WHITE]


def test_a_shadow_follows_its_patch_removed_from_the_axes():
    fig, ax, square, _ = shadowed_square()
    square.remove()
    square.set_height(1.8)
    assert pixels(fig, ax, LATE_POINTS) == [SHADE, WHITE, SHADE, WHITE]


def test_a_shadow_stays_visible_when_its_patch_is_hidden():
    fig, ax, square, _ = shadowed_square()
    square.set_visible(False)
    assert pixels(fig, ax, LATE_POINTS) == [SHADE, WHITE, WHITE, WHITE]


def test_a_shadow_takes_no_offset_through_set():
    _, _, _, shadow = shadowed_square()
    with pytest.raises(AttributeError, match="unexpected keyword argument 'ox'"):
        shadow.set(ox=1)


@pytest.mark.parametrize("kwargs", [
    dict(shadow=True, wedgeprops=dict(width=0.4)),
    dict(shadow=dict(ox=-0.04, oy=-0.04, shade=0.9), explode=[0.1, 0, 0.2],
         radius=0.9, startangle=45, frame=True, wedgeprops=dict(width=0.3)),
])
def test_a_donut_pie_casts_ring_shadows(kwargs):
    fig, ax = canvas((-1.5, 1.5), (-1.5, 1.5))
    ax.pie([3, 2, 1], **kwargs)
    ax.set_xlim(-1.5, 1.5)
    ax.set_ylim(-1.5, 1.5)
    # The hole, where full sectors' shadows would show.
    assert pixels(fig, ax, [(0, 0), (0.1, -0.1), (-0.15, 0.1)]) == [WHITE] * 3


X = np.array([0, 1, 0, 1, 2, 2.])
Y = np.array([0, 0, 1, 1, 0, 1.])
#: The centres of the triangles of `grid`, in their order.
CENTRES = [(1 / 3, 1 / 3), (2 / 3, 2 / 3), (4 / 3, 1 / 3), (5 / 3, 2 / 3)]


def grid():
    """Four triangles over two unit squares, and a figure filled by an
    Axes showing them."""
    fig = plt.figure(figsize=(2, 2), dpi=50)
    ax = fig.add_axes([0, 0, 1, 1])
    ax.set_axis_off()
    triangulation = mtri.Triangulation(X, Y, [[0, 1, 2], [1, 3, 2], [1, 4, 3], [4, 5, 3]])
    return fig, ax, triangulation


def test_a_gouraud_tripcolor_hides_what_the_mask_hides_when_it_draws():
    fig, ax, triangulation = grid()
    ax.tripcolor(triangulation, X + Y, shading='gouraud')
    ax.set_xlim(0, 2)
    ax.set_ylim(0, 1)
    triangulation.set_mask([True, False, True, False])
    assert pixels(fig, ax, CENTRES) == [
        WHITE, (50, 130, 134), WHITE, (120, 199, 92)]
    triangulation.set_mask(None)
    assert pixels(fig, ax, CENTRES) == [
        (55, 69, 122), (50, 130, 134), (51, 156, 128), (120, 199, 92)]


def test_a_trimesh_made_alone_follows_its_mask():
    fig, ax, triangulation = grid()
    mesh = TriMesh(triangulation)
    mesh.set_array(X + Y)
    ax.add_collection(mesh)
    ax.set_xlim(0, 2)
    ax.set_ylim(0, 1)
    triangulation.set_mask([False, True, False, True])
    assert pixels(fig, ax, CENTRES) == [
        (55, 69, 122), WHITE, (51, 156, 128), WHITE]


#: Two points beyond the triangles' centres: inside the right square's
#: upper corner and the left square's.
CORNERS = [(1.9, 0.9), (0.9, 0.9)]


def drawn_then_changed(change):
    """The colours at `CENTRES` and `CORNERS` of a Gouraud `tripcolor` of a
    triangulation of its own points, drawn, then *change(triangulation)*
    made, then drawn again."""
    fig, ax, _ = grid()
    triangulation = mtri.Triangulation(
        X.copy(), Y.copy(), [[0, 1, 2], [1, 3, 2], [1, 4, 3], [4, 5, 3]])
    ax.tripcolor(triangulation, X + Y, shading='gouraud')
    ax.set_xlim(0, 2)
    ax.set_ylim(0, 1)
    fig.canvas.draw()
    change(triangulation)
    return pixels(fig, ax, CENTRES + CORNERS)


def test_a_gouraud_tripcolor_draws_points_moved_in_place():
    def move(triangulation):
        triangulation.x[3] = 0.6
        triangulation.y[5] = 0.5
    assert drawn_then_changed(move) == [
        (55, 69, 122), (51, 147, 130), (52, 166, 125), WHITE, WHITE, WHITE]


def test_a_gouraud_tripcolor_draws_a_mask_assigned_or_triangles_changed_in_place():
    def assign_mask(triangulation):
        triangulation.mask = np.array([False, True, False, True])

    def turn_triangle(triangulation):
        triangulation.triangles[3] = [4, 5, 1]
    assert drawn_then_changed(assign_mask) == [
        (55, 69, 122), WHITE, (51, 156, 128), WHITE, WHITE, WHITE]
    assert drawn_then_changed(turn_triangle) == [
        (55, 69, 122), (50, 130, 134), (104, 147, 110), (182, 188, 72),
        (230, 218, 48), (52, 167, 125)]


def test_a_flat_tripcolor_keeps_the_triangles_unmasked_when_it_was_made():
    fig, ax, triangulation = grid()
    ax.tripcolor(triangulation, X + Y)
    ax.set_xlim(0, 2)
    ax.set_ylim(0, 1)
    triangulation.set_mask([True, False, True, False])
    assert pixels(fig, ax, CENTRES) == [
        (68, 1, 84), (42, 120, 142), (34, 168, 132), (253, 231, 37)]


def hits(fig, contains):
    """How many points of a 3-pixel grid over the canvas *contains*
    (a ``contains`` method) says are in its artist."""
    width, height = fig.canvas.get_width_height()
    return sum(bool(contains(MouseEvent("motion_notify_event", fig.canvas, x, y))[0])
               for x in range(0, width, 3) for y in range(0, height, 3))


def test_a_rotated_boxed_text_and_an_annotation_arrow_are_hit_in_their_patches():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    text = ax.text(0.3, 0.4, 'Hello', fontsize=14, rotation=30, bbox=dict(fc='y'))
    note = ax.annotate('note', (0.9, 0.2), (0.5, 0.8), arrowprops=dict(arrowstyle='-|>'))
    fig.canvas.draw()
    assert [hits(fig, text.contains), hits(fig, text.get_bbox_patch().contains),
            hits(fig, note.contains), hits(fig, note.arrow_patch.contains)] == [
        261, 212, 74, 19]


def test_a_patch_in_no_figure_is_hit_through_its_transform():
    fig = plt.figure()

    def inside(patch, x, y):
        return patch.contains(MouseEvent("motion_notify_event", fig.canvas, x, y))

    square = Rectangle((0, 0), 10, 10)
    assert [inside(square, 5, 5), inside(square, 15, 5), inside(square, 10.5, 5)] == [
        (True, {}), (False, {}), (True, {})]
    turned = Rectangle((0, 0), 10, 10, transform=Affine2D().rotate_deg(30).translate(100, 100))
    assert [inside(turned, 100, 105)[0], inside(turned, 105, 99)[0]] == [True, False]


def test_hatches_read_back_as_given():
    fig, ax = plt.subplots()
    listed = ax.bar(['a', 'b', 'c'], [1, -2, 3], hatch=['/', '', 'x'])
    empty = ax.bar([1, 2], [1, 2], hatch='')
    unhatched = ax.bar([1, 2], [1, 2])
    wedges, _ = ax.pie([1, 2], hatch=['', '/'])
    layers = ax.stackplot([0, 1], [1, 2], hatch='')
    assert [repr(patch.get_hatch()) for patch in [*listed, *empty, *unhatched, *wedges]] == [
        "np.str_('/')", "np.str_('')", "np.str_('x')", "np.str_('')", "np.str_('')",
        "None", "None", "np.str_('')", "np.str_('/')"]
    assert repr(layers[0].get_hatch()) == "np.str_('')"
    square = Rectangle((0, 0), 1, 1, hatch='/')
    square.set(hatch='')
    assert (repr(square.get_hatch()), repr(square.properties()['hatch'])) == ("''", "''")
    square.set_hatch(None)
    assert square.get_hatch() is None
    dots = ax.scatter([0], [0], hatch='/')
    dots.set_hatch('')
    assert repr(dots.get_hatch()) == "''"


def test_an_annotation_arrow_is_in_the_annotations_figure():
    fig = plt.figure()
    left, right = fig.subfigures(1, 2)
    ax = right.subplots()
    loose = Annotation('n', (0.5, 0.5), (0.1, 0.8), arrowprops=dict(arrowstyle='->'))
    assert loose.arrow_patch.get_figure() is None
    ax.add_artist(loose)
    note = ax.annotate('n', (0.2, 0.2), (0.6, 0.6), arrowprops=dict(arrowstyle='->'))
    for arrow in (loose.arrow_patch, note.arrow_patch):
        assert (arrow.figure, arrow.get_figure(root=True), arrow.axes) == (right, fig, None)
        assert arrow.figure.canvas is fig.canvas
    box = AnnotationBbox(TextArea('b'), (0.3, 0.3), xybox=(0.7, 0.2),
                         arrowprops=dict(arrowstyle='->'))
    ax.add_artist(box)
    assert box.arrow_patch.get_figure() is right


def test_an_annotation_arrow_and_a_text_box_are_fresh_after_their_owners_draw():
    fig, ax = plt.subplots()
    note = ax.annotate('n', (0.2, 0.2), (0.6, 0.6), arrowprops=dict(arrowstyle='->'),
                       bbox=dict(fc='y'))
    text = ax.text(0.1, 0.1, 'boxed', bbox=dict(fc='c'))
    parts = (note.arrow_patch, note.get_bbox_patch(), text.get_bbox_patch())
    assert [part.stale for part in parts] == [True, True, True]
    fig.canvas.draw()
    assert [part.stale for part in parts] == [False, False, False]
    note.arrow_patch.set_color('r')
    assert note.arrow_patch.stale
    note.set_visible(False)
    fig.canvas.draw()
    assert note.arrow_patch.stale
