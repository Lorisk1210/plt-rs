"""
The documented API where the generated programs still differed from
matplotlib (task DOC8): what a user sees first, the images, then values.
The expected values are matplotlib 3.11's; the tests pass under it too.
"""

import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs.lines import Line2D


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def _canvas(fig):
    fig.canvas.draw()
    return np.asarray(fig.canvas.buffer_rgba()).copy()


def test_a_tick_label_listed_by_another_axes_is_drawn_there():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    label = ax.get_xticklabels()[0]
    fig2, ax2 = plt.subplots()
    before = _canvas(fig2)
    # The Axes lists the label before giving it the figure fails.
    with pytest.raises(RuntimeError, match="more than one figure"):
        ax2.add_artist(label)
    assert label in ax2.get_children()
    rows, columns = np.nonzero((_canvas(fig2) != before).any(axis=2))
    # The label of the tick at -0.25, left of the first Axes' view, where
    # that Axes' transforms put it.
    assert label.get_text() == "\N{MINUS SIGN}0.25"
    assert (columns.min(), columns.max()) == (26, 66)
    assert (rows.min(), rows.max()) == (437, 447)


def test_labelling_contours_applies_no_pending_autoscale():
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    fig, ax = plt.subplots()
    contours = ax.contour(z, levels=3)
    # The view the set autoscaled when it was made is asked for again; its
    # labels are placed in it without applying that, so that removing the
    # set leaves the margins to the next draw.
    assert len(contours.clabel()) == 3
    contours.remove()
    fig.canvas.draw()
    assert ax.get_xlim() == (-0.25, 5.25)
    assert ax.get_ylim() == (-0.2, 4.2)


def test_a_view_its_scale_maps_onto_one_point_draws_nothing_in_data():
    fig, ax = plt.subplots()
    ax.set_yscale('log')
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        # Limits beyond 1 on a logit axis: its data transform is not finite.
        ax.set(yscale='logit')
        image = _canvas(fig)
        assert np.isnan(ax.transData.transform((0.5, 0.9))).all()
        assert ax.get_tightbbox().bounds == pytest.approx((80.0, 52.8, 496.0, 369.6))
    assert (image == 255).all()


def test_an_image_in_a_view_of_no_height_is_not_made():
    for clip_on in (True, False):
        fig, ax = plt.subplots()
        ax.set_axis_off()
        blank = _canvas(fig)
        ax.imshow([[1, 2], [3, 4]], clip_on=clip_on)
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            ax.yaxis.set_view_interval(0, 0, ignore=True)
            # Its box is at NaN, outside whatever clips it.
            assert (_canvas(fig) == blank).all()


def test_texts_beyond_tick_labels_at_nan_are_at_nan():
    x = np.linspace(0, 2 * np.pi, 7)
    fig, ax = plt.subplots(figsize=(4, 3))
    ax.plot(x, np.sin(x), label='a')
    ax.set_xlabel('x label')
    ax.set_title('title')
    ax.text(0.5, 0.5, 'in the axes', transform=ax.transAxes)
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        # The margin below 0 has no square root: the view is NaN.
        ax.set_xscale('function', functions=(np.square, np.sqrt))
        image = _canvas(fig)
        assert ax.xaxis.get_major_formatter().get_offset() == '1e\N{MINUS SIGN}12'
        x, y = ax.xaxis.offsetText.get_position()
        assert (x, np.isnan(y)) == (1, True)
        assert ax.xaxis.get_tightbbox() is None
    rows, columns = np.nonzero((image[..., :3] != 255).any(axis=2))
    # The title and the text in axes fractions, nothing at the bottom.
    assert (rows.min(), rows.max()) == (14, 151)
    assert (columns.min(), columns.max()) == (188, 281)


def test_a_3d_line_is_measured_by_the_2d_data_it_was_given():
    x = np.linspace(0, 2 * np.pi, 7)
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    line, = ax.plot(x, np.sin(x), np.arange(7.0))
    line.set_xdata([3, 2, 1])
    with pytest.raises(ValueError, match="shape mismatch"):
        line.get_bbox()
    line.set_ydata([1, 2, 3])
    assert line.get_bbox().get_points().tolist() == [[1.0, 1.0], [3.0, 3.0]]
    assert line.get_xydata().tolist() == [[3.0, 1.0], [2.0, 2.0], [1.0, 3.0]]
    line.set_drawstyle('steps-mid')
    assert line.get_path().vertices.tolist() == [
        [3.0, 1.0], [2.5, 1.0], [2.5, 2.0], [1.5, 2.0], [1.5, 3.0], [1.0, 3.0]]
    # Until the next draw projects the 3-D data again.
    fig.canvas.draw()
    assert len(line.get_xdata()) == 7
    assert line.get_data_3d()[0].tolist() == x.tolist()


def test_a_polar_wedge_follows_the_radial_scale():
    fig = plt.figure(figsize=(3, 3))
    ax = fig.add_subplot(projection='polar')
    ax.plot(np.linspace(0, 2 * np.pi, 7), [3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])
    ax.set_thetamax(270)
    ax.set_rorigin(-1)
    ax.set_rscale('symlog')
    fig.canvas.draw()
    # The inner radius is the origin's, both through the scale.
    assert ax.patch.r == pytest.approx(0.4952049594385384)
    assert ax.patch.width == pytest.approx(0.3775728235562328)


def test_rc_markers_are_kept_as_given():
    fig, ax = plt.subplots()
    fig.canvas.draw()
    with plt.rc_context():
        # matplotlib's validator takes any string; a line fails to take it.
        plt.rcParams['lines.marker'] = 'nope'
        assert plt.rcParams['lines.marker'] == 'nope'
        assert plt.rcParams.copy()['lines.marker'] == 'nope'
        for make in (lambda: ax.plot([1, 2]), lambda: Line2D([0], [0])):
            with pytest.raises(ValueError, match="Unrecognized marker style 'nope'"):
                make()
        assert len(ax.plot([1, 2], 'r-')) == 1
        assert Line2D([0], [0], marker='x').get_marker() == 'x'
        with plt.rc_context({'lines.marker': 'o'}):
            assert ax.plot([1, 2])[0].get_marker() == 'o'
        assert plt.rcParams['lines.marker'] == 'nope'
        plt.rcParams['scatter.marker'] = 'zzz'
        assert plt.rcParams['scatter.marker'] == 'zzz'
        with pytest.raises(ValueError, match="Supported markers"):
            plt.rcParams['lines.marker'] = [1, 2]
        plt.rcParams['lines.marker'] = 'x'
        assert ax.plot([1, 2])[0].get_marker() == 'x'
    assert plt.rcParams['lines.marker'] == 'None'
    assert ax.plot([1, 2])[0].get_marker() == 'None'


def test_text_is_measured_without_the_glyphs_no_font_has():
    with plt.rc_context({'font.enable_last_resort': False}):
        fig, ax = plt.subplots()
        missing = ax.text(0.1, 0.2, 'glyph 漢')
        present = ax.text(0.1, 0.4, 'glyph ')
        with pytest.warns(UserWarning, match="Glyph 28450"):
            fig.canvas.draw()
            extent = missing.get_window_extent()
        assert extent.width == present.get_window_extent().width == 43.0
        assert extent.height == pytest.approx(present.get_window_extent().height)


def test_contour_labels_on_a_line_off_the_canvas_fail():
    x = np.linspace(0, 2 * np.pi, 7)
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    fig = plt.figure()
    ax = fig.add_subplot(projection='polar')
    ax.axis([0, 1, 0, 2])
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        ax.bar(x, [3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5], log=True)
        contours = ax.contour(z)
        with pytest.raises(ValueError, match="cannot convert float NaN to integer"):
            contours.clabel()
