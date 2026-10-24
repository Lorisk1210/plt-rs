"""matplotlib's own non-image tests of the Agg canvas
(``matplotlib/tests/test_agg.py``): saved colours and alpha, canvas sizes,
and what the bitmap formats record (JPEG resolution, GIF and WebP
transparency).

Area ``mpl_agg``. Every probe runs in the style matplotlib's test fixture
applies (``classic`` plus ``_classic_test_patch``), given as ``style``.

Ops
---
``Figure.savefig.pixel``  {figsize, facecolor, alpha} -> the first pixel of
    ``Figure(figsize)`` with face colour ``facecolor`` and patch alpha
    ``alpha``, saved twice as a PNG with ``facecolor=fig.get_facecolor(),
    edgecolor="none"`` and read back with ``imread`` (RGBA in 0..1)
    (``test_repeated_save_with_alpha``).
``Figure.savefig.too_large``  {figsize} -> saving ``Figure(figsize)`` as a
    PNG: the error (``test_too_large_image``).
``FigureCanvasAgg.buffer_rgba``  {figsize, dpi} -> the shape of the drawn
    canvas of ``Figure(figsize, dpi)`` (``test_rendered_height_floating_point_precision``).
``Figure.savefig.jpeg``  {dpi} -> the resolution Pillow reads from a figure
    with ``plot([0, 1, 2], [0, 1, 0])`` saved as a JPEG at ``dpi``
    (``test_jpeg_dpi``).
``Figure.savefig.gif``  {transparent} -> ``{mode, min_alpha}`` of the same
    figure saved as a GIF: Pillow's mode and the smallest alpha of the image
    in RGBA (``test_gif_no_alpha``, ``test_gif_alpha``: whether the palette's
    transparent index is used).
``Figure.savefig.webp``  {transparent} -> Pillow's mode of the same figure
    saved as a WebP (``test_webp_alpha``).

The tests that draw a figure and only check that it works
(``test_large_single_path_collection``, ``test_marker_with_nan``,
``test_long_path``, ``test_chunksize``, ``test_non_tuple_rgbaface``) are
figure cases of ``mpl_tests_agg``; ``test_agg_filter`` is the case
``mpl_misc/agg_filter``.

Left out: Pillow's keyword arguments (``test_pil_kwargs_png``,
``test_pil_kwargs_tiff``, ``test_pil_kwargs_webp``; plt-rs writes lossless
WebP), AVIF (``test_pil_kwargs_avif``, ``test_avif_alpha``),
``test_draw_path_collection_error_handling`` (a lone path given as the
paths of a collection is a type error in Rust) and ``test_chunksize_fails``
(the Agg renderer's own limits on splitting huge paths).
"""

import io

import matplotlib.style
import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.image import imread
from PIL import Image

from spec_support import ProbeSet

probes = ProbeSet("mpl_agg")

CLASSIC = ["classic", "_classic_test_patch"]


def probe(op, fn, **args):
    def styled(style, **kw):
        with matplotlib.style.context(style):
            return fn(**kw)

    return probes.probe(op, styled, style=CLASSIC, **args)


def _figure(**kw):
    fig = Figure(**kw)
    FigureCanvasAgg(fig)
    return fig


def _pixel(figsize, facecolor, alpha):
    fig = _figure(figsize=figsize)
    fig.set_facecolor(facecolor)
    fig.patch.set_alpha(alpha)
    buf = io.BytesIO()
    fig.savefig(buf, facecolor=fig.get_facecolor(), edgecolor="none")
    buf.seek(0)
    fig.savefig(buf, facecolor=fig.get_facecolor(), edgecolor="none")
    buf.seek(0)
    return np.round(imread(buf)[0, 0] * 255).astype(int).tolist()


def test_repeated_save_with_alpha():
    probe("Figure.savefig.pixel", _pixel, figsize=[1, 0.4], facecolor=[0, 1, 0.4], alpha=0.25)


def _too_large(figsize):
    _figure(figsize=figsize).savefig(io.BytesIO())


def test_too_large_image():
    probe("Figure.savefig.too_large", _too_large, figsize=[300, 2 ** 25])


def _buffer_shape(figsize, dpi):
    fig = _figure(figsize=figsize, dpi=dpi)
    fig.canvas.draw()
    return list(fig.canvas.buffer_rgba().shape)


def test_rendered_height_floating_point_precision():
    probe("FigureCanvasAgg.buffer_rgba", _buffer_shape, figsize=[1, 2.03], dpi=100)


def _plotted(format, **savefig):
    fig = _figure()
    fig.subplots().plot([0, 1, 2], [0, 1, 0])
    buf = io.BytesIO()
    fig.savefig(buf, format=format, **savefig)
    buf.seek(0)
    return Image.open(buf)


def _jpeg(dpi):
    return list(_plotted("jpg", dpi=dpi).info["dpi"])


def test_jpeg_dpi():
    probe("Figure.savefig.jpeg", _jpeg, dpi=200)


def _gif(transparent):
    im = _plotted("gif", transparent=transparent)
    return {"mode": im.mode, "min_alpha": im.convert("RGBA").getextrema()[3][0]}


def test_gif_alpha():
    for transparent in [False, True]:
        probe("Figure.savefig.gif", _gif, transparent=transparent)


def _webp(transparent):
    return _plotted("webp", transparent=transparent).mode


def test_webp_alpha():
    for transparent in [False, True]:
        probe("Figure.savefig.webp", _webp, transparent=transparent)
