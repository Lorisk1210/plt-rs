"""
Smoke tests of what the differential tester's ``api`` programs found in
the core 2-D modules after task APIE (task APIF). The expected values were
recorded from matplotlib 3.11.2 running the same code.
"""

import io
import logging

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import (artist as martist, backend_bases, bezier, colorbar, colors, lines, patches,
                   path, sankey, ticker, transforms)
from pltrs.axis import Axis
from pltrs.backends import backend_agg, backend_pdf, backend_pgf
from pltrs.figure import Figure

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def test_logit_locator_refuses_polar_axes_first():
    ax = plt.figure().add_subplot(projection='polar')
    ax.plot(np.linspace(0, 6, 7), [3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])
    ax.set(ylim=(-1, 1), yscale='logit')
    with pytest.raises(NotImplementedError, match="^Polar axis cannot be logit scaled yet$"):
        ax.set(yticklabels=['a'])


def test_logit_locator_gives_a_list_for_a_zoomed_view():
    _, ax = plt.subplots()
    minor = ticker.LogitLocator(minor=True)
    minor.set_axis(ax.xaxis)
    assert minor.tick_values(0.4, 0.6) == []
    assert isinstance(minor.tick_values(0.01, 0.99), np.ndarray)


def test_animated_axes_are_not_ticked_when_drawn():
    fig = plt.figure()
    ax = fig.add_subplot(projection='polar')
    ax.plot(np.linspace(0, 6, 7), [3.0, 1.0, 4.0, 1.5, 5.0, 9.0, 2.5])
    ax.set(ylim=(0.5, 1.05), yscale='logit', yticks=[0.5], animated=True)
    fig.canvas.draw()


def test_status_bar_texts_follow_the_formatters():
    _, ax = plt.subplots()
    x = np.linspace(0, 2 * np.pi, 7)
    ax.plot(x, np.sin(x))
    ax.xaxis_date()
    assert ax.format_xdata(0.5) == '1970'
    assert ax.format_coord(0.5, 0.25) == '(x, y) = (1970, 0.250)'
    with pytest.raises(ValueError, match="^cannot convert float NaN to integer$"):
        ax.format_coord(np.nan, 1)
    polar = plt.figure().add_subplot(projection='polar')
    assert polar.format_coord(0.5, 0.5) == 'θ=0.159π (28.6°), r=0.500'


def test_streamplot_takes_its_colour_before_checking():
    _, ax = plt.subplots()
    args = (np.arange(6), np.arange(5), np.ones((5, 6)), np.ones((5, 6)))
    with pytest.raises(ValueError, match="integration_direction"):
        ax.streamplot(*args, integration_direction='nope')
    lines_ = ax.streamplot(*args).lines
    assert colors.to_hex(lines_.get_edgecolor()[0]) == '#ff7f0e'


def test_figimage_checks_in_matplotlibs_order():
    fig = plt.figure()
    with pytest.raises(ValueError, match="'nonexistent' is not a valid value for cmap"):
        fig.figimage(np.arange(6), cmap='nonexistent')
    with pytest.raises(IndexError, match="^tuple index out of range$"):
        fig.figimage(np.arange(6), resize=True)
    with pytest.raises(TypeError, match=r"^Invalid shape \(6,\) for image data$"):
        fig.figimage(np.arange(6), vmin=0, norm='log')
    image = fig.figimage(np.arange(12.0).reshape(3, 4), resize=True)
    assert tuple(fig.get_size_inches()) == (0.04, 0.03)
    assert image.get_extent() == (-0.5, 3.5, -0.5, 2.5)


def test_a_base_axis_cannot_be_cleared():
    _, ax = plt.subplots()
    with pytest.raises(AttributeError, match="^'Axis' object has no attribute '_init'$"):
        Axis(ax)


def test_a_bezier_segment_of_no_points():
    curve = bezier.BezierSegment(np.empty((0, 2)))
    assert curve.degree == -1
    np.testing.assert_array_equal(curve([0.1, 0.2]), np.zeros((2, 2)))
    dims, roots = curve.axis_aligned_extrema()
    assert dims.size == roots.size == 0
    with pytest.raises(ValueError, match="^cannot call `vectorize` on size 0 inputs"):
        curve.polynomial_coefficients


def test_sankey_keeps_an_offset_it_cannot_use():
    diagram = sankey.Sankey(offset=(1.0, -1.0))
    assert diagram.offset == (1.0, -1.0)
    with pytest.raises(ValueError, match="^setting an array element with a sequence"):
        diagram.add()
    with pytest.raises(TypeError, match=r"^unsupported operand type\(s\) for /=: 'str' and 'float'$"):
        sankey.Sankey(ax=plt.gca()).add(rotation='vertical')
    with pytest.raises(ValueError, match="^Axis limits cannot be NaN or Inf$"):
        sankey.Sankey().finish()


def test_patch_data_matplotlib_cannot_take():
    arrow = patches.Arrow(0.1, 0.1, 0.5, 0.3)
    with pytest.raises(ValueError, match=r"^setting an array element with a sequence\.$"):
        arrow.set_data(np.ones((5, 6)))
    _, ax = plt.subplots()
    steps = ax.stairs([1, 3, 2], [0, 1, 2, 3])
    with pytest.raises(ValueError, match=r"`len\(values\) = 30` and `len\(edges\) = 4`"):
        steps.set_data(np.ones((5, 6)))
    square = path.Path([(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)], closed=True)
    steps.set_path(square)
    np.testing.assert_array_equal(steps.get_path().vertices, square.vertices)


def test_connection_patch_keeps_unknown_coordinates():
    _, ax = plt.subplots()
    patch = patches.ConnectionPatch((0, 0), (1, 1), 'nope')
    assert (patch.coords1, patch.coords2) == ('nope', 'nope')
    with pytest.raises(ValueError, match="^nope is not a valid coordinate transformation$"):
        patch.get_path()
    ax.add_artist(patch)
    with pytest.raises(ValueError, match="^nope is not a valid coordinate transformation$"):
        ax.figure.canvas.draw()


def test_colorbar_values_it_cannot_bracket():
    _, ax = plt.subplots()
    with pytest.raises(ValueError, match=r"^could not broadcast input array from shape \(2,\) into shape \(1,\)$"):
        colorbar.Colorbar(ax, values=np.array([[0.0, 1.0], [2.0, 3.0]]))
    with pytest.raises(IndexError, match="^index 2 is out of bounds for axis 0 with size 2$"):
        colorbar.Colorbar(ax, values=[1.0])


def test_paths_whose_vertices_are_a_list():
    p = path.Path([(0, 0), (1, 0), (1, 1), (0, 0)], [1, 2, 2, 79])
    p.vertices = [(0, 0), (1, 0), (0, 1)]
    p.codes = [1, 2, 2]
    with pytest.raises(TypeError, match="^only integer scalar arrays"):
        p.get_extents()
    with pytest.raises(AttributeError, match="'list' object has no attribute 'reshape'"):
        p.interpolated(3)
    assert p.get_extents(transforms.Affine2D()).bounds == (0, 0, 1, 1)
    p.codes = None
    with pytest.raises(AttributeError, match="'list' object has no attribute 'min'"):
        p.get_extents()


def test_units_replace_the_categories_of_an_axis():
    _, ax = plt.subplots()
    axis = ax.xaxis
    axis.update_units(['a', 'b'])
    with pytest.raises(ValueError, match='^Provided unit "None" is not valid'):
        axis.set_units(None)
    assert axis.get_units() is None
    with pytest.raises(ValueError, match='^Provided unit "inches" is not valid'):
        axis.set_units('inches')
    assert axis.get_units() == 'inches'


def test_data_intervals_leave_the_view():
    _, ax = plt.subplots()
    ax.plot(np.linspace(0, 2 * np.pi, 7), np.arange(7))
    ax.xaxis.set_view_interval(0.0, 4.0, ignore=True)
    ax.xaxis.set_data_interval(0.0, 4.0)
    assert tuple(ax.get_xlim()) == (0.0, 4.0)


def test_limits_typed_and_refused_as_matplotlib_sets_them():
    _, ax = plt.subplots()
    ax.plot(np.arange('2024-01-01', '2024-01-08', dtype='datetime64[D]'), np.arange(7))
    ax.set_yscale('log')
    with pytest.warns(UserWarning, match="identical low and high xlims"):
        low, high = ax.set_xlim(1, 1)
    assert (low, high) == (-729.0, 731.0) and type(low) is float
    with pytest.raises(ValueError, match="^Axis limits cannot be NaN or Inf$"):
        ax.set_ylim(np.inf, 0)


def test_scale_norms_on_values_they_cannot_map():
    for norm in (colors.AsinhNorm(), colors.SymLogNorm(1.0)):
        norm.autoscale([1, 5])
        assert norm.inverse(float('nan')) is np.ma.masked
        assert np.ma.getmaskarray(norm.inverse([0.5])).tolist() == [False]
    with pytest.raises(TypeError, match="^ufunc 'divide' not supported"):
        colors.AsinhNorm(vmin=0, vmax=1)('v')
    with pytest.raises(TypeError, match="^ufunc 'absolute' did not contain a loop"):
        colors.SymLogNorm(1.0, vmin=0, vmax=1)('v')


def test_prints_notify_the_figure_as_matplotlibs_do():
    fig = plt.figure(figsize=(4, 3), dpi=80, layout='constrained')
    fig.subplots(2, 2)
    seen = []
    fig.add_callback(lambda artist: seen.append(str(artist)))
    fig.savefig(io.BytesIO(), format='png')
    assert seen == ['Figure(320x240)'] * 2
    seen.clear()
    fig.savefig(io.BytesIO(), format='pdf', facecolor='r')
    assert seen == ['Figure(320x240)', 'Figure(288x216)', 'Figure(288x216)',
                    'Figure(288x216)']


def test_the_figure_background_ignores_the_figures_sketch_and_clip():
    for setting in (lambda fig: fig.set_sketch_params(scale=2),
                    lambda fig: fig.set_clip_path(patches.Circle((0.5, 0.5), 0.3))):
        fig, ax = plt.subplots()
        ax.plot([1, 2, 3])
        setting(fig)
        fig.canvas.draw()
        assert np.asarray(fig.canvas.buffer_rgba())[..., 3].min() == 255


def test_axes_a_figure_lacks():
    fig = plt.figure()
    sub = fig.add_subfigure(fig.add_gridspec(1, 2)[0, 1])
    own = fig.add_subplot(121)
    other = plt.figure().add_subplot()
    with pytest.raises(ValueError, match="^Axes has not been added yet$"):
        fig.sca(other)
    with pytest.raises(KeyError):
        fig.delaxes(other)
    with pytest.raises(ValueError, match=r"^list.remove\(x\): x not in list$"):
        sub.delaxes(own)


def test_colorbars_of_a_mappable_in_another_figure():
    vfig = plt.figure(figsize=(3, 2), dpi=50)
    vax = vfig.add_subplot()
    fig = plt.figure(figsize=(4, 3), dpi=80)
    fig.subplots(2, 2)
    with pytest.warns(UserWarning, match="<Figure size 150x100 with 2 Axes> than "
                      "<Figure size 320x240 with 4 Axes>"):
        cb = fig.colorbar(vax.imshow(np.arange(30.0).reshape(5, 6)))
    assert cb.ax.figure is vfig and cb.ax in vfig.axes
    assert all(type(spine).__name__ == 'Spine'
               for ax in fig.axes for spine in ax.spines.values())


def test_constrained_figures_are_laid_out_when_drawn():
    fig = plt.figure(figsize=(4, 3), dpi=80, layout='constrained')
    ax = fig.subplots(2, 2)[0, 0]
    assert fig.get_tightbbox().extents.tolist() == pytest.approx(
        [0.17777777777777776, 0.09388888888888886, 3.7125, 2.692761501736111])
    assert ax.get_position().bounds == pytest.approx((0.125, 0.53, 0.35227272727272724, 0.35))
    fig.canvas.draw()
    assert ax.get_position().bounds == pytest.approx(
        (0.11284805555555555, 0.5925937037037037, 0.3486094444444444, 0.375929129050926))


def test_vlines_limits_go_through_the_scales():
    _, ax = plt.subplots()
    ax.plot([0.1, 7.28], [-0.866, 9.87])
    ax.set_xscale('log')
    ax.set_yscale('log', base=2)
    ax.vlines(np.arange(-3, 4), [0], [0.5, 0.6, 0.7, 0.9, 0.7, 0.6, 0.5])
    # 2 ** -1000, as numpy's power gives it there.
    assert ax.dataLim.minpos.tolist() == pytest.approx([0.1, 9.332636185031453e-302])
    assert ax.dataLim.bounds == (0.0, -0.866, 7.28, 9.87 + 0.866)


def test_pdf_renderers_draw_with_pdf_graphics_contexts():
    pdf = backend_pdf.PdfFile(io.BytesIO())
    pdf.newPage(4, 3)
    renderer = backend_pdf.RendererPdf(pdf, 72, 3, 4)
    assert type(renderer.new_gc()).__name__ == 'GraphicsContextPdf'
    gc = backend_agg.FigureCanvasAgg(Figure()).get_renderer().new_gc()
    square = path.Path([(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)], closed=True)
    with pytest.raises(AttributeError, match="'GraphicsContextBase' object has no attribute 'fill'"):
        renderer.draw_markers(gc, path.Path.unit_circle(), transforms.Affine2D(), square,
                              transforms.IdentityTransform())
    renderer.draw_markers(gc, path.Path.unit_circle(), transforms.Affine2D(),
                          path.Path([(0, 0)]), transforms.IdentityTransform())


def test_streamed_pgf_pictures_refuse_images():
    message = ("^streamed pgf-code does not support raster graphics, consider using the "
               "pgf-to-pdf option$")
    fig, ax = plt.subplots()
    ax.imshow(np.arange(12.0).reshape(3, 4))
    with pytest.raises(ValueError, match=message):
        fig.savefig(io.BytesIO(), format='pgf')
    renderer = backend_pgf.RendererPgf(fig, io.StringIO())
    with pytest.raises(ValueError, match=message):
        renderer.draw_image(renderer.new_gc(), 0, 0, np.zeros((3, 4, 4), np.uint8))


def test_subplot_specs_between_cells_in_any_order():
    fig = plt.figure()
    spec = fig.add_gridspec(3, 3)[1:, 0]
    spec.num2 = 1
    assert spec.rowspan == range(1, 1)
    assert spec.get_position(fig).bounds == pytest.approx(
        (0.125, 0.3817647058823529, 0.5014705882352941, 0.4982352941176471))


def test_lines_in_no_figure_are_hit_in_their_own_coordinates(caplog):
    canvas = backend_agg.FigureCanvasAgg(Figure())

    def event(x, y):
        return backend_bases.MouseEvent('button_press_event', canvas, x, y, 1)

    line = lines.Line2D([0, 10, 20], [0, 10, 0])
    with caplog.at_level(logging.WARNING):
        inside, details = line.contains(event(5, 9))
    assert inside and details['ind'].tolist() == [0]
    assert 'no figure set when check if mouse is on line' in caplog.text
    assert not line.contains(event(100, 100))[0]
    steps = lines.Line2D([0, 10, 20], [0, 10, 0], drawstyle='steps')
    assert steps.contains(event(10, 5))[1]['ind'].tolist() == [1, 1]


def test_postscript_logs_transparency_before_a_failing_draw(caplog):
    fig, ax = plt.subplots()
    ax.tripcolor([0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4], alpha=0.5)
    # Drawn after the partially transparent mesh, it fails to draw.
    ax.imshow(np.arange(4.0).reshape(2, 2), zorder=3).set_clim(5, 1)
    with caplog.at_level(logging.WARNING):
        with pytest.raises(ValueError, match="minvalue must be less than or equal to maxvalue"):
            fig.savefig(io.BytesIO(), format='ps')
    assert 'does not support transparency' in caplog.text


def test_scalar_offset_follows_the_settings_after_a_draw():
    fig, ax = plt.subplots()
    ax.plot([1e-7, 3e-7])
    fig.canvas.draw()
    formatter = ax.yaxis.get_major_formatter()
    formatter.set_useMathText(True)
    assert formatter.get_offset() == '$\\times\\mathdefault{10^{−7}}\\mathdefault{}$'


def test_path_collection_extents_of_no_transforms():
    square = path.Path([(0, 0), (1, 0), (1, 1), (0, 1), (0, 0)], closed=True)
    identity = transforms.IdentityTransform()
    extents = path.get_path_collection_extents(
        identity, [square], [], [[0, 0], [2, 2]], identity)
    assert extents.bounds == (0.0, 0.0, 3.0, 3.0)


def test_hatch_colours_are_typed_as_given():
    rect = patches.Rectangle((0, 0), 1, 1)
    rect.set_hatchcolor('r')
    assert rect.get_hatchcolor() == (1.0, 0.0, 0.0, 1)
    rect.set_alpha(0.5)
    assert rect.get_hatchcolor() == (1.0, 0.0, 0.0, 0.5)


@pytest.mark.parametrize("call", [
    lambda ax: ax.plot([5, 9], [5, 9], alpha=2),
    lambda ax: ax.scatter([5], [9], alpha=2),
    lambda ax: ax.bar([7], [8], nope=1),
    lambda ax: ax.text(3, 3, 'a', nope=1),
    lambda ax: ax.imshow(np.ones((2, 2)), alpha=3),
    lambda ax: ax.tripcolor([0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4], alpha=1.5),
    lambda ax: ax.tricontourf([0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4], alpha=1.5),
    lambda ax: ax.inset_axes([0, 0, 0.5, 0.5], nope=1),
])
def test_failing_calls_leave_the_axes_as_they_were(call):
    fig, ax = plt.subplots()
    ax.plot([0, 1], [0, 1])
    data_limits = ax.dataLim.bounds
    with pytest.raises((ValueError, AttributeError)):
        call(ax)
    assert len(ax.get_children()) == 11 and not ax.containers
    assert ax.dataLim.bounds == data_limits
    assert ax.get_xlim() == ax.get_ylim() == (-0.05, 1.05)


def test_postscript_logs_no_transparency_when_the_draw_before_fails(caplog):
    for tight in (False, True):
        fig, ax = plt.subplots()
        ax.tripcolor([0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4], alpha=0.5)
        ax.tripcolor([0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4], vmin=5)
        if not tight:
            fig.tight_layout()
        with caplog.at_level(logging.WARNING):
            with pytest.raises(ValueError, match="minvalue must be less than or equal to maxvalue"):
                fig.savefig(io.BytesIO(), format='ps', bbox_inches='tight' if tight else None)
        assert 'transparency' not in caplog.text


def test_arrows_measure_through_the_view_before_autoscaling():
    fig, ax = plt.subplots()
    ax.scatter([0, 2], [0, 2])
    arrow = patches.FancyArrowPatch((0, 0), (1, 1), shrinkA=8, shrinkB=8)
    ax.add_patch(arrow)
    assert arrow.get_path().vertices[0].tolist() == pytest.approx(
        [0.012811357211353236, 0.013148776795197659])
    assert ax.transData.inverted().transform((300, 300)).tolist() == pytest.approx(
        [0.44354839, 0.66883117])
    ax.get_xlim()
    assert arrow.get_path().vertices[0].tolist() == pytest.approx(
        [0.028192615259508502, 0.028934938343966043])


def test_a_drawing_cut_to_a_tight_box_sees_moved_transforms():
    seen = []

    class Probe(martist.Artist):
        def draw(self, renderer):
            ax = self.axes
            seen.append((ax.transData.transform((0, 0)).tolist(),
                         ax.get_figure().transFigure.transform((0, 0)).tolist(),
                         ax.get_figure().bbox.bounds))

    fig, ax = plt.subplots()
    ax.plot([0, 1])
    ax.add_artist(Probe())
    fig.savefig(io.BytesIO(), format='png', bbox_inches='tight')
    (data, figure, bounds), = seen[-1:]
    assert data == pytest.approx([64.268, 50.411], abs=1e-3)
    assert figure == pytest.approx([-38.278, -19.189], abs=1e-3)
    assert bounds == pytest.approx((0, 0, 547.722, 413.211), abs=1e-3)
    assert ax.transData.transform((0, 0)).tolist() == pytest.approx([102.545, 69.6], abs=1e-3)
    assert fig.bbox.bounds == (0, 0, 640, 480)


def test_drawing_without_rendering_ends_with_a_draw_event():
    from pltrs.backends.backend_pdf import FigureCanvasPdf
    for canvas, renderer in ((None, 'RendererAgg'), (FigureCanvasPdf, 'MixedModeRenderer')):
        fig = Figure()
        fig.add_subplot().plot([1, 2])
        if canvas is not None:
            canvas(fig)
        events = []
        fig.canvas.mpl_connect('draw_event', lambda e: events.append(type(e.renderer).__name__))
        fig.draw_without_rendering()
        assert events == [renderer]
