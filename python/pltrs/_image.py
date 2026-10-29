"""
The interpolation constants of images and the image comparison of the
tests (matplotlib's native ``_image``).

pltrs (Rust) compares images (``pltrs::testing::calculate_rms_and_diff``).
"""

import enum

from ._native import testing as _testing

__all__ = [
    'NEAREST', 'BILINEAR', 'BICUBIC', 'SPLINE16', 'SPLINE36', 'HANNING',
    'HAMMING', 'HERMITE', 'KAISER', 'QUADRIC', 'CATROM', 'GAUSSIAN',
    'BESSEL', 'MITCHELL', 'SINC', 'LANCZOS', 'BLACKMAN',
]


class _InterpolationType(enum.IntEnum):
    """The interpolation kernels images are resampled with."""

    NEAREST = 0
    BILINEAR = 1
    BICUBIC = 2
    SPLINE16 = 3
    SPLINE36 = 4
    HANNING = 5
    HAMMING = 6
    HERMITE = 7
    KAISER = 8
    QUADRIC = 9
    CATROM = 10
    GAUSSIAN = 11
    BESSEL = 12
    MITCHELL = 13
    SINC = 14
    LANCZOS = 15
    BLACKMAN = 16


globals().update(_InterpolationType.__members__)


def calculate_rms_and_diff(expected_image, actual_image):
    """
    The root mean square of the difference of two images of shape
    ``(M, N, 3)`` or ``(M, N, 4)``, and the difference image, as a
    ``(M, N, 3)`` array of bytes ten times as strong.
    """
    return _testing.calculate_rms_and_diff(expected_image, actual_image)
