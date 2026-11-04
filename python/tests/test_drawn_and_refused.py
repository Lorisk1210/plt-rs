"""
The differential tester's (``cargo xtask diff``) leftovers: what draws
leave on quiver plots and their keys, streamplot arrows, secondary axes
before their first draw, the moments fonts are looked up and missing
glyphs warned about, unscaled `.FT2Font` glyphs, setter documentation,
and saves that need more memory than there is. Each expected value is
matplotlib 3.11's.
"""

import io
import logging
import os
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import artist, collections, patches, ticker, transforms
from pltrs.ft2font import FT2Font, Kerning


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def test_streamplot_arrows_are_fancy_arrow_patches():
    ax = plt.figure().add_subplot()
    z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
    stream = ax.streamplot(np.arange(6), np.arange(5), np.ones((5, 6)), z)
    assert type(stream.arrows) is collections.PatchCollection
    assert len(ax.patches) == 66
    arrow = ax.patches[0]
    assert type(arrow) is patches.FancyArrowPatch
    assert arrow.get_linewidth() == 1.5
    # The cycle's colour as the cycle holds it: the int alpha of a triple.
    assert arrow.get_facecolor() == (0.12156862745098039, 0.4666666666666667,
                                     0.7058823529411765, 1)
    assert [type(c).__name__ for c in ax.get_children()][:2] == [
        "LineCollection", "FancyArrowPatch"]


def test_quiver_sizes_and_paths_follow_the_first_draw():
    fig, ax = plt.subplots()
    q = ax.quiver([0, 1], [0, 1], [1, -1], [1, 2])
    assert (q.get_paths(), q.scale, q.width) == ([], None, None)
    with pytest.raises(AttributeError):
        q.span
    fig.canvas.draw()
    paths = q.get_paths()
    assert len(paths) == 2 and paths[0].vertices.shape == (8, 2)
    assert paths[0].codes is None
    assert type(q.scale) is np.float64 and q.scale == pytest.approx(32.85253385885597)
    assert q.width == pytest.approx(0.0075) and q.span == pytest.approx(1)
    np.testing.assert_allclose(paths[0].vertices[:3], [
        [-0.002652, 0.002652], [0.003923, 0.009226], [-0.004032, 0.011877]], atol=1e-6)
    # The scale stays: longer components draw longer arrows.
    q.set_UVC([2, -2], [2, 4])
    fig.canvas.draw()
    assert q.scale == pytest.approx(32.85253385885597)
    np.testing.assert_allclose(q.get_paths()[0].vertices[1], [0.034362, 0.039665],
                               atol=1e-6)
    given = ax.quiver([0, 1], [0, 1], [1, -1], [1, 2], scale=2, width=0.01)
    fig.canvas.draw()
    assert (given.scale, given.width) == (2, 0.01)


def test_quiver_key_vector_is_drawn_into_one_collection():
    fig, ax = plt.subplots()
    q = ax.quiver([0, 1], [0, 1], [1, -1], [1, 2])
    key = ax.quiverkey(q, 0.5, 0.5, 1, 'k')
    vector = key.vector
    assert type(vector) is collections.PolyCollection
    assert vector.get_paths() == []
    fig.canvas.draw()
    assert key.vector is vector
    path, = vector.get_paths()
    assert path.vertices.shape == (9, 2)
    assert list(path.codes) == [1, 2, 2, 2, 2, 2, 2, 2, 79]
    np.testing.assert_allclose(path.vertices[0], [-0.01522, 0.003044], atol=1e-6)


def test_secondary_axes_take_their_limits_when_drawn():
    fig, ax = plt.subplots()
    secondary = ax.secondary_xaxis('top', functions=(lambda v: 2 * v, lambda v: v / 2))
    assert secondary.get_autoscale_on() and secondary.get_autoscalex_on()
    assert secondary.get_xlim() == (0, 1) and secondary.get_data_ratio() == 2
    with pytest.warns(UserWarning, match="fixed number of ticks"):
        labels = secondary.set_xticklabels(['a', 'b'])
    assert len(labels) == 6
    fig.canvas.draw()
    assert not secondary.get_autoscalex_on()
    assert secondary.get_xlim() == (0, 2) and secondary.get_data_ratio() == 1


def test_symlog_minor_ticks_leave_the_major_ones_out():
    ax = plt.figure().add_subplot()
    ax.set_xlim(0, 2)
    ax.set_xscale('symlog')
    assert list(ax.xaxis.get_majorticklocs()) == [0, 2]
    assert list(ax.xaxis.get_minorticklocs()) == []
    assert ax.get_xminorticklabels() == []


def test_reading_tick_labels_leaves_the_aspect_alone():
    ax = plt.figure().add_subplot()
    ax.axis('equal')
    ax.imshow(np.zeros((5, 6)))
    ax.minorticks_on()
    plt.findobj(match=plt.Text)
    assert ax.get_xlim() == (-0.5, 5.5)


def test_axes_str_shows_the_active_position():
    ax = plt.figure().add_subplot(projection='polar')
    ax.set_anchor('NE')
    ax.set_aspect(2)
    assert str(ax) == "PolarAxes(0.125,0.11;0.775x0.77)"
    ax.get_position()
    assert str(ax) == "PolarAxes(0.61125,0.11;0.28875x0.77)"


def test_asinh_ticks_of_a_narrow_range_far_from_zero():
    ticks = ticker.AsinhLocator(linear_width=1).tick_values(1e12, 1e12 + 3)
    assert len(ticks) == 11
    np.testing.assert_array_equal(ticks, np.linspace(1e12, 1e12 + 3, 11))


def test_interval_checks_are_typed_as_comparisons():
    assert type(transforms._interval_contains(
        (np.float64(0), np.float64(1)), 0.5)) is np.bool_
    assert type(transforms._interval_contains((0, 1), 0.5)) is bool
    assert transforms._interval_contains_open((0, 1), 1) is False


def font_logs(action):
    """The font manager's "not found" log lines *action* makes."""
    stream = io.StringIO()
    handler = logging.StreamHandler(stream)
    logger = logging.getLogger("pltrs.font_manager")
    logger.addHandler(handler)
    try:
        action()
    finally:
        logger.removeHandler(handler)
    return stream.getvalue().count("Font family 'xkcd' not found")


def test_fonts_are_looked_up_where_matplotlib_looks_them_up():
    fig, ax = plt.subplots()
    ax.set_xticks([])
    ax.set_yticks([])
    text = ax.text(0.1, 0.1, 'hello', family='xkcd')
    # A layout, then the line measured (uncached), then cached.
    assert font_logs(text.get_window_extent) == 2
    assert font_logs(text.get_window_extent) == 1
    # A layout and the line drawn.
    assert font_logs(fig.canvas.draw) == 2
    ax.text(0.1, 0.1, 'hello\nworld', family='xkcd')
    assert font_logs(fig.canvas.draw) == 6
    labels = plt.subplots()[1]
    labels.set_xticks([0, 1], ['A', 'C'], family='xkcd')
    labels.set_yticks([])
    assert font_logs(labels.figure.canvas.draw) == 10
    assert font_logs(labels.figure.canvas.draw) == 8


def test_missing_glyphs_are_warned_about_when_measured():
    fig, ax = plt.subplots()
    text = ax.text(0.3, 0.4, 'with\ttab')

    def count(action):
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            action()
        return len(caught)
    assert count(text.get_window_extent) == 1
    assert count(text.get_tightbbox) == 0
    assert count(fig.canvas.draw) == 0


def test_ft2font_loads_unscaled_without_a_size():
    font = FT2Font(os.path.join(pltrs.get_data_path(), 'fonts/ttf/DejaVuSans.ttf'))
    glyph = font.load_char(ord('A'))
    assert glyph.bbox == (16, 0, 1384, 1493)
    assert (glyph.horiAdvance, glyph.linearHoriAdvance) == (1401, 0)
    assert (glyph.vertBearingX, glyph.vertBearingY, glyph.vertAdvance) == (-684, 277, 2048)
    assert [font.get_kerning(36, 57, mode) for mode in Kerning] == [0, 0, -131]
    font.set_size(12, 72)
    assert font.load_char(ord('A')).bbox == (6, 0, 519, 576)


def test_setters_document_matplotlib_s_value_types():
    inspector = artist.ArtistInspector(plt.Line2D)
    assert inspector.get_valid_values('antialiased') == 'bool'
    assert inspector.get_valid_values('clip_path') == 'Patch or (Path, Transform) or None'
    assert inspector.get_valid_values('sketch_params') == (
        '(scale: float, length: float, randomness: float)')
    assert artist.ArtistInspector(patches.Rectangle).get_valid_values('hatch') == (
        "{'/', '\\\\', '|', '-', '+', 'x', 'o', 'O', '.', '*'}")


def test_saves_beyond_memory_raise_memory_error():
    # 5725 x 480436 pixels: more than any canvas pltrs allocates (11 GB),
    # which matplotlib's allocation refuses under the 6 GiB address space
    # of the fuzzer's workers, raising `MemoryError`.
    fig = plt.figure(dpi=1000)
    ax = fig.add_axes([0.1, 0.1, 0.8, 100])
    ax.plot([1, 2, 3])
    with pytest.raises(MemoryError):
        fig.savefig(io.BytesIO(), format='png', bbox_inches='tight')
