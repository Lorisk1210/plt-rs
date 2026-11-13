"""
Drawing parts of an Axes onto its canvas between full draws (the
``draw_artist`` and ``redraw_in_frame`` of matplotlib's ``axes._base``),
for blitting: widgets and animations draw their animated artists over a
background saved with ``canvas.copy_from_bbox``.

pltrs (Rust) draws them onto the raster canvas it keeps for the figure,
placed and clipped as the figure was last laid out.
"""

from .._native import backends as _backends


class BlittingMixin:
    """The ``draw_artist`` and ``redraw_in_frame`` methods of `~.axes.Axes`."""

    def draw_artist(self, a):
        """
        Efficiently redraw a single artist.
        """
        a.draw(self.get_figure(root=True).canvas.get_renderer())

    def redraw_in_frame(self):
        """
        Efficiently redraw Axes data, but not axis ticks, labels, etc.
        """
        # matplotlib's `draw` of the Axes applies its aspect first, which
        # changes the limits of an Axes adjusting its data limits.
        self.apply_aspect()
        _backends.redraw_in_frame(self._native)
