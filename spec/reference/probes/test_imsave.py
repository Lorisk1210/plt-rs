"""Writing arrays as image files (``matplotlib.image.imsave``).

Area ``imsave``. JPEG and TIFF go through Pillow, whose output plt-rs
reproduces byte for byte (libjpeg-turbo's defaults for JPEG).

Argument conventions
--------------------
* ``data``: nested lists — ``(M, N)`` scalars, or ``(M, N, 3|4)`` colours.
* ``dtype``: ``"float"`` (default) or ``"uint8"``, the numpy dtype of
  ``data``.
* ``format``: the ``format`` argument (``"jpeg"``, ``"jpg"``, ``"tiff"``,
  ``"tif"``, ``"raw"`` ...).
* ``cmap``, ``vmin``, ``vmax``, ``origin``, ``dpi``: the keywords of the
  same name (``dpi`` defaults to 100); ``quality`` becomes
  ``pil_kwargs={"quality": quality}``.

Ops
---
``image.imsave``  {data, dtype?, format, cmap?, vmin?, vmax?, origin?, dpi?,
                  quality?} -> str
    the file's bytes in base64; errors are ``{"error": "<ExceptionType>"}``.
``image.thumbnail``  {data, scale, interpolation} -> {shape: [rows, cols], rgba: str}
    the colour image ``data`` (floats) saved as PNG, passed through
    ``thumbnail(infile, thumbfile.png, scale, interpolation)``, and the
    thumbnail read back: its shape and RGBA bytes in base64.
"""

import base64
import io
import os
import tempfile

import numpy as np
from matplotlib import image

from spec_support import ProbeSet

probes = ProbeSet("imsave")


def _imsave(data, format, dtype="float", cmap=None, vmin=None, vmax=None, origin=None, dpi=100,
            quality=None):
    array = np.asarray(data, dtype=np.uint8 if dtype == "uint8" else float)
    buffer = io.BytesIO()
    pil_kwargs = None if quality is None else {"quality": quality}
    image.imsave(buffer, array, format=format, cmap=cmap, vmin=vmin, vmax=vmax, origin=origin,
                 dpi=dpi, pil_kwargs=pil_kwargs)
    return base64.b64encode(buffer.getvalue()).decode("ascii")


def _ramp(rows, cols):
    return [[float(np.sin(0.4 * r) * np.cos(0.3 * c) + 0.05 * r * c) for c in range(cols)]
            for r in range(rows)]


def _colours(rows, cols, alpha):
    pixels = []
    for r in range(rows):
        row = []
        for c in range(cols):
            colour = [(r * 0.13 + c * 0.07) % 1, (r * 0.29 + 0.3) % 1, (c * 0.21 + 0.6) % 1]
            if alpha:
                colour.append(((r + 2 * c) % 5) / 4)
            row.append(colour)
        pixels.append(row)
    return pixels


def _bytes(rows, cols):
    return [[[(r * 37 + c * 11) % 256, (r * 53) % 256, (c * 29 + 7) % 256, (r * c * 41) % 256]
             for c in range(cols)] for r in range(rows)]


def test_scalars():
    for format in ["jpeg", "tiff"]:
        probes.probe("image.imsave", _imsave, data=_ramp(6, 9), format=format)
        probes.probe("image.imsave", _imsave, data=_ramp(40, 33), format=format, cmap="RdBu",
                     vmin=-0.5, vmax=1.5, origin="lower", dpi=72)
    probes.probe("image.imsave", _imsave, data=_ramp(17, 23), format="jpg", cmap="magma",
                 quality=92)
    probes.probe("image.imsave", _imsave, data=_ramp(9, 9), format="jpeg", cmap="gray",
                 quality=10, dpi=150.6)


def test_colours():
    probes.probe("image.imsave", _imsave, data=_colours(5, 7, False), format="jpeg", quality=90)
    probes.probe("image.imsave", _imsave, data=_colours(5, 7, False), format="tiff", origin="lower")
    probes.probe("image.imsave", _imsave, data=_colours(11, 6, True), format="jpeg")
    probes.probe("image.imsave", _imsave, data=_colours(11, 6, True), format="tiff", dpi=96)
    probes.probe("image.imsave", _imsave, data=_bytes(8, 13), dtype="uint8", format="jpeg")
    probes.probe("image.imsave", _imsave, data=_bytes(8, 13), dtype="uint8", format="tiff")


def test_refused_formats():
    for format in ["tif", "raw", "rgba"]:
        assert isinstance(probes.probe("image.imsave", _imsave, data=_ramp(2, 2), format=format),
                          Exception)


def _thumbnail(data, scale, interpolation):
    with tempfile.TemporaryDirectory() as tmp:
        source, target = os.path.join(tmp, "image.png"), os.path.join(tmp, "thumb.png")
        image.imsave(source, np.asarray(data, dtype=float))
        image.thumbnail(source, target, scale=scale, interpolation=interpolation)
        thumb = (image.imread(target) * 255).round().astype(np.uint8)
    return {"shape": list(thumb.shape[:2]), "rgba": base64.b64encode(thumb.tobytes()).decode()}


def test_thumbnails():
    data = _colours(40, 60, True)
    for scale, interpolation in [(0.1, "bilinear"), (0.5, "bilinear"), (0.25, "nearest"),
                                 (0.3, "hanning"), (2.0, "bicubic")]:
        probes.probe("image.thumbnail", _thumbnail, data=data, scale=scale,
                     interpolation=interpolation)
