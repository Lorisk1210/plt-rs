"""
The children of every artist and the search through them (matplotlib's
``Artist.get_children`` and ``Artist.findobj``, from matplotlib 3.11.2
under the Matplotlib License).
"""


class ChildrenSearchMixin:
    """The ``get_children`` and ``findobj`` methods of `.Artist`."""

    def get_children(self):
        r"""Return a list of the child `.Artist`\s of this `.Artist`."""
        return []

    def findobj(self, match=None, include_self=True):
        """
        Find artist objects.

        Recursively find all `.Artist` instances contained in the artist.

        Parameters
        ----------
        match
            A filter criterion for the matches. This can be

            - *None*: Return all objects contained in artist.
            - A function with signature ``def match(artist: Artist) -> bool``.
              The result will only contain artists for which the function
              returns *True*.
            - A class instance: e.g., `.Line2D`. The result will only contain
              artists of this class or its subclasses (``isinstance`` check).

        include_self : bool
            Include *self* in the list to be checked for a match.

        Returns
        -------
        list of `.Artist`
        """
        from .artist import Artist
        if match is None:  # always return True
            def matchfunc(x):
                return True
        elif isinstance(match, type) and issubclass(match, Artist):
            def matchfunc(x):
                return isinstance(x, match)
        elif callable(match):
            matchfunc = match
        else:
            raise ValueError('match must be None, a matplotlib.artist.Artist '
                             'subclass, or a callable')

        artists = [found for child in self.get_children()
                   for found in child.findobj(matchfunc)]
        if include_self and matchfunc(self):
            artists.append(self)
        return artists
