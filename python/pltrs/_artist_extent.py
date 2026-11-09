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
        Get the artist's bounding box in display space, ignoring clipping.

        The bounding box's width and height are non-negative.

        Subclasses should override for inclusion in the bounding box "tight"
        calculation.  Default is to return an empty bounding box at 0, 0.

        .. warning::

          The extent can change due to any changes in the transform stack, such
          as changing the Axes limits, the figure size, the canvas used (as is
          done when saving a figure), or the DPI.

          Relying on a once-retrieved window extent can lead to unexpected
          behavior in various cases such as interactive figures being resized or
          moved to a screen with different dpi, or figures that look fine on
          screen render incorrectly when saved to file.

          To get accurate results you may need to manually call
          `~.Figure.savefig` or `~.Figure.draw_without_rendering` to have
          Matplotlib compute the rendered size.

        Parameters
        ----------
        renderer : `~matplotlib.backend_bases.RendererBase`, optional
            Renderer used to draw the figure (i.e. ``fig.canvas.get_renderer()``).

        See Also
        --------
        .Artist.get_tightbbox :
            Get the artist bounding box, taking clipping into account.
        """
        return Bbox([[0, 0], [0, 0]])

    def _check_figure_renderer(self, renderer):
        """
        Fail as matplotlib's measuring methods do when they are given no
        *renderer* and the artist has no figure to take one from
        (``self.get_figure(root=True)._get_renderer()``): one removed from
        its figure, say. pltrs (Rust) measures with its figure's own.
        """
        if renderer is None and self.get_figure(root=True) is None:
            raise AttributeError(
                "'NoneType' object has no attribute '_get_renderer'",
                name="_get_renderer", obj=None)

    def _laid_out_extent(self, renderer=None):
        """
        The artist's box in display space where pltrs (Rust) lays it out
        (the figure laid out first), a zero box outside any figure or for
        an artist taking no space: the extent of the artists whose layout
        pltrs (Rust) computes (legends, tables, offset boxes), which
        matplotlib's compute from theirs, measured with *renderer* when the
        program gives one (see ``text._measured_by``).
        """
        from .text import _measured_by_given
        if self._figure is None:
            return Bbox([[0, 0], [0, 0]])
        with _measured_by_given(renderer, self):
            points = _artist.window_extent(self._native)
        return Bbox([[0, 0], [0, 0]]) if points is None else Bbox(points)

    def _extent_as_is(self, renderer=None):
        """
        The artist's box in display space where pltrs (Rust) lays it out
        in the figure as it is now (not laid out first: with the view
        limits autoscaling has not computed yet and the positions a layout
        engine has not set yet), a zero box outside any figure or for an
        artist taking no space: the extent of the artists matplotlib lays
        out when they are measured (legends, tables, offset boxes),
        measured with *renderer* when the program gives one.
        """
        from .text import _measured_by_given
        if self._figure is None:
            return Bbox([[0, 0], [0, 0]])
        with _measured_by_given(renderer, self):
            points = _artist.window_extent_as_is(self._native)
        return Bbox([[0, 0], [0, 0]]) if points is None else Bbox(points)

    def get_tightbbox(self, renderer=None):
        """
        Get the artist's bounding box in display space, taking clipping into account.

        Parameters
        ----------
        renderer : `~matplotlib.backend_bases.RendererBase`, optional
            Renderer used to draw the figure (i.e. ``fig.canvas.get_renderer()``).

        Returns
        -------
        `.Bbox` or None
            The enclosing bounding box (in figure pixel coordinates), or None
            if clipping results in no intersection.

        See Also
        --------
        .Artist.get_window_extent :
            Get the artist bounding box, ignoring clipping.
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
