"""
What the differential tester's hostile programs (ninth round) found pltrs
drawing or doing otherwise than matplotlib 3.11: calls that fail after
matplotlib took colours from the cycle or set labels, the angles of
axisartist's tick labels turned by their transform, polar lines drawn
with the radial frame of an earlier draw, contour labels taken back when
their lines cannot be cut, refused colours met draw by draw, and the
rotated rectangles of the selector widgets. Each expected value is
recorded from matplotlib.
"""

import io
import math

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import colors as mcolors
from pltrs import sankey, ticker as mticker, widgets
from pltrs.mpl_toolkits import axisartist


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


X = np.linspace(0, 2 * np.pi, 7)
VALUES = np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])
SIGNAL = np.sin(np.arange(256) * 0.3)


def next_color(ax):
    """The next colour of the lines' cycle of *ax*, as a hex string."""
    return mcolors.to_hex(ax._get_lines.get_next_color())


@pytest.mark.parametrize("plot", [
    lambda ax: ax.psd(SIGNAL, NFFT=64, Fc='1'),
    lambda ax: ax.csd(SIGNAL, SIGNAL, NFFT=64, Fc='1'),
    lambda ax: ax.magnitude_spectrum(SIGNAL, Fc='1'),
])
def test_a_spectrum_shifted_by_text_fails_before_it_plots(plot):
    fig, ax = plt.subplots()
    with pytest.raises(TypeError):
        plot(ax)
    assert (len(ax.lines), ax.get_xlabel(), ax.get_ylabel()) == (0, '', '')


def test_failed_plotting_calls_take_their_colours_first():
    fig, ax = plt.subplots()
    with pytest.raises(ValueError):
        ax.stairs([1, 3, 2], edges=3)
    with pytest.raises(ValueError):
        ax.stackplot(X, np.sin(X) + 1, VALUES, baseline=3)
    with pytest.raises(AttributeError):
        ax.plot(X, VALUES, nonexistent_kw=1)
    assert next_color(ax) == '#9467bd'


def test_a_stackplot_its_units_refuse_takes_its_colours_first():
    fig, ax = plt.subplots()
    ax.stackplot(np.datetime64('2024-01-01') + np.arange(7), np.sin(X) + 1, VALUES)
    with pytest.raises(TypeError, match="Failed to convert value"):
        ax.stackplot('data', np.sin(X) + 1, VALUES)
    assert next_color(ax) == '#9467bd'


def test_a_stackplot_of_given_colours_or_refused_labels_leaves_the_cycle():
    fig, ax = plt.subplots()
    ax.stackplot(X, np.sin(X) + 1, VALUES, colors=['r'])
    with pytest.raises(TypeError):
        ax.stackplot(X, np.sin(X) + 1, VALUES, labels=None)
    assert next_color(ax) == '#1f77b4'


def test_a_pie_pltrs_refuses_takes_its_colours_once():
    fig, ax = plt.subplots()
    # matplotlib makes the wedges, then fails to set the limits around them.
    with pytest.raises(ValueError, match="Axis limits cannot be NaN or Inf"):
        ax.pie([1, 2, 3], center=(float('nan'), 0.4))
    wedges = ax.pie([1, 2, 3], center=(0.3, 0.4))[0]
    assert [mcolors.to_hex(w.get_facecolor()) for w in wedges] == [
        '#d62728', '#9467bd', '#8c564b']


def test_a_sankey_patch_label_of_none_is_empty():
    fig, ax = plt.subplots()
    diagram = sankey.Sankey(ax=ax)
    diagram.add(patchlabel=None, flows=[1, -1])
    assert diagram.finish()[0].text.get_text() == ''


def test_a_refused_colour_fails_where_the_draw_reaches_its_collection():
    fig, ax = plt.subplots()
    for color in ('', 'R'):
        with pytest.raises(ValueError):
            ax.errorbar(X, np.sin(X), yerr=0.2, ecolor=color)
    outcomes = []
    for _ in range(3):
        try:
            fig.canvas.draw()
            outcomes.append('ok')
        except ValueError as error:
            outcomes.append(str(error))
    assert outcomes == ["'' is not a valid color value.",
                        "'R' is not a valid color value.", 'ok']


def test_no_line_widths_fail_the_pdf_renderer_reusing_paths():
    fig, ax = plt.subplots()
    ax.scatter(X, np.sin(X), linewidths=[])
    fig.savefig(io.BytesIO(), format='png')
    with pytest.raises(ValueError, match="zero-size array to reduction operation maximum"):
        fig.savefig(io.BytesIO(), format='pdf')


def test_a_refused_colorbar_leaves_the_axes_current():
    fig, ax = plt.subplots()
    image = ax.imshow(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)))
    with pytest.raises(TypeError):
        fig.colorbar(image, nonexistent_kw=1)
    assert plt.gca() is ax
    assert len(fig.axes) == 2


def test_contour_labels_whose_lines_cannot_be_cut_are_taken_back():
    fig, ax = plt.subplots()
    cs = ax.contour(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)), levels=3)
    cs.clabel()
    with pytest.raises(TypeError):
        cs.labels(True, None)
    assert len(cs.labelTexts) == 3


def test_contour_labels_follow_the_alpha_and_limits_of_the_set():
    fig, ax = plt.subplots()
    cs = ax.contour(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)), levels=3)
    cs.clabel()
    cs.set(alpha=0.5)
    cs.set(clim=[0, 1])
    assert [text.get_alpha() for text in cs.labelTexts] == [0.5, 0.5, 0.5]
    assert [mcolors.to_hex(text.get_color()) for text in cs.labelTexts] == [
        '#440154', '#440154', '#fde725']


def test_tick_labels_turned_by_their_transform_keep_the_angle_draws_left():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    labels = ax.axis['bottom'].major_ticklabels
    labels.set_rotation(3)
    labels.set_transform_rotates_text(True)
    assert labels.get_rotation() == pytest.approx(2.2363925865657674, abs=1e-9)
    fig.canvas.draw()
    assert labels.get_rotation() == pytest.approx(0.3830574573150115, abs=1e-9)
    renderer = fig.canvas.get_renderer()
    assert [np.round(b.bounds, 2).tolist() for b in labels.get_window_extents(renderer)][:2] == [
        [68.95, 29.69, 22.07, 14.0], [168.16, 29.7, 22.05, 13.97]]


def test_empty_tick_labels_measure_as_points():
    fig = plt.figure()
    ax = fig.add_subplot(axes_class=axisartist.Axes)
    labels = ax.axis['bottom'].major_ticklabels
    fig.canvas.draw()
    labels.set_locs_angles_labels([((0.5, 0.5), 90, 'a'), ((0.2, 0.5), 90, '')])
    fig.canvas.draw()
    extents = labels.get_window_extents(fig.canvas.get_renderer())
    assert len(extents) == len(labels._locs_angles_labels)


def test_a_polar_line_keeps_the_radial_frame_of_its_first_draw():
    fig = plt.figure()
    ax = fig.add_subplot(projection='polar')
    ax.plot(X, VALUES)
    fig.canvas.draw()
    ax.set_ylim(2, 0)
    fig.canvas.draw()
    image = np.asarray(fig.canvas.buffer_rgba())[..., :3].astype(int)
    assert int(((image[..., 2] > 150) & (image[..., 0] < 100)).sum()) == 834


def test_a_selector_rectangle_turns_with_the_aspect_of_the_axes():
    fig, ax = plt.subplots()
    ax.plot(X, np.sin(X))
    selector = widgets.RectangleSelector(ax, lambda e1, e2: None, interactive=True,
                                         useblit=False)
    selector.extents = (1, 3, -0.5, 0.5)
    selector.rotation = 45
    fig.canvas.draw()
    image = np.asarray(fig.canvas.buffer_rgba())[..., :3].astype(int)
    # The rectangle's red face, the lighter for its alpha.
    face = (image[..., 0] > 230) & (image[..., 1] > 180) & (image[..., 1] < 220)
    rows, columns = np.nonzero(face)
    assert (rows.min(), rows.max(), columns.min(), columns.max()) == (141, 367, 132, 359)
    assert math.isclose(selector._selection_artist._aspect_ratio_correction,
                        ax._get_aspect_ratio())


def test_dates_on_an_axis_keeping_its_locator_format_by_the_year():
    # The converter's formatter follows its own locator, which the locator
    # set on the axis keeps from ticking: a year, the unit it starts with.
    fig, ax = plt.subplots()
    ax.xaxis.set_major_locator(mticker.MaxNLocator(4))
    ax.plot(np.datetime64('2024-01-01') + np.arange(7), X)
    fig.canvas.draw()
    assert [t.get_text() for t in ax.get_xticklabels()] == [
        '2023', '2024', '2024', '2024', '2024']
