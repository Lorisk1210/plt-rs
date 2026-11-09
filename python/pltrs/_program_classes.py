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

from ._native import artist as _artist

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
def takes_from_program(cls, name):
    """Whether the class *cls* takes its attribute *name* from a class of
    the program's rather than one of this package (False when it has
    none)."""
    for klass in cls.__mro__:
        if name in vars(klass):
            path = getattr(sys.modules.get(klass.__module__), "__file__", None)
            return path is None or not os.path.realpath(path).startswith(
                _PACKAGE_DIR + os.sep)
    return False


def from_program(function):
    """Whether *function*, a function or a method as a class holds it
    (``RendererBase.new_gc = types.MethodType(...)``), is defined outside
    this package (a callable of another kind always is)."""
    code = getattr(getattr(function, "__func__", function), "__code__", None)
    return code is None or not os.path.realpath(code.co_filename).startswith(
        _PACKAGE_DIR + os.sep)
