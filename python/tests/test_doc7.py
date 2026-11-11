"""
What the differential tester's programs using the API as documented
(orders, api, pickling, rcParams) still found pltrs doing otherwise than
matplotlib 3.11 after two rounds: parts measured once removed or cleared,
axes sharing an axis across figures, hosts placed by locators, the
colours a collection last mapped, clip paths given to plotting methods,
table cells before their table places them. Each expected value is
recorded from matplotlib.
"""

import io
import pickle

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors, lines, patches, text, ticker
from pltrs.mpl_toolkits.axes_grid1.inset_locator import inset_axes

X = np.linspace(0, 2 * np.pi, 7)
Y = np.sin(X)
VALUES = np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def roundtrip(obj):
    return pickle.loads(pickle.dumps(obj, pickle.HIGHEST_PROTOCOL))


def extents(box):
    return np.asarray(box.extents)


def test_a_cleared_figures_axes_still_measures_its_parts():
    fig, ax = plt.subplots()
    spine = ax.spines['left']
    fig.canvas.draw()
    fig.clear()
    assert spine.axes is ax and spine.figure is fig
    np.testing.assert_allclose(
        extents(spine.get_window_extent()), [75.13888889, 52.8, 80.0, 422.4])
    np.testing.assert_allclose(
        extents(ax.get_tightbbox()), [48.27777778, 29.18888889, 587.0, 427.67615017])
    np.testing.assert_allclose(
        extents(ax.xaxis.get_tightbbox()), [69.0, 29.18888889, 587.0, 43.07777778])
    assert [label.get_text() for label in ax.get_xticklabels()] == [
        '0.0', '0.2', '0.4', '0.6', '0.8', '1.0']


def test_clear_makes_new_titles_and_leaves_the_old_ones():
    for clear in ("axes", "figure"):
        fig, ax = plt.subplots()
        title = ax.set_title('title')
        fig.canvas.draw()
        ax.clear() if clear == "axes" else fig.clear()
        assert title is not ax.title and ax.title.get_text() == ''
        assert title.get_text() == 'title' and title.get_position() == (0.5, 1.0)
        assert title.axes is ax and title.figure is fig
        np.testing.assert_allclose(
            extents(title.get_window_extent()),
            [311.0, 426.72942708, 345.0, 443.73333333])
        with pytest.raises(NotImplementedError, match="cannot remove artist"):
            title.remove()


def test_a_removed_subfigure_keeps_its_place_and_its_axes():
    fig = plt.figure()
    sub = fig.subfigures(1, 2)[0]
    ax = sub.subplots()
    fig.canvas.draw()
    sub.remove()
    assert sub.axes == [ax] and ax.figure is sub
    np.testing.assert_allclose(extents(sub.get_window_extent()), [0, 0, 320, 480])
    np.testing.assert_allclose(extents(sub.bbox), [0, 0, 320, 480])
    np.testing.assert_allclose(
        extents(sub.get_tightbbox()), [8.27777778, 29.18888889, 299.0, 427.67615017])
    np.testing.assert_allclose(
        extents(ax.get_window_extent()), [40.0, 52.8, 288.0, 422.4])


@pytest.mark.parametrize("clear", ["axes", "figure"])
def test_ticks_dropped_by_a_clear_keep_what_they_had(clear):
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    label = ax.get_xticklabels()[0]
    fig.canvas.draw()
    ax.clear() if clear == "axes" else fig.clear()
    assert label.get_text() == '\N{MINUS SIGN}0.25'
    assert label.get_position() == (-0.25, 0)
    assert label is not ax.get_xticklabels()[0]
    # Placed by the transform its axis gave it, over the cleared view.
    np.testing.assert_allclose(
        extents(label.get_window_extent()), [-65.5, 29.18888889, -22.5, 43.07777778])
    with pytest.raises(NotImplementedError, match="cannot remove artist"):
        label.remove()


def test_an_artist_only_given_its_figure_has_no_removal_method():
    fig, ax = plt.subplots()
    note = text.Text(0.5, 0.5, 'a')
    note.set_figure(fig)
    with pytest.raises(NotImplementedError, match="cannot remove artist"):
        note.remove()
    assert note.figure is fig
    ax.add_artist(note)
    note.remove()
    with pytest.raises(ValueError, match="x not in list"):
        note.remove()
    line = lines.Line2D([0, 1], [0, 1])
    line.set_figure(fig)
    ax.add_line(line)
    line.remove()
    patch = patches.Rectangle((0, 0), 1, 1)
    patch.figure = fig
    with pytest.raises(NotImplementedError):
        patch.remove()


def test_axes_of_two_figures_share_an_axis():
    fig1, ax1 = plt.subplots()
    fig2, ax2 = plt.subplots()
    ax2.sharex(ax1)
    assert ax2.get_shared_x_axes().joined(ax1, ax2) and ax2._sharex is ax1
    siblings = ax2.get_shared_x_axes().get_siblings(ax2)
    assert len(siblings) == 2 and ax1 in siblings and ax2 in siblings
    ax1.plot([1, 2])
    assert ax2.get_xlim() == (-0.05, 1.05) == ax1.get_xlim()
    ax2.set_xlim(3, 4)
    assert ax1.get_xlim() == (3.0, 4.0)
    assert not ax1.get_autoscalex_on() and not ax2.get_autoscalex_on()
    ax1.set_xscale('log')
    assert ax2.get_xscale() == 'log'
    assert ax1.xaxis.get_major_locator() is ax2.xaxis.get_major_locator()
    locator = ticker.MultipleLocator(0.5)
    ax2.xaxis.set_major_locator(locator)
    assert ax1.xaxis.get_major_locator() is locator and locator.axis is ax2.xaxis
    ax2.set_xscale('linear')
    ax2.plot([10, 20], [1, 2])
    ax1.autoscale(axis='x')
    assert ax1.get_xlim() == (-1.0, 21.0) == ax2.get_xlim()
    calls = []
    ax1.callbacks.connect('xlim_changed', lambda ax: calls.append('ax1'))
    ax2.callbacks.connect('xlim_changed', lambda ax: calls.append('ax2'))
    ax2.set_xlim(0, 5)
    assert calls == ['ax2', 'ax1']


def test_a_subplot_made_sharing_with_another_figures_axes():
    fig1, axs = plt.subplots(2, 2, sharex=True, sharey=True)
    axs[0, 0].imshow(np.arange(12).reshape(3, 4))
    plt.figure()
    ax = plt.subplot(1, 1, 1, sharex=axs[0, 0], sharey=axs[0, 0])
    ax.imshow(np.arange(30).reshape(5, 6))
    assert ax._sharex is axs[0, 0] and ax._sharey is axs[0, 0]
    assert ax.get_xlim() == (-0.5, 5.5) == axs[1, 1].get_xlim()
    assert ax.get_ylim() == (4.5, -0.5) == axs[1, 1].get_ylim()
    assert len(ax.get_shared_x_axes().get_siblings(ax)) == 5
    # (The grouper holds the Axes of every figure alive.)
    assert [len(group) for group in ax.get_shared_y_axes() if ax in group] == [5]


def test_sharing_across_figures_ends_with_a_clear_or_a_removal():
    fig1, ax1 = plt.subplots()
    fig2, ax2 = plt.subplots()
    ax2.sharex(ax1)
    ax2.set_xlim(0, 5)
    fig2.clear()
    assert not ax2.get_shared_x_axes().joined(ax1, ax2)
    assert ax1.get_shared_x_axes().get_siblings(ax1) == [ax1]
    ax1.set_xlim(100, 200)
    assert ax2.get_xlim() == (0.0, 5.0)
    fig3, ax3 = plt.subplots()
    ax3.sharey(ax1)
    ax3.remove()
    ax1.set_ylim(5, 6)
    assert ax3.get_ylim() == (0.0, 1.0)
    # A closed figure's axes still share.
    fig4 = plt.figure()
    ax4 = plt.subplot(1, 1, 1, sharey=ax1)
    plt.close(fig1)
    ax4.set_ylim(0, 3)
    assert ax1.get_ylim() == (0.0, 3.0)


def test_a_pickle_of_a_figure_sharing_across_figures_holds_both():
    fig1, ax1 = plt.subplots()
    fig2, ax2 = plt.subplots()
    ax2.sharex(ax1)
    ax2.set_xlim(0, 5)
    copy = roundtrip(fig2)
    ax = copy.axes[0]
    siblings = ax.get_shared_x_axes().get_siblings(ax)
    assert len(siblings) == 2
    other = next(sibling for sibling in siblings if sibling is not ax)
    assert other.figure is not copy and other.figure is not fig1
    ax.set_xlim(7, 8)
    assert other.get_xlim() == (7.0, 8.0) and ax1.get_xlim() == (0.0, 5.0)


def test_a_host_placed_by_a_locator_leaves_the_layout_when_drawn():
    fig, ax = plt.subplots()
    inset = inset_axes(ax, width='40%', height='30%', loc='lower left')
    assert inset.get_in_layout()
    fig.canvas.draw()
    assert not inset.get_in_layout()
    np.testing.assert_allclose(
        extents(inset.get_tightbbox()),
        [55.22222222, 36.13333333, 300.84444444, 175.90059462])
    # A parasite calls its host's locator, which is an artist of the host.
    inset.get_aux_axes()
    with pytest.raises(ValueError, match="Can not reset the Axes"):
        inset.get_tightbbox()


@pytest.mark.parametrize("fmt", ["png", "pdf", "eps", "svg"])
def test_an_unpickled_inset_stays_where_its_locator_puts_it(fmt):
    fig, ax = plt.subplots()
    inset_axes(ax, width='40%', height='30%', loc='lower left')
    fig.canvas.draw()
    copy = roundtrip(fig)
    copy.savefig(io.BytesIO(), format=fmt)
    np.testing.assert_allclose(
        copy.axes[-1].get_position().bounds, [0.135851, 0.124468, 0.31, 0.231],
        atol=1e-6)


def test_mapped_colours_are_those_of_the_last_draw():
    fig, ax = plt.subplots()
    dots = ax.scatter(X, Y, c=VALUES, cmap='viridis')
    fig.canvas.draw()
    before = dots.get_facecolors()
    dots.set_array(VALUES[::-1])
    np.testing.assert_array_equal(dots.get_facecolors(), before)
    dots.set_clim(0, 100)
    np.testing.assert_array_equal(dots.get_facecolors(), before)
    fig.canvas.draw()
    after = dots.get_facecolors()
    assert not np.array_equal(after, before)
    np.testing.assert_allclose(
        after, plt.get_cmap('viridis')(colors.Normalize(0, 100)(VALUES[::-1])))


def test_a_clip_path_given_to_a_plotting_method_stands_alone():
    fig, ax = plt.subplots()
    circle = patches.Circle((1.5, 1), 0.5, transform=ax.transData)
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    image = ax.imshow(z, extent=[1, 2, 0.5, 1.5], clip_path=circle)
    assert image.get_clip_box() is None and image.get_clip_path() is not None
    plain = ax.imshow(z)
    assert plain.get_clip_box() is not None
    dots = ax.scatter(X, Y, clip_path=circle)
    assert dots.get_clip_box() is None
    bar = ax.bar([1, 2], [3, 4], clip_path=circle)[0]
    assert bar.get_clip_box() is None
    # Given afterwards, the clip path replaces only the path.
    plain.set(clip_path=circle)
    assert plain.get_clip_box() is not None
    fig.canvas.draw()
    assert roundtrip(fig).axes[0].images[0].get_clip_box() is None


def test_formatters_made_under_use_locale_use_it():
    pltrs.rcParams['axes.formatter.use_locale'] = True
    fig, ax = plt.subplots()
    ax.plot([0, 1], [12345.5, 12348.5])
    fig.canvas.draw()
    formatter = ax.yaxis.get_major_formatter()
    assert formatter.get_useLocale() is True
    assert [label.get_text() for label in ax.get_xticklabels()][1:3] == ['0.0', '0.2']
    pltrs.rcParams['axes.formatter.use_locale'] = False
    fig, ax = plt.subplots()
    assert ax.yaxis.get_major_formatter().get_useLocale() is False


def test_a_cell_is_where_it_was_made_until_its_table_places_it():
    fig, ax = plt.subplots()
    table = ax.table([['a', 'b'], ['c', 'd']])
    cell = table[0, 1]
    renderer = fig.canvas.get_renderer()
    cell.draw(renderer)
    bounds = cell.get_text_bounds(renderer)
    np.testing.assert_allclose(bounds[0], 0.431855, atol=1e-6)
    assert cell.get_xy() == (0, 0)
    fig.canvas.draw()
    np.testing.assert_allclose(cell.get_xy(), (0.5, -0.0450937950937951))
    np.testing.assert_allclose(cell.get_text_bounds(renderer)[0], 0.931855, atol=1e-6)


@pytest.mark.parametrize("how", ["clear", "remove", "axclear"])
def test_a_cell_of_a_removed_table_stays_where_it_was_placed(how):
    fig, ax = plt.subplots()
    table = ax.table([['a', 'b'], ['c', 'd']], loc='center')
    cell = table[0, 1]
    fig.canvas.draw()
    {"clear": fig.clear, "remove": table.remove, "axclear": ax.clear}[how]()
    np.testing.assert_allclose(
        extents(cell.get_window_extent()), [328.0, 237.6, 576.0, 254.26666667])
    assert cell.get_xy() == (0.5, 0.5) and cell.get_width() == 0.5
    np.testing.assert_allclose(
        cell.get_text_bounds(fig.canvas.get_renderer()),
        [0.9318548387096776, 0.5031521619656384, 0.01814516129032251,
         0.038789471162518074])


def test_a_spine_the_program_made_is_in_data_coordinates_until_placed():
    from pltrs import spines
    fig, ax = plt.subplots()
    ax.plot(X, Y)
    fig.canvas.draw()
    # In the place of a side of an Axes drawn before, and beside the four.
    left = ax.spines['left'] = spines.Spine.linear_spine(ax, 'left')
    extra = ax.spines['extra'] = spines.Spine.linear_spine(ax, 'right')
    fig.canvas.draw()
    at = ax.transData.transform([[0, 0], [1, 0]])[:, 0]
    np.testing.assert_allclose(left.get_window_extent().extents[[0, 2]], at[[0, 0]])
    np.testing.assert_allclose(extra.get_window_extent().extents[[0, 2]], at[[1, 1]])
    assert extra.get_position() == ('outward', 0.0)
    fig.canvas.draw()
    np.testing.assert_allclose(extra.get_window_extent().extents[[0, 2]], [576.0, 576.0])
    # Made before the first draw, a side's spine is placed with the ticks.
    fig, ax = plt.subplots()
    left = ax.spines['left'] = spines.Spine.linear_spine(ax, 'left')
    fig.canvas.draw()
    assert left.axis is None
    np.testing.assert_allclose(left.get_window_extent().extents[[0, 2]], [80.0, 80.0])


def test_a_drawing_areas_children_keep_the_outline_of_their_first_draw():
    from pltrs import offsetbox
    fig, ax = plt.subplots()
    ax.axis('off')
    area = offsetbox.DrawingArea(20, 20, 0, 0, clip=True)
    area.add_artist(patches.Rectangle((0, 0), 100, 100, color='k'))
    ax.add_artist(offsetbox.AnchoredOffsetbox('center', child=area, frameon=False))

    def ink():
        fig.canvas.draw()
        return int((np.asarray(fig.canvas.buffer_rgba())[..., 0] < 128).sum())

    first = ink()
    assert 700 < first < 850  # 20 points square at 100 dpi
    area.width = area.height = 60
    # matplotlib's area clips only the children that have no clip path yet.
    assert abs(ink() - first) < 60
    area.add_artist(patches.Rectangle((0, 0), 100, 100, color='k'))
    assert ink() > 6 * first


def test_an_unpickled_image_is_drawn_with_its_norm_as_it_is():
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    fig, ax = plt.subplots(figsize=(3, 2.5))
    ax.imshow(z + 1.5, norm=colors.LogNorm(), cmap='plasma', interpolation='nearest')
    fig.canvas.draw()
    copy = roundtrip(fig)
    image = copy.axes[0].images[0]
    # A pickle keeps no callbacks: nothing tells the image of its norm.
    assert {name: len(found) for name, found in image.norm.callbacks.callbacks.items()} == {
        'changed': 0}
    image.set_clim(2, 3)
    assert image.get_clim() == (2.0, 3.0)
    copy.canvas.draw()
    rgba = image.make_image(copy.canvas.get_renderer())[0]
    assert tuple(rgba[rgba.shape[0] // 2, rgba.shape[1] // 2]) == (12, 7, 134, 255)
    dots = roundtrip(plt.figure().add_subplot().scatter(X, Y, c=VALUES)).figure
    dots = dots.axes[0].collections[0]
    dots.figure.canvas.draw()
    dots.set_clim(0, 100)
    dots.figure.canvas.draw()
    np.testing.assert_allclose(
        dots.get_facecolors(), plt.get_cmap('viridis')(colors.Normalize(0, 100)(VALUES)))
