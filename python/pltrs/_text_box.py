"""
The box behind a text as a patch (matplotlib's ``Text.get_bbox_patch``).

pltrs (Rust) draws a text's box as part of the text, not as an artist of
its own. ``get_bbox_patch`` hands out a *mirror*, as the ticks of an axis
are (`._axis_artists`): a loose `.FancyBboxPatch` filled from the box's
look (``_native.text.bbox_patch``) whose setters give the box their
changes once they ran (``restyle_bbox``). The mirror sits on the unit
square until `place` moves it where the text draws the box
(``Text.update_bbox_position_size``), and stays there.
"""

from ._axis_artists import _pushing
from ._native import text as _text
from .patches import FancyBboxPatch
from .transforms import Affine2D


@_pushing
class _BoxMirror(FancyBboxPatch):
    """The box of the text ``_owner`` as a patch."""

    _owner = None
    #: Where `place` put the patch: the size of the frame, the matrix
    #: placing it in display pixels and the mutation scale.
    _frame = None

    def _push(self):
        _text.restyle_bbox(self._owner._native, self._native)

    def _move_to_frame(self):
        """Put the patch where `_frame` says, its owner unchanged."""
        (width, height), matrix, mutation_scale = self._frame
        FancyBboxPatch.set_bounds(self, 0., 0., width, height)
        FancyBboxPatch.set_transform(self, Affine2D(matrix))
        FancyBboxPatch.set_mutation_scale(self, mutation_scale)


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
        if patch._frame is not None:
            patch._move_to_frame()
    return patch


def place(text):
    """Move the patch of the box of *text* where the text draws the box in
    its figure's current layout, if it has a box there."""
    patch = mirror(text)
    frame = _text.bbox_frame(text._native)
    if patch is None or frame is None:
        return
    patch._frame = frame
    patch._move_to_frame()
