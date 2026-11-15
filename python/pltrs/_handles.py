"""
One Python object per pltrs (Rust) artist.

A native function returning an artist returns a new native handle each
time (``_native.Native``), but matplotlib code expects the same object
back: ``ax.lines[0] is line``, attributes set on an artist stay, and a
subclass stays a subclass. So each figure keeps a registry of the objects
of its artists, by artist id, and native handles become objects through
`wrap`.

The registry lives on the root `.Figure` (``figure._artists``, a
`Registry`), and every artist object refers to its figure, so an artist's
object lives as long as its figure and the figure as long as any of its
artists' objects. The objects of artists removed from the figure (by
``remove``, a clear, a legend or colorbar replaced) go only as long as the
program holds them, as matplotlib's figure holds no removed artist: the
registry moves them to its weak part now and then (`Registry.sweep`), the
object of a removed artist holding those of the removed artists it owns
(an Axes removed, its lines). Once no object reaches a removed artist,
pltrs (Rust) lets it go too (it keeps removed artists while a handle
reaches them).
"""

import importlib
import weakref

from ._native import artist as _artist


class Registry(dict):
    """
    A root figure's objects of its artists, by artist id: those of the
    artists in the figure, and (`removed`, weakly) those of removed
    artists. `wrap` and `adopt` sweep the objects of removed artists into
    the weak part whenever the registry has doubled since its last sweep
    (those out of the figure at two sweeps running: an artist made for
    the figure is out of it until it is added).
    """

    #: The fewest objects a registry holds before its first sweep.
    FIRST_SWEEP = 64

    def __init__(self, *args):
        super().__init__(*args)
        #: Weak references to the objects of removed artists, by key (a
        #: plain dict: `wrap` reads it without a Python call; dead ones go
        #: at the next sweep).
        self.removed = {}
        #: How many objects the registry holds at its next sweep.
        self.sweep_at = self.FIRST_SWEEP
        #: The keys of the artists found out of the figure at the last
        #: sweep, which the next one moves if they still are (an artist
        #: made for the figure is out of it until it is added).
        self.out = set()

    def __reduce__(self):
        # What a pickle or a copy of the figure keeps: the figure's own.
        return (type(self), (dict(self),))

    def find(self, key):
        """The object of artist *key*, removed or not; None without one."""
        obj = self.get(key)
        if obj is None:
            reference = self.removed.get(key)
            obj = None if reference is None else reference()
        return obj

    def sweep(self, native, at_once=False):
        """Moves the objects of the removed artists into the weak part,
        each held by the object of the removed artist owning it, if it has
        one (*native*: a handle of the figure): those out of the figure
        at the last sweep too, unless *at_once* (the figure was cleared:
        all of them)."""
        keys = list(self)
        roots = _artist.removed_roots(native, keys)
        out = {key: root for key, root in zip(keys, roots) if root is not None}
        moved = {key: (root, self.pop(key)) for key, root in out.items()
                 if at_once or key in self.out}
        self.out = set(out) - set(moved)
        self.removed = {key: reference for key, reference in self.removed.items()
                        if reference() is not None}
        for key, (root, obj) in moved.items():
            self.removed[key] = weakref.ref(obj)
            owner = self.find(root) if root != key else None
            if owner is not None:
                owner.__dict__.setdefault("_removed_tree", {})[key] = obj
        self.sweep_at = max(self.FIRST_SWEEP, 2 * (len(self) - len(self.out)))


def wrap(native, cls, figure):
    """
    The object of *figure* for the artist the handle *native* reaches:
    the one made before, or a new *cls* around it.

    ``None`` stays ``None`` (an optional artist that is absent).
    """
    if native is None:
        return None
    figure = figure._figure  # The root keeps the registry.
    key = _artist.id(native)
    registry = figure._artists
    obj = registry.get(key)
    if obj is None:
        # (Inline, not through `Registry.find`: the plain path makes no
        # Python call more.)
        reference = getattr(registry, "removed", {}).get(key)
        obj = None if reference is None else reference()
        if obj is None:
            obj = cls._from_native(native, figure)
        registry[key] = obj
        if len(registry) >= getattr(registry, "sweep_at", len(registry) + 1):
            registry.sweep(native)
    return obj


def wrap_all(natives, cls, figure):
    """`wrap` for each handle of *natives*, as a list."""
    return [wrap(native, cls, figure) for native in natives]


#: The module of the package defining each class `_native.artist.class_name`
#: names (`artist` for the others).
_CLASS_MODULES = {
    "Line2D": "lines",
    "Text": "text", "Annotation": "text",
    "AxesImage": "image", "PcolorImage": "image", "BboxImage": "image",
    "Collection": "collections", "PatchCollection": "collections",
    "PathCollection": "collections", "PolyCollection": "collections",
    "LineCollection": "collections", "QuadMesh": "collections",
    "EventCollection": "collections", "EllipseCollection": "collections",
    "TriMesh": "collections",
    **dict.fromkeys(
        ["Poly3DCollection", "Line3DCollection", "Path3DCollection",
         "Patch3DCollection"], "mpl_toolkits.mplot3d.art3d"),
    "Quiver": "quiver", "QuiverKey": "quiver",
    "QuadContourSet": "contour",
    "Legend": "legend",
    "Table": "table",
    "Spine": "spines",
    "Axes": "axes",
    "InsetIndicator": "inset",
    **dict.fromkeys(
        ["Rectangle", "Ellipse", "Circle", "Polygon", "Wedge", "PathPatch",
         "RegularPolygon", "CirclePolygon", "StepPatch", "FancyArrow", "Arrow",
         "Arc", "Annulus", "FancyBboxPatch", "FancyArrowPatch",
         "ConnectionPatch"], "patches"),
}


def wrap_any(native, figure):
    """
    `wrap` for a handle of an artist of any kind: the object made before,
    or a new object of matplotlib's class for the artist's kind.
    """
    if native is None:
        return None
    obj = figure._artists.get(_artist.id(native))
    if obj is not None:
        return obj
    name = _artist.class_name(native)
    module = importlib.import_module(
        f".{_CLASS_MODULES.get(name, 'artist')}", __package__)
    return wrap(native, getattr(module, name), figure)


def wrap_patch(native, figure):
    """`wrap` for a handle of a patch: the object made before, or a new
    object of matplotlib's class for the patch's shape (`.Patch` for a
    shape of no class of its own)."""
    from . import patches
    name = _artist.class_name(native)
    return wrap(native, getattr(patches, name, patches.Patch), figure)


def adopt(artist, native, figure):
    """
    Make *artist*, a loose artist or one removed from *figure*, the object
    of the artist the handle *native* reaches in *figure*, once an axes or
    the figure adopted it (``_native.artist.adopt_into_axes``); return it.
    """
    from ._computed_transforms import track
    figure = figure._figure  # The root keeps the registry.
    artist._native = native
    artist._figure = figure
    artist._recouple()
    registry = figure._artists
    registry[_artist.id(native)] = artist
    if len(registry) >= getattr(registry, "sweep_at", len(registry) + 1):
        registry.sweep(native)
    # What it owned is in the figure again with it.
    for key, owned in artist.__dict__.pop("_removed_tree", {}).items():
        figure._artists[key] = owned
    track(artist, figure)
    # What waited for the artist to be in a figure (an arrow clipped to it).
    for callback in artist.__dict__.pop("_on_adopt", ()):
        callback()
    return artist


def forget(artist):
    """Drop *artist* from its figure's registry (once it is removed): its
    object stays the artist's only while the program holds it."""
    registry = artist._figure._artists
    obj = registry.pop(_artist.id(artist._native), None)
    if obj is not None and isinstance(registry, Registry):
        registry.removed[_artist.id(artist._native)] = weakref.ref(obj)


def forget_removed(figure):
    """Sweep the objects of the artists removed from *figure* (its root's)
    into the weak part of its registry (`Registry.sweep`) at once."""
    registry = figure._figure._artists
    if isinstance(registry, Registry):
        registry.sweep(figure._native, at_once=True)
