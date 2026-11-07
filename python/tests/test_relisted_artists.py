"""
Artists given to an Axes or a figure that already hold them elsewhere, as
the differential tester's ``orders`` and api programs compare them with
matplotlib 3.11: matplotlib's ``add_artist`` lists an artist of another
figure before giving it the figure fails, moves an artist removed from
another figure, lists a part of the Axes (a spine, a title) or an artist
of the figure once more, and the removals and draws that follow fail as
matplotlib's do. Each expected value is recorded from matplotlib.
"""

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors, lines, patches
from pltrs.offsetbox import AnchoredText, AnnotationBbox, OffsetImage, TextArea
from pltrs.quiver import QuiverKey

FIGURES = r"^Can not put single artist in more than one figure$"
NOT_IN_LIST = r"^list.remove\(x\): x not in list$"


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_an_artist_of_another_figure_is_listed_before_add_artist_fails():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    label = ax.get_xticklabels()[0]
    fig2, ax2 = plt.subplots()
    with pytest.raises(RuntimeError, match=FIGURES):
        fig2.add_artist(label)
    with pytest.raises(RuntimeError, match=FIGURES):
        ax2.add_artist(label)
    assert label.axes is ax2 and label.figure is fig
    assert label in ax2.get_children()
    label.remove()
    assert label not in ax2.get_children()


def test_the_other_add_methods_give_the_figure_first():
    fig, ax = plt.subplots()
    table = ax.table([['a', 'b'], ['c', 'd']], loc='center')
    cell = table[0, 1]
    fig2, ax2 = plt.subplots()
    with pytest.raises(RuntimeError, match=FIGURES):
        ax2.add_table(table)
    with pytest.raises(RuntimeError, match=FIGURES):
        ax2.add_patch(cell)
    with pytest.raises(RuntimeError, match=FIGURES):
        ax2.add_line(ax.plot([1, 2])[0])


def test_an_axes_removed_from_its_figure_moves_into_another_axes():
    fig = plt.figure()
    other = fig.add_subplot(projection='polar')
    fig.canvas.draw()
    other.remove()
    fig2, ax2 = plt.subplots()
    ax2.add_artist(other)
    assert other.axes is ax2 and other.figure is fig2
    assert other in ax2.get_children()
    other.remove()
    assert other.axes is None and other.figure is None
    fig2.canvas.draw()


def test_a_figure_image_removed_moves_into_an_axes_of_another_figure():
    fig = plt.figure()
    image = fig.figimage(np.ones((2, 2)))
    image.remove()
    fig2, ax2 = plt.subplots()
    ax2.add_artist(image)
    assert image.axes is ax2 and image.figure is fig2
    image.remove()
    with pytest.raises(ValueError, match=NOT_IN_LIST):
        image.remove()


def test_a_spine_listed_again_and_removed_fails_the_next_draws():
    fig, ax = plt.subplots()
    spine = ax.spines['left']
    fig.canvas.draw()
    with pytest.raises(NotImplementedError, match="^cannot remove artist$"):
        spine.remove()
    ax.add_artist(spine)
    assert spine.axes is ax
    assert ax.get_children().count(spine) == 2
    spine.remove()
    assert spine.axes is None and spine.figure is None
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'viewLim'"):
        fig.canvas.draw()
    ax.add_artist(spine)
    fig.canvas.draw()


def test_a_title_listed_again_and_removed_fails_the_draws_it_is_visible_in():
    fig, ax = plt.subplots()
    title = ax.set_title('title')
    ax.add_artist(title)
    title.remove()
    assert title.axes is None and title.figure is None
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'dpi'"):
        fig.canvas.draw()
    title.set_visible(False)
    fig.canvas.draw()


def test_a_3d_axis_listed_again_and_removed_fails_the_next_draws():
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    axis = ax.xaxis
    axis.set_visible(False)
    ax.add_artist(axis)
    axis.remove()
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute '_get_scaled_limits'"):
        fig.canvas.draw()


def test_removed_offset_boxes_and_indicators_come_back():
    fig, ax = plt.subplots()
    ins = ax.inset_axes([0.6, 0.6, 0.3, 0.3])
    anchored = ax.add_artist(AnchoredText('anchored', loc='upper left'))
    box = ax.add_artist(AnnotationBbox(TextArea('box'), (0.5, 0.5)))
    indicator = ax.indicate_inset_zoom(ins)
    fig.canvas.draw()
    for artist in (anchored, box, indicator):
        artist.remove()
        ax.add_artist(artist)
        assert artist.axes is ax
        artist.remove()
        with pytest.raises(ValueError, match=NOT_IN_LIST):
            artist.remove()
    fig.canvas.draw()


def test_a_box_of_an_annotation_box_is_listed_again():
    fig, ax = plt.subplots()
    image = OffsetImage(np.ones((3, 3)), zoom=2)
    ax.add_artist(AnnotationBbox(image, (0.5, 0.5)))
    fig.canvas.draw()
    ax.add_artist(image)
    assert image.axes is ax
    image.remove()


def test_an_artist_of_the_figure_is_listed_in_an_axes_too():
    fig, ax = plt.subplots()
    line = lines.Line2D([0, 1, 2], [2, 0, 1])
    fig.add_artist(line)
    ax.add_artist(line)
    assert line.axes is ax
    ax.add_line(line)
    assert line in fig.artists and line in ax.get_children()


def test_an_artist_of_an_axes_is_listed_in_its_figure_too():
    fig, ax = plt.subplots()
    line = ax.plot([1, 2])[0]
    assert fig.add_artist(line) is line
    assert line in fig.artists and line in ax.lines
    fig.canvas.draw()


def test_a_figure_or_subfigure_joins_no_other_figure():
    message = ("^The parent and root figures of a \\(Sub\\)Figure are set at "
               "instantiation and cannot be changed.$")
    fig = plt.figure()
    sub = fig.subfigures(1, 2)[0]
    fig2 = plt.figure()
    for artist in (fig, sub):
        with pytest.raises(ValueError, match=message):
            fig2.add_artist(artist)


def test_a_cleared_figure_keeps_its_super_labels_and_subfigures():
    fig = plt.figure()
    title = fig.suptitle('super')
    sub = fig.subfigures(1, 2)[0]
    fig.canvas.draw()
    fig.clear()
    assert title.figure is fig
    with pytest.raises(ValueError, match=NOT_IN_LIST):
        title.remove()
    title.get_window_extent()
    sub.remove()


def test_a_removed_connection_patch_needs_its_axes_to_be_measured():
    fig, ax = plt.subplots()
    patch = ax.add_patch(patches.ConnectionPatch((0.1, 0.1), (0.9, 0.9), 'data'))
    fig.canvas.draw()
    patch.remove()
    for measure in (patch.get_window_extent, patch.get_tightbbox):
        with pytest.raises(AttributeError,
                           match="'NoneType' object has no attribute 'transData'"):
            measure()
    ax.add_patch(patch)
    patch.get_window_extent()


def test_removing_a_colorbar_of_a_removed_mappable_fails_at_its_end():
    fig1, ax1 = plt.subplots()
    fig2, ax2 = plt.subplots()
    image = ax1.imshow(np.ones((3, 3)))
    with pytest.warns(UserWarning, match="^Adding colorbar to a different Figure"):
        colorbar = fig2.colorbar(image)
    image.remove()
    with pytest.raises(AttributeError,
                       match="'NoneType' object has no attribute 'set_subplotspec'"):
        colorbar.remove()
    assert colorbar.ax not in fig2.axes


def test_an_artist_told_it_has_no_axes_reads_none():
    fig, ax = plt.subplots()
    ins = ax.inset_axes([0.6, 0.6, 0.3, 0.3])
    indicator = ax.indicate_inset_zoom(ins)
    indicator.axes = None
    assert indicator.axes is None and indicator.rectangle.axes is None
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'xaxis'"):
        fig.canvas.draw()
    indicator.axes = ax
    assert indicator.axes is ax
    fig.canvas.draw()


def test_a_figure_pyplot_never_numbered_says_so():
    fig = plt.figure()
    other = type(fig)()
    with pytest.raises(AttributeError, match=(
            "^'Figure' object has no attribute 'number'. In the future this"
            "will change to returning 'None' instead.$")):
        other.number


def test_a_quiver_key_is_added_by_quiverkey_only():
    fig, ax = plt.subplots()
    q = ax.quiver([0], [0], [1], [1])
    alone = QuiverKey(q, 0.5, 0.5, 2, 'k')
    assert alone.figure is None and alone.axes is None
    assert alone not in ax.get_children()
    key = ax.quiverkey(q, 0.5, 0.5, 2, 'k')
    assert key.axes is ax and key in ax.get_children()
    fig.canvas.draw()


def test_a_quiver_key_keeps_coordinates_and_colours_until_drawn():
    fig, ax = plt.subplots()
    q = ax.quiver([0], [0], [1], [1])
    key = ax.quiverkey(q, 0.5, 0.5, 1, 'k', coordinates='nope', color='notacolor')
    assert key.coord == 'nope' and key.color == 'notacolor'
    with pytest.raises(ValueError, match=(
            "^'nope' is not a valid value for coordinates. Supported values are "
            "'data', 'axes', 'figure', 'inches'$")):
        fig.canvas.draw()
    key.coord = 'axes'
    key.remove()
    colored = ax.quiverkey(q, 0.5, 0.5, 1, 'k', color='notacolor')
    with pytest.raises(ValueError, match="^'notacolor' is not a valid color value.$"):
        fig.canvas.draw()
    colored.remove()
    with pytest.raises(KeyError, match="^'nope'$"):
        QuiverKey(q, 0.5, 0.5, 1, 'k', labelpos='nope')
    assert colors.is_color_like('k')


def test_an_axes_added_back_is_the_last_of_its_figure():
    fig, ax = plt.subplots()
    ax2 = fig.add_subplot(2, 2, 4)
    colorbar = fig.colorbar(ax.imshow(np.ones((2, 2))))
    fig.delaxes(ax)
    ax3 = fig.add_subplot(2, 2, 1)
    fig.add_axes(ax)
    ax4 = fig.add_subplot(2, 2, 2)
    assert fig.axes == [ax2, colorbar.ax, ax3, ax, ax4]
    fig.delaxes(ax2)
    fig.add_subplot(ax2)
    assert fig.axes == [colorbar.ax, ax3, ax, ax4, ax2]
