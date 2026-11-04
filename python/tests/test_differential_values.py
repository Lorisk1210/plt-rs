"""
Values, types, staleness and warnings the differential tester (``cargo
xtask diff``) found pltrs answering otherwise than matplotlib 3.11: each
expected value recorded from matplotlib.
"""

import io
import logging
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import collections, colors, gridspec, patches, ticker, transforms
from pltrs.offsetbox import AnchoredOffsetbox, AnnotationBbox, OffsetImage, TextArea, VPacker

Z = np.outer(np.linspace(-1, 1, 5), np.linspace(0, 2, 6))
VALUES = np.array([3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])


@pytest.fixture(autouse=True)
def matplotlib_defaults():
    """matplotlib's default rcParams, whatever the look in force."""
    with pltrs.rc_context():
        pltrs.rcdefaults()
        yield
    plt.close("all")


def runtime_warnings(action):
    """The messages of the `RuntimeWarning`\\s *action* issues."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        try:
            action()
        except ValueError:
            pass
    return [str(w.message) for w in caught if w.category is RuntimeWarning]


# -- values typed as matplotlib keeps them ------------------------------------

def test_dash_patterns_are_scaled_as_given():
    lines = collections.LineCollection([[(0, 0), (1, 1)]])
    lines.set_linewidth(2)
    lines.set_linestyle((0, (3, 1)))
    (offset, dashes), = lines.get_linestyle()
    assert isinstance(offset, np.int64) and offset == 0
    assert [type(d) for d in dashes] == [np.int64, np.int64] and dashes == [6, 2]
    spine = plt.figure().add_subplot().spines["left"]
    spine.set_linestyle((0, (3, 1)))
    assert spine.get_linestyle() == (0, (3, 1))
    spine.set_linestyle('dashdot')
    assert spine.get_linestyle() == 'dashdot'


def test_hist_steps_bars_and_hatches():
    ax = plt.figure().add_subplot()
    step = ax.hist(VALUES, bins=[0, 2, 4, 10], histtype='step')[2][0]
    assert step.get_label() is None
    assert step.get_facecolor()[3] == 0 and type(step.get_facecolor()[3]) is int
    filled = ax.hist(VALUES, bins=[0, 2, 4, 10], histtype='stepfilled')[2][0]
    assert type(filled.get_facecolor()[3]) is int
    bar = ax.bar([1, 2], [3, 4], hatch='//')[0]
    assert type(bar.get_hatch()) is np.str_
    bar.set_fill(False)
    assert bar.get_facecolor()[3] == 0 and type(bar.get_facecolor()[3]) is int


def test_lines_keep_the_data_they_were_made_with():
    ax = plt.figure().add_subplot()
    line, = ax.plot([1, 2, 3])
    assert line.get_xdata().dtype == np.float64 and line.get_ydata().dtype == np.int64
    caps = ax.errorbar([1, 2], [1, 2], yerr=[0.1, 0.2], capsize=3)[1]
    assert caps[0].get_xdata().dtype == object
    assert caps[0].get_markeredgecolor() == (1.0, 0.4980392156862745, 0.054901960784313725)
    none_fmt = ax.errorbar([1, 2], [1, 2], yerr=0.1, fmt='none', capsize=2)
    assert none_fmt[0] is None and none_fmt[1][0].get_mec() == 'C0'
    markers, _, baseline = ax.stem(VALUES)
    assert markers.get_xdata().dtype == np.int64
    assert markers.get_color() == (0.12156862745098039, 0.4666666666666667,
                                   0.7058823529411765, 1)
    assert baseline.get_data()[0].tolist() == [0, 6]
    axline = ax.axline((0, 0), (1, 2))
    assert axline.get_data() == ([0, 1], [0, 1])


def test_scatter_faces_and_sizes():
    points = plt.figure().add_subplot().scatter(
        [1, 2], [1, 2], s=50, facecolors='none', edgecolors='k')
    assert points.get_fill()
    assert points.get_sizes().dtype == np.int64


def test_ticks_typed_as_their_locators_give_them():
    ax = plt.figure().add_subplot()
    ax.set_xscale('symlog')
    ticks = ax.get_xticks()
    assert isinstance(ticks, list) and ticks == [0.0, 1.0]
    assert type(ticks[0]) is np.float64
    ax.yaxis.set_major_locator(ticker.NullLocator())
    assert ax.get_yticks() == []
    assert isinstance(ax.get_yticks(minor=True), np.ndarray)
    fig, ax = plt.subplots()
    ax.psd(np.sin(np.linspace(0, 20, 300)))
    assert ax.get_yticks().dtype == np.int64


def test_subplot_parameters_and_specs():
    fig, axs = plt.subplots(2, 2)
    fig.tight_layout()
    assert type(fig.subplotpars.left) is np.float64
    fig = plt.figure()
    grid = fig.add_gridspec(2, 3)
    spec = fig.add_subplot(grid[0, :2]).get_subplotspec()
    assert type(spec.num1) is np.int64 and type(spec.get_geometry()[0]) is int
    fig = plt.figure()
    gs = gridspec.GridSpec(2, 1)
    fig.add_subplot(gs[0])
    gs.tight_layout(fig, rect=[0, 0, 0.5, 1])
    assert type(gs.top) is np.float64
    gs.update(top=0.9)
    assert gs.top == 0.9 and type(gs.top) is float


def test_bbox_predicates_are_numpy_scalars():
    bbox = transforms.Bbox([[0, 0], [1, 2]])
    assert type(bbox.contains(0.5, 0.5)) is np.bool_
    assert type(bbox.overlaps(transforms.Bbox.unit())) is np.bool_
    assert type(bbox.count_contains([[0.5, 0.5], [3, 3]])) is np.int64


def test_arrow_styles_derive_from_curve():
    assert "_Curve" in [c.__name__ for c in patches.ArrowStyle.CurveB.__mro__]
    assert "_Curve" not in [c.__name__ for c in patches.ArrowStyle.Fancy.__mro__]


def test_paths_shared_or_made_anew():
    ax = plt.figure().add_subplot()
    span = ax.axvspan(0.1, 0.3)
    bar = ax.bar([1, 2], [3, 4])[0]
    assert span.get_path() is bar.get_path()
    box = patches.FancyBboxPatch((0.2, 0.2), 0.4, 0.3, boxstyle='round,pad=0.05')
    assert box.get_path() is not box.get_path()


def test_child_axes_and_specs():
    ax = plt.figure().add_subplot()
    assert ax.inset_axes([0.5, 0.5, 0.3, 0.3]).axes is ax
    assert ax.secondary_xaxis('top').axes is ax
    with pytest.raises(AttributeError, match="no attribute 'get_position'"):
        ax.set_subplotspec(None)


def test_text_measures():
    fig = plt.figure()
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    width, height, descent = renderer.get_text_width_height_descent(
        '$x^2$', pltrs.font_manager.FontProperties(), True)
    assert type(width) is np.float64
    assert type(pltrs.mathtext.MathTextParser('path').parse('$x^2$').width) is np.float64
    with pytest.raises(AttributeError, match="'NoneType' object has no attribute 'dpi'"):
        pltrs.text.Text(0, 0, 'loose').get_window_extent(renderer)


def test_contour_widths_and_hatched_bands():
    fig, ax = plt.subplots()
    lines = ax.contour(Z, 3, linewidths=2)
    assert lines.get_linewidth().tolist() == [2, 2, 2, 2, 2]
    assert lines.get_linewidth().dtype == np.int64
    lines.set_linestyle((0, (3, 1)))
    lines.set_linewidth(2)
    assert len(lines.get_linestyle()) == 1
    fig, ax = plt.subplots()
    filled = ax.contourf(Z, hatches=['//', None])
    assert len(ax.collections) == 1
    assert filled.get_facecolor().shape == (8, 4)
    fig.canvas.draw()
    assert filled.get_facecolor().shape == (1, 4)
    line, = ax.plot([1, 2])
    assert line.get_label() == '_child1'


def test_spy_keeps_a_masked_array():
    image = plt.figure().add_subplot().spy(Z > 0)
    assert isinstance(image.get_array(), np.ma.MaskedArray)
    assert image.get_clim() == (False, True)


def test_titles_are_made_anew_by_clear():
    ax = plt.figure().add_subplot()
    title = ax.set_title("a")
    ax.cla()
    assert ax.set_title("b") is not title


# -- tables ------------------------------------------------------------------

def test_table_cells_take_their_placement():
    fig, ax = plt.subplots()
    cell = ax.table([['a', 'b'], ['c', 'd']], loc='center')[0, 1]
    assert cell.xy == (0, 0) and type(cell.get_height()) is np.float64
    cell.set_bounds((0, 0, 2, 3))
    assert (cell.get_width(), cell.get_height()) == (2, 3)
    assert type(cell.get_width()) is int
    cell.set_xy((1, 2))
    fig.canvas.draw()
    assert [type(v) for v in cell.xy] == [np.float64, np.float64]
    assert cell.get_fontsize() == 10.0
    assert not cell.stale
    cell.set_width(0.001)
    fig.canvas.draw()
    assert cell.get_fontsize() == 1.0


# -- staleness ---------------------------------------------------------------

def test_staleness_follows_matplotlibs_draws_and_measures():
    fig, ax = plt.subplots()
    note = ax.annotate('note', xy=(0.2, 0.2), xytext=(0.6, 0.7),
                       arrowprops=dict(arrowstyle='->'))
    fig.canvas.draw()
    assert not note.stale and not ax.xaxis.stale
    note.get_window_extent()
    assert note.stale
    fig.canvas.draw()
    ax.get_tightbbox()
    assert ax.stale and fig.stale
    fig.canvas.draw()
    ax.get_position()
    assert ax.stale
    fig, ax = plt.subplots()
    ax.plot([1, 2])
    clipped = ax.annotate('out', xy=(0.2, 0.2), xytext=(0.6, 0.7),
                          arrowprops=dict(arrowstyle='->'))
    fig.canvas.draw()
    assert clipped.stale
    inset = ax.inset_axes([0.5, 0.5, 0.3, 0.3])
    indicator = ax.indicate_inset_zoom(inset)
    fig.canvas.draw()
    assert indicator.stale


def test_boxes_inside_boxes():
    fig, ax = plt.subplots()
    image = OffsetImage(Z, zoom=2)
    ax.add_artist(AnnotationBbox(image, (0.5, 0.5)))
    area = TextArea("hi")
    ax.add_artist(AnchoredOffsetbox('upper left', child=VPacker(children=[area])))
    assert image.get_figure() is fig and image.axes is None
    assert area.axes is ax and image.stale_callback is None
    fig.canvas.draw()
    assert not image.stale and not area.stale
    image.set_zoom(3)
    assert image.stale


# -- images ------------------------------------------------------------------

def test_norms_left_unscaled_scale_when_drawn():
    fig, ax = plt.subplots()
    image = ax.imshow((Z * 50).astype(np.uint8))
    image.set_norm(colors.LogNorm())
    assert image.get_clim() == (None, None)
    ax.set_xlim((0, 10))
    ax.set_ylim((-1, 1))
    ax.set_aspect('equal')
    fig.canvas.draw()
    assert image.get_clim() == (156.0, 246.0)
    fig = plt.figure()
    figure_image = fig.figimage(Z)
    assert figure_image.get_clim() == (None, None)
    fig.canvas.draw()
    assert figure_image.get_clim() == (-2.0, 2.0)


def test_a_norm_that_cannot_map_raises_valueerror():
    fig, ax = plt.subplots()
    ax.imshow(Z, norm='log').set_clim((0, 1))
    with pytest.raises(ValueError, match="Invalid vmin or vmax"):
        fig.canvas.draw()


# -- warnings and logs -------------------------------------------------------

def test_numpy_runtime_warnings_of_locators():
    assert runtime_warnings(lambda: ticker.LogLocator().tick_values(5, -5)) == [
        "invalid value encountered in log10"]
    assert runtime_warnings(lambda: ticker.AsinhLocator(1.0).tick_values(5, 5)) == [
        "divide by zero encountered in divide"]
    fig, ax = plt.subplots()
    ax.set_ylim((-1, 1))
    ax.set_yscale('logit')
    assert "One of rtol or atol is not valid, atol: nan, rtol: 0" in runtime_warnings(
        ax.get_yminorticklabels)


def test_postscript_logs_transparency(caplog):
    fig, ax = plt.subplots()
    ax.plot([1, 2], alpha=0.5)
    with caplog.at_level(logging.WARNING):
        fig.savefig(io.BytesIO(), format="eps")
    assert ["The PostScript backend does not support transparency; partially "
            "transparent artists will be rendered opaque."] == [
        r.getMessage() for r in caplog.records if "transparency" in r.getMessage()]


def test_ticks_sit_at_zero_until_placed():
    fig, ax = plt.subplots()
    ax.plot([1, 2, 3])
    tick = ax.xaxis.get_major_ticks()[0]
    assert tick.get_loc() == 0 and type(tick.get_loc()) is int
    fig.canvas.draw()
    assert tick.get_loc() == -0.25 and type(tick.get_loc()) is np.float64
