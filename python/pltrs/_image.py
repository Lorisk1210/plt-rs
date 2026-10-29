"""
The interpolation constants of images, Agg's resampler and the image
comparison of the tests (matplotlib's native ``_image``).

pltrs (Rust) resamples (``pltrs::image::resample_pixels``) and compares
images (``pltrs::testing::calculate_rms_and_diff``); this module checks the
arguments as matplotlib's wrapper does and computes the lookup mesh of a
non-affine transform through the transform object (the checks, their
messages and the docstring of ``resample`` are matplotlib 3.11.2's
``_image_wrapper.cpp``, under the Matplotlib License).
"""

import operator

import numpy as np

from ._native import image as _native_image
from ._native import testing as _testing

__all__ = [
    'NEAREST', 'BILINEAR', 'BICUBIC', 'SPLINE16', 'SPLINE36', 'HANNING',
    'HAMMING', 'HERMITE', 'KAISER', 'QUADRIC', 'CATROM', 'GAUSSIAN',
    'BESSEL', 'MITCHELL', 'SINC', 'LANCZOS', 'BLACKMAN',
    'resample', 'calculate_rms_and_diff',
]


class _InterpolationType:
    """
    The interpolation kernels images are resampled with: an enumeration
    behaving as matplotlib's (a pybind11 enumeration, whose members are
    neither integers nor `enum` members, but convert to integers).
    """

    __slots__ = ('_value',)

    _NAMES = ('NEAREST', 'BILINEAR', 'BICUBIC', 'SPLINE16', 'SPLINE36',
              'HANNING', 'HAMMING', 'HERMITE', 'KAISER', 'QUADRIC',
              'CATROM', 'GAUSSIAN', 'BESSEL', 'MITCHELL', 'SINC', 'LANCZOS',
              'BLACKMAN')

    def __init__(self, value):
        self._value = operator.index(value)

    @property
    def name(self):
        """The member's name (``'???'`` for a value naming none)."""
        if 0 <= self._value < len(self._NAMES):
            return self._NAMES[self._value]
        return '???'

    @property
    def value(self):
        """The member's integer value."""
        return self._value

    def __int__(self):
        return self._value

    def __index__(self):
        return self._value

    def __hash__(self):
        return self._value

    def __eq__(self, other):
        return type(other) is type(self) and other._value == self._value

    def __ne__(self, other):
        return not self == other

    def __reduce__(self):
        return type(self), (self._value,)

    def __repr__(self):
        return f'<{type(self).__name__}.{self.name}: {self._value}>'

    def __str__(self):
        return f'{type(self).__name__}.{self.name}'


_InterpolationType.__members__ = {
    name: _InterpolationType(value)
    for value, name in enumerate(_InterpolationType._NAMES)}
globals().update(_InterpolationType.__members__)


def _incompatible_arguments(*args):
    """pybind11's error for arguments of the wrong types."""
    return TypeError(
        "resample(): incompatible function arguments. The following argument "
        "types are supported:\n"
        "    1. (input_array: numpy.ndarray, output_array: numpy.ndarray, "
        "transform: object, interpolation: matplotlib._image._InterpolationType "
        "= <_InterpolationType.NEAREST: 0>, resample: bool = False, alpha: float "
        "= 1, norm: bool = False, radius: float = 1) -> None\n\n"
        f"Invoked with: {', '.join(map(repr, args))}")


def _transform_mesh(transform, shape):
    """
    The source position of the centre of every output pixel of *shape*,
    mapped back through the inverse of the non-affine *transform*, as an
    ``(M, N, 2)`` array.
    """
    rows, cols = shape
    y, x = np.mgrid[:rows, :cols]
    centres = np.column_stack([x.ravel() + 0.5, y.ravel() + 0.5])
    mesh = np.asarray(transform.inverted().transform(centres), dtype=float)
    if mesh.ndim != 2:
        raise RuntimeError("Inverse transformed mesh array should be 2D not "
                           f"{mesh.ndim}D")
    if mesh.shape != (rows * cols, 2):
        raise RuntimeError("Inverse transformed mesh array should have shape "
                           f"({rows * cols}, 2) not ({mesh.shape[0]}, "
                           f"{mesh.shape[1]})")
    return mesh.reshape(rows, cols, 2)


def resample(input_array, output_array, transform, interpolation=NEAREST,
             resample=False, alpha=1, norm=False, radius=1):
    """
    Resample input_array, blending it in-place into output_array, using an affine
    transform.

    Parameters
    ----------
    input_array : 2-d or 3-d NumPy array of float, double or `numpy.uint8`
        If 2-d, the image is grayscale.  If 3-d, the image must be of size 4 in
        the last dimension and represents RGBA data.
    output_array : 2-d or 3-d NumPy array of float, double or `numpy.uint8`
        The dtype and number of dimensions must match `input_array`.
    transform : matplotlib.transforms.Transform instance
        The transformation from the input array to the output array.
    interpolation : int, default: NEAREST
        The interpolation method.  Must be one of the following constants
        defined in this module:

          NEAREST, BILINEAR, BICUBIC, SPLINE16, SPLINE36, HANNING, HAMMING,
          HERMITE, KAISER, QUADRIC, CATROM, GAUSSIAN, BESSEL, MITCHELL, SINC,
          LANCZOS, BLACKMAN
    resample : bool, optional
        When `True`, use a full resampling method.  When `False`, only resample
        when the output image is larger than the input image.
    alpha : float, default: 1
        The transparency level, from 0 (transparent) to 1 (opaque).
    norm : bool, default: False
        Whether to norm the interpolation function.
    radius: float, default: 1
        The radius of the kernel, if method is SINC, LANCZOS or BLACKMAN.
    """
    if (input_array is None or not isinstance(output_array, np.ndarray)
            or not isinstance(interpolation, _InterpolationType)):
        raise _incompatible_arguments(input_array, output_array, transform,
                                      interpolation, resample, alpha, norm,
                                      radius)
    input_array = np.asarray(input_array)
    ndim = input_array.ndim
    if ndim not in (2, 3):
        raise ValueError("Input array must be a 2D or 3D array")
    if ndim == 3 and input_array.shape[2] != 4:
        raise ValueError("3D input array must be RGBA with shape (M, N, 4), "
                         f"has trailing dimension of {input_array.shape[2]}")
    if output_array.ndim != ndim:
        raise ValueError(f"Input ({ndim}D) and output ({output_array.ndim}D) "
                         "arrays have different dimensionalities")
    if ndim == 3 and output_array.shape[2] != 4:
        raise ValueError("3D output array must be RGBA with shape (M, N, 4), "
                         f"has trailing dimension of {output_array.shape[2]}")
    if output_array.dtype != input_array.dtype:
        raise ValueError("Input and output arrays have mismatched types")
    if not output_array.flags.c_contiguous:
        raise ValueError("Output array must be C-contiguous")
    if not output_array.flags.writeable:
        raise ValueError("Output array must be writeable")

    if transform is None or transform.is_affine:
        matrix = np.eye(3) if transform is None else np.asarray(transform, float)
        if matrix.shape != (3, 3):
            raise ValueError("Invalid affine transformation matrix")
        mapping = ("affine", matrix)
    else:
        mapping = ("mesh", _transform_mesh(transform, output_array.shape[:2]))
    # matplotlib's wrapper takes alpha and radius as C floats.
    settings = (interpolation.name.lower(), bool(resample),
                float(np.float32(alpha)), bool(norm), float(np.float32(radius)))
    dtype = input_array.dtype
    if dtype in (np.uint8, np.int8):
        # Agg reads signed bytes as unsigned ones, as matplotlib's does.
        target = output_array.view(np.uint8)
        target[...] = _native_image.resample_bytes(
            input_array.view(np.uint8), target, mapping, settings)
    elif dtype in (np.float32, np.float64):
        output_array[...] = _native_image.resample_floats(
            input_array, output_array, dtype == np.float32, mapping, settings)
    elif dtype in (np.uint16, np.int16):
        raise NotImplementedError("pltrs: _image.resample of 16-bit arrays")
    else:
        raise ValueError("arrays must be of dtype byte, short, float32 or "
                         "float64")


def calculate_rms_and_diff(expected_image, actual_image):
    """
    The root mean square of the difference of two images of shape
    ``(M, N, 3)`` or ``(M, N, 4)``, and the difference image, as a
    ``(M, N, 3)`` array of bytes ten times as strong.
    """
    return _testing.calculate_rms_and_diff(expected_image, actual_image)
