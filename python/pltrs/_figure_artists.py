"""
The list of the artists of a figure (matplotlib's ``Figure.artists``).

matplotlib's figure draws the artists its ``artists`` list holds, so code
adds artists by appending to it (``fig.artists.append(patch)``). pltrs
(Rust) keeps the artists of a figure; the list `Figure.artists` gives adds
an artist appended to it to the figure, as `.Figure.add_artist` does.
"""


class FigureArtists(list):
    """The artists of *figure*, in a list adding those appended to it to the
    figure."""

    def __init__(self, figure, artists):
        super().__init__(artists)
        self._figure = figure

    def append(self, artist):
        """Add *artist* to the figure, and to the list."""
        self._figure.add_artist(artist)
        super().append(artist)
