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

Last, the hook places the axis labels given position setters of the
program's (`follow_label`) where the draw will, through those setters.
The child Axes of an Axes are those
of the list the program assigned it, as the list is then
(`follow_child_axes`).

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
from .._python_drawing import drawing_renderer

#: What locators of the program's place, per root figure, in the order
#: they were followed: the order matplotlib's draw places the Axes in (its
#: figure's), whose locators may fail.
_followed = weakref.WeakKeyDictionary()

#: The Axes whose class of the program's autoscales them, per root figure.
_autoscaled = weakref.WeakKeyDictionary()

#: The axis labels (their `._axis_artists.AxisLabel`) given position setters
#: of the program's, per root figure.
_labels = weakref.WeakKeyDictionary()


def follow(owner, figure):
    """
    Have the root *figure* call ``owner._locate_by_program(renderer)``
    before every layout of a draw, while *owner* (an Axes or a divider)
    lives, until `forget`.
    """
    _owners(figure)[owner] = None


def follow_autoscale(ax, figure):
    """Have the root *figure* bring the view limits of *ax*, whose class
    overrides ``autoscale_view``, up to date through it before every
    layout of a draw, while *ax* lives."""
    _owners(figure)
    _autoscaled.setdefault(figure, weakref.WeakSet()).add(ax)


#: The Axes whose child Axes are a list the program assigned, per root
#: figure.
_child_lists = weakref.WeakKeyDictionary()


def follow_child_axes(ax, figure):
    """Have the root *figure* draw the child Axes of *ax* as the list the
    program assigned it (``ax.child_axes = [...]``) is before every layout
    of a draw, changes made in place included, while *ax* lives."""
    _owners(figure)
    _child_lists.setdefault(figure, weakref.WeakSet()).add(ax)


def follow_label(label, figure):
    """Have the root *figure* place the axis label *label* (an
    `._axis_artists.AxisLabel`, whose text the program gave a ``set_x``,
    ``set_y`` or ``set_position`` of its own) through its setters before
    every layout of a draw, while it lives."""
    _owners(figure)
    _labels.setdefault(figure, weakref.WeakSet()).add(label)


def follow_engine(figure):
    """Have the root *figure* run its layout engine of the program's before
    every layout of a draw."""
    _owners(figure)


def _owners(figure):
    """What the root *figure* places by locators of the program's, its
    layout hook installed."""
    owners = _followed.get(figure)
    if owners is None:
        owners = _followed[figure] = weakref.WeakKeyDictionary()
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


def place_to_measure(ax, renderer):
    """Place *ax* where its locator of the program's says now, as
    matplotlib's ``Axes.get_tightbbox`` calls the locator before it
    measures the Axes (with the figure's renderer when *renderer* is
    None); nothing for an Axes placed otherwise."""
    figure = ax.get_figure(root=True)
    if figure is not None and ax in _followed.get(figure, ()):
        ax._locate_by_program(figure._get_renderer() if renderer is None else renderer)


def forget(owner, figure):
    """Stop placing *owner* by a locator of the program's for the root
    *figure*."""
    if figure is not None and figure in _followed:
        _followed[figure].pop(owner, None)


def _first_layout_of_print(figure):
    """Whether this layout of *figure* is the first of the print running,
    or no print runs: matplotlib lays a figure printed with a layout engine
    or cut to its tight box out in the draw before the print, and prints it
    with the engine turned off."""
    options = getattr(figure.canvas, "_print_options", None)
    if not options or not options.get("predrawn"):
        return True
    first = not options.get("engine_ran")
    options["engine_ran"] = True
    return first


def _locate(figure):
    """Give the Axes with assigned child lists those children, autoscale
    the Axes of classes of the program's that autoscale themselves, run
    the layout engine of the program's of *figure*, then place everything
    it follows, with its renderer."""
    for ax in list(_child_lists.get(figure, ())):
        ax._follow_child_axes()
    for ax in list(_autoscaled.get(figure, ())):
        ax._unstale_viewLim()
    engine = figure.get_layout_engine()
    if figure.axes and engine is not None and (
            engine._executes_in_python() or engine._lays_out_kept_grids(figure)) and (
            _first_layout_of_print(figure)):
        try:
            engine.execute(figure)
        except ValueError:
            # As matplotlib's draw, which leaves the layout as it is.
            pass
    owners = list(_followed.get(figure, ()))
    if owners:
        renderer = drawing_renderer(figure)
        for owner in owners:
            owner._locate_by_program(renderer)
    labels = list(_labels.get(figure, ()))
    if labels:
        renderer = drawing_renderer(figure)
        for label in labels:
            label.place_by_program(renderer)
