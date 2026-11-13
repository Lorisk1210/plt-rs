"""
The arrow of an annotation as a patch (matplotlib's
``Annotation.arrow_patch`` and ``AnnotationBbox.arrow_patch``).

pltrs (Rust) draws the arrow of an annotation (or of an annotation box) as
part of it, placed from the text (or the box) at every draw.
``arrow_patch`` is a *mirror*, as the ticks of an axis are
(`._axis_artists`): a loose `.FancyArrowPatch` filled from the arrow's
look (``_native.annotation.arrow_patch``,
``_native.offsetbox.annotation_arrow_patch``) whose setters give the arrow
their changes once they ran (``restyle_arrow``), and whose extent is
where the annotation draws the arrow.
"""

import functools

import numpy as np

from . import _mirrors
from ._native import annotation as _annotation, offsetbox as _offsetbox
from .patches import FancyArrowPatch
from .transforms import Bbox


class _AnnotationArrow(_mirrors.Mirroring):
    """The arrow of *owner*, an annotation, which a `.FancyArrowPatch`
    stands for."""

    #: The native function giving the arrow's look as a loose patch.
    native_patch = staticmethod(_annotation.arrow_patch)
    #: The native function giving the arrow the look of a patch.
    native_restyle = staticmethod(_annotation.restyle_arrow)
    #: The native function giving the arrow's display extents as drawn.
    native_extent = staticmethod(_annotation.arrow_extent)
    #: The native function giving the arrow's display outline as drawn.
    native_display_path = staticmethod(_annotation.arrow_display_path)

    def __init__(self, owner):
        self.owner = owner

    def push(self, artist):
        self.native_restyle(self.owner._native, artist._native)

    def window_extent(self, artist, renderer=None, dpi=None):
        """The display box of the arrow as the annotation draws it."""
        extents = self.native_extent(self.owner._native)
        return Bbox.null() if extents is None else Bbox.from_extents(*extents)

    def display_path(self, artist):
        """The outline of the arrow as the annotation draws it (no vertex
        while the annotation is hidden)."""
        arrays = self.native_display_path(self.owner._native)
        return (np.empty((0, 2)), None) if arrays is None else arrays


class AnnotationBboxArrow(_AnnotationArrow):
    """The arrow of *owner*, an annotation box that joined a figure."""

    native_patch = staticmethod(_offsetbox.annotation_arrow_patch)
    native_restyle = staticmethod(_offsetbox.restyle_annotation_arrow)
    native_extent = staticmethod(_offsetbox.annotation_arrow_extent)
    native_display_path = staticmethod(_offsetbox.annotation_arrow_display_path)


def mirror(owner, mirroring=_AnnotationArrow):
    """The patch of the arrow of *owner* (an annotation, or an annotation
    box with *mirroring* `AnnotationBboxArrow`), the same object while the
    arrow stays; None without an arrow."""
    native = mirroring.native_patch(owner._native)
    if native is None:
        owner.__dict__.pop("_arrow_mirror", None)
        return None
    patch = owner.__dict__.get("_arrow_mirror")
    if patch is None:
        patch = owner._arrow_mirror = _mirrors.attach(
            FancyArrowPatch._from_native(native, None), mirroring(owner))
    else:
        patch._native = native
    return patch


def assign(owner, patch, take):
    """
    ``owner.arrow_patch = patch``: while *owner* has ``arrowprops``,
    matplotlib places and draws the patch assigned at its next draw and
    measure (``update_positions``), the arrow taking its look here
    (*take(patch)*), and fails there on None at the first method it calls
    on it (``set_mutation_scale``, ``set_positions``); without, it never
    reads it again.
    """
    from . import _unusable, _values
    _unusable.drop(owner, "arrow_patch")
    _values.take(owner, "arrow_patch")
    if owner.arrowprops is not None and patch is not None:
        take(patch)
        return
    _values.assign(owner, "arrow_patch", patch)
    if owner.arrowprops is not None:
        _unusable.keep(owner, "arrow_patch", patch,
                       functools.partial(_first_call, owner._first_arrow_call))


def _first_call(name, arrow):
    """Fail as matplotlib's ``update_positions`` fails calling *name* on
    the arrow assigned (None)."""
    return getattr(arrow, name)


def check_assigned(owner):
    """Fail as matplotlib's ``update_positions`` does where the arrow of
    *owner* (with ``arrowprops``) was assigned None."""
    from . import _values
    if owner.arrowprops is not None and _values.assigned(owner, "arrow_patch", lambda: owner) is None:
        _first_call(owner._first_arrow_call, None)


def adopt(owner, patch, mirroring):
    """Give the arrow of *owner* the look of *patch*, the arrow matplotlib
    made before pltrs (Rust) did (an annotation box's, until it joins a
    figure), and make *patch* its mirror, the arrow *mirroring* stands
    for, so that the object stays."""
    mirroring.native_restyle(owner._native, patch._native)
    patch._native = mirroring.native_patch(owner._native)
    owner._arrow_mirror = _mirrors.attach(patch, mirroring(owner))
