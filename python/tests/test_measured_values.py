"""
Window extents, transforms, positions and limits as matplotlib 3.11 gives
them before and after a draw: what the differential tester (``cargo xtask
diff``) found pltrs answering otherwise. matplotlib measures most artists
in the figure as it is (stale view limits, positions no layout engine set
yet) and updates some state only when it draws. Every expected value was
recorded from matplotlib.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import patches
from pltrs.mpl_toolkits.mplot3d import proj3d
from pltrs.offsetbox import AnchoredOffsetbox, AnnotationBbox, OffsetImage, TextArea


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def rounded(values, digits=4):
    """*values*, flattened, as floats rounded to *digits*."""
    return tuple(round(float(v), digits) for v in np.ravel(values))


def kinds(values):
    """The type names of *values*."""
    return tuple(type(v).__name__ for v in values)


def test_annotations_are_measured_as_they_are():
    fig, ax = plt.subplots()
    note = ax.annotate("a", (0.2, 0.2), xytext=(10, 10), textcoords="offset points")
    assert type(note.get_transform()).__name__ == "IdentityTransform"
    assert note.is_transform_set()
    ax.plot([0, 4], [0, 4])
    note = ax.annotate("a", (1, 1))
    # At x = 1 of the unit view, which autoscaling has not changed yet.
    assert rounded(note.get_window_extent().bounds) == (576.0, 419.0634, 9.0, 13.8889)
    fig.canvas.draw()
    assert rounded(note.get_window_extent().bounds) == (215.2727, 150.2634, 9.0, 13.8889)


def test_annotation_boxes_are_placed_when_measured():
    fig, ax = plt.subplots()
    ax.plot([0, 10], [0, 10])
    box = AnnotationBbox(TextArea("box"), (5, 5))
    with pytest.raises(AttributeError):
        box.get_window_extent()
    ax.add_artist(box)
    assert rounded(box.get_window_extent().bounds, 2) == (2541.66, 1888.08, 36.69, 25.45)
    box.set_fontsize(14)
    assert rounded(box.get_window_extent().bounds, 2) == (2539.43, 1885.85, 41.13, 29.89)
    fig.canvas.draw()
    assert rounded(box.get_window_extent().bounds, 2) == (307.43, 222.65, 41.13, 29.89)


def test_offset_images_and_anchored_boxes():
    fig, ax = plt.subplots()
    image = OffsetImage(np.zeros((4, 6)), zoom=2)
    ax.add_artist(AnchoredOffsetbox(loc=3, child=image))
    assert rounded(image.get_offset()) == (0.0, 0.0)
    assert rounded(image.get_window_extent().bounds, 2) == (0.0, 0.0, 16.67, 11.11)
    fig.canvas.draw()
    assert rounded(image.get_window_extent().bounds, 2) == (92.5, 65.3, 16.67, 11.11)
    box = AnchoredOffsetbox(loc=1, child=TextArea("x"))
    ax.add_artist(box)
    assert rounded(box.get_bbox_to_anchor().bounds) == (80.0, 52.8, 496.0, 369.6)


def test_legends_are_measured_as_they_are():
    fig, ax = plt.subplots()
    ax.plot([0, 1], label="a")
    legend = ax.legend()
    fig.set_layout_engine("constrained")
    assert rounded(legend.get_window_extent().bounds, 2) == (86.94, 390.01, 59.0, 25.45)
    fig.canvas.draw()
    assert rounded(legend.get_window_extent().bounds, 2) == (42.83, 443.44, 59.0, 25.45)


def test_spine_paths_span_the_view_once_drawn():
    fig, ax = plt.subplots()
    left = ax.spines["left"]
    # Drawing the y-axis measures the left spine, animated or not.
    left.set_animated(True)
    assert rounded(left.get_path().vertices) == (0.0, 0.999, 0.0, 0.999)
    ax.set_ylim(2, 3)
    fig.canvas.draw()
    assert rounded(left.get_path().vertices) == (0.0, 2.0, 0.0, 3.0)
    assert rounded(ax.spines["bottom"].get_path().vertices) == (0.0, 0.0, 1.0, 0.0)


def test_text_positions_keep_the_types_given():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    assert kinds(ax.text(1, 2, "s").get_position()) == ("int", "int")
    assert kinds(ax.annotate("a", (1, 2), xytext=(3, 4)).get_position()) == ("int", "int")
    assert kinds(fig.text(0, 1, "f").get_position()) == ("int", "int")
    assert kinds(ax.get_xticklabels()[0].get_position()) == ("float64", "int")
    assert kinds(ax.xaxis.label.get_position()) == ("float", "int")
    assert kinds(ax.yaxis.label.get_position()) == ("int", "float")
    assert kinds(ax.title.get_position()) == ("float", "float")


def test_tick_labels_outside_the_view_are_measured():
    fig, ax = plt.subplots()
    ax.plot([0, 2], [0, 2])
    label = ax.get_xticklabels()[0]
    assert label.get_text() == "\N{MINUS SIGN}0.25"
    assert rounded(label.get_window_extent().bounds, 2) == (24.68, 29.19, 43.0, 13.89)


def test_limits_and_their_types():
    fig, ax = plt.subplots()
    ax.plot([0, 10], [0, 10])
    ax.errorbar([1], [1], yerr=0.5)
    # Errorbars without limit marks leave autoscaling pending.
    assert rounded(ax._viewLim.bounds) == (0.0, 0.0, 1.0, 1.0)
    tick = ax.xaxis.get_major_ticks()[0]
    assert kinds(ax.set_xlim(0, 10)) == ("float", "float")
    assert kinds(ax.get_xlim()) == ("float64", "float64")
    assert kinds(tick.get_view_interval()) == ("float64", "float64")


def test_categories_after_other_data_leave_the_view_stale():
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    fig, ax = plt.subplots()
    ax.stairs([1, 2], baseline=None, orientation="horizontal")
    ax.quiver(z, z)
    ax.bar(["a", "b"], [1, 2])
    assert rounded(ax._viewLim.bounds) == (0.95, -0.1, 1.1, 2.2)
    # The change of unit recomputed the data limits, the quiver's included.
    assert rounded(ax.dataLim.bounds) == (-0.4, 0.0, 5.4, 4.0)
    assert rounded(ax.get_xlim(), 2) == (-0.67, 5.27)
    fig, ax = plt.subplots()
    ax.bar(["a", "b"], [1, 2])
    assert rounded(ax._viewLim.bounds) == (-0.055, 0.0, 0.11, 1.0)


def test_relim_counts_quiver_positions():
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    fig, ax = plt.subplots()
    ax.quiver(z, z)
    ax.relim()
    assert rounded(ax.dataLim.bounds) == (0.0, 0.0, 5.0, 4.0)


def test_collection_limits_and_transforms():
    fig, ax = plt.subplots()
    filled = ax.fill_between([0, 1, 2], [1, 3, 1], 0.5)
    assert rounded(filled.get_datalim(ax.transData).bounds) == (0.0, 0.5, 2.0, 2.5)
    assert rounded(ax.dataLim.bounds) == (0.0, 0.5, 2.0, 2.5)
    points = ax.scatter([1], [2], s=[36])
    # Sizes are scaled to the resolution by the draw.
    assert rounded(points.get_transforms())[0] == 6.0
    fig.canvas.draw()
    assert rounded(points.get_transforms())[0] == 8.3333


def test_quiver_transforms():
    fig, ax = plt.subplots()
    q = ax.quiver([0, 1], [0, 1], [1, -1], [1, 2])
    key = ax.quiverkey(q, 0.8, 0.9, 1, "one")
    assert type(q.get_transform()).__name__ == "CompositeGenericTransform"
    assert type(key.get_transform()).__name__ == "CompositeGenericTransform"
    fig.canvas.draw()
    assert rounded(q.get_transform().get_matrix(), 3)[:2] == (496.0, 0.0)


def test_patch_geometry():
    circle = patches.Circle((1, 2), 3)
    assert rounded(circle.get_patch_transform().get_matrix())[:6] == (3, 0, 1, 0, 3, 2)
    circle.set_radius(1)
    assert rounded(circle.get_patch_transform().get_matrix())[:6] == (1, 0, 1, 0, 1, 2)
    fig, ax = plt.subplots()
    assert rounded(ax.axline((0, 0), (2, 3)).get_xydata()) == (0.0, 0.0, 1.0, 1.0)
    arrow = ax.add_patch(patches.FancyArrowPatch(
        (0.2, 0.2), (0.8, 0.8), arrowstyle="-|>", mutation_scale=40,
        transform=ax.transAxes))
    # One pixel per point until a draw sets the arrow's resolution.
    assert rounded(arrow.get_window_extent().bounds, 2) == (180.81, 127.92, 293.49, 218.7)
    fig.canvas.draw()
    assert rounded(arrow.get_window_extent().bounds, 2) == (181.43, 128.38, 291.9, 217.51)
    image = ax.imshow(np.zeros((2, 3)), extent=[0, 1, 0, 2])
    assert image.get_extent() == [0, 1, 0, 2]


def test_table_cells_until_and_after_placing():
    fig, ax = plt.subplots()
    cell = ax.table([["a", "b"], ["c", "d"]], loc="center")[0, 1]
    assert cell.get_xy() == (0, 0)
    assert rounded(cell.get_bbox().bounds) == (0.0, 0.0, 0.5, 0.0451)
    assert cell.is_transform_set()
    cell.set_x(0.2)
    assert cell.get_xy() == (0.2, 0)
    fig.canvas.draw()
    assert kinds(cell.get_xy()) == ("float64", "float64")
    assert rounded(cell.get_xy()) == (0.5, 0.5)
    cell.set_xy((1, 2))
    assert cell.get_xy() == (1, 2)


def test_figure_tight_box_with_subfigures():
    fig = plt.figure()
    left, right = fig.subfigures(1, 2)
    left.subplots().plot([1, 2])
    right.text(0.5, 0.5, "x")
    assert rounded(fig.get_tightbbox().bounds, 2) == (0.08, 0.29, 4.8, 3.93)


def test_positions_of_axes_with_locators():
    fig, ax = plt.subplots()
    inset = ax.inset_axes([0.5, 0.5, 0.25, 0.25])
    inset.set_box_aspect(2)
    sec = ax.secondary_xaxis("top")
    secy = ax.secondary_yaxis(0.3)
    # Only drawing places them and applies their aspect.
    assert rounded(inset.get_position().bounds) == (0.5125, 0.495, 0.1938, 0.1925)
    assert rounded(sec.get_position().bounds) == (0.0, 1.0, 1.0, 0.0001)
    assert rounded(secy.get_position().bounds) == (0.0, 1.0, 0.0001, 1.0)
    fig.canvas.draw()
    assert rounded(inset.get_position().bounds) == (0.5733, 0.495, 0.0722, 0.1925)
    assert rounded(sec.get_position().bounds) == (0.125, 0.88, 0.775, 0.0)
    assert rounded(sec.get_position(original=True).bounds) == (0.0, 1.0, 1.0, 0.0001)
    # The figure places the axes it does not draw too.
    fig, ax = plt.subplots()
    sec = ax.secondary_xaxis("top")
    sec.set_animated(True)
    fig.canvas.draw()
    assert rounded(sec.get_position().bounds) == (0.125, 0.88, 0.775, 0.0)


def test_polar_axes():
    fig = plt.figure()
    ax = fig.add_subplot(projection="polar")
    assert type(ax.get_rlabel_position()).__name__ == "float64"
    with pytest.raises(NotImplementedError):
        ax.set_xscale("log")


def test_3d_axes_positions_and_projections():
    fig = plt.figure()
    ax = fig.add_subplot(projection="3d")
    assert rounded(ax.get_position().bounds) == (0.2238, 0.11, 0.5775, 0.77)
    ax.set_anchor("W")
    line = ax.plot([0, 1, 2], [0, 1, 0], [0, 1, 2])[0]
    drawn = ax.plot([0, 1, 2], [0, 1, 0], [0, 1, 2])[0]
    line.set_animated(True)
    fig.canvas.draw()
    assert rounded(ax.get_position().bounds) == (0.125, 0.11, 0.5775, 0.77)
    # The draw projects the lines it draws, not the animated one.
    assert rounded(line.get_xdata()) == (0.0, 1.0, 2.0)
    assert rounded(drawn.get_xdata(), 3) == (-0.071, 0.025, 0.021)
    assert kinds(proj3d.proj_transform(1, 1, 1, ax.get_proj())) == ("float64",) * 3
    many = proj3d.proj_transform([1, 2], [1, 2], [0, 1], ax.get_proj())
    assert [np.shape(v) for v in many] == [(2,)] * 3
