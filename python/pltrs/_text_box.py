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

from . import _mirrors
from ._native import patches as _patches, text as _text
from .patches import FancyBboxPatch
from .transforms import Affine2D


class _TextBox(_mirrors.Mirroring):
    """The box of *text*, which a `.FancyBboxPatch` stands for."""

    def __init__(self, text):
        self.text = text
        #: Where `place` put the patch: the size of the frame, the matrix
        #: placing it in display pixels and the mutation scale.
        self.frame = None

    def push(self, artist):
        _text.restyle_bbox(self.text._native, artist._native)

    def move_to_frame(self, patch):
        """Put *patch* where `frame` says, the box unchanged."""
        (width, height), matrix, mutation_scale = self.frame
        with _mirrors.detached(patch):
            patch.set_bounds(0., 0., width, height)
            patch.set_transform(Affine2D(matrix))
            patch.set_mutation_scale(mutation_scale)


def mirror(text):
    """The patch of the box of *text*, the same object while the box stays;
    None without a box."""
    native = _text.bbox_patch(text._native)
    if native is None:
        text.__dict__.pop("_bbox_mirror", None)
        return None
    patch = text.__dict__.get("_bbox_mirror")
    if patch is None:
        patch = text._bbox_mirror = _mirrors.attach(
            FancyBboxPatch._from_native(native, None), _TextBox(text))
        style = text.__dict__.get("_bbox_style")
        if style is not None:  # A style of the program's, kept as given.
            patch._bbox_transmuter = (
                style, _patches.get_boxstyle(patch._native))
    else:
        patch._native = native
        if patch._mirror.frame is not None:
            patch._mirror.move_to_frame(patch)
    return patch


def place(text):
    """Move the patch of the box of *text* where the text draws the box in
    its figure's current layout, if it has a box there."""
    patch = mirror(text)
    frame = _text.bbox_frame(text._native)
    if patch is None or frame is None:
        return
    patch._mirror.frame = frame
    patch._mirror.move_to_frame(patch)
