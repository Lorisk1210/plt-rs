"""
Whether a figure or subfigure needs drawing again: the ``stale`` flag
matplotlib keeps on its artists, kept here for figures.

As in matplotlib 3.11.2 (under the Matplotlib License), a figure starts
stale, becomes stale when a subfigure is added to it or one of its
subfigures becomes stale, and stops being stale, with all its subfigures,
when it is drawn. pltrs (Rust) draws figures from scratch, so the other
artists are never stale.
"""


def _stale_figure_callback(self, val):
    """Make the figure holding *self* stale too (matplotlib's callback of
    subfigures)."""
    if (fig := self.get_figure(root=False)) is not None:
        fig.stale = val


class StaleMixin:
    """The ``stale`` flag of `.FigureBase`."""

    _stale = True
    stale_callback = None

    @property
    def stale(self):
        """
        Whether the figure is 'stale' and needs to be re-drawn for the
        output to match the internal state of the figure.
        """
        return self._stale

    @stale.setter
    def stale(self, val):
        self._stale = val
        # if the figure is animated it does not take normal part in the draw
        # stack and is not expected to be drawn as part of the normal draw
        # loop (when not saving) so do not propagate this change
        if self.get_animated():
            return
        if val and self.stale_callback is not None:
            self.stale_callback(self, val)

    def _set_drawn(self):
        """Mark the figure and its subfigures as drawn, no longer stale."""
        self._stale = False
        for subfig in self.subfigs:
            subfig._set_drawn()
