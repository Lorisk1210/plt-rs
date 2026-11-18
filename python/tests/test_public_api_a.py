"""
matplotlib's public names of artists, figures, axes, legends and colorbars
that pltrs (Python) lacked until pltrs (Rust) backed them, each tried once.
The expected values were recorded from matplotlib 3.11.2 running the same
code.
"""

import platform
import warnings

import numpy as np
import pytest

import pltrs
import pltrs.pyplot as plt
from pltrs import (artist, axis, collections, colorbar, colors as mcolors,
                   gridspec, hatch, image, layout_engine, legend,
                   legend_handler, lines, offsetbox, patches, pyplot, quiver,
                   spines, testing, text, ticker)
from pltrs.patches import ArrowStyle, BoxStyle
from pltrs.path import Path
from pltrs.transforms import Bbox, TransformedBbox
from pltrs.tri import Triangulation

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def rounded(values, decimals=3):
    return np.round(np.asarray(values, float), decimals).tolist()


def deprecated(get):
    """The value *get* returns and whether it warned of a deprecation."""
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        value = get()
    return value, [w.category.__name__ for w in caught]


def test_module_names_of_artist_lines_pyplot_text_and_testing():
    assert artist.reduce(lambda a, b: a + b, [1, 2, 3]) == 6
    drawn = artist.allow_rasterization(lambda a, r: 5)
    assert drawn.__name__ == "<lambda>" and drawn._supports_rasterization
    assert sorted(lines.lineStyles) == ['', ' ', '-', '--', '-.', ':', 'None']
    assert sorted(lines.drawStyles) == sorted(lines.STEP_LOOKUP_MAP) == [
        'default', 'steps', 'steps-mid', 'steps-post', 'steps-pre']
    assert lines.fillStyles == ('full', 'left', 'right', 'bottom', 'top', 'none')
    assert len(lines.lineMarkers) == 41
    assert lines.ls_mapper_r == {'solid': '-', 'dashed': '--',
                                 'dashdot': '-.', 'dotted': ':'}
    assert pyplot.TYPE_CHECKING is False
    assert type(pyplot.annotations).__name__ == "_Feature"
    assert type(text.fontManager).__name__ == "FontManager"
    assert callable(testing.ipython_in_subprocess)
    assert offsetbox.DEBUG is False
    assert axis.GRIDLINE_INTERPOLATION_STEPS == 180


def test_artist_sticky_edges_update_from_and_clip_path():
    fig, ax = plt.subplots()
    l1, = ax.plot([0, 1, 2], [3, 1, 2], label="one", alpha=0.5, lw=4, color="r")
    l1.sticky_edges.y.append(0.5)
    l1.sticky_edges.x[:] = [0, 2]
    assert (l1.sticky_edges.x, l1.sticky_edges.y) == ([0, 2], [0.5])
    assert list(l1.sticky_edges) == [[0, 2], [0.5]]
    l2 = lines.Line2D([0, 1], [1, 0])
    l2.update_from(l1)
    assert (l2.get_label(), l2.get_alpha(), l2.get_linewidth(),
            l2.get_color()) == ("one", 0.5, 4.0, "r")
    assert (l2.sticky_edges.x, l2.sticky_edges.y) == ([0, 2], [0.5])
    assert l2.get_transform() is ax.transData
    l1.recache(always=True)
    l1.recache()
    assert l1.get_transformed_clip_path_and_affine() == (None, None)
    l1.set_clip_path(patches.Circle((0.5, 0.5), 0.2, transform=ax.transAxes))
    path, affine = l1.get_transformed_clip_path_and_affine()
    assert len(path.vertices) == 26 and affine is not None


def test_collection_names():
    fig, ax = plt.subplots()
    pc = collections.PolyCollection([[(0, 0), (1, 0), (1, 1)]], facecolors="none")
    assert pc.get_fill() is False
    assert collections.LineCollection([[(0, 0), (1, 1)]]).get_fill() is False
    assert collections.PolyCollection([[(0, 0), (1, 0), (1, 1)]]).get_fill()
    pc.set_verts_and_codes([[(0, 0), (1, 0), (1, 1), (0, 0)]], [[1, 2, 2, 79]])
    assert [p.codes.tolist() for p in pc.get_paths()] == [[1, 2, 2, 79]]
    with pytest.raises(ValueError, match="'codes' must be a 1D list"):
        pc.set_verts_and_codes([[(0, 0)]], [])
    assert collections.RegularPolyCollection(5, rotation=0.3).get_rotation() == 0.3
    src = ax.scatter([1, 2], [3, 4], c=[1, 2], s=[10, 20], linewidths=2,
                     hatch="//")
    dst = collections.PathCollection([Path.unit_circle()])
    dst.update_from(src)
    assert dst.get_linewidth().tolist() == [2]
    assert dst.get_hatch() == "//" and dst.get_cmap().name == "viridis"
    assert dst.get_array().tolist() == [1, 2]
    assert src.get_transOffset() is src.get_offset_transform()
    src.set_transOffset(ax.transAxes)
    assert src.get_offset_transform() is ax.transAxes
    tri = Triangulation([0, 1, 0, 1], [0, 0, 1, 1], [[0, 1, 2], [1, 3, 2]])
    tri.set_mask([True, False])
    paths = collections.TriMesh.convert_mesh_to_paths(tri)
    assert [p.vertices.tolist() for p in paths] == [[[1, 0], [1, 1], [0, 1]]]
    assert [p.codes for p in paths] == [None]


@pytest.mark.parametrize("cls, vertices, count, sums, codes", [
    (hatch.HorizontalHatch, 12, 6, [6, 6], 18),
    (hatch.VerticalHatch, 12, 6, [6, 6], 18),
    (hatch.NorthEastHatch, 14, 6, [7, 7], 21),
    (hatch.SouthEastHatch, 14, 6, [7, 7], 21),
    (hatch.SmallCircles, 416, 2, [208, 204.96], 2816),
    (hatch.LargeCircles, 416, 2, [208, 202.68], 2816),
    (hatch.SmallFilledCircles, 208, 2, [104, 103.2], 1408),
    (hatch.Stars, 88, 2, [44, 45.333333], 784),
])
def test_hatch_pattern_classes(cls, vertices, count, sums, codes):
    pattern = cls("-|/\\oO.*xx++", 2)
    assert isinstance(pattern, hatch.HatchPatternBase)
    assert pattern.num_vertices == vertices
    assert getattr(pattern, "num_lines", getattr(pattern, "num_rows", None)) == count
    v = np.zeros((vertices, 2))
    c = np.zeros(vertices, dtype=Path.code_type)
    pattern.set_vertices_and_codes(v, c)
    assert rounded(v.sum(axis=0), 6) == sums and int(c.sum()) == codes
    assert issubclass(hatch.Circles, hatch.Shapes) and not hatch.Shapes.filled


def test_text_update_from_and_bbox_position():
    t1 = text.Text(0.5, 0.5, "abc", color="g", ha="right", va="top",
                   rotation=30, fontsize=20)
    t2 = text.Text(0, 0, "xyz")
    t2.update_from(t1)
    assert (t2.get_text(), t2.get_color(), t2.get_ha(), t2.get_va(),
            t2.get_rotation(), t2.get_fontsize()) == (
                "xyz", "g", "right", "top", 30.0, 20.0)
    fig, ax = plt.subplots()
    t3 = ax.text(0.2, 0.3, "boxed", bbox=dict(boxstyle="round", pad=0.5),
                 fontsize=12)
    fig.canvas.draw()
    t3.update_bbox_position_size(fig.canvas.get_renderer())
    patch = t3.get_bbox_patch()
    assert rounded(patch.get_bbox().bounds, 1) == [0, 0, 51, 17]
    assert rounded(patch.get_transform().get_matrix(), 1) == [
        [1, 0, 179.2], [0, 1, 159.7], [0, 0, 1]]
    assert round(patch.get_mutation_scale(), 3) == 16.667
    assert isinstance(patch.get_boxstyle(), BoxStyle.Round)


def test_fancy_patch_styles_and_mutation():
    fb = patches.FancyBboxPatch((0, 0), 1, 2, boxstyle="round,pad=0.2",
                                mutation_scale=3, mutation_aspect=0.5)
    assert isinstance(fb.get_boxstyle(), BoxStyle.Round)
    assert fb.get_boxstyle().pad == 0.2
    assert (fb.get_mutation_scale(), fb.get_mutation_aspect()) == (3, 0.5)
    sawtooth = BoxStyle("sawtooth", tooth_size=0.1)
    fb.set_boxstyle(sawtooth)
    assert fb.get_boxstyle() is sawtooth
    fb.set_boxstyle("larrow", pad=0.5)
    assert fb.get_boxstyle().pad == 0.5
    fb.set_mutation_scale(5)
    fb.set_mutation_aspect(None)
    assert (fb.get_mutation_scale(), fb.get_mutation_aspect()) == (5, 1)
    assert "==========" in fb.set_boxstyle()
    with pytest.raises(ValueError, match="Unknown style: 'hexagon'"):
        fb.set_boxstyle("hexagon")
    fa = patches.FancyArrowPatch((0, 0), (1, 1), arrowstyle="->",
                                 connectionstyle="arc3,rad=0.2",
                                 mutation_scale=20)
    assert isinstance(fa.get_arrowstyle(), ArrowStyle.CurveB)
    assert fa.get_connectionstyle().rad == 0.2
    assert (fa.get_mutation_scale(), fa.get_mutation_aspect()) == (20, 1)
    fa.set_arrowstyle("fancy", head_length=0.6)
    fa.set_connectionstyle("angle", angleA=0, angleB=90, rad=5)
    fancy, angle = fa.get_arrowstyle(), fa.get_connectionstyle()
    assert (fancy.head_length, fancy.head_width, fancy.tail_width) == (0.6, 0.4, 0.4)
    assert (angle.angleA, angle.angleB, angle.rad) == (0, 90, 5)
    fa.set_arrowstyle("-[", widthB=2.0)
    assert fa.get_arrowstyle().widthB == 2.0
    fa.set_mutation_scale(7)
    fa.set_mutation_aspect(2)
    assert (fa.get_mutation_scale(), fa.get_mutation_aspect()) == (7, 2)


def test_offsetbox_frames_and_positions():
    fig, ax = plt.subplots()
    at = offsetbox.AnchoredText("hi", loc="upper left")
    ax.add_artist(at)
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    at.update_frame(Bbox.from_bounds(10, 20, 30, 40), fontsize=12)
    assert at.patch.get_bbox().bounds == (10, 20, 30, 40)
    assert at.patch.get_mutation_scale() == 12
    pb = offsetbox.PaddedBox(offsetbox.TextArea("text"), pad=5, draw_frame=True)
    ab = offsetbox.AnnotationBbox(pb, (0.5, 0.5))
    ax.add_artist(ab)
    fig.canvas.draw()
    ab.update_positions(renderer)
    assert rounded(ab.patch.get_bbox().bounds, 1) == [307.7, 223.5, 40.6, 28.2]
    assert round(ab.patch.get_mutation_scale(), 3) == 13.889
    pb.update_frame(Bbox.from_bounds(1, 2, 3, 4), 7)
    assert pb.patch.get_bbox().bounds == (1, 2, 3, 4)
    assert pb.patch.get_mutation_scale() == 7
    ab.xybox = (0.7, 0.3)
    fig.canvas.draw()
    ab.update_positions(renderer)
    assert rounded(ab.patch.get_bbox().bounds, 1) == [406.9, 149.6, 40.6, 28.2]


def test_ticker_and_axis_names():
    fig = plt.figure()
    f = ticker.ScalarFormatter()
    ax = fig.add_subplot(2, 2, 4)
    ax.set_xlim(0, 0.5)
    ax.xaxis.set_major_formatter(f)
    fig.canvas.draw()
    assert deprecated(lambda: f.format) == ("%1.1f", ["MatplotlibDeprecationWarning"])
    assert ticker.PercentFormatter(xmax=5).convert_to_pct(
        np.array([1, 2.5])).tolist() == [20, 50]
    ax3 = fig.add_subplot(2, 2, 3)
    ax3.set_xlabel("label")
    ax3.set_xscale("log")
    ax3.set_xlim(1, 100)
    ax3.xaxis.grid(True)
    ax3.xaxis.clear()
    assert (ax3.get_xlabel(), ax3.get_xscale(), ax3.get_xlim()) == ("", "linear", (1, 100))
    assert isinstance(ax3.xaxis.get_major_locator(), ticker.AutoLocator)
    assert ax3.xaxis._major_tick_kw.get("gridOn") is False
    assert deprecated(lambda: ax3.xaxis.converter) == (
        None, ["MatplotlibDeprecationWarning"])
    ax4 = fig.add_subplot(2, 2, 1)
    ax4.xaxis.set_default_intervals()
    ax4.yaxis.set_default_intervals()
    assert ax4.xaxis.get_view_interval().tolist() == [0, 1]
    tick = ax4.xaxis.get_major_ticks()[0]
    for method, args in [(axis.Tick.get_view_interval, ()),
                         (axis.Tick.update_position, (1,))]:
        with pytest.raises(NotImplementedError, match="Derived must override"):
            method(tick, *args)


def test_legend_anchor_columns_and_dragging():
    fig, ax = plt.subplots()
    ax.plot([1, 2], label="a")
    leg = ax.legend(bbox_to_anchor=(1, 0.5), loc="center left")
    assert rounded(leg.get_bbox_to_anchor().bounds, 2) == [576, 237.6, 0, 0]
    leg.set_bbox_to_anchor((0.1, 0.2, 0.3, 0.4))
    assert rounded(leg.get_bbox_to_anchor().bounds, 2) == [129.6, 126.72, 148.8, 147.84]
    leg.set_bbox_to_anchor(None)
    assert leg.get_bbox_to_anchor().bounds == ax.bbox.bounds
    leg.set_ncols(3)
    assert leg._ncols == 3
    drag = leg.set_draggable(True, update="bbox")
    assert isinstance(drag, legend.DraggableLegend)
    assert isinstance(drag, offsetbox.DraggableOffsetBox)
    assert drag._update == "bbox" and leg.get_draggable()
    assert leg.set_draggable(False) is None and not leg.get_draggable()
    with pytest.raises(ValueError, match="'xx' is not a valid value for update"):
        legend.DraggableLegend(leg, update="xx")


def test_legend_handler_methods():
    fig, ax = plt.subplots()
    l1, = ax.plot([1, 2], label="a")
    leg = ax.legend()
    h = legend_handler.HandlerLine2D(numpoints=3)
    assert [rounded(v) for v in h.get_xdata(leg, 0, 0, 20, 7, 10)] == [
        [3, 10, 17], [3, 10, 17]]
    arts = h.create_artists(leg, l1, 0, 0, 20, 7, 10, ax.transAxes)
    assert [rounded(a.get_xdata()) for a in arts] == [[3, 10, 17]]
    assert mcolors.same_color(arts[0].get_color(), l1.get_color())
    assert legend_handler.HandlerBase(xpad=0.5, ypad=0.2).adjust_drawing_area(
        leg, l1, 1, 2, 20, 7, 10) == (-4, 0, 15, 5)
    assert rounded(legend_handler.HandlerNpointsYoffsets().get_ydata(
        leg, 0, 0, 20, 7, 10)) == [2.625]
    assert rounded(legend_handler.HandlerStem().get_ydata(
        leg, 0, 0, 20, 7, 10)) == [4.812]
    assert legend_handler.HandlerErrorbar(yerr_size=0.2).get_err_size(
        leg, 0, 0, 20, 7, 10) == (5, 2)
    sc = ax.scatter([1, 2], [3, 4], s=[10, 40])
    hr = legend_handler.HandlerPathCollection()
    assert hr.get_sizes(leg, sc, 0, 0, 20, 7, 10) == [25]
    coll = hr.create_collection(sc, [5, 6], [(0, 0), (1, 1)], ax.transAxes)
    assert isinstance(coll, collections.PathCollection)
    assert coll.get_sizes().tolist() == [5, 6]
    rect = patches.Rectangle((0, 0), 1, 1, facecolor="r")
    drawn, = legend_handler.HandlerPatch().create_artists(
        leg, rect, 0, 0, 20, 7, 10, ax.transAxes)
    assert isinstance(drawn, patches.Rectangle)
    assert (drawn.get_facecolor(), drawn.get_width()) == ((1, 0, 0, 1), 20)
    with pytest.raises(NotImplementedError, match="Derived must override"):
        legend_handler.HandlerBase().create_artists(leg, rect, 0, 0, 20, 7, 10, None)


@pytest.mark.filterwarnings("ignore::PendingDeprecationWarning")
def test_figure_frame_current_axes_and_layout_pads():
    fig = plt.figure(figsize=(4, 3))
    assert (fig.get_frameon(), fig.frameon, fig.get_edgecolor(),
            fig.get_linewidth()) == (True, True, (1, 1, 1, 1), 0)
    fig.set_frameon(False)
    fig.set_edgecolor("r")
    fig.set_linewidth(3)
    assert (fig.get_frameon(), fig.frameon, fig.get_edgecolor(),
            fig.get_linewidth()) == (False, False, (1, 0, 0, 1), 3)
    fig.frameon = True
    assert fig.get_frameon()
    ax = fig.gca()
    assert fig.gca() is ax and len(fig.axes) == 1
    ax2 = fig.add_subplot(212)
    assert fig.gca() is ax2
    fig.sca(ax)
    assert fig.gca() is ax and plt.gca() is ax
    sf = fig.subfigures(1, 2)[0]
    sax = sf.add_subplot()
    assert sf.gca() is sax
    sf.sca(sax)
    assert fig.gca() is sax
    extra = fig.get_default_bbox_extra_artists()
    assert [type(a).__name__ for a in extra[:4]] == [
        "Axes", "Axes", "SubFigure", "SubFigure"]
    assert len(extra) == 19
    assert len(ax.get_default_bbox_extra_artists()) == 5
    fig2 = plt.figure(layout="constrained")
    assert deprecated(lambda: fig2.set_constrained_layout_pads(
        w_pad=0.1, h_pad=0.2))[1] == ["PendingDeprecationWarning"]
    assert fig2.get_constrained_layout_pads() == (0.1, 0.2, 0.02, 0.02)
    sf2 = fig2.subfigures(1, 1)
    assert sf2.get_constrained_layout_pads() == (0.1, 0.2, 0.02, 0.02)
    assert rounded(sf2.get_constrained_layout_pads(relative=True)) == [
        0.016, 0.042, 0.02, 0.02]
    fig3 = plt.figure()
    canvas = fig3.canvas
    fig3.set_canvas(canvas)
    assert fig3.canvas is canvas


def test_gridspec_ratios_layout_engine_and_images():
    gs = gridspec.GridSpec(2, 3)
    gs.set_width_ratios([1, 2, 3])
    gs.set_height_ratios([2, 1])
    assert (gs.get_width_ratios(), gs.get_height_ratios()) == ([1, 2, 3], [2, 1])
    gs.set_width_ratios(None)
    assert gs.get_width_ratios() == [1, 1, 1]
    with pytest.raises(ValueError, match="Expected the given number of height"):
        gs.set_height_ratios([1, 2, 3])
    fig = plt.figure()
    gs2 = fig.add_gridspec(1, 2)
    a1, a2 = fig.add_subplot(gs2[0]), fig.add_subplot(gs2[1])
    gs2.set_width_ratios([1, 3])
    fig.canvas.draw()
    assert rounded(a1.get_position().bounds) == [0.125, 0.11, 0.352, 0.77]
    assert rounded(a2.get_position().bounds) == [0.548, 0.11, 0.352, 0.77]
    with pytest.raises(NotImplementedError):
        layout_engine.LayoutEngine().set()
    fig = plt.figure(figsize=(2, 2), dpi=50)
    fi = fig.figimage(np.arange(16).reshape(4, 4), xo=10, yo=5)
    fig.canvas.draw()
    im, x, y, trans = fi.make_image(fig.canvas.get_renderer())
    assert (im.shape, x, y) == ((4, 4, 4), 10, 5)
    assert rounded(trans.get_matrix()) == [[1, 0, 0], [0, -1, 4], [0, 0, 1]]
    ax = fig.add_subplot()
    bi = image.BboxImage(TransformedBbox(Bbox([[0.2, 0.2], [0.6, 0.8]]),
                                         ax.transAxes),
                         data=np.arange(6).reshape(2, 3))
    ax.add_artist(bi)
    fig.canvas.draw()
    im, x, y, trans = bi.make_image(fig.canvas.get_renderer())
    assert (im.shape, x, y) == ((47, 31, 4), 28, 26)
    assert rounded(trans.get_matrix()) == [[10.333, 0, 0], [0, -23.1, 46.6], [0, 0, 1]]


def test_colorbar_axes_and_ticks():
    fig, (b1, b2) = plt.subplots(1, 2)
    cax, kw = colorbar.make_axes([b1, b2], shrink=0.5)
    assert kw == {"orientation": "vertical", "ticklocation": "right"}
    assert rounded(cax.get_position().bounds) == [0.784, 0.302, 0.014, 0.385]
    cax2, kw2 = colorbar.make_axes(b1, location="top", fraction=0.1)
    assert (kw2["orientation"], kw2["ticklocation"]) == ("horizontal", "top")
    assert rounded(cax2.get_position().bounds) == [0.125, 0.803, 0.282, 0.019]
    with pytest.raises(TypeError, match="location and orientation are mutually"):
        colorbar.make_axes(b1, location="left", orientation="horizontal")
    fig, b3 = plt.subplots()
    cax3, kw3 = colorbar.make_axes_gridspec(b3, orientation="horizontal")
    fig.canvas.draw()
    assert kw3 == {"orientation": "horizontal", "ticklocation": "bottom"}
    assert rounded(cax3.get_position().bounds) == [0.125, 0.174, 0.775, 0.052]
    assert rounded(b3.get_position().bounds) == [0.125, 0.341, 0.775, 0.539]
    fig, b4 = plt.subplots()
    cb = fig.colorbar(b4.imshow(np.arange(4).reshape(2, 2)))
    cb.ax.yaxis.set_major_locator(ticker.FixedLocator([0.5, 1.5]))
    cb.update_ticks()
    assert rounded(cb.get_ticks()) == [0, 0.5, 1, 1.5, 2, 2.5, 3]


def test_contour_labeler_and_contour_set_names():
    fig, ax = plt.subplots()
    xx, yy = np.meshgrid(np.linspace(-2, 2, 30), np.linspace(-2, 2, 30))
    cs = ax.contour(xx, yy, xx ** 2 + yy ** 2, levels=[1, 2, 3])
    assert cs.alpha is None and cs.linestyles is None
    cs.clabel(fmt="%1.1f")
    assert (len(cs.labelTexts), len(cs.labelXYs)) == (3, 3)
    assert rounded(cs.labelLevelList) == [1, 2, 3]
    assert [t.get_text() for t in cs.labelTexts] == ["1.0", "2.0", "3.0"]
    assert cs.get_text(1.0, "%1.2f") == "1.00"
    assert cs.get_text(2.0, {2.0: "two"}) == "two"
    assert cs.get_text(3.0, lambda v: f"<{v}>") == "<3.0>"
    assert cs.too_close(*cs.labelXYs[0], 1.0)
    assert not cs.too_close(-1e6, -1e6, 1.0)
    assert cs.print_label(np.array([[0, 0], [100, 0]]), 10)
    assert not cs.print_label(np.array([[0, 0], [1, 0]]), 10)
    x, y, index = cs.locate_label(
        np.column_stack([np.linspace(0, 200, 50), np.zeros(50)]), 20)
    assert (rounded([x, y]), index) == ([40.816, 0], 10)
    cs.add_label(10, 20, 0, 1.0, 0.5)
    assert len(cs.labelTexts) == 4
    assert rounded(cs.labelTexts[-1].get_position()) == [-2.565, -2.355]
    assert cs.labelTexts[-1].get_text() == "1.0"
    cs.remove()
    assert len(ax.texts) == 0
    fig, ax = plt.subplots()
    cs2 = ax.contour(xx, yy, xx ** 2 + yy ** 2, levels=[1, 2],
                     linestyles="dashed", alpha=0.5)
    assert (cs2.alpha, cs2.linestyles) == (0.5, "dashed")
    cs2.clabel()
    before = len(cs2.labelTexts)
    cs2.labels(True, 4)
    # matplotlib 3.11.2 on arm64 macOS and on aarch64 Linux, whose
    # contourpy fuses the products of its crossings, finds room for one
    # label more on these circles (recorded under both).
    fused = platform.machine() in ("arm64", "aarch64")
    assert len(cs2.labelTexts) == 2 * before + (1 if fused else 0)


def test_barbs_and_quiver_key():
    fig, ax = plt.subplots()
    barbs = ax.barbs([0, 1], [0, 1], [10, 20], [5, 0])
    barbs.set_UVC([30, 40], [0, 5])
    assert (rounded(barbs.u), rounded(barbs.v)) == ([30, 40], [0, 5])
    assert len(barbs.get_paths()) == 2
    barbs.set_UVC([30, 40], [0, 5], [1, 2])
    assert rounded(barbs.get_array()) == [1, 2]
    q = ax.quiver([0, 1], [0, 1], [1, 1], [1, 2])
    key = ax.quiverkey(q, 0.5, 0.5, 1, "label", labelcolor="r",
                       fontproperties={"size": 7})
    warned = ["MatplotlibDeprecationWarning"]
    assert deprecated(lambda: key.kw) == ({}, warned)
    color, caught = deprecated(lambda: key.labelcolor)
    assert mcolors.same_color(color, "r") and caught == warned
    font, caught = deprecated(lambda: key.fontproperties)
    assert font.get_size() == 7 and caught == warned
    fig.canvas.draw()
    verts, caught = deprecated(lambda: key.verts)
    assert len(verts) == 1 and caught == warned


def test_spine_shapes():
    fig, ax = plt.subplots()
    spine = ax.spines["left"]
    assert np.allclose(spine.get_spine_transform().transform([(0.5, 0.5)]),
                       ax.get_yaxis_transform(which="tick1").transform([(0.5, 0.5)]))
    spine.set_patch_circle((0.5, 0.5), 0.2)
    assert spine._patch_type == "circle"
    spine.set_patch_arc((0.5, 0.5), 0.2, 0, 90)
    assert spine._patch_type == "arc"
    spine.set_patch_line()
    assert spine._patch_type == "line"
    spine.set_position(("outward", 10))
    spine.clear()
    assert spine.get_position() == ("outward", 0.0)
    arc = spines.Spine.arc_spine(ax, "top", (0.5, 0.5), 0.5, 0, 180)
    assert (arc.spine_type, arc._patch_type) == ("top", "arc")
    circle = spines.Spine.circular_spine(ax, (0.5, 0.5), 0.5)
    assert isinstance(circle, spines.Spine) and circle._patch_type == "circle"


def test_table_cell_text_measures_and_drawing():
    fig, ax = plt.subplots()
    table = ax.table(cellText=[["a", "longer text"]], loc="center")
    cell = table[0, 1]
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    assert isinstance(cell.get_text(), text.Text)
    assert cell.get_text() is cell.get_text()
    assert cell.get_text().get_text() == "longer text"
    cell.get_text().set_text("new")
    assert cell.get_text().get_text() == "new"
    cell.get_text().set_text("longer text")
    assert rounded(cell.get_text_bounds(renderer)) == [0.795, 0.481, 0.155, 0.039]
    assert round(cell.get_required_width(renderer), 4) == 0.1857
    cell.set_width(0.05)
    assert cell.auto_set_font_size(renderer) == cell.get_fontsize() == 2
    assert cell.set(width=0.3, lw=2) == [None, None]
    assert cell.get_width() == 0.3
    with pytest.raises(AttributeError, match="unexpected keyword argument 'foo'"):
        cell.set(foo=1)
    cell.set_figure(fig)
    with pytest.raises(RuntimeError, match="more than one figure"):
        cell.set_figure(plt.figure())
    cell.set_transform(ax.transAxes)
    cell.draw(renderer)
