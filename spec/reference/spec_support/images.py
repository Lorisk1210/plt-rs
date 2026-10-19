"""Deterministic images and image files for the probes that read them.

The ``imread`` areas record ``image.imread`` of files made here from
formulas and :func:`spec_support.rng` noise, passed to the probe in
base64.
"""

import base64
import io

import numpy as np
from matplotlib import image

from .data import rng


def ramp_rgb(width, height, seed=1):
    """Smooth colour ramps with some deterministic noise."""
    y, x = np.mgrid[0:height, 0:width]
    base = np.stack([
        128 + 100 * np.sin(0.35 * x + 0.1 * y),
        (9 * x + 5 * y) % 256,
        128 + 90 * np.cos(0.2 * x - 0.3 * y),
    ], axis=-1)
    noise = rng(seed).integers(width * height * 3, -24, 24).reshape(height, width, 3)
    return np.clip(base + noise, 0, 255).astype(np.uint8)


def noise(width, height, channels, seed):
    """Uniform random bytes."""
    values = rng(seed).integers(width * height * channels, 0, 256)
    return values.astype(np.uint8).reshape(height, width, channels)


def pillow_bytes(im, format, **options):
    """The bytes Pillow writes for ``im`` in ``format``."""
    buffer = io.BytesIO()
    im.save(buffer, format, **options)
    return buffer.getvalue()


def imread_file(data, format, case):
    """``image.imread`` of a base64 file, as nested lists."""
    return image.imread(io.BytesIO(base64.b64decode(data)), format=format).tolist()


def probe_imread(probes, format, case, data):
    """Records ``image.imread`` of the file ``data`` in ``probes``."""
    return probes.probe("image.imread", imread_file,
                        data=base64.b64encode(data).decode("ascii"), format=format, case=case)
