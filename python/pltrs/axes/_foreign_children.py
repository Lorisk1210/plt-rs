"""
What an Axes lists among its children that its figure does not hold:
objects that are no artists (matplotlib's ``add_artist`` of a container or
a colorbar), and collections of another figure (its ``add_collection``).

matplotlib's `add_artist` sets the object's Axes and lists it among the
Axes' children before it fails to give it the figure (the object has no
``set_figure``), and keeps it there: the Axes lists it among its children,
removing it takes it out of them again, and every draw of its figure fails
on it, asking it whether it is animated (when saving, for its z-order).
Its ``add_collection`` lists a collection of another figure among the
Axes' children before giving it the figure fails, and the Axes then draws
it there too, through the transforms of the collection's own figure.
pltrs (Rust) holds the artists of one figure only, so the Axes keeps such
objects itself (`add`, `of`), after its other children, and the draw hook
of its figure fails on them as matplotlib's draw does (`check_drawable`);
a collection of another figure is drawn by a stand-in in the Axes
(`add_drawn`, `_Drawing`), whose ``draw`` is the collection's.
"""

from operator import attrgetter

from .. import _python_drawing
from ..artist import Artist


class _Drawing(Artist):
    """
    The stand-in in an Axes for an artist of another figure the Axes
    draws (`add_drawn`): drawn at the artist's z-order through the
    artist's own ``draw``, with the renderer drawing the Axes' figure.
    """

    def __init__(self, drawn):
        super().__init__()
        self._drawn = drawn
        self.set_zorder(drawn.get_zorder())

    def draw(self, renderer):
        self._drawn.draw(renderer)

    def _take_out(self, drawn):
        """The removal method of the artist drawn: it is no longer listed
        nor drawn here."""
        self.axes.__dict__["_drawings"].remove(self)
        self.remove()


def of(ax):
    """The objects added to *ax* that are no artists, in the order they
    were added."""
    return ax.__dict__.get("_foreign", [])


def listed(child):
    """What an Axes lists for its child *child*: the artist a stand-in
    draws (`add_drawn`), else the child."""
    return child._drawn if isinstance(child, _Drawing) else child


def is_stand_in(child):
    """Whether *child* is the stand-in of an artist of another figure."""
    return isinstance(child, _Drawing)


def drawn(ax):
    """The collections of other figures *ax* draws (`add_drawn`), in the
    order they were added."""
    return [drawing._drawn for drawing in ax.__dict__.get("_drawings", ())]


def add(ax, a):
    """Add *a*, which is no artist, to *ax* as matplotlib's `add_artist`
    adds it, up to where that fails on it; return it."""
    from .._computed_transforms import follow
    a.axes = ax
    foreign = ax.__dict__.setdefault("_foreign", [])
    foreign.append(a)
    a._remove_method = foreign.remove
    follow(ax._figure)
    a.set_figure(ax.get_figure(root=False))
    if not a.is_transform_set():
        a.set_transform(ax.transData)
    if a.get_clip_path() is None:
        a.set_clip_path(ax.patch)
    ax.stale = True
    return a


def add_drawn(ax, collection):
    """
    Add *collection*, of another figure, to *ax* as matplotlib's
    ``add_collection`` adds it, up to where giving it the figure fails:
    labelled for its place among the children if it has no label, listed
    and drawn there.
    """
    if not collection.get_label():
        collection.set_label(f'_child{len(ax._children)}')
    drawing = _Drawing(collection)
    ax.add_artist(drawing)
    _python_drawing.draw_through_python(drawing, False)
    ax.__dict__.setdefault("_drawings", []).append(drawing)
    collection._remove_method = drawing._take_out
    raise RuntimeError("Can not put single artist in more than one figure")


def forget(ax):
    """Forget what *ax* lists that its figure does not hold, as
    matplotlib's clear decouples the Axes' children from it."""
    for a in ax.__dict__.pop("_foreign", ()):
        a._remove_method = None
        a._parent_figure = None
        a.axes = None
    for drawing in ax.__dict__.pop("_drawings", ()):
        drawing._drawn.__dict__.pop("_remove_method", None)
        drawing._drawn._decouple(removed=False)


def check_drawable(figure):
    """
    Fail as matplotlib's draw of the root *figure* fails on the objects
    added to its Axes that are no artists: each visible Axes asks its
    children whether they are animated (unless the canvas is saving), then
    sorts them by z-order.
    """
    saving = figure.canvas.is_saving()
    for ax in figure.axes:
        foreign = of(ax)
        if not foreign or not ax.get_visible():
            continue
        drawn = foreign if saving else [a for a in foreign if not a.get_animated()]
        sorted(drawn, key=attrgetter("zorder"))
