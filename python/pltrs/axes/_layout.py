"""
What layout engines measure of an Axes (matplotlib's
``Axes.get_tightbbox``), as a mixin the Axes classes include.
"""

import numpy as np

from .. import _unusable
from ..artist import _AXIS_MEASURING
from .._native import figure_layout as _figure_layout, insets as _insets
from .._program_classes import class_cache, takes_from_program
from ..text import _measured_by_given
from ..transforms import Bbox


@class_cache
def _measures_axis(cls):
    """Whether the axis class *cls* measures its axes itself (one of
    `.artist._AXIS_MEASURING` is the program's)."""
    return any(takes_from_program(cls, name) for name in _AXIS_MEASURING)


def _measures_itself(axis):
    """Whether *axis* measures itself: its class does, or the program
    assigned it one of `.artist._AXIS_MEASURING`."""
    own = vars(axis)
    return ((_measures_axis(type(axis)) or any(name in own for name in _AXIS_MEASURING))
            and not own.get("_held"))


class LayoutMixin:
    """The tight bounding box of an Axes."""

    def _axes_measure_themselves(self):
        """Whether an axis of the Axes is of a class measuring it, which
        matplotlib's `get_tightbbox` asks: the Axes is then measured in
        Python, here, wherever pltrs (Rust) takes its tight box."""
        return any(_measures_itself(axis) for axis in self._axis_map.values())

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
        if not (self.axison and self._frameon):
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

    def _first_measuring_steps(self, renderer):
        """What matplotlib's `get_tightbbox` asks of *renderer* before it
        measures a text (whose metrics refuse what is no renderer):
        nothing, but where a subclass' children convert points with it."""

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
            renderer that will be used to draw the figures (i.e.
            ``fig.canvas.get_renderer()``)

        bbox_extra_artists : list of `.Artist` or ``None``
            List of artists to include in the tight bounding box.  If
            ``None`` (default), then all artist children of the Axes are
            included in the tight bounding box.

        call_axes_locator : bool, default: True
            If *call_axes_locator* is ``False``, it does not call the
            ``_axes_locator`` attribute, which is necessary to get the correct
            bounding box. ``call_axes_locator=False`` can be used if the
            caller is only interested in the relative size of the tightbbox
            compared to the Axes bbox.

        for_layout_only : default: False
            The bounding box will *not* include the x-extent of the title and
            the xlabel, or the y-extent of the ylabel.

        Returns
        -------
        `.BboxBase`
            Bounding box in figure pixel coordinates.

        See Also
        --------
        matplotlib.axes.Axes.get_window_extent
        matplotlib.axis.Axis.get_tightbbox
        matplotlib.spines.Spine.get_window_extent
        """
        self._check_figure_renderer(renderer)
        _unusable.check(self, measuring=True)
        # matplotlib calls the Axes' locator first: one that is an artist
        # (axes_grid1's anchored locators) takes the Axes as its own, and
        # refuses a second one (a parasite calling its host's).
        locator = self.get_axes_locator()
        called = getattr(locator, "_called_with", None)
        if called is not None and call_axes_locator:
            called(self)
        elif (locator and called is None
                and getattr(locator, "_place", None) is None):
            # A locator of the program's, which pltrs (Rust) only follows
            # in a draw (`._program_locators`): the Axes is measured where
            # it places it now, as matplotlib applies it first, or (not
            # called) at its original position, until the next draw.
            if call_axes_locator:
                self._locate_by_program(
                    renderer if renderer is not None
                    else self.get_figure(root=True)._get_renderer())
            else:
                _insets.set_located_position(self._native, None)
        self._place_titles_by_program(renderer)
        self._first_measuring_steps(renderer)
        if bbox_extra_artists is None and takes_from_program(
                type(self), "get_default_bbox_extra_artists"):
            bbox_extra_artists = self.get_default_bbox_extra_artists()
        if self._axes_measure_themselves():
            return self._tightbbox_of_parts(renderer, bbox_extra_artists, for_layout_only)
        with _measured_by_given(renderer, self):
            extent = _figure_layout.axes_tightbbox(
                self._native, bool(for_layout_only), bbox_extra_artists is None)
        if extent is None:
            return None
        # matplotlib applies the aspect and places the ticks anew.
        self.stale = True
        x0, y0, x1, y1 = extent
        boxes = [Bbox([[x0, y0], [x1, y1]])]
        boxes.extend(_extra_boxes(bbox_extra_artists or (), renderer))
        return Bbox.union(boxes)

    def _tightbbox_of_parts(self, renderer, bbox_extra_artists, for_layout_only):
        """matplotlib's `get_tightbbox` from the Axes' parts, each asked
        for its box (an axis of the program's through its own measuring):
        its axes, its box, its titles and the artists *bbox_extra_artists*
        (its default ones when None)."""
        if not self.get_visible():
            return None
        if renderer is None:
            renderer = self.get_figure(root=True)._get_renderer()
        self.apply_aspect()
        boxes = []
        for axis in self._axis_map.values():
            if self.axison and axis.get_visible():
                box = (axis.get_tightbbox(renderer, for_layout_only=True) if for_layout_only
                       else axis.get_tightbbox(renderer))
                if box:
                    boxes.append(box)
        boxes.append(self.get_window_extent(renderer))
        for title in (self.title, self._left_title, self._right_title):
            if title.get_visible():
                box = title.get_window_extent(renderer)
                if for_layout_only and box.width > 0:
                    box.x0 = (box.x0 + box.x1) / 2 - 0.5
                    box.x1 = box.x0 + 1.0
                boxes.append(box)
        if bbox_extra_artists is None:
            bbox_extra_artists = self.get_default_bbox_extra_artists()
        boxes.extend(_extra_boxes(bbox_extra_artists, renderer))
        # matplotlib applies the aspect and places the ticks anew.
        self.stale = True
        return Bbox.union([box for box in boxes if box.width != 0 or box.height != 0])


def _extra_boxes(artists, renderer):
    """The tight boxes of *artists* that an Axes' tight box takes in: those
    of some width and height (matplotlib's `Axes.get_tightbbox`)."""
    for artist in artists:
        bbox = artist.get_tightbbox(renderer)
        if bbox is not None and 0 < bbox.width < np.inf and 0 < bbox.height < np.inf:
            yield bbox
