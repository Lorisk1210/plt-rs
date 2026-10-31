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

from ._axis_artists import _pushing
from ._native import annotation as _annotation
from .patches import FancyArrowPatch
from .transforms import Bbox


@_pushing
class _ArrowMirror(FancyArrowPatch):
    """The arrow of the annotation ``_owner`` as a patch."""

    _owner = None

    def _push(self):
        _annotation.restyle_arrow(self._owner._native, self._native)

    def get_window_extent(self, renderer=None):
        """The display box of the arrow as the annotation draws it."""
        extents = _annotation.arrow_extent(self._owner._native)
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
        patch = annotation._arrow_mirror = _ArrowMirror._from_native(native, None)
        patch._owner = annotation
    else:
        patch._native = native
    return patch
