"""
Axes placed by locators of the program's: any callable
``locator(ax, renderer)`` returning the `.Bbox` of the axes in figure
fractions (or None for its original position), given to
`~.Axes.set_axes_locator` (or to an axes_grid1 ``Divider.set_locator``,
which places the divided rectangle, or a secondary axis located by a
transform other than its parent's ``transData`` or ``transAxes``).

matplotlib calls such a locator whenever it draws the axes. pltrs (Rust)
cannot call Python while it lays a figure out, so each root figure keeps
what such locators place (`follow`), and its layout hook, which pltrs
(Rust) runs before every layout of a draw
(``_native.insets.set_layout_hook``), has each of them place itself where
its locator says (its ``_locate_by_program(renderer)``) until the next
draw. Before the first draw an axes keeps its original position, as in
matplotlib.
"""

import weakref

from .._native import insets as _insets

#: What locators of the program's place, per root figure.
_followed = weakref.WeakKeyDictionary()


def follow(owner, figure):
    """
    Have the root *figure* call ``owner._locate_by_program(renderer)``
    before every layout of a draw, while *owner* (an Axes or a divider)
    lives, until `forget`.
    """
    owners = _followed.get(figure)
    if owners is None:
        owners = _followed[figure] = weakref.WeakSet()
        figure_ref = weakref.ref(figure)

        def hook():
            alive = figure_ref()
            if alive is not None:
                _locate(alive)

        _insets.set_layout_hook(figure._native, hook)
    owners.add(owner)


def followed(figure):
    """What the root *figure* places by locators of the program's."""
    return list(_followed.get(figure, ()))


def forget(owner, figure):
    """Stop placing *owner* by a locator of the program's for the root
    *figure*."""
    if figure is not None and figure in _followed:
        _followed[figure].discard(owner)


def _locate(figure):
    """Place everything *figure* follows, with its renderer."""
    owners = list(_followed.get(figure, ()))
    if owners:
        renderer = figure._get_renderer()
        for owner in owners:
            owner._locate_by_program(renderer)
