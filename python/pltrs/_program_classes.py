"""
Telling the program's classes from this package's.

pltrs (Rust) does the work of the package's classes itself; where a class
of the program's overrides a method (an artist's ``draw`` or
``get_transform``, a path effect's ``draw_path``), pltrs calls it back
instead.
"""

import functools
from numbers import Real
import os
import sys
import weakref

from ._native import artist as _artist, caches as _caches

#: Where the package's modules are.
_PACKAGE_DIR = os.path.dirname(os.path.realpath(__file__))


def set_artist_props(artist, had_transform, axes=None):
    """
    Set on *artist*, just added to a figure or to *axes*, what matplotlib's
    ``add_*`` methods set on an artist they add (``_set_artist_props``):
    its figure, its transform unless it *had_transform*, its axes. pltrs
    (Rust) set them natively; this calls the methods a class of the
    program's overrides, which may do more (an artist holding another
    sets them on it too).
    """
    from .axes._base import _AxesBase
    cls = type(artist)
    # matplotlib's ``add_axes`` joins an Axes without setting its figure,
    # which an Axes' `set_figure` would make its boxes and transforms anew.
    if takes_from_program(cls, "set_figure") and not isinstance(artist, _AxesBase):
        artist.set_figure(artist.get_figure(root=False))
    if not had_transform and takes_from_program(cls, "set_transform"):
        artist.set_transform(artist.get_transform())
    if axes is not None and takes_from_program(cls, "axes"):
        artist.axes = axes


def take_class_defaults(artist):
    """Give *artist* the z-order its class declares when the class is the
    program's (``class RibbonBoxImage(AxesImage): zorder = 1``): matplotlib
    reads ``zorder`` from the class until it is set; and set its axes to
    None through the ``axes`` property of such a class, as matplotlib's
    ``Artist.__init__`` does."""
    cls = type(artist)
    zorder = getattr(cls, "zorder", None)
    if isinstance(zorder, Real) and takes_from_program(cls, "zorder"):
        _artist.set_zorder(artist._native, float(zorder))
    if takes_from_program(cls, "axes"):
        try:
            artist.axes = None
        except AttributeError:
            # A read-only ``axes`` property.
            pass


@functools.cache
def _outside_package(path):
    """Whether the source file *path* lies outside this package."""
    return not os.path.realpath(path).startswith(_PACKAGE_DIR + os.sep)


def class_cache(function):
    """
    `functools.cache` for a *function* of a class and at most one hashable
    argument more, holding the class weakly: a class of the program's made
    for a figure (defined in a function, made by ``type()``) goes once the
    program forgets it, with what its methods refer to, rather than living
    as long as the cache. A hit runs no Python code (the native lookup of
    `_native.caches`, bound in a `functools.partial`); a miss computes,
    stores, and has the class' entries forgotten when the class goes.
    """
    entries = {}
    owners = set()

    def forget(owner):
        owners.discard(owner)
        for key in [key for key in entries if key[0] == owner]:
            del entries[key]

    def miss(cls, *args):
        value = entries[(id(cls), *args)] = function(cls, *args)
        if id(cls) not in owners:
            owners.add(id(cls))
            # The entries go before the class' id can be another's.
            weakref.finalize(cls, forget, id(cls))
        return value

    lookup = _caches.get1 if function.__code__.co_argcount == 1 else _caches.get2
    cached = functools.partial(lookup, entries, miss)
    cached.__wrapped__ = function
    cached.__doc__ = function.__doc__
    return cached


@class_cache
def takes_from_program(cls, name):
    """Whether the class *cls* takes its attribute *name* from a class of
    the program's rather than one of this package (False when it has
    none)."""
    for klass in cls.__mro__:
        if name in vars(klass):
            path = getattr(sys.modules.get(klass.__module__), "__file__", None)
            return path is None or _outside_package(path)
    return False


class Held:
    """
    What pltrs (Rust) holds for an object of the program's handed to it
    for a figure (a scale, a transform an artist is given, a path effect):
    the object, weakly, while the figure's Python object holds it
    (`held`). pltrs (Rust)'s references are no part of the graph Python's
    collector sees, so an object of the program's that refers back to its
    figure (a scale keeping its axis, a transform keeping its Axes) would
    keep the figure alive for good if pltrs (Rust) held it; the figure's
    own reference is seen. A pickle holds the object itself (`held`
    objects read back from one are held by pltrs (Rust) alone).
    """

    __slots__ = ("_target", "__weakref__")

    def __init__(self, target):
        self._target = weakref.ref(target)

    def __reduce__(self):
        return (_itself, (self.target(),))

    def target(self):
        """The object; `ReferenceError` once it is gone (its figure gone)."""
        target = self._target()
        if target is None:
            raise ReferenceError("the object of the program's pltrs held is gone")
        return target


def _itself(target):
    return target


def held(target, owner):
    """*target* as pltrs (Rust) holds it for *owner*, which holds it from
    then on (``_held_for_native``): a root figure, which keeps every object
    handed over for it, as pltrs (Rust) may share it among its artists and
    axes, or an artist in no figure yet. A `Held`, or *target* itself
    without an owner or where it takes no weak reference."""
    if owner is None:
        return target
    try:
        proxy = Held(target)
    except TypeError:
        return target
    vars(owner).setdefault("_held_for_native", {})[id(target)] = target
    return proxy


def unheld(value):
    """The object *value* (`held`) stands for."""
    return value.target() if isinstance(value, Held) else value


def from_program(function):
    """Whether *function*, a function or a method as a class holds it
    (``RendererBase.new_gc = types.MethodType(...)``), is defined outside
    this package (a callable of another kind always is)."""
    code = getattr(getattr(function, "__func__", function), "__code__", None)
    return code is None or _outside_package(code.co_filename)
