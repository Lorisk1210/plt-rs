"""
What layout engines measure of an Axes (matplotlib's
``Axes.get_tightbbox``), as a mixin the Axes classes include.
"""

import numpy as np

from .._native import figure_layout as _figure_layout
from ..transforms import Bbox


class LayoutMixin:
    """The tight bounding box of an Axes."""

    def get_default_bbox_extra_artists(self):
        """
        Return a default list of artists that are used for the bounding box
        calculation.

        Artists are excluded either by not being visible or
        ``artist.set_in_layout(False)``.
        """
        from .. import axis as maxis, offsetbox
        from ._base import _AxesBase

        artists = self.get_children()

        for axis in self._axis_map.values():
            # axis tight bboxes are calculated separately inside
            # Axes.get_tightbbox() using for_layout_only=True
            artists.remove(axis)
        if not (self.axison and self.get_frame_on()):
            # don't do bbox on spines if frame not on.
            for spine in self.spines.values():
                artists.remove(spine)

        artists.remove(self.title)
        artists.remove(self._left_title)
        artists.remove(self._right_title)

        # always include types that do not internally implement clipping
        # to Axes. may have clip_on set to True and clip_box equivalent
        # to ax.bbox but then ignore these properties during draws.
        noclip = (_AxesBase, maxis.Axis,
                  offsetbox.AnnotationBbox, offsetbox.OffsetBox)
        return [a for a in artists if a.get_visible() and a.get_in_layout()
                and (isinstance(a, noclip) or not a._fully_clipped_to_axes())]

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
        # matplotlib applies the aspect and places the ticks anew.
        self.stale = True
        x0, y0, x1, y1 = extent
        boxes = [Bbox([[x0, y0], [x1, y1]])]
        for artist in bbox_extra_artists or ():
            bbox = artist.get_tightbbox(renderer)
            if (bbox is not None and 0 < bbox.width < np.inf
                    and 0 < bbox.height < np.inf):
                boxes.append(bbox)
        return Bbox.union(boxes)
