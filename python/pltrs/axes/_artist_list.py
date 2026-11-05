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

    def __init__(self, axes, prop_name,
                 valid_types=None, invalid_types=None):
        """
        Parameters
        ----------
        axes : `~matplotlib.axes.Axes`
            The Axes from which this sublist will pull the children
            Artists.
        prop_name : str
            The property name used to access this sublist from the Axes;
            used to generate deprecation warnings.
        valid_types : list of type, optional
            A list of types that determine which children will be returned
            by this sublist. If specified, then the Artists in the sublist
            must be instances of any of these types. If unspecified, then
            any type of Artist is valid (unless limited by
            *invalid_types*.)
        invalid_types : tuple, optional
            A list of types that determine which children will *not* be
            returned by this sublist. If specified, then Artists in the
            sublist will never be an instance of these types. Otherwise, no
            types will be excluded.
        """
        self._axes = axes
        self._prop_name = prop_name

        def type_check(artist):
            return ((not valid_types or isinstance(artist, valid_types)) and
                    (not invalid_types or not isinstance(artist, invalid_types)))

        self._artists = lambda: [artist for artist in axes._children
                                 if type_check(artist)]

    @classmethod
    def _of(cls, prop_name, artists):
        """
        The sublist *prop_name* of the artists the callable *artists*
        returns (in the order they were added), read whenever it is used.
        """
        sublist = cls.__new__(cls)
        sublist._prop_name = prop_name
        sublist._artists = artists
        return sublist

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
