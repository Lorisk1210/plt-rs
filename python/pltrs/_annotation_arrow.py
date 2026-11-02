"""
The arrow of an annotation as a patch (matplotlib's
``Annotation.arrow_patch``).

pltrs (Rust) draws an annotation's arrow as part of the annotation, placed
from the text at every draw. ``arrow_patch`` is a *mirror*, as the ticks of
an axis are (`._axis_artists`): a loose `.FancyArrowPatch` filled from the
arrow's look (``_native.annotation.arrow_patch``) whose setters give the
arrow their changes once they ran (``restyle_arrow``), and whose extent is
where the annotation draws the arrow.
"""

from . import _mirrors
from ._native import annotation as _annotation
from .patches import FancyArrowPatch
from .transforms import Bbox


class _AnnotationArrow(_mirrors.Mirroring):
    """The arrow of *annotation*, which a `.FancyArrowPatch` stands for."""

    def __init__(self, annotation):
        self.annotation = annotation

    def push(self, artist):
        _annotation.restyle_arrow(self.annotation._native, artist._native)

    def window_extent(self, artist, renderer=None, dpi=None):
        """The display box of the arrow as the annotation draws it."""
        extents = _annotation.arrow_extent(self.annotation._native)
        return Bbox.null() if extents is None else Bbox.from_extents(*extents)


def mirror(annotation):
    """The patch of the arrow of *annotation*, the same object while the
    arrow stays; None without an arrow."""
    native = _annotation.arrow_patch(annotation._native)
    if native is None:
        annotation.__dict__.pop("_arrow_mirror", None)
        return None
    patch = annotation.__dict__.get("_arrow_mirror")
    if patch is None:
        patch = annotation._arrow_mirror = _mirrors.attach(
            FancyArrowPatch._from_native(native, None), _AnnotationArrow(annotation))
    else:
        patch._native = native
    return patch
