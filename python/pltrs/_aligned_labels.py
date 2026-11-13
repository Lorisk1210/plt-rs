"""
The groups of Axes whose labels or titles a figure aligns (matplotlib's
``Figure._align_label_groups``: a `.cbook.Grouper` each for the x labels,
the y labels and the titles), as pltrs (Rust) keeps them: programs join
Axes into them themselves (ultraplot aligns the labels of the subplots a
label spans so).
"""

from . import _handles
from ._native import figure_layout as _figure_layout
from .axes._sharing import SharedGroups

#: The texts aligned, by the keys of matplotlib's mapping.
KEYS = ("x", "y", "title")


class AlignedLabels(SharedGroups):
    """The Axes of one (sub)figure whose texts *which* (a key of `KEYS`)
    align."""

    def __init__(self, figure, which):
        self._frame = figure
        self._which = which

    def _group_of(self, a):
        if getattr(a, "_native", None) is None or a.figure is None:
            return [a]
        return _handles.wrap_all(
            _figure_layout.aligned_with(self._frame._native, self._which, a._native),
            type(a), a._figure)

    def join(self, a, *args):
        """
        Join given arguments into the same set.  Accepts one or more arguments.
        """
        for b in args:
            _figure_layout.join_aligned(self._frame._native, self._which, a._native, b._native)


def groups(figure):
    """matplotlib's ``_align_label_groups`` of *figure*."""
    return {key: AlignedLabels(figure, key) for key in KEYS}
