"""
Artists matplotlib draws as parts of another artist, which pltrs (Rust)
draws with it: an `.Axis` with its Axes, a table's cells with the table,
the boxes inside an offset box with the box added to the figure.

matplotlib's ``draw`` of each of them ends with ``self.stale = False``; in
pltrs the artist that draws them counts its draws
(``_native.artist.draw_count``), and a part's ``stale`` reads as fresh
once its drawer drew since the part last went stale (a hidden part is
not drawn and stays stale, as does an animated axis, which its Axes
leaves out of its draws).
"""

from ._native import artist as _artist


class DrawnWith:
    """
    Mixin of an `.Artist` drawn by the artist its ``_drawer()`` returns
    (None while there is none), whose draws make it fresh.
    """

    @property
    def stale(self):
        """
        Whether the artist is 'stale' and needs to be re-drawn for the output
        to match the internal state of the artist.
        """
        self._follow_drawer()
        return _artist.get_stale(self._native)

    @stale.setter
    def stale(self, val):
        self._follow_drawer()
        _artist.set_stale(self._native, bool(val))
        self._went_stale(val)

    def _follow_drawer(self):
        """Make the artist fresh if its drawer drew it since this was
        last checked."""
        drawer = self._drawer()
        if drawer is None:
            return
        draws = _artist.draw_count(drawer._native)
        if draws != self.__dict__.get("_drawer_draws", 0):
            self._drawer_draws = draws
            if self._drawn_by_drawer():
                _artist.set_stale(self._native, False)

    def _drawn_by_drawer(self):
        """Whether the drawer's draws draw the part: unless it is hidden."""
        return self.get_visible()
