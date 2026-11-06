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

The hook first brings up to date the view limits of the Axes whose
class of the program's autoscales them (`follow_autoscale`: their
``autoscale_view``, as matplotlib's draw does through
``_unstale_viewLim``; cartopy's ``GeoAxes`` keeps the view within its
map), then runs the figure's layout engine of a class of the program's
(`follow_engine`): its ``execute(figure)``, which matplotlib's draw runs
before it draws the Axes (plotnine places its panels so).
"""

import weakref

from .._native import insets as _insets
from .._program_classes import takes_from_program

#: What locators of the program's place, per root figure.
_followed = weakref.WeakKeyDictionary()

#: The Axes whose class of the program's autoscales them, per root figure.
_autoscaled = weakref.WeakKeyDictionary()


def follow(owner, figure):
    """
    Have the root *figure* call ``owner._locate_by_program(renderer)``
    before every layout of a draw, while *owner* (an Axes or a divider)
    lives, until `forget`.
    """
    _owners(figure).add(owner)


def follow_autoscale(ax, figure):
    """Have the root *figure* bring the view limits of *ax*, whose class
    overrides ``autoscale_view``, up to date through it before every
    layout of a draw, while *ax* lives."""
    _owners(figure)
    _autoscaled.setdefault(figure, weakref.WeakSet()).add(ax)


def follow_engine(figure):
    """Have the root *figure* run its layout engine of the program's before
    every layout of a draw."""
    _owners(figure)


def _owners(figure):
    """What the root *figure* places by locators of the program's, its
    layout hook installed."""
    owners = _followed.get(figure)
    if owners is None:
        owners = _followed[figure] = weakref.WeakSet()
        figure_ref = weakref.ref(figure)

        def hook():
            alive = figure_ref()
            if alive is not None:
                _locate(alive)

        _insets.set_layout_hook(figure._native, hook)
    return owners


def followed(figure):
    """What the root *figure* places by locators of the program's."""
    return list(_followed.get(figure, ()))


def forget(owner, figure):
    """Stop placing *owner* by a locator of the program's for the root
    *figure*."""
    if figure is not None and figure in _followed:
        _followed[figure].discard(owner)


def _locate(figure):
    """Autoscale the Axes of classes of the program's that autoscale
    themselves, run the layout engine of the program's of *figure*, then
    place everything it follows, with its renderer."""
    for ax in list(_autoscaled.get(figure, ())):
        ax._unstale_viewLim()
    engine = figure.get_layout_engine()
    if (figure.axes and engine is not None
            and takes_from_program(type(engine), "execute")):
        try:
            engine.execute(figure)
        except ValueError:
            # As matplotlib's draw, which leaves the layout as it is.
            pass
    owners = list(_followed.get(figure, ()))
    if owners:
        renderer = figure._get_renderer()
        for owner in owners:
            owner._locate_by_program(renderer)
