"""
What matplotlib's Agg extension (``matplotlib.backends._backend_agg``)
gives Python: `BufferRegion`, the pixels `.RendererAgg.copy_from_bbox`
saves, and the pixels of a renderer (`RendererAgg`, which pltrs (Rust)
draws for `.backend_agg.RendererAgg`).
"""

import numpy as np

from .. import _pybind
from .._native import backends as _backends

#: The constructor signature matplotlib's Agg extension refuses arguments
#: with (pybind11's message, followed by the arguments' reprs).
_RENDERER_SIGNATURE = (
    "__init__(): incompatible constructor arguments. The following argument "
    "types are supported:\n    1. matplotlib.backends._backend_agg.RendererAgg("
    "width: typing.SupportsInt | typing.SupportsIndex, height: "
    "typing.SupportsInt | typing.SupportsIndex, dpi: typing.SupportsFloat | "
    "typing.SupportsIndex)\n\nInvoked with: ")


def check_renderer_arguments(width, height, dpi):
    """Refuse the size and resolution of a renderer as matplotlib's Agg
    extension refuses them (its ``RendererAgg(int(width), int(height),
    dpi)``): a size no unsigned int holds, a resolution that is no
    number."""
    width, height = int(width), int(height)
    if (not all(0 <= side < 1 << 32 for side in (width, height))
            or not (hasattr(dpi, "__float__") or hasattr(dpi, "__index__"))):
        raise TypeError(_RENDERER_SIGNATURE + ", ".join(
            map(repr, (width, height, dpi))))
    if dpi <= 0:
        raise ValueError("dpi must be positive")
    if width >= 1 << 23 or height >= 1 << 23:
        raise ValueError(
            f"Image size of {width}x{height} pixels is too large. It must be "
            "less than 2^23 in each direction.")


#: The signatures of matplotlib's ``RendererAgg.restore_region`` (C++),
#: ``{module}`` standing for this module.
_RESTORE_REGION = (
    "(self: {module}.RendererAgg, region: BufferRegion) -> None",
    "(self: {module}.RendererAgg, region: BufferRegion, "
    + ", ".join(f"{name}: typing.SupportsInt | typing.SupportsIndex"
                for name in ("xx1", "yy1", "xx2", "yy2", "x", "y"))
    + ") -> None",
)


def rect(bbox):
    """The extents ``(x0, y0, x1, y1)`` of *bbox* as matplotlib's extension
    converts a rectangle (``type_caster<agg::rect_d>``): None for an empty
    one, else a ``2×2`` or 4-item array of numbers."""
    if bbox is None:
        return 0.0, 0.0, 0.0, 0.0
    try:
        points = np.asarray(bbox, dtype=float)
    except (TypeError, ValueError):
        points = None
    if points is None or points.shape not in ((2, 2), (4,)):
        raise ValueError("Invalid bounding box")
    return tuple(map(float, points.ravel()))


def check_restore_region(renderer, region, *coordinates):
    """Refuse to restore *region* in *renderer* (its ``_renderer``) at
    *coordinates* (none, or the integer corners and position) if it is no
    region, as matplotlib's renderer refuses it."""
    if not isinstance(region, BufferRegion):
        raise _pybind.incompatible(
            "restore_region", [signature.format(module=__name__)
                               for signature in _RESTORE_REGION],
            (renderer, region, *coordinates), {})


class BufferRegion:
    """
    The pixels of a rectangle of the canvas, saved by
    `RendererAgg.copy_from_bbox` to be put back by `~RendererAgg.restore_region`.
    """

    def __init__(self, native):
        self._native = native

    def get_extents(self):
        """The saved rectangle ``(x0, y0, x1, y1)``, in canvas pixels from its
        top-left corner."""
        return _backends.region_extents(self._native)

    def __buffer__(self, flags):
        """The saved pixels, ``(height, width, 4)`` RGBA bytes rows from the
        top (matplotlib's region exposes them through the buffer protocol:
        ``memoryview(region)``, ``np.asarray(region)``)."""
        return memoryview(_backends.region_rgba(self._native))


class RendererAgg:
    """
    The pixels of the Agg renderer *renderer* (a `.backend_agg.RendererAgg`),
    as matplotlib's C++ renderer, its ``_renderer``, gives them through the
    buffer protocol: ``(height, width, 4)`` RGBA bytes, rows from the top
    (``memoryview(renderer._renderer)``, ``np.asarray(renderer._renderer)``).
    The renderer draws through its own methods.
    """

    def __init__(self, renderer):
        self._owner = renderer

    def __buffer__(self, flags):
        """The renderer's pixels (see `.backend_agg.RendererAgg.buffer_rgba`)."""
        return self._owner.buffer_rgba()
