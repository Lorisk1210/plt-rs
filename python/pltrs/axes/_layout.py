"""
What layout engines measure of an Axes (matplotlib's
``Axes.get_tightbbox``), as a mixin the Axes classes include.
"""

import numpy as np

from .._native import figure_layout as _figure_layout
from ..transforms import Bbox


class LayoutMixin:
    """The tight bounding box of an Axes."""

    def get_tightbbox(self, renderer=None, *, call_axes_locator=True,
                      bbox_extra_artists=None, for_layout_only=False):
        """
        Return the tight bounding box of the Axes, including axis and their
        decorators (xlabel, title, etc).

        Artists that have ``artist.set_in_layout(False)`` are not included
        in the bbox.

        Parameters
        ----------
        renderer : `.RendererBase` subclass
            The renderer (pltrs (Rust) measures the figure itself).
        bbox_extra_artists : list of `.Artist` or ``None``
            List of artists to include in the tight bounding box.  If
            ``None`` (default), then all artist children of the Axes are
            included in the tight bounding box.
        call_axes_locator : bool, default: True
            pltrs (Rust) always places the Axes with its locator.
        for_layout_only : default: False
            The bounding box will *not* include the x-extent of the title and
            the xlabel, or the y-extent of the ylabel.

        Returns
        -------
        `.BboxBase`
            Bounding box in figure pixel coordinates.
        """
        extent = _figure_layout.axes_tightbbox(
            self._native, bool(for_layout_only), bbox_extra_artists is None)
        if extent is None:
            return None
        x0, y0, x1, y1 = extent
        boxes = [Bbox([[x0, y0], [x1, y1]])]
        for artist in bbox_extra_artists or ():
            bbox = artist.get_tightbbox(renderer)
            if (bbox is not None and 0 < bbox.width < np.inf
                    and 0 < bbox.height < np.inf):
                boxes.append(bbox)
        return Bbox.union(boxes)
