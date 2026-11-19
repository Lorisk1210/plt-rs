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

from . import _mirrors, _unusable, _values, cbook
from ._native import annotation as _annotation, artist as _artist
from ._native import offsetbox as _offsetbox, patches as _patches
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
        # The owner's draws draw the arrow, which leaves it fresh.
        self.owner = self.drawer = owner

    def push(self, artist):
        self.native_restyle(self.owner._native, artist._native)

    def figure(self, artist, root=False):
        """The owner's figure, which matplotlib's ``set_figure`` gives
        the arrow too (None until the owner joins one)."""
        return self.owner.get_figure(root=root)

    def window_extent(self, artist, renderer=None, dpi=None):
        """The display box of the arrow as the annotation draws it."""
        extents = self.native_extent(self.owner._native)
        return Bbox.null() if extents is None else Bbox.from_extents(*extents)

    def display_path(self, artist):
        """The outline of the arrow as the annotation draws it (no vertex
        while the annotation is hidden)."""
        arrays = self.native_display_path(self.owner._native)
        return (np.empty((0, 2)), None) if arrays is None else arrays

    def place_start(self, patch):
        """Give the arrow *patch* the patch clipping its start as
        matplotlib's ``update_positions`` sets it each time it places the
        arrow: the one ``arrowprops`` give, else the owner's
        (`start_patch`); once per draw, none until the first, so that one
        the program sets reads back until the next."""
        owner = self.owner
        draws = _artist.draw_count(owner._native)
        if not draws or patch.__dict__.get("_start_placed_at") == draws:
            return
        patch._start_placed_at = draws
        arrowprops = owner.arrowprops
        patch.patchA = (arrowprops["patchA"] if "patchA" in arrowprops
                        else self.start_patch())

    def start_patch(self):
        """The patch matplotlib's ``update_positions`` clips the start of
        the arrow to unless ``arrowprops`` give one: the text's box, none
        for an empty text, else a `.Rectangle` 4 points larger than the
        text, in pixels, made at each placing."""
        owner = self.owner
        if owner.get_bbox_patch() is not None:
            return owner.get_bbox_patch()
        if owner.get_text() == "":
            return None
        draws = _artist.draw_count(owner._native)
        made = owner.__dict__.get("_arrow_start")
        if made is None or made[0] != draws:
            from .patches import Rectangle
            from .transforms import IdentityTransform
            x0, y0, x1, y1 = owner._text_extent() or (0, 0, 0, 0)
            pad = owner.get_figure(root=True).dpi * 4 / 72
            made = owner._arrow_start = (draws, Rectangle(
                xy=(x0 - pad / 2, y0 - pad / 2),
                width=x1 - x0 + pad, height=y1 - y0 + pad,
                transform=IdentityTransform(), clip_on=False))
        return made[1]


class AnnotationBboxArrow(_AnnotationArrow):
    """The arrow of *owner*, an annotation box that joined a figure."""

    native_patch = staticmethod(_offsetbox.annotation_arrow_patch)
    native_restyle = staticmethod(_offsetbox.restyle_annotation_arrow)
    native_extent = staticmethod(_offsetbox.annotation_arrow_extent)
    native_display_path = staticmethod(_offsetbox.annotation_arrow_display_path)

    def start_patch(self):
        """The patch of the box (``update_positions``)."""
        return self.owner.patch


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
        # Made as matplotlib makes it, ``FancyArrowPatch((0, 0), (1, 1),
        # **arrowprops)``: its ends read back as those ints until its owner
        # places it, its styles and sizes as given.
        patch._given_positions = [(0, 0), (1, 1)]
        _keep_given(patch, owner.arrowprops)
    else:
        patch._native = native
    return patch


def _keep_given(patch, arrowprops):
    """Let the arrow *patch* read back what ``FancyArrowPatch(**arrowprops)``
    keeps: the style objects given, or made of the names given (a style
    the program registered is known to pltrs (Rust) only as the program's),
    the shrinking as given (2, an int, by default) and the patch clipping
    the start."""
    from .patches import ArrowStyle, ConnectionStyle
    props = cbook.normalize_kwargs(arrowprops, FancyArrowPatch)
    for name, key, registry, native_get in (
            ("arrowstyle", "_arrow_transmuter", ArrowStyle, _patches.get_arrowstyle),
            ("connectionstyle", "_connector", ConnectionStyle,
             _patches.get_connectionstyle)):
        given = props.get(name)
        if given is not None:
            style = registry(given) if isinstance(given, str) else given
            patch.__dict__[key] = (style, native_get(patch._native))
    for name in ("shrinkA", "shrinkB"):
        _values.remember(patch, name, props.get(name, 2))
    patch.patchA = props.get("patchA")


def assign(owner, patch, take):
    """
    ``owner.arrow_patch = patch``: while *owner* has ``arrowprops``,
    matplotlib places and draws the patch assigned at its next draw and
    measure (``update_positions``), the arrow taking its look here
    (*take(patch)*), and fails there on None at the first method it calls
    on it (``set_mutation_scale``, ``set_positions``); without, it never
    reads it again.
    """
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
