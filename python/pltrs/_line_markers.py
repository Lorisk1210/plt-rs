"""
A line's marker as matplotlib's ``Line2D._marker``: the `.MarkerStyle` the
line draws its markers with.

pltrs (Rust) keeps a line's marker, and `.Line2D.get_marker` describes it.
matplotlib's own tick classes and those of libraries turn a tick's mark by
replacing the transform of that object (``line._marker._transform = t``,
as polar and ternary ticks do in ``update_position``). A `LineMarker` is
the style of a line as it is when first asked for, and hands the line a
transform set on it: the marker's own shape through the new transform,
which is the marker with a user transform undoing its standard one first.
"""

from .markers import MarkerStyle
from .transforms import Affine2D


class LineMarker(MarkerStyle):
    """The `.MarkerStyle` of *line*, whose ``_transform`` the line follows."""

    def __init__(self, line):
        super().__init__(line.get_marker(), line.get_fillstyle())
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
        standard = MarkerStyle(marker, fillstyle)._transform
        # ``standard + user`` is the transform set (and what the style was
        # made with, if anything).
        user = standard.inverted() + self._transform
        if self._user_transform is not None:
            user = user + self._user_transform
        line._set_native(
            marker=MarkerStyle(marker, fillstyle, Affine2D(user.get_matrix()),
                               self._user_capstyle, self._user_joinstyle),
            fillstyle=fillstyle)
