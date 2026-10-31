"""
The box behind a text as a patch (matplotlib's ``Text.get_bbox_patch``).

pltrs (Rust) draws a text's box as part of the text, not as an artist of
its own. ``get_bbox_patch`` hands out a *mirror*, as the ticks of an axis
are (`._axis_artists`): a loose `.FancyBboxPatch` filled from the box's
look (``_native.text.bbox_patch``) whose setters give the box their
changes once they ran (``restyle_bbox``).
"""

from ._axis_artists import _pushing
from ._native import text as _text
from .patches import FancyBboxPatch


@_pushing
class _BoxMirror(FancyBboxPatch):
    """The box of the text ``_owner`` as a patch."""

    _owner = None

    def _push(self):
        _text.restyle_bbox(self._owner._native, self._native)


def mirror(text):
    """The patch of the box of *text*, the same object while the box stays;
    None without a box."""
    native = _text.bbox_patch(text._native)
    if native is None:
        text.__dict__.pop("_bbox_mirror", None)
        return None
    patch = text.__dict__.get("_bbox_mirror")
    if patch is None:
        patch = text._bbox_mirror = _BoxMirror._from_native(native, None)
        patch._owner = text
    else:
        patch._native = native
    return patch
