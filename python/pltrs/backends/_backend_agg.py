"""
What matplotlib's Agg extension (``matplotlib.backends._backend_agg``)
gives Python besides its renderer: `BufferRegion`, the pixels
`.RendererAgg.copy_from_bbox` saves.
"""

from .._native import backends as _backends


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
