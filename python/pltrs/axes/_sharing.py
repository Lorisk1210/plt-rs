"""
The groups of axes sharing an axis as matplotlib's `.cbook.Grouper` gives
them (``Axes._shared_axes[name]``, which ``get_shared_x_axes()`` views):
pltrs (Rust) keeps the groups of each figure, and `SharedGroups` reads
them with the Grouper's protocol (``joined``, ``get_siblings``,
membership and iteration over every group of the live figures, as
matplotlib's class-wide Grouper holds the axes of every figure).
"""

import itertools
import weakref

#: The root figures alive, each with the order it was made in.
_FIGURES = weakref.WeakKeyDictionary()
_made = itertools.count()


def track(figure):
    """Count the root *figure* among the figures whose axes the groups
    iterate over."""
    _FIGURES[figure] = next(_made)


def _all_axes():
    """The axes of the live figures, the figures in the order they were
    made, each figure's axes (and their insets) in its order."""
    def with_children(axes):
        for ax in axes:
            yield ax
            yield from with_children(ax.child_axes)
    for figure in sorted(list(_FIGURES), key=_FIGURES.get):
        yield from with_children(figure.axes)


class SharedGroups:
    """
    matplotlib's `.cbook.Grouper` protocol over the groups of axes sharing
    one axis; a subclass reads the group of an axes from pltrs (Rust)
    (``_siblings(a)``, *a* included).
    """

    def __contains__(self, item):
        return len(self._group_of(item)) > 1

    def __iter__(self):
        """Iterate over each group of axes sharing the axis, as a list."""
        seen = set()
        for ax in _all_axes():
            if id(ax) in seen:
                continue
            group = self._group_of(ax)
            seen.update(map(id, group))
            if len(group) > 1:
                yield group

    def _group_of(self, a):
        """The axes sharing the axis with *a*, *a* included (only *a* for
        an object that is no axes in a figure)."""
        if getattr(a, "_native", None) is None or a.figure is None:
            return [a]
        return self._siblings(a)

    def joined(self, a, b):
        """Return whether *a* and *b* are members of the same set."""
        return a.figure is b.figure and any(x is b for x in self._group_of(a))

    def get_siblings(self, a, *, include_self=True):
        """
        Return all the items joined with *a*.

        *a* is included in the list if *include_self* is True.
        """
        siblings = self._group_of(a)
        return siblings if include_self else [x for x in siblings if x is not a]
