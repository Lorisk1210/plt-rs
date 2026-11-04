"""
Norms a draw scales. matplotlib's draw maps the data of an image or a
collection through its norm, whose ``__call__`` gives the norm object the
limits it lacks. pltrs (Rust) maps the data with its own copy of the norm,
so a mappable gives the norm object the limits that draw scaled it to the
next time it is read (``norm``, ``get_clim``): as long as the norm is the
one pltrs (Rust) was last handed and the mappable was drawn since.
"""

from ._native import artist as _artist


class DrawScaledNorm:
    """The norm and limits of a mappable read as its last draw scaled
    them; a class gives the limits its draw scales the norm to
    (``_scale_by_draw``)."""

    @property
    def norm(self):
        self._scale_as_drawn()
        return self._colorizer.norm

    @norm.setter
    def norm(self, norm):
        self._colorizer.norm = norm

    def get_clim(self):
        """
        Return the values (min, max) that are mapped to the colormap limits.
        """
        self._scale_as_drawn()
        return super().get_clim()

    def _hand_mapping(self, hand):
        """Hand pltrs (Rust) the mapping with *hand(replaced)*, *replaced*
        saying whether the norm is new: the limits the last draw scaled the
        norm it replaces stay that norm's."""
        replaced = self._norm_replaced()
        if not replaced:
            self._scale_as_drawn()
        hand(replaced)
        self._handed_draws = _artist.draw_count(self._native)

    def _scale_as_drawn(self):
        """Give the norm the limits it lacks that the draws since it was
        handed to pltrs (Rust) scaled it to."""
        norm = self._colorizer.norm
        if (norm.scaled() or self._figure is None
                or norm is not self.__dict__.get("_handed_norm")
                or _artist.draw_count(self._native)
                <= self.__dict__.get("_handed_draws", 0)):
            return
        with norm.callbacks.blocked():
            self._scale_by_draw(norm)
