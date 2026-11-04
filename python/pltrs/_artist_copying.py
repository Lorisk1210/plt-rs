"""
The properties copied between artists and the sticky edges of `.Artist`
(matplotlib's ``Artist.update_from`` and ``Artist.sticky_edges``).

pltrs (Rust) keeps both (``_native.artist.update_from``, ``sticky_edges``):
autoscaling reads the sticky edges where it runs, so the lists
``sticky_edges`` hands out give every change in place to the artist.
"""

from collections import namedtuple

from ._native import artist as _artist

#: What the Python side keeps of the transform and the clip regions it
#: gave pltrs (Rust), copied with them.
_REFERENCES = ("_given_transform", "_clipbox", "_clippath")


class _StickyEdgeList(list):
    """
    A list of sticky edges of one axis of an artist: every change in
    place (``edges[:] = ...``, ``append``, ...) is given to the artist.
    """

    def __init__(self, owner, values):
        super().__init__(values)
        self._owner = owner

    def _changed(self):
        self._owner._push_sticky_edges()

    def _changing(method):
        def changing(self, *args):
            result = method(self, *args)
            self._changed()
            return result
        changing.__name__ = method.__name__
        changing.__doc__ = method.__doc__
        return changing

    __setitem__ = _changing(list.__setitem__)
    __delitem__ = _changing(list.__delitem__)
    __iadd__ = _changing(list.__iadd__)
    __imul__ = _changing(list.__imul__)
    append = _changing(list.append)
    extend = _changing(list.extend)
    insert = _changing(list.insert)
    pop = _changing(list.pop)
    remove = _changing(list.remove)
    clear = _changing(list.clear)
    reverse = _changing(list.reverse)
    del _changing

    def sort(self, *, key=None, reverse=False):
        list.sort(self, key=key, reverse=reverse)
        self._changed()

    def _refill(self, values):
        """Hold *values*, which the artist has already."""
        list.__setitem__(self, slice(None), values)


class _XYPair(namedtuple("_XYPair", "x y")):
    """The ``x`` and ``y`` sticky edge lists of an artist (matplotlib's
    ``_XYPair``, a named tuple), which give the artist their changes."""

    def __new__(cls, native):
        return cls._holding(native, *_artist.get_sticky_edges(native))

    @classmethod
    def _holding(cls, native, x, y):
        """The pair of the artist *native*, whose edges are *x* and *y*."""
        pair = super().__new__(cls, _StickyEdgeList(None, x), _StickyEdgeList(None, y))
        pair._native = native
        for edges in pair:
            edges._owner = pair
        return pair

    def __reduce__(self):
        # The edges are pickled: the artist's figure is read back after
        # the pair is, so the pair cannot ask the artist for them.
        return (_XYPair._holding, (self._native, list(self.x), list(self.y)))

    def _push_sticky_edges(self):
        _artist.set_sticky_edges(self._native, [float(v) for v in self.x],
                                 [float(v) for v in self.y])


class CopyingMixin:
    """`update_from` and `sticky_edges` of `.Artist`."""

    @property
    def sticky_edges(self):
        """
        ``x`` and ``y`` sticky edge lists for autoscaling.

        When performing autoscaling, if a data limit coincides with a value in
        the corresponding sticky_edges list, then no margin will be added--the
        view limit "sticks" to the edge. A typical use case is histograms,
        where one usually expects no margin on the bottom edge (0) of the
        histogram.

        Moreover, margin expansion "bumps" against sticky edges and cannot
        cross them.  For example, if the upper data limit is 1.0, the upper
        view limit computed by simple margin application is 1.2, but there is a
        sticky edge at 1.1, then the actual upper view limit will be 1.1.

        This attribute cannot be assigned to; however, the ``x`` and ``y``
        lists can be modified in place as needed.

        Examples
        --------
        >>> artist.sticky_edges.x[:] = (xmin, xmax)
        >>> artist.sticky_edges.y[:] = (ymin, ymax)

        """
        pair = self.__dict__.get("_sticky_edges")
        if pair is None or pair._native is not self._native:
            # Made anew when the artist joins a figure under a new handle.
            pair = self._sticky_edges = _XYPair(self._native)
        else:
            # The values the lists were given stay while pltrs (Rust) has
            # them (an integer stays one).
            for edges, native in zip(pair, _artist.get_sticky_edges(self._native)):
                if [float(v) for v in edges] != native:
                    edges._refill(native)
        return pair

    def update_from(self, other):
        """Copy properties from *other* to *self*."""
        self._took_from(other, _artist.update_from(self._native, other._native))

    def _took_from(self, other, references):
        """
        Keep the Python objects of what pltrs (Rust) copied from *other*:
        its path effects, and its transform and clip regions when
        *references* says they were copied too.
        """
        if references:
            for key in _REFERENCES:
                if key in other.__dict__:
                    self.__dict__[key] = other.__dict__[key]
                else:
                    self.__dict__.pop(key, None)
            if "_given_transform" not in other.__dict__ and other.is_transform_set():
                # The transform object of *other* (``ax.transData``), which
                # this artist may not reach from its own figure yet.
                self._given_transform = other.get_transform()
        if "_sticky_edges" in other.__dict__:
            # The values themselves, as matplotlib copies the lists.
            for mine, theirs in zip(self.sticky_edges, other.sticky_edges):
                mine._refill(list(theirs))
        if "_path_effects" in other.__dict__:
            self._path_effects = other._path_effects
        else:
            self.__dict__.pop("_path_effects", None)
        self.pchanged()
        self.stale = True
