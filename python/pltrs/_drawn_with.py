"""
Artists matplotlib draws as parts of another artist, which pltrs (Rust)
draws with it: an `.Axis` with its Axes, a table's cells with the table,
the boxes inside an offset box with the box added to the figure, the
background patch of an Axes or a figure with its owner (a mirror's part,
`._mirrors.Mirroring.drawer`).

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

    #: The changes pltrs (Rust) makes to its alpha, clipping and snapping
    #: do not reach what draws it: it goes stale through its setter
    #: (`.Artist.set_alpha`), which runs its stale callback.
    _staled_by_setters = True

    def _follow_drawer(self):
        """Make the artist fresh if its drawer drew it since this was
        last checked."""
        follow(self, self._drawer(), self._drawn_by_drawer)

    def _drawn_by_drawer(self):
        """Whether the drawer's draws draw the part: unless it is hidden."""
        return self.get_visible()


def changed(part, drawer):
    """Note that the artist *part* changed after the draws *drawer* (an
    artist) made so far, which leave it stale: a change pltrs (Rust) made,
    which passes no ``stale`` setter."""
    part._drawer_draws = _artist.draw_count(drawer._native)


def made(part, draws):
    """Note that the artist *part* stands for one matplotlib made when the
    artist drawing it had drawn *draws* times (pltrs makes the part when
    a program first reads it): stale until a later draw."""
    part._drawer_draws = draws


def follow(part, drawer, drawn_by_drawer):
    """Make the artist *part* fresh if *drawer* (an artist, or None while
    there is none) drew since this was last checked and
    *drawn_by_drawer()* says that its draws draw the part."""
    if drawer is None:
        return
    draws = _artist.draw_count(drawer._native)
    if draws != part.__dict__.get("_drawer_draws", 0):
        part._drawer_draws = draws
        if drawn_by_drawer():
            _artist.set_stale(part._native, False)
