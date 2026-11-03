"""
Argument combinations real programs use that matplotlib's suite never
reaches, each answered as matplotlib 3.11 answers it. The expected values
were recorded from matplotlib 3.11.2 running the same code.
"""

import datetime
import io
import logging
import os
import re

import numpy as np
import pytest

import pltrs.pyplot as plt
from pltrs import (_image, cbook, collections, colors, cm, dates as mdates,
                   font_manager as fm, image, patches, rc_context, ticker)
from pltrs.backends.backend_pdf import PdfPages
from pltrs.mpl_toolkits.axes_grid1 import Divider, Size
from pltrs.mpl_toolkits.axes_grid1.axes_size import _Base
from pltrs.mpl_toolkits.mplot3d import art3d
from pltrs.scale import FuncScale, LogScale
from pltrs.transforms import Affine2D, Bbox

from test_smoke import close_figures  # noqa: F401 (autouse: rcdefaults)


def test_plot_and_autoscale_view_scale_only_the_axes_asked_for():
    fig, ax = plt.subplots()
    ax.plot([0, 10], [0, 5], scalex=False)
    assert ax.get_xlim() == (0, 1) and ax.get_ylim() == (-0.25, 5.25)
    assert tuple(ax.dataLim.bounds) == (0, 0, 10, 5)
    fig, ax = plt.subplots()
    ax.plot([0, 10], [0, 5])
    ax.get_xlim()
    ax.plot([0, 20], [0, 50], scalex=False, scaley=False)
    assert (ax.get_xlim(), ax.get_ylim()) == ((-0.5, 10.5), (-0.25, 5.25))
    ax.autoscale_view(scaley=False)
    assert (ax.get_xlim(), ax.get_ylim()) == ((-1, 21), (-0.25, 5.25))
    ax.autoscale_view(scalex=False)
    assert ax.get_ylim() == (-2.5, 52.5)


def test_margins_tight_keeps_or_changes_the_rounding():
    with rc_context({'axes.autolimit_mode': 'round_numbers'}):
        fig, ax = plt.subplots()
        ax.plot([0.3, 4.7], [0.3, 4.7])
        ax.margins(0.0, None, tight=False)
        assert ax.get_xlim() == (0, 5)
        ax.margins(0.0, None, tight=None)
        assert ax.get_xlim() == (0, 5)
        ax.margins(0.0, None, tight=True)
        assert ax.get_xlim() == (0.3, 4.7)


def test_legend_prop_sets_the_font_of_the_labels():
    fig, ax = plt.subplots()
    ax.plot([1, 2], label='a')
    legend = ax.legend(prop={'size': 20, 'weight': 'bold'})
    assert legend.texts[0].get_fontsize() == 20 and legend._fontsize == 20
    assert legend.texts[0].get_weight() == 'bold'
    fig.canvas.draw()
    assert np.allclose(legend.get_window_extent().bounds,
                       [93.89, 358.51, 119, 50], atol=0.01)
    legend = ax.legend(prop={'family': 'serif'})
    assert legend.texts[0].get_fontsize() == 10
    assert legend.texts[0].get_family() == ['serif']


def test_colorbar_alpha_and_ticks_with_labels():
    fig, ax = plt.subplots()
    sm = cm.ScalarMappable(norm=colors.Normalize(0, 1), cmap='viridis')
    assert fig.colorbar(sm, ax=ax, alpha=0.5).alpha == 0.5
    image_ = ax.imshow([[0, 1]], alpha=0.3)
    assert fig.colorbar(image_, ax=ax, alpha=0.5).alpha == 0.3
    fig, ax = plt.subplots()
    cb = fig.colorbar(sm, ax=ax)
    cb.set_ticks([0, 0.5, 1], labels=['lo', 'mid', 'hi'], color='r')
    labels = cb.ax.get_yticklabels()
    assert [t.get_text() for t in labels] == ['lo', 'mid', 'hi']
    assert [t.get_color() for t in labels] == ['r'] * 3
    cb.set_ticks([0.25, 0.75], minor=True)
    assert list(cb.get_ticks(minor=True)) == [0.25, 0.75]
    cb.set_ticklabels(['a', 'b', 'c'])
    assert [t.get_text() for t in cb.ax.get_yticklabels()] == ['a', 'b', 'c']
    cb.set_ticks(ticker.MultipleLocator(0.2))
    assert np.allclose(cb.get_ticks(), [-0.2, 0, 0.2, 0.4, 0.6, 0.8, 1, 1.2])


def _labelled(fmt, **kwargs):
    """The texts of the labels `clabel(fmt=fmt)` puts on a bump."""
    x, y = np.meshgrid(np.linspace(-2, 2, 30), np.linspace(-2, 2, 30))
    fig, ax = plt.subplots()
    cs = ax.contour(x, y, np.exp(-x**2 - y**2) * 1000,
                    levels=[200, 400, 600, 800])
    return sorted({t.get_text() for t in cs.clabel(fmt=fmt, **kwargs)})


def test_clabel_formats_with_formatters_and_dicts():
    formatter = ticker.ScalarFormatter(useOffset=False)
    formatter.create_dummy_axis()
    assert _labelled(formatter) == ['200', '400', '600', '800']
    assert _labelled(ticker.StrMethodFormatter("{x:.0f} m")) == [
        '200 m', '400 m', '600 m', '800 m']
    percent = ticker.PercentFormatter(1000)
    percent.create_dummy_axis()
    assert _labelled(percent) == ['20.000%', '40.000%', '60.000%', '80.000%']
    assert _labelled({200: 'a', 400: 'b'}, levels=[200, 400]) == ['a', 'b']


def test_clabel_manual_places_labels_by_events():
    from pltrs.backend_bases import MouseButton, MouseEvent
    x, y = np.meshgrid(np.linspace(-2, 2, 30), np.linspace(-2, 2, 30))
    fig, ax = plt.subplots()
    cs = ax.contour(x, y, x**2 + y**2, levels=[1, 2])
    fig.canvas.draw()
    near = ax.transData.transform((1.0, 0.0))
    clicks = iter([(MouseButton.LEFT, near), (MouseButton.MIDDLE, near)])

    def start_event_loop(timeout=0):
        for button, (px, py) in clicks:
            MouseEvent("button_press_event", fig.canvas, px, py,
                       button)._process()

    fig.canvas.start_event_loop = start_event_loop
    fig.canvas.stop_event_loop = lambda: None
    labels = cs.clabel(manual=True, inline=False)
    assert len(labels) == 1 and labels[0].get_text() == '1'
    cs.pop_label()
    assert cs.labelTexts == []


def test_arrows_are_clipped_to_their_patches():
    fig, ax = plt.subplots()
    ax.set(xlim=(0, 10), ylim=(0, 10))
    c1 = ax.add_patch(patches.Circle((2, 5), 1))
    c2 = patches.Circle((8, 5), 1)
    arrow = patches.FancyArrowPatch((2, 5), (8, 5), patchA=c1, patchB=c2,
                                    arrowstyle='-', shrinkA=0, shrinkB=0)
    ax.add_patch(arrow)
    ax.add_patch(c2)
    fig.canvas.draw()
    assert np.allclose(arrow.get_window_extent().bounds, [228.8, 237.6, 198.4, 0])
    arrow.set_patchB(None)
    fig.canvas.draw()
    assert np.allclose(arrow.get_window_extent().bounds, [228.8, 237.6, 248, 0])
    link = ax.add_artist(patches.ConnectionPatch(
        (2, 5), (8, 5), 'data', patchA=c1, patchB=c2))
    fig.canvas.draw()
    assert np.allclose(link.get_window_extent().bounds, [228.8, 237.6, 198.4, 0])


def test_metadata_of_imsave_and_pdfpages():
    data = np.arange(12, dtype=float).reshape(3, 4)
    for fmt, written in [('pdf', True), ('svg', True), ('eps', False)]:
        buffer = io.BytesIO()
        image.imsave(buffer, data, format=fmt, metadata={'Title': 'My image'})
        assert (b'My image' in buffer.getvalue()) == written
    buffer = io.BytesIO()
    with PdfPages(buffer, metadata={'Author': 'Me', 'Title': 'Doc'}) as pdf:
        fig, ax = plt.subplots()
        pdf.attach_note("hello", positionRect=[10, 20, 30, 40])
        pdf.savefig(fig)
    document = buffer.getvalue()
    assert b'/Author (Me)' in document and b'/Title (Doc)' in document
    rects = re.findall(rb'/Rect \[([^\]]*)\]', document)
    assert [list(map(float, rect.split())) for rect in rects] == [[10, 20, 30, 40]]


def test_eps_titles_name_the_file(tmp_path):
    fig, ax = plt.subplots()
    path = tmp_path / "x.eps"
    fig.savefig(path, metadata={'Creator': 'C'})
    head = path.read_bytes()[:400].decode('latin-1').splitlines()
    assert [line for line in head if line.startswith(('%%Title', '%%Creator'))] == [
        '%%Title: x.eps', '%%Creator: C']


def test_tight_bbox_with_extra_artists():
    fig, ax = plt.subplots()
    text = fig.text(0.95, 0.95, "far away text", fontsize=30)
    ax.set_title("T")
    assert np.allclose(fig.get_tightbbox().bounds, [0.483, 0.292, 8.37, 4.588],
                       atol=1e-3)
    assert np.allclose(fig.get_tightbbox(bbox_extra_artists=[]).bounds,
                       [0.483, 0.292, 5.387, 4.142], atol=1e-3)
    assert np.allclose(fig.get_tightbbox(bbox_extra_artists=[text]).bounds,
                       [0.483, 0.292, 8.37, 4.588], atol=1e-3)
    from PIL import Image
    buffer = io.BytesIO()
    fig.savefig(buffer, format='png', bbox_inches='tight', bbox_extra_artists=[])
    buffer.seek(0)
    assert Image.open(buffer).size == (558, 434)


def test_3d_plot_takes_several_datasets_and_scale_objects():
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    lines = ax.plot([0, 1, 2], [0, 1, 4], 'r-', [0, 1, 2], [3, 2, 1], 'b--',
                    label='x')
    assert [line.get_color() for line in lines] == ['r', 'b']
    assert [line.get_label() for line in lines] == ['x', 'x']
    lines = ax.plot([0, 1, 2], [0, 1, 4], [5, 6, 7], 'g', [1, 2, 3], [1, 1, 1])
    assert [list(line.get_data_3d()[2]) for line in lines] == [[5, 6, 7]] * 2
    with pytest.raises(TypeError, match="multiple values for argument 's'"):
        ax.scatter([0, 1], [0, 1], [0, 1], 'z', 20, None, True, 'o')
    bars = ax.bar3d([0], [0], [0], 1, 1, 1, None, 'average', True, None, [5])
    assert list(bars.get_sizes()) == [5]
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    ax.plot([1, 10, 100], [1, 2, 3], [1, 2, 3])
    ax.set_xscale(LogScale(ax.xaxis, base=10))
    assert ax.get_xscale() == 'log'
    assert np.allclose(ax.get_xlim(), (0.7147705767941855, 139.90503141372943))
    ax.set_zscale(FuncScale(ax.zaxis, (lambda x: x**2, np.sqrt)))
    assert ax.get_zscale() == 'function'
    assert np.allclose(ax.get_zlim(), (0.9583333333333334, 3.0416666666666665))


def test_3d_ticks_with_labels_depthshade_and_lifted_path_collections():
    fig = plt.figure()
    ax = fig.add_subplot(projection='3d')
    ax.set_xticks([0, 0.5, 1], labels=['a', 'b', 'c'], color='r', fontsize=7)
    fig.canvas.draw()
    assert [t.get_text() for t in ax.get_xticklabels()] == ['a', 'b', 'c']
    with pytest.raises(ValueError, match="Incorrect use of keyword argument"):
        ax.set_xticks([0, 1], color='r')
    points = ax.scatter([0, 1, 2], [0, 1, 2], [0, 1, 2])
    points.set_depthshade(True, depthshade_minalpha=0.1)
    assert points.get_depthshade()
    markers = collections.PathCollection(
        [patches.Circle((0, 0), 1).get_path()], offsets=[[0.5, 0.5], [0.2, 0.3]],
        sizes=[50])
    art3d.patch_collection_2d_to_3d(markers, zs=[0.1, 0.9])
    assert type(markers).__name__ == 'Path3DCollection'
    ax.add_collection3d(markers)
    fig.canvas.draw()
    assert np.column_stack(markers._offsets3d).tolist() == [
        [0.5, 0.5, 0.1], [0.2, 0.3, 0.9]]


def test_rrule_week_numbers():
    rule = mdates.rrulewrapper(mdates.WEEKLY, byweekno=[1, 20, -1],
                               byweekday=mdates.MO,
                               dtstart=datetime.datetime(2020, 1, 1))
    assert rule.between(datetime.datetime(2020, 1, 1),
                        datetime.datetime(2022, 1, 31)) == [
        datetime.datetime(*day) for day in [
            (2020, 5, 11), (2020, 12, 28), (2021, 1, 4), (2021, 5, 17),
            (2021, 12, 27), (2022, 1, 3)]]


def test_apply_aspect_at_a_position():
    fig = plt.figure(figsize=(4, 2), dpi=100)
    ax = fig.add_subplot()
    ax.set_aspect('equal')
    ax.apply_aspect(Bbox.from_bounds(0, 0, 1, 1))
    assert tuple(ax.bbox.bounds) == (100, 0, 200, 200)
    fig = plt.figure(figsize=(4, 2), dpi=100)
    ax = fig.add_subplot()
    ax.apply_aspect(Bbox.from_bounds(0.1, 0.2, 0.3, 0.4))
    assert np.allclose(ax.bbox.bounds, (40, 40, 120, 80))


def test_axes_locators_of_the_program():
    fig = plt.figure(figsize=(4, 3), dpi=100)
    ax = fig.add_subplot()
    ax.set_axes_locator(lambda a, r: Bbox.from_bounds(0.2, 0.3, 0.4, 0.5))
    assert np.allclose(ax.get_position().bounds, (0.125, 0.11, 0.775, 0.77))
    fig.canvas.draw()
    assert np.allclose(ax.get_position().bounds, (0.2, 0.3, 0.4, 0.5))

    class Pixels:
        def __call__(self, ax, renderer):
            return ax.get_figure(root=False).transSubfigure.inverted(
            ).transform_bbox(Bbox.from_bounds(10, 20, 100, 50))

    inner = fig.add_axes([0.5, 0.5, 0.3, 0.3])
    inner.set_axes_locator(Pixels())
    fig.canvas.draw()
    assert np.allclose(inner.bbox.bounds, (10, 20, 100, 50))
    inner.set_axes_locator(None)
    fig.canvas.draw()
    assert np.allclose(inner.bbox.bounds, (200, 150, 120, 90))


def test_dividers_with_sizes_and_locators_of_the_program():
    class Half(_Base):
        def get_size(self, renderer):
            return 0.5, 0.0

    fig = plt.figure(figsize=(4, 4))
    divider = Divider(fig, (0.1, 0.1, 0.8, 0.8),
                      [Half(), Size.Fixed(1), Half()], [Size.Scaled(1)])
    ax = fig.add_axes((0, 0, 1, 1))
    ax.set_axes_locator(divider.new_locator(0, 0))
    fig.canvas.draw()
    assert np.allclose(ax.get_position().bounds, (0.1, 0.1, 0.275, 0.8))
    divider.set_locator(lambda a, r: Bbox.from_bounds(0, 0, 0.5, 0.5))
    fig.canvas.draw()
    assert np.allclose(ax.get_position().bounds, (0, 0, 0.125, 0.5))


def test_secondary_axes_located_by_any_transform():
    fig, ax = plt.subplots()
    secondary = ax.secondary_xaxis(0.25, transform=fig.transFigure)
    fig.canvas.draw()
    assert np.allclose(secondary.get_window_extent().bounds, (80, 120, 496, 0),
                       atol=1e-6)
    fig.set_size_inches(5, 6)
    fig.canvas.draw()
    assert np.allclose(secondary.get_window_extent().bounds, (62.5, 150, 387.5, 0),
                       atol=1e-6)


def test_unsampled_images_short_resampling_and_urls():
    fig = plt.figure(figsize=(2, 2), dpi=100)
    ax = fig.add_axes([0.1, 0.1, 0.8, 0.8])
    im = ax.imshow(np.arange(12.).reshape(3, 4))
    fig.canvas.draw()
    pixels, x, y, t = im.make_image(fig.canvas.get_renderer(), unsampled=True)
    assert pixels.shape == (3, 4, 4) and pixels[0, :2].tolist() == [
        [68, 1, 84, 255], [72, 33, 114, 255]]
    assert (round(x, 6), round(y, 6)) == (20, 40)
    assert np.allclose(t.get_matrix(), [[40, 0, 0], [0, -40, 120], [0, 0, 1]])
    data = (np.arange(16).reshape(4, 4) * 1000).astype(np.uint16)
    out = np.zeros((8, 8), np.uint16)
    _image.resample(data, out, Affine2D().scale(2), _image.BILINEAR)
    assert out[::2, ::2].tolist() == [
        [0, 750, 1750, 2750], [3000, 3750, 4750, 5750],
        [7000, 7750, 8750, 9750], [11000, 11750, 12750, 13750]]
    fig, ax = plt.subplots()
    im = ax.imshow([[0, 1]], url="https://example.com")
    buffer = io.BytesIO()
    fig.savefig(buffer, format="svg")
    assert im.get_url() == "https://example.com"
    assert b'https://example.com' in buffer.getvalue()


def test_afm_fonts_by_directory(caplog):
    core = cbook._get_data_path('fonts/pdfcorefonts')
    serif = fm.FontProperties(family=['serif'], weight='bold')
    assert fm.findfont(serif, fontext='afm', directory=core).endswith(
        'Times-Bold.afm')
    with caplog.at_level(logging.WARNING):
        found = fm.fontManager._find_fonts_by_props(
            fm.FontProperties(family=['Times', 'Nope', 'Courier']),
            fontext='afm', directory=core)
    assert [os.path.basename(path) for path in found] == [
        'Times-Roman.afm', 'Courier.afm']
    assert "findfont: Font family 'Nope' not found." in caplog.messages
    with pytest.raises(ValueError, match="Failed to find font Nope"):
        fm.findfont(fm.FontProperties(family=['Nope']), fontext='afm',
                    directory=core.parent.parent, fallback_to_default=False)


def test_axis_tight_bbox_for_layout_collapses_the_label():
    fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
    ax.set_xlabel("a label much, much, much, much, much longer than its axis")
    fig.canvas.draw()
    assert np.allclose(ax.xaxis.get_tightbbox().bounds, (-6, -10.5, 422, 33.8),
                       atol=0.05)
    assert np.allclose(ax.xaxis.get_tightbbox(for_layout_only=True).bounds,
                       (39, -10.5, 332, 33.8), atol=0.05)


def test_quiverkey_vector_properties_and_pie_label_usetex():
    fig, ax = plt.subplots()
    q = ax.quiver([0, 1], [0, 1], [1, 1], [1, 0])
    ax.quiverkey(q, 0.5, 0.9, 1, "1 m/s", linewidth=3, edgecolor='r',
                 facecolor='g')
    with pytest.raises(AttributeError, match="unexpected keyword argument 'nope'"):
        ax.quiverkey(q, 0.5, 0.9, 1, "1 m/s", nope=1)
    fig.canvas.draw()
    pixels = np.asarray(fig.canvas.buffer_rgba())
    assert ((pixels[..., 0] == 255) & (pixels[..., 1] == 0)).any()
    assert ((pixels[..., 1] == 128) & (pixels[..., 0] == 0)).any()
    fig, ax = plt.subplots()
    pie = ax.pie([1, 2, 3])
    texts = ax.pie_label(pie, "{frac:.0%}", textprops={'usetex': True})
    assert [t.get_text() for t in texts] == ['17\\%', '33\\%', '50\\%']
    texts = ax.pie_label(pie, "{frac:.0%}", textprops={'usetex': False})
    assert [t.get_text() for t in texts] == ['17%', '33%', '50%']


def test_polar_rlim_and_rscale_take_matplotlibs_arguments():
    fig = plt.figure()
    ax = fig.add_subplot(projection='polar')
    ax.plot([0, 1], [1, 100])
    ax.set_rscale('log', base=2)
    ax.set_rlim(rmin=1, rmax=64, emit=False)
    assert ax.get_yscale() == 'log' and ax.get_ylim() == (1, 64)


def test_callable_pickers_given_when_making_lines():
    def picker(artist, event):
        return True, {}

    fig, ax = plt.subplots()
    line, = ax.plot([0, 1], picker=picker)
    assert line.get_picker() is picker
    assert ax.axhline(0.5, picker=picker).get_picker() is picker
