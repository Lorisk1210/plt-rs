"""
Where every artist is on the canvas (matplotlib's
``Artist.get_window_extent`` and ``Artist.get_tightbbox``), as pltrs
(Rust) lays the figure out.
"""

from ._native import artist as _artist
from .transforms import Bbox


class ExtentMixin:
    """The extent methods of `.Artist`."""

    def get_window_extent(self, renderer=None):
        """
        Get the artist's bounding box in display space.

        The bounding box's width and height are non-negative.

        Subclasses should override for inclusion in the bounding box
        "tight" calculation.  Default is to return an empty bounding
        box at 0, 0.
        """
        return Bbox([[0, 0], [0, 0]])

    def _laid_out_extent(self):
        """
        The artist's box in display space where pltrs (Rust) lays it out
        (the figure laid out first), a zero box outside any figure or for
        an artist taking no space: the extent of the artists whose layout
        pltrs (Rust) computes (legends, tables, offset boxes), which
        matplotlib's compute from theirs.
        """
        points = None if self._figure is None else \
            _artist.window_extent(self._native)
        return Bbox([[0, 0], [0, 0]]) if points is None else Bbox(points)

    def _extent_as_is(self):
        """
        The artist's box in display space where pltrs (Rust) lays it out
        in the figure as it is now (not laid out first: with the view
        limits autoscaling has not computed yet and the positions a layout
        engine has not set yet), a zero box outside any figure or for an
        artist taking no space: the extent of the artists matplotlib lays
        out when they are measured (legends, tables, offset boxes).
        """
        points = None if self._figure is None else \
            _artist.window_extent_as_is(self._native)
        return Bbox([[0, 0], [0, 0]]) if points is None else Bbox(points)

    def get_tightbbox(self, renderer=None):
        """
        Like `.Artist.get_window_extent`, but includes any clipping.

        Parameters
        ----------
        renderer : `~matplotlib.backend_bases.RendererBase` subclass, optional
            renderer that will be used to draw the figures (i.e.
            ``fig.canvas.get_renderer()``)

        Returns
        -------
        `.Bbox` or None
            The enclosing bounding box (in figure pixel coordinates).
            Returns None if clipping results in no intersection.
        """
        bbox = self.get_window_extent(renderer)
        if self.get_clip_on():
            clip_box = self.get_clip_box()
            if clip_box is not None:
                bbox = Bbox.intersection(bbox, clip_box)
            clip_path = self.get_clip_path()
            if clip_path is not None and bbox is not None:
                clip_path = clip_path.get_fully_transformed_path()
                bbox = Bbox.intersection(bbox, clip_path.get_extents())
        return bbox
