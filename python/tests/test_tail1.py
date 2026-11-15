"""
What the last differences of the differential tester's generated programs
(task TAIL1) found pltrs doing otherwise than matplotlib 3.11: glyphs an
`FT2Font` loaded, mathtext rendered to an image, parasite axes between
their host's draws, shared tickers, picks, colorbars made in a log-scaled
Axes and contour labels on degenerate lines. Each expected
value is recorded from matplotlib.
"""

import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import backend_bases, colorbar, colors, font_manager, ft2font, mathtext
from pltrs.backends.backend_agg import FigureCanvasAgg
from pltrs.figure import Figure
from pltrs.mpl_toolkits.axes_grid1 import host_subplot


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def runtime_warnings(work):
    """The messages of the warnings *work* issues, each one said."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        work()
    return [str(w.message) for w in caught]


def dejavu_at_12pt():
    font = ft2font.FT2Font(font_manager.findfont("DejaVu Sans"))
    font.set_size(12, 72)
    return font


def test_a_glyph_loaded_unscaled_has_font_units():
    font = dejavu_at_12pt()
    glyph = font.load_glyph(36, flags=ft2font.LoadFlags.NO_SCALE)
    assert (glyph.width, glyph.height, glyph.horiBearingX, glyph.horiBearingY,
            glyph.horiAdvance, glyph.linearHoriAdvance, glyph.vertBearingY,
            glyph.vertAdvance, glyph.bbox) == (
        1368, 1493, 16, 1493, 1401, 537984, 277, 2048, (16, 0, 1384, 1493))
    vertices, codes = font.get_path()
    assert len(vertices) == 15
    assert vertices[:2].tolist() == [[10.9375, 20.21875], [6.65625, 8.609375]]
    image = np.zeros((40, 40), np.uint8)
    font.draw_glyph_to_bitmap(
        image, 0, 0, font.load_glyph(36, flags=ft2font.LoadFlags.NO_SCALE))
    assert int(image.sum()) == 42253


def test_loaded_glyphs_keep_the_size_they_were_loaded_at():
    font = dejavu_at_12pt()
    font.set_text("Hi")
    font.set_size(30, 72)
    font.draw_glyphs_to_bitmap()
    image = np.asarray(font.get_image())
    assert (image.shape, int(image.sum())) == ((12, 12), 8707)
    font = dejavu_at_12pt()
    glyph = font.load_char(65)
    font.set_size(30, 72)
    image = np.zeros((40, 40), np.uint8)
    font.draw_glyph_to_bitmap(image, 0, 0, glyph)
    assert int(image.sum()) == 6582


def test_an_unscaled_glyph_among_the_text_is_drawn_unscaled():
    font = dejavu_at_12pt()
    font.set_text("Hi")
    font.load_glyph(36, flags=ft2font.LoadFlags.NO_SCALE)
    font.draw_glyphs_to_bitmap()
    image = np.asarray(font.get_image())
    assert (image.shape, int(image.sum())) == ((12, 12), 18878)


def test_math_rendered_to_an_image_takes_hinted_glyph_boxes():
    parse = mathtext.MathTextParser("agg").parse("$x^2$", dpi=72)
    assert parse[:5] == (0, 0, 12.314583333333333, 13.229999999999999,
                         2.0999999999999996)
    image = np.asarray(parse[5])
    assert image.shape == (14, 13)
    # The superscript's top row, placed as the box shipped a second time
    # from the image's corner rounds.
    assert image[0].tolist() == [0, 0, 0, 0, 0, 0, 0, 116, 255, 235, 88, 0, 0]
    assert image[5].tolist() == [0, 0, 178, 81, 1, 151, 124, 0, 0, 0, 0, 0, 0]
    plain = mathtext.MathTextParser("agg").parse("x", dpi=72)
    assert plain[2:5] == (7.59375, 9.0, 2.0)


def test_a_stix_fraction_lands_where_matplotlibs_rounding_puts_it():
    pltrs.rcParams["text.hinting"] = "auto"
    prop = font_manager.FontProperties(size=11, math_fontfamily="stix")
    parse = mathtext.MathTextParser("agg").parse(r"$\frac{\alpha}{2}$", dpi=96,
                                                prop=prop)
    image = np.asarray(parse[5])
    assert image.shape == (19, 9)
    assert image[0].tolist() == [0, 0, 89, 82, 136, 128, 98, 0, 0]
    assert image[6].tolist() == [0, 255, 255, 255, 255, 255, 255, 255, 255]


def twinned_host():
    fig = plt.figure()
    host = host_subplot(121, figure=fig)
    par = host.twinx()
    par.plot([0, 1, 2], [3, 1, 2])
    fig.canvas.draw()
    return fig, host, par


def test_a_parasite_stays_where_its_host_was_drawn_until_drawn_again():
    fig, host, par = twinned_host()
    renderer = fig.canvas.get_renderer()
    host.set(subplotspec=fig.add_gridspec(2, 2)[0, 0])
    assert np.round(par.get_tightbbox(renderer).bounds, 2).tolist() == [
        80.0, 52.8, 266.18, 369.6]
    assert par.yaxis.get_tick_space() == 13
    assert np.round(host.get_tightbbox(renderer).get_points(), 2).tolist() == [
        [48.28, 52.8], [346.18, 427.68]]
    fig.canvas.draw()
    assert par.yaxis.get_tick_space() == 6


def test_a_shared_locator_reads_the_axis_it_is_bound_to():
    fig = plt.figure()
    host = host_subplot(121, figure=fig)
    host.plot([0, 1, 2], [0.2, 0.5, 0.8])
    par = host.twiny()
    par.plot([3.0, 1.0, 4.0], [0, 1.5, 6.25])
    fig.canvas.draw()
    host.set_yscale("logit")
    host.set_ylim(-1, 1)
    locator = par.yaxis.get_minor_locator()
    assert locator is host.yaxis.get_minor_locator()
    assert locator.axis is host.yaxis
    # The parasite's own smallest positive datum (1.5, above 1) would turn
    # the logit locator's limits around: it reads its host's.
    assert runtime_warnings(
        lambda: np.testing.assert_array_equal(par.yaxis.get_minorticklocs(), [])) == []


def test_a_pick_over_another_figure_reaches_no_axes_children():
    fig = plt.figure()
    ax = fig.add_subplot()
    ax.set_yscale("logit")
    ax.set_ylim(-1, 1)
    other = Figure(figsize=(3, 2), dpi=50)
    canvas = FigureCanvasAgg(other)
    other.add_subplot()
    canvas.draw()
    event = backend_bases.MouseEvent("button_press_event", canvas, 60, 40, 1)
    assert event.inaxes is not None
    assert runtime_warnings(lambda: ax.pick(event)) == []


def test_a_pick_asks_no_helper_of_the_axes_it_does_not_reach():
    """A scroll over a twin picks the twin's children only: the formatter
    of the Axes under it is not asked (one that saves the figure would
    recurse)."""
    fig, ax = plt.subplots()
    twin = ax.twiny()
    fig.canvas.draw()
    calls = []
    ax.xaxis.set_major_formatter(
        pltrs.ticker.FuncFormatter(lambda v, p: calls.append(v) or str(v)))
    x, y = ax.transAxes.transform((0.5, 0.5))
    event = backend_bases.MouseEvent("scroll_event", fig.canvas, x, y, "up", step=0)
    assert event.inaxes is twin
    event._process()
    assert calls == []


def test_a_colorbar_in_a_log_scaled_axes_redraws_while_it_limits_its_norm():
    fig, ax = plt.subplots()
    colorbar.Colorbar(ax, norm=colors.LogNorm(1, 10))
    assert runtime_warnings(lambda: colorbar.Colorbar(ax, norm=None)) == [
        "Attempt to set non-positive ylim on a log-scaled axis will be ignored.",
        "Attempting to set identical low and high ylims makes transformation "
        "singular; automatically expanding.",
        "Attempt to set non-positive ylim on a log-scaled axis will be ignored.",
    ]
    assert ax.get_yscale() == "linear"
    assert ax.get_ylim() == (0.0, 1.0)


def test_a_log_norm_refuses_the_limits_before_the_colorbar_redraws():
    fig, ax = plt.subplots()
    colorbar.Colorbar(ax, norm=colors.LogNorm(1, 10))

    def unscaled_log():
        with pytest.raises(ValueError, match="Invalid vmin or vmax"):
            colorbar.Colorbar(ax, norm="log")

    assert runtime_warnings(unscaled_log) == []


def test_a_label_located_on_a_line_with_an_infinite_vertex_warns_as_numpy():
    fig, ax = plt.subplots()
    contour = ax.contour(np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6)))
    contour.clabel()
    line = np.array([[0.0, 0.0], [10.0, 0.0], [20.0, 0.0], [30.0, np.inf]])
    found = []
    said = runtime_warnings(lambda: found.append(contour.locate_label(line, 5)))
    assert found == [(20.0, 0.0, 2)]
    assert said == ["invalid value encountered in multiply",
                    "invalid value encountered in subtract"]
