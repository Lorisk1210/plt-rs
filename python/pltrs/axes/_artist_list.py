"""
The type-specific sublists of an Axes' children (``ax.lines``,
``ax.patches``, ...), matplotlib's ``_AxesBase.ArtistList``: immutable
sequences read from pltrs (Rust) whenever they are used (matplotlib
3.11.2's class, under the Matplotlib License).
"""

from collections.abc import Sequence


class ArtistList(Sequence):
    """
    A sublist of Axes children based on their type.

    The type-specific children sublists were made immutable in Matplotlib
    3.7.  In the future these artist lists may be replaced by tuples. Use
    as if this is a tuple already.
    """

    def __init__(self, prop_name, artists):
        """
        Parameters
        ----------
        prop_name : str
            The property name used to access this sublist from the Axes.
        artists : callable
            Returns the artists of the sublist, in the order they were added.
        """
        self._prop_name = prop_name
        self._artists = artists

    def __repr__(self):
        return f'<Axes.ArtistList of {len(self)} {self._prop_name}>'

    def __len__(self):
        return len(self._artists())

    def __iter__(self):
        yield from self._artists()

    def __getitem__(self, key):
        return self._artists()[key]

    def __add__(self, other):
        if isinstance(other, (list, ArtistList)):
            return [*self, *other]
        if isinstance(other, tuple):
            return (*self, *other)
        return NotImplemented

    def __radd__(self, other):
        if isinstance(other, list):
            return other + list(self)
        if isinstance(other, tuple):
            return other + tuple(self)
        return NotImplemented
