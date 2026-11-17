"""
The groups of axes sharing an axis as matplotlib's `.cbook.Grouper` gives
them (``Axes._shared_axes[name]``, which ``get_shared_x_axes()`` views):
pltrs (Rust) keeps the groups of each figure, and `SharedGroups` reads
them with the Grouper's protocol (``joined``, ``get_siblings``,
membership and iteration over every group of the live figures, as
matplotlib's class-wide Grouper holds the axes of every figure).

Axes of different figures share too (``ax2.sharex(ax1)``): pltrs (Rust)
links them (``_native.axes_base.shared_across``), and the handles it
gives of the Axes of another figure become the objects of that figure
(`_peer`). The links do not pickle with a figure's native state, so an
Axes pickles the Axes of other figures it shares with (`with_peers`),
which brings their figures along, as matplotlib's pickled Axes do, and
joins them again when it is read back (`rejoin`).
"""

import itertools
import weakref

from .. import _handles, _own_state, colors as _colors
from .._native import archive as _archive, axes_base as _axes_base

#: The root figures alive, each with the order it was made in.
_FIGURES = weakref.WeakKeyDictionary()
_made = itertools.count()

#: The root figures alive, by the key of their native figure.
_BY_KEY = weakref.WeakValueDictionary()

#: The axes pltrs (Rust) links across figures.
_LINKED = ("x", "y")


def track(figure):
    """Count the root *figure* among the figures whose axes the groups
    iterate over."""
    _FIGURES[figure] = next(_made)
    _BY_KEY[_archive.figure_key(figure._native)] = figure
    if _colors._luts_read:
        # The colormaps whose table a program read are asked by every
        # figure, this new one too (`..colors._LutWatch`).
        for watch in list(_colors._luts_read.values()):
            _own_state.follow(watch, figure)


def _peer(native):
    """The Axes object, in its own figure, of the handle *native* of an
    Axes of another figure; None once that figure is gone."""
    figure = _BY_KEY.get(_archive.figure_key(native))
    return None if figure is None else _handles.wrap_any(native, figure)


def across(a, name):
    """The Axes of other figures sharing axis *name* with *a*."""
    if name not in _LINKED:
        return []
    peers = map(_peer, _axes_base.shared_across(a._native, name))
    return [peer for peer in peers if peer is not None]


def shared_from(a, name):
    """The Axes whose axis *name* *a* shares (matplotlib's ``_sharex``,
    ``_sharey``): one of its figure, or of another."""
    own = a._shared_from(name)
    if own is not None or name not in _LINKED:
        return own
    native = _axes_base.shared_from_across(a._native, name)
    return None if native is None else _peer(native)


def with_peers(a, state):
    """*state*, the pickled state of the Axes *a*, with the Axes of other
    figures it shares axes with, by axis: ``(peers, the one it took the
    axis of)``."""
    peers = {name: (found, shared_from(a, name))
             for name in _LINKED if (found := across(a, name))}
    if peers:
        state["_shared_across"] = peers
    return state


def rejoin(a):
    """Join the Axes *a*, read back from a pickle, with the Axes of other
    figures it shared axes with (`with_peers`): those read back already;
    the others join it when their figure is."""
    for name, (peers, followed) in a.__dict__.pop("_shared_across", {}).items():
        for peer in peers:
            if getattr(peer, "_native", None) is not None:
                _axes_base.join_across(a._native, name, peer._native,
                                       peer is followed)


def _all_axes():
    """The axes of the live figures, the figures in the order they were
    made, each figure's axes (and their insets) in its order."""
    def with_children(axes):
        for ax in axes:
            yield ax
            yield from with_children(ax._child_axes_now())
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
        return [*self._siblings(a), *across(a, getattr(self, "_name", None))]

    def joined(self, a, b):
        """Return whether *a* and *b* are members of the same set."""
        return any(x is b for x in self._group_of(a))

    def get_siblings(self, a, *, include_self=True):
        """
        Return all the items joined with *a*.

        *a* is included in the list if *include_self* is True.
        """
        siblings = self._group_of(a)
        return siblings if include_self else [x for x in siblings if x is not a]
