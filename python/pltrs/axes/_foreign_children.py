"""
What an Axes lists among its children that its figure does not hold:
objects that are no artists (matplotlib's ``add_artist`` of a container or
a colorbar), artists pltrs (Rust) holds elsewhere (its ``add_artist`` of
an artist of another figure, of one removed from another figure, of a part
of the Axes or of an artist of the figure itself), and collections of
another figure (its ``add_collection``).

matplotlib's `add_artist` sets the object's Axes and lists it among the
Axes' children before it fails to give it the figure (the object has no
``set_figure``), and keeps it there: the Axes lists it among its children,
removing it takes it out of them again, and every draw of its figure fails
on it, asking it whether it is animated (when saving, for its z-order).
An artist of another figure stays listed the same way, and the Axes draws
it there, through the transforms of its own figure; so does a collection
of another figure ``add_collection`` was given. An artist removed from
another figure moves into the Axes and its figure. A part of the Axes (a
spine, a title) and an artist of the figure itself are listed once more,
and drawn twice, there and as they were.
pltrs (Rust) holds the artists of one figure only, each in one place, so
the Axes keeps such objects itself (`add`, `of`), after its other
children, and the draw hook of its figure fails on them as matplotlib's
draw does (`check_drawable`); an artist is drawn by a stand-in in the Axes
(`relist`, `add_drawn`, `_Drawing`), whose ``draw`` is the artist's.
"""

from operator import attrgetter

from .. import _mirrors, _python_drawing
from .._native import artist as _artist, mplot3d as _mplot3d
from ..artist import Artist


class _Drawing(Artist):
    """
    The stand-in in an Axes for an artist pltrs (Rust) holds elsewhere
    that the Axes draws (`relist`, `add_drawn`): drawn at the artist's
    z-order through the artist's own ``draw``, with the renderer drawing
    the Axes' figure.
    """

    def __init__(self, drawn, parent):
        super().__init__()
        self._drawn = drawn
        self._parent = parent
        self.set_zorder(drawn.get_zorder())

    def draw(self, renderer):
        # A part pltrs (Rust) draws with its owner (a tick's label) is
        # drawn here as the artist matplotlib has.
        alone = _mirrors.ask(self._drawn, "alone")
        (self._drawn if alone is _mirrors.OWN else alone).draw(renderer)

    def _take_out(self, drawn):
        """The removal method of the artist drawn: it is no longer listed
        nor drawn here, and one moved here from another figure leaves the
        Axes' figure again."""
        self._parent.__dict__["_drawings"].remove(self)
        drawn.__dict__.pop("_listing", None)
        drawn.__dict__.pop("_listed_in", None)
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


def relisted(artist):
    """Whether *artist* is listed by a stand-in (`relist`, `add_drawn`),
    whose removal method it has."""
    return "_listing" in artist.__dict__


def drawn(ax, cls):
    """The artists of class *cls* pltrs (Rust) holds elsewhere that *ax*
    draws (`relist`, `add_drawn`), in the order they were added."""
    return [drawing._drawn for drawing in ax.__dict__.get("_drawings", ())
            if isinstance(drawing._drawn, cls)]


def held_elsewhere(ax, a):
    """
    Whether *a*, an artist given to *ax*'s `add_artist`, is one pltrs
    (Rust) holds elsewhere (see `relist`): an artist of another
    (sub)figure, one removed from another figure, a part of *ax* (its
    spines and titles), an artist the figure itself holds, a box of a tree
    of offset boxes, or one pltrs (Rust) holds in no Axes (an Axes,
    ``_held_in_axes``).
    """
    figure = a.get_figure(root=False)
    if figure is not None and figure is not ax.get_figure(root=False):
        return True
    if "_tree_owner" in a.__dict__:
        return figure is not None
    if a._figure is None:
        return False
    if not getattr(a, "_held_in_axes", True):
        return True
    if _artist.is_part(a._native):
        owner = _artist.axes(a._native)
        return (a._figure is ax._figure and owner is not None
                and _artist.id(owner) == _artist.id(ax._native))
    if figure is None:
        return a._figure is not ax._figure
    return _artist.axes(a._native) is None and _artist.is_attached(a._native)


def relist(ax, a):
    """
    Add *a*, an artist pltrs (Rust) holds elsewhere (`held_elsewhere`), to
    *ax* as matplotlib's `add_artist` adds it: with *ax* as its Axes (an
    artist of another Axes fails), listed among its children and drawn
    there by a stand-in. An artist of another figure stays there once
    giving it *ax*'s figure fails; one removed from another figure moves
    into *ax*'s, which it then reads as its own (``_listed_in``), and is
    measured and drawn in its own geometry still. Return *a*.
    """
    a.axes = ax
    _list_drawn(ax, a)
    if a._figure is ax._figure:
        # A part of the Axes decoupled since it was last listed is back.
        a._recouple()
    check_figure(ax, a)
    if a.get_figure(root=False) is None:
        a._listed_in = ax
        a.pchanged()
    elif a.get_clip_path() is None:
        a.set_clip_path(ax.patch)
    ax.stale = True
    return a


def add_held(ax, artist):
    """
    Begin adding *artist* to *ax* as matplotlib's add methods but
    `add_artist` and ``add_collection`` begin: they fail on an artist of
    another (sub)figure, giving it *ax*'s figure first, and list one pltrs
    (Rust) holds elsewhere in the figure once more (`relist`), which this
    then lists. Return whether it did.
    """
    check_figure(ax, artist)
    if held_elsewhere(ax, artist):
        relist(ax, artist)
        return True
    return False


def check_figure(ax, artist):
    """Fail as matplotlib fails to give *artist*, of another (sub)figure,
    *ax*'s figure."""
    figure = artist.get_figure(root=False)
    if figure is not None and figure is not ax.get_figure(root=False):
        raise RuntimeError("Can not put single artist in more than one figure")


def _list_drawn(parent, artist):
    """List *artist* among the children of *parent*, an Axes or a
    (sub)figure, drawn there by a stand-in, and taken out again by its
    removal."""
    drawing = _Drawing(artist, parent)
    parent.add_artist(drawing)
    _python_drawing.draw_through_python(drawing, False)
    parent.__dict__.setdefault("_drawings", []).append(drawing)
    artist._listing = drawing
    artist._remove_method = drawing._take_out


def held_in_axes(figure, artist):
    """Whether *artist*, given to the `.Figure.add_artist` of *figure*,
    of its figure, is an artist an Axes of the figure holds, which
    matplotlib's figure lists once more (`relist_in_figure`)."""
    return (artist._figure is figure._figure and _artist.is_attached(artist._native)
            and _artist.axes(artist._native) is not None)


def relist_in_figure(figure, artist, clip):
    """Add *artist*, which an Axes of *figure* holds (`held_in_axes`), to
    *figure* as matplotlib's `.Figure.add_artist` adds it: listed among
    its artists (and drawn there by a stand-in) once more, and with *clip*
    clipped to the figure's patch unless it has a clip path; return it."""
    _list_drawn(figure, artist)
    if clip and artist.get_clip_path() is None:
        artist.set_clip_path(figure.patch)
    figure.stale = True
    return artist


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
        collection.set_label(f'_child{len(ax._children_now())}')
    _list_drawn(ax, collection)
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
    added to its Axes that are no artists, and on those an Axes' ``spines``
    holds (``ax.spines['top'] = None``), which are among its children:
    each visible Axes asks its children whether they are animated (unless
    the canvas is saving), then sorts them by z-order. A 3-D Axes has
    drawn its panes, gridlines and axes by then: its failure is recorded
    with pltrs (Rust), whose draw raises it there.
    """
    saving = figure.canvas.is_saving()
    for ax in figure.axes:
        foreign = [*of(ax), *(spine for spine in getattr(ax, "spines", {}).values()
                              if not isinstance(spine, Artist))]
        three_d = getattr(ax, "name", None) == "3d"
        failure = None
        if foreign and ax.get_visible():
            try:
                drawn = foreign if saving else [a for a in foreign if not a.get_animated()]
                sorted(drawn, key=attrgetter("zorder"))
            except Exception as exc:
                if not three_d:
                    raise
                failure = (type(exc).__name__, str(exc))
        if three_d and (failure or ax.__dict__.get("_children_failure")):
            ax._children_failure = failure
            _mplot3d.set_children_failure(ax._native, failure)
