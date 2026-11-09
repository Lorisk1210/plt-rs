"""
Vector fields: `~.Axes.quiver`, `~.Axes.quiverkey`, `~.Axes.barbs` and
`~.Axes.streamplot`.
"""

from ... import _docstring
from ... import quiver as mquiver, streamplot as mstream
from ..._preprocess import _preprocess_data
from ._helpers import _make_axes_method, convert_units


class FieldsMixin:
    """`quiver`, `quiverkey`, `barbs` and `streamplot`."""

    def _quiver_units(self, args, kwargs):
        if len(args) > 3:
            x, y = args[0:2]
            x, y = convert_units(self, "x", x), convert_units(self, "y", y)
            return (x, y) + args[2:]
        return args

    @_preprocess_data()
    @_docstring.interpd
    def quiver(self, *args, **kwargs):
        """%(quiver_doc)s"""
        # Make sure units are handled for x and y values
        args = self._quiver_units(args, kwargs)
        q = mquiver.Quiver(self, *args, **kwargs)
        self.add_collection(q)
        return q

    @_docstring.copy(mquiver.QuiverKey.__init__)
    def quiverkey(self, Q, X, Y, U, label, **kwargs):
        qk = mquiver.QuiverKey(Q, X, Y, U, label, **kwargs)
        self.add_artist(qk)
        return qk

    @_preprocess_data()
    @_docstring.interpd
    def barbs(self, *args, **kwargs):
        """%(barbs_doc)s"""
        # Make sure units are handled for x and y values
        args = self._quiver_units(args, kwargs)
        b = mquiver.Barbs(self, *args, **kwargs)
        self.add_collection(b)
        return b

    streamplot = _preprocess_data(
        replace_names=["x", "y", "u", "v", "start_points"])(
        _make_axes_method(mstream.streamplot))
