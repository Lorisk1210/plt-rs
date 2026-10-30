"""
Vector fields: `~.Axes.quiver`, `~.Axes.quiverkey`, `~.Axes.barbs` and
`~.Axes.streamplot`.
"""

from ... import quiver as mquiver, streamplot as mstream
from ..._preprocess import _preprocess_data
from ._helpers import convert_units


class FieldsMixin:
    """`quiver`, `quiverkey`, `barbs` and `streamplot`."""

    def _quiver_units(self, args, kwargs):
        if len(args) > 3:
            x, y = args[0:2]
            x, y = convert_units(self, "x", x), convert_units(self, "y", y)
            return (x, y) + args[2:]
        return args

    @_preprocess_data()
    def quiver(self, *args, **kwargs):
        """
        Plot a 2D field of arrows.

        Call signature::

            quiver([X, Y], U, V, [C], /, **kwargs)

        Returns
        -------
        `~matplotlib.quiver.Quiver`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        # Make sure units are handled for x and y values
        args = self._quiver_units(args, kwargs)
        return mquiver.Quiver(self, *args, **kwargs)

    def quiverkey(self, Q, X, Y, U, label, **kwargs):
        """
        Add a key to a quiver plot.

        Returns
        -------
        `~matplotlib.quiver.QuiverKey`
        """
        return mquiver.QuiverKey(Q, X, Y, U, label, **kwargs)

    @_preprocess_data()
    def barbs(self, *args, **kwargs):
        """
        Plot a 2D field of wind barbs.

        Call signature::

            barbs([X, Y], U, V, [C], /, **kwargs)

        Returns
        -------
        barbs : `~matplotlib.quiver.Barbs`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        # Make sure units are handled for x and y values
        args = self._quiver_units(args, kwargs)
        return mquiver.Barbs(self, *args, **kwargs)

    streamplot = _preprocess_data(
        replace_names=["x", "y", "u", "v", "start_points"])(mstream.streamplot)
