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
``Figure.savefig.png_text``  {metadata?} -> {chunks, text}: the chunk types
    before the image data of ``Figure(figsize=(1, 1), dpi=10)`` saved as a
    PNG with ``metadata`` (none when absent; a ``null`` value drops the
    key), and the text entries Pillow reads from it, in file order:
    matplotlib's default ``Software`` entry (recorded as ``<default software>``,
    since each library names itself there), the keys PNG defines, custom
    ones and text outside Latin-1 (``test_pil_kwargs_png``, through
    ``metadata`` rather than Pillow's ``pnginfo``).

The tests that draw a figure and only check that it works
(``test_large_single_path_collection``, ``test_marker_with_nan``,
``test_long_path``, ``test_chunksize``, ``test_non_tuple_rgbaface``) are
figure cases of ``mpl_tests_agg``; ``test_agg_filter`` is the case
``mpl_misc/agg_filter``.

Left out: Pillow's keyword arguments (``test_pil_kwargs_tiff``,
``test_pil_kwargs_webp``; pltrs writes lossless WebP), AVIF (``test_pil_kwargs_avif``, ``test_avif_alpha``),
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


def _chunk_types(png):
    """The chunk types of ``png`` before its first ``IDAT``."""
    types, pos = [], 8
    while png[pos + 4:pos + 8] != b"IDAT":
        length = int.from_bytes(png[pos:pos + 4], "big")
        types.append(png[pos + 4:pos + 8].decode("ascii"))
        pos += 12 + length
    return types


# The default ``Software`` entry names the library writing the file
# (``Matplotlib version...`` here, ``pltrs v...`` in pltrs), so it is
# recorded as this placeholder.
_DEFAULT_SOFTWARE = f"Matplotlib version{matplotlib.__version__}, https://matplotlib.org/"


def _png_text(metadata=None):
    fig = _figure(figsize=(1, 1), dpi=10)
    buf = io.BytesIO()
    fig.savefig(buf, format="png", metadata=metadata)
    png = buf.getvalue()
    text = Image.open(io.BytesIO(png)).text.items()
    return {"chunks": _chunk_types(png),
            "text": [[key, "<default software>" if key == "Software" and value == _DEFAULT_SOFTWARE
                      else value] for key, value in text]}


def test_png_metadata():
    probe("Figure.savefig.png_text", _png_text)
    probe("Figure.savefig.png_text", _png_text, metadata={})
    probe("Figure.savefig.png_text", _png_text, metadata={"Software": None})
    probe("Figure.savefig.png_text", _png_text, metadata={"Software": "test"})
    probe("Figure.savefig.png_text", _png_text, metadata={"Title": "Signal", "Software": "s"})
    probe("Figure.savefig.png_text", _png_text, metadata={
        "Title": "Signal", "Author": "Zoë", "Description": "日本語の説明",
        "Copyright": "CC0", "Creation Time": "2026-10-24", "Disclaimer": "none",
        "Warning": "hot", "Source": "plt-rs", "Comment": "first line\nsecond line",
        "Custom key": "value", "Unset": None})
