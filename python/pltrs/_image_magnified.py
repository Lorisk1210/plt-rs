"""
What matplotlib's images compute with a magnification that is no positive
finite number (``make_image(renderer, magnification)``): pltrs (Rust)
resamples an image at a positive magnification, so these follow
matplotlib's arithmetic on the box the image fills, which fails (or finds
that nothing shows) as numpy's does.
"""

import math
from numbers import Real

import numpy as np

from .transforms import Affine2D, IdentityTransform


def plain(magnification):
    """Whether pltrs (Rust) magnifies by *magnification*: a positive finite
    number."""
    return (isinstance(magnification, Real) and math.isfinite(magnification)
            and magnification > 0)


def resampled_shows(extents, magnification, round_to_pixel_border):
    """
    Whether an image whose clipped box has the display *extents* shows
    once magnified by *magnification*, as matplotlib's ``_make_image``
    finds it: it magnifies the box (rounding it to the pixels when
    *round_to_pixel_border*), then makes the output of its size, raising
    numpy's errors for a size that is no count.
    """
    magnified = np.asarray(extents, dtype=float) * magnification
    if round_to_pixel_border:
        magnified = [np.floor(magnified[0] + 0.5), np.ceil(magnified[1] - 0.5 - 1e-8),
                     np.floor(magnified[2] + 0.5 + 1e-8), np.ceil(magnified[3] - 0.5)]
    # The size of the box, which matplotlib reads from it each time (its
    # `width` and `height`), numpy warning each time.
    def width():
        return magnified[2] - magnified[0]

    def height():
        return magnified[3] - magnified[1]

    if width() == 0 or height() == 0:
        return False
    # The magnification scales the transform to the output, then the
    # output is made.
    Affine2D().scale(magnification).translate(-magnified[0], -magnified[1])
    np.zeros((int(height()), int(width())))
    return True


def nonuniform_image(image, magnification):
    """Fail as matplotlib's `.NonUniformImage` fails to sample its Axes' box
    at *magnification*."""
    extents = (image.axes.bbox.extents * magnification + 0.5).astype(int)
    left, bottom, right, top = extents / magnification
    width = int((right - left) * magnification)
    height = int((top - bottom) * magnification)
    np.linspace(left, right, width + 1)
    np.linspace(bottom, top, height + 1)


def pcolor_image(image, magnification):
    """
    `.PcolorImage.make_image` at *magnification* as matplotlib computes it,
    when the Axes' box is sampled by no pixel: an empty image at the corner
    the magnification divides; None when there are pixels to sample. Fails
    as numpy's sampling does.
    """
    left, bottom, right, top = (image.axes.bbox.extents * magnification + 0.5).astype(int)
    width, height = right - left, top - bottom
    view = image.axes.viewLim
    np.linspace(view.x0, view.x1, width + 1)
    np.linspace(view.y0, view.y1, height + 1)
    if width and height:
        return None
    return (np.zeros((height, width, 4), np.uint8), left / magnification,
            bottom / magnification, IdentityTransform())
