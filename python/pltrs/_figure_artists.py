"""
The lists of artists a figure draws (matplotlib's ``Figure.artists``,
``lines``, ``patches`` and ``texts``).

matplotlib's figure draws the artists its lists hold as they are: code
adds artists by putting them in a list (``fig.artists.append(patch)``,
``fig.lines.extend(lines)``, ``fig.texts += [text]``), and an artist put
there without a transform of its own is drawn in display pixels (its
transform is the identity), unclipped. pltrs (Rust) keeps the artists of
a figure: a list of these has the figure draw each artist put in it in
that way, and stop drawing each artist taken out of it.

`Figure.artists` and `Figure.texts` list what pltrs (Rust) draws on the
figure itself, the artists `~.Figure.add_artist` and `~.Figure.text`
added included; `Figure.lines` and `Figure.patches` the artists put in
them. `Figure.legends` lists the legends pltrs (Rust) draws on the
figure: a legend made for it (``Legend(fig, ...)``) is drawn once it is
put in the list, as `~.Figure.legend` and libraries building legends
themselves (seaborn) put it there.
"""

from .transforms import IdentityTransform


class DrawnList(list):
    """A list of artists *figure* draws: those put in it are drawn, those
    taken out no longer."""

    def __init__(self, figure, artists=(), *, texts=False):
        super().__init__(artists)
        self._figure = figure
        #: Whether the list is the figure's texts, which lists the texts
        #: put in it there rather than among its artists.
        self._texts = texts

    def __reduce__(self):
        # Made of its figure and its artists again (a list's own pickle
        # would put the artists back through `extend` before the list has
        # its figure).
        return (type(self), (self._figure, list(self)), {"_texts": self._texts})

    def _put(self, artists):
        """Have the figure draw *artists* as they are; return them."""
        artists = list(artists)
        for artist in artists:
            if not artist.is_transform_set():
                artist.set_transform(IdentityTransform())
            artist._add_to_figure(self._figure, False)
            if self._texts:
                from ._native import figure as _figure
                _figure.list_among_texts(self._figure._native, artist._native)
        return artists

    @staticmethod
    def _take(artists):
        """Stop drawing *artists*."""
        for artist in artists:
            if artist.get_figure(root=True) is not None:
                artist.remove()

    def append(self, artist):
        """Draw *artist* on the figure, and add it to the list."""
        super().extend(self._put([artist]))

    def extend(self, artists):
        """Draw *artists* on the figure, and add them to the list."""
        super().extend(self._put(artists))

    def __iadd__(self, artists):
        self.extend(artists)
        return self

    def insert(self, index, artist):
        """Draw *artist* on the figure, and insert it in the list."""
        super().insert(index, *self._put([artist]))

    def __setitem__(self, index, value):
        if isinstance(index, slice):
            old, new = self[index], self._put(value)
        else:
            old, new = [self[index]], self._put([value])[0]
        self._take(old)
        super().__setitem__(index, new)

    def __delitem__(self, index):
        old = self[index] if isinstance(index, slice) else [self[index]]
        super().__delitem__(index)
        self._take(old)

    def remove(self, artist):
        """Stop drawing *artist*, and remove it from the list."""
        super().remove(artist)
        self._take([artist])

    def pop(self, index=-1):
        """Stop drawing the artist at *index*; remove and return it."""
        artist = super().pop(index)
        self._take([artist])
        return artist

    def clear(self):
        """Stop drawing every artist of the list, and empty it."""
        old = list(self)
        super().clear()
        self._take(old)


class LegendList(DrawnList):
    """The legends *figure* draws (matplotlib's ``Figure.legends``): a
    legend made for the figure and put in the list is drawn, one taken out
    no longer."""

    def _put(self, legends):
        """Have the figure draw *legends*, those it does not draw yet as
        they were made for it; return them."""
        legends = list(legends)
        for legend in legends:
            if legend not in self:
                legend._add_to_figure(self._figure, False)
        return legends


def replace(current, artists):
    """Have the figure of *current*, a `DrawnList`, draw *artists* in place
    of the artists of *current* (assigning a figure's list): return the
    list of *artists*."""
    if artists is current:
        return current
    artists = list(artists)
    kept = {id(artist) for artist in artists}
    current._take([artist for artist in current if id(artist) not in kept])
    known = {id(artist) for artist in current}
    current._put([artist for artist in artists if id(artist) not in known])
    return DrawnList(current._figure, artists, texts=current._texts)
