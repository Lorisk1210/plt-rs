"""
What changes the drawing of every artist (matplotlib's
``Artist.set_path_effects`` and ``Artist.set_agg_filter``): pltrs (Rust)
draws the path effects of `.patheffects`, which describe themselves to it
(``AbstractPathEffect._native_spec``), and calls an agg filter function
with what the artist drew.
"""

from ._native import artist as _artist
from ._native import patheffects as _patheffects


class EffectsMixin:
    """The path effect and image filter methods of `.Artist`."""

    def set_path_effects(self, path_effects):
        """
        Set the path effects.

        Parameters
        ----------
        path_effects : list of `.AbstractPathEffect`
        """
        _patheffects.set_path_effects(
            self._native, [effect._native_spec() for effect in path_effects])
        self._path_effects = path_effects

    def get_path_effects(self):
        if "_path_effects" in self.__dict__:
            return self._path_effects
        from . import rcParams
        return rcParams['path.effects']

    def set_agg_filter(self, filter_func):
        """
        Set the agg filter.

        Parameters
        ----------
        filter_func : callable
            A filter function, which takes a (m, n, depth) float array
            and a dpi value, and returns a (m, n, depth) array and two
            offsets from the bottom left corner of the image

            .. ACCEPTS: a filter function, which takes a (m, n, 3) float array
                and a dpi value, and returns a (m, n, 3) array and two offsets
                from the bottom left corner of the image
        """
        _artist.set_agg_filter(self._native, filter_func)
        self._agg_filter = filter_func

    def get_agg_filter(self):
        """Return filter function to be used for agg filter."""
        return self.__dict__.get("_agg_filter")
