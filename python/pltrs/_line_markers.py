"""
A line's marker as matplotlib's ``Line2D._marker``: the `.MarkerStyle` the
line draws its markers with.

pltrs (Rust) keeps a line's marker, and `.Line2D.get_marker` describes it.
matplotlib's own tick classes and those of libraries turn a tick's mark by
replacing the transform of that object (``line._marker._transform = t``,
as polar and ternary ticks do in ``update_position``). This module's
`MarkerStyle` is the style of a line as it is when first asked for, and
hands the line a transform set on it: the marker's own shape through the
new transform, which is the marker with a user transform undoing its
standard one first. It has the name of the class it derives from, as
``type(line._marker)`` is matplotlib's own; pickles find it in this
module under that name.
"""

from . import _mirrors, markers
from .transforms import Affine2D


class MarkerStyle(markers.MarkerStyle):
    """The `.MarkerStyle` of *line*, whose ``_transform`` the line follows."""

    def __init__(self, line):
        # With the user transform, cap and join styles of a style given, or
        # those pltrs (Rust) gave the line (a legend handle copies them).
        given = line.__dict__.get("_given_marker")
        if isinstance(given, markers.MarkerStyle):
            styled = (given.get_user_transform(), given._user_capstyle, given._user_joinstyle)
        else:
            matrix = line._get("marker_transform")
            styled = (None if matrix is None else Affine2D(matrix),
                      line._get("marker_capstyle"), line._get("marker_joinstyle"))
        super().__init__(line.get_marker(), line.get_fillstyle(), *styled)
        # (Held as the line holds the style: both pickle with the figure.)
        self.__dict__["_line"] = line

    def __setattr__(self, name, value):
        super().__setattr__(name, value)
        if name == "_transform" and "_line" in self.__dict__:
            self._hand_over()

    def _hand_over(self):
        """Have the line draw the marker through the transform set on it."""
        line = self._line
        marker, fillstyle = self.get_marker(), self.get_fillstyle()
        standard = markers.MarkerStyle(marker, fillstyle)._transform
        # ``standard + user`` is the transform set (and what the style was
        # made with, if anything).
        user = standard.inverted() + self._transform
        if self._user_transform is not None:
            user = user + self._user_transform
        line._set_native(
            marker=markers.MarkerStyle(marker, fillstyle, Affine2D(user.get_matrix()),
                               self._user_capstyle, self._user_joinstyle),
            fillstyle=fillstyle)
        mirror = line.__dict__.get("_mirror")
        if mirror is not None:
            # The mark of a tick, which its axis draws, turns with it.
            _mirrors.push(mirror, line)
