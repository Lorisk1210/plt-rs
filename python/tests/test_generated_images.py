"""
What the generated programs of the differential tester (``cargo xtask diff
--generated``) drew or measured otherwise than matplotlib 3.11: the
behaviours behind them, each expected value recorded from matplotlib.
"""

import io

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import transforms

Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def pixels(fig):
    """The figure printed as RGBA, row by row from the top."""
    buffer = io.BytesIO()
    fig.savefig(buffer, format="rgba")
    width, height = fig.canvas.get_width_height()
    return np.frombuffer(buffer.getvalue(), np.uint8).reshape(height, width, 4)


def table_2x2():
    fig, ax = plt.subplots()
    return fig, ax.table([['a', 'b'], ['c', 'd']], loc='center')


def test_rotated_cells_widen_the_grid_of_their_table():
    fig, table = table_2x2()
    table[0, 1].set_angle(30)
    fig.canvas.draw()
    np.testing.assert_allclose(
        table[0, 1].get_window_extent().extents,
        [316.81673881673885, 192.516454968463, 542.7743001385409, 299.3502116982036])
    np.testing.assert_allclose(
        table.get_window_extent().extents,
        [80.00000000000003, 175.8497883017963, 576.0, 299.3502116982036])


def test_a_table_transform_leaves_its_cells_theirs():
    fig, table = table_2x2()
    table.set_transform(transforms.IdentityTransform())
    fig.canvas.draw()
    np.testing.assert_allclose(
        table.get_window_extent().extents,
        [-162360.0, -25437.28, -161864.0, -25403.94666666667])
    np.testing.assert_allclose(table[1, 1].get_xy(), (-327.0, -68.96666666666667))


def test_drawing_gives_tick_labels_their_place_and_text_back():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    label = ax.get_xticklabels()[2]
    label.set_text('x')
    label.set_position((0.3, 0.4))
    assert (label.get_text(), label.get_position()) == ('x', (0.3, 0.4))
    fig.canvas.draw()
    assert label.get_text() == '0.25'
    assert [float(v) for v in label.get_position()] == [0.25, 0.4]
    label.set_transform(transforms.IdentityTransform())
    np.testing.assert_allclose(label.get_window_extent().extents,
                               [-15.25, -13.488888888888889, 15.75, 0.4])


def test_3d_axes_are_measured_where_they_stand_before_a_draw():
    ax = plt.figure().add_subplot(projection='3d')
    np.testing.assert_allclose(
        ax.zaxis.get_tightbbox().extents,
        [544.0799914155052, 181.43435648133786, 594.5695291823085, 341.9414293001186])
    np.testing.assert_allclose(ax.zaxis.get_majorticklocs(),
                               [-0.2, 0, 0.2, 0.4, 0.6, 0.8, 1, 1.2])


def test_infinite_lines_draw_their_data_through_the_view():
    fig, ax = plt.subplots()
    line = ax.axline((0, 0), (1, 2), marker='o', linestyle='none')
    line.set_xdata([0.5, 1.5])
    line.set_ydata([0.5, -0.5])
    fig.canvas.draw()
    np.testing.assert_allclose(
        line.get_window_extent().extents,
        [323.8333333333333, -136.16666666666666, 828.1666666666666, 241.76666666666662])


def test_event_rows_without_colours_map_their_array_onto_the_edges():
    fig, ax = plt.subplots()
    rows = ax.eventplot([1, 2, 4])
    rows[0].set_array(np.arange(3.0))
    fig.canvas.draw()
    np.testing.assert_allclose(
        rows[0].get_edgecolor(),
        [[0.267004, 0.004874, 0.329415, 1.0], [0.127568, 0.566949, 0.550556, 1.0],
         [0.993248, 0.906157, 0.143936, 1.0]], atol=1e-6)


def test_filled_contours_stay_mapped_unless_their_faces_are_none():
    fig, ax = plt.subplots()
    filled = ax.contourf(Z)
    filled.set_facecolor('g')
    fig.canvas.draw()
    np.testing.assert_allclose(
        filled.get_facecolor()[:2],
        [[0.282327, 0.094955, 0.417331, 1.0], [0.258965, 0.251537, 0.524736, 1.0]],
        atol=1e-6)
    assert pixels(fig)[240, 320].tolist() == [31, 160, 136, 255]
    fig, ax = plt.subplots()
    ax.contourf(Z).set_facecolor('none')
    assert pixels(fig)[240, 320].tolist() == [255, 255, 255, 255]


def test_every_band_of_a_hatched_contour_set_takes_its_properties():
    fig, ax = plt.subplots()
    ax.contourf(Z, hatches=['//', None]).set_clip_box(None)
    assert pixels(fig)[57, 100:110].tolist() == (
        [[29, 151, 128, 255]] * 7 + [[28, 147, 125, 255], [19, 103, 87, 255],
                                     [17, 93, 79, 255]])


def test_agg_draws_one_item_per_path_or_offset():
    x = np.linspace(0, 2 * np.pi, 7)
    y = np.sin(x)
    fig, ax = plt.subplots()
    ax.set_ylim(4.5, -1.2)
    ax.set_xlim(-0.5, 6.6)
    ax.fill_between(x, y, 0.5, where=y > 0, interpolate=True).set_sizes([10, 20])
    image = pixels(fig)
    assert image[400, 500].tolist() == [255, 255, 255, 255]
    assert image[300, 500].tolist() == [31, 119, 180, 255]


def test_arrows_measured_between_draws_start_from_the_box_last_drawn():
    fig, ax = plt.subplots()
    note = ax.annotate('note', xy=(0.2, 0.2), xytext=(0.6, 0.7),
                       arrowprops=dict(arrowstyle='->'))
    note.set_animated(True)
    note.set_backgroundcolor('y')
    fig.canvas.draw()
    np.testing.assert_allclose(
        note.get_window_extent().extents,
        [181.5439248251, 128.7797660947, 409.6, 322.0723003472], atol=1e-9)
