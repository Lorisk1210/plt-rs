r"""
Container classes for `.Artist`\s (matplotlib's ``offsetbox``).

`OffsetBox`
    The base of all container artists defined in this module.

`AnchoredOffsetbox`, `AnchoredText`
    Anchor and align an arbitrary `.Artist` or a text relative to the parent
    axes or a specific anchor point.

`DrawingArea`
    A container with fixed width and height. Children have a fixed position
    inside the container and may be clipped.

`HPacker`, `VPacker`
    Containers for layouting their children vertically or horizontally.

`PaddedBox`
    A container to add a padding around an `.Artist`.

`TextArea`
    Contains a single `.Text` instance.

The classes, their geometry and their API are matplotlib 3.11.2's, under
the Matplotlib License; the packing rules underneath (`_get_packed_offsets`,
`_get_aligned_offsets`) are pltrs (Rust)'s (``pltrs::offsetbox``).
"""

import functools

import numpy as np

from . import _api, _handles, _live_transforms, cbook, rcParams
from . import text as mtext
from . import transforms as mtransforms
from ._native import offsetbox as _offsetbox
from .artist import Artist
from .font_manager import FontManager, FontProperties
from .transforms import Bbox, BboxBase, TransformedBbox


DEBUG = False


def _update_frame(box, bbox, fontsize=None):
    """Fit the frame of *box* to *bbox*, its box style scaled by
    *fontsize* (matplotlib's ``update_frame``)."""
    box.patch.set_bounds(bbox.bounds)
    if fontsize:
        box.patch.set_mutation_scale(fontsize)


def _compat_get_offset(meth):
    """
    Decorator for the get_offset method of OffsetBox and subclasses, that
    allows supporting both the new signature (self, bbox, renderer) and the old
    signature (self, width, height, xdescent, ydescent, renderer).
    """
    sigs = [lambda self, width, height, xdescent, ydescent, renderer: locals(),
            lambda self, bbox, renderer: locals()]

    @functools.wraps(meth)
    def get_offset(self, *args, **kwargs):
        params = _api.select_matching_signature(sigs, self, *args, **kwargs)
        bbox = (params["bbox"] if "bbox" in params else
                Bbox.from_bounds(-params["xdescent"], -params["ydescent"],
                                 params["width"], params["height"]))
        return meth(params["self"], bbox, params["renderer"])
    return get_offset


def _get_packed_offsets(widths, total, sep, mode="fixed"):
    r"""
    Pack boxes specified by their *widths*.

    For simplicity of the description, the terminology used here assumes a
    horizontal layout, but the function works equally for a vertical layout.

    There are three packing *mode*\s:

    - 'fixed': The elements are packed tight to the left with a spacing of
      *sep* in between. If *total* is *None* the returned total will be the
      right edge of the last box. A non-*None* total will be passed unchecked
      to the output. In particular this means that right edge of the last
      box may be further to the right than the returned total.

    - 'expand': Distribute the boxes with equal spacing so that the left edge
      of the first box is at 0, and the right edge of the last box is at
      *total*. The parameter *sep* is ignored in this mode. A total of *None*
      is accepted and considered equal to 1. The total is returned unchanged
      (except for the conversion *None* to 1). If the total is smaller than
      the sum of the widths, the laid out boxes will overlap.

    - 'equal': If *total* is given, the total space is divided in N equal
      ranges and each box is left-aligned within its subspace.
      Otherwise (*total* is *None*), *sep* must be provided and each box is
      left-aligned in its subspace of width ``(max(widths) + sep)``. The
      total width is then calculated to be ``N * (max(widths) + sep)``.

    Parameters
    ----------
    widths : list of float
        Widths of boxes to be packed.
    total : float or None
        Intended total length. *None* if not used.
    sep : float or None
        Spacing between boxes.
    mode : {'fixed', 'expand', 'equal'}
        The packing mode.

    Returns
    -------
    total : float
        The total width needed to accommodate the laid out boxes.
    offsets : array of float
        The left offsets of the boxes.
    """
    _api.check_in_list(["fixed", "expand", "equal"], mode=mode)
    if sep is None:
        if mode == "fixed":
            raise TypeError("unsupported operand type(s) for +: 'float' and "
                            "'NoneType'")
        if mode == "equal" and total is None:
            raise ValueError("total and sep cannot both be None when "
                             "using layout mode 'equal'")
        sep = 0.
    return _offsetbox.packed([float(w) for w in widths],
                             None if total is None else float(total),
                             float(sep), mode)


def _get_aligned_offsets(yspans, height, align="baseline"):
    """
    Align boxes each specified by their ``(y0, y1)`` spans.

    For simplicity of the description, the terminology used here assumes a
    horizontal layout (i.e., vertical alignment), but the function works
    equally for a vertical layout.

    Parameters
    ----------
    yspans
        List of (y0, y1) spans of boxes to be aligned.
    height : float or None
        Intended total height. If None, the maximum of the heights
        (``y1 - y0``) in *yspans* is used.
    align : {'baseline', 'left', 'top', 'right', 'bottom', 'center'}
        The alignment anchor of the boxes.

    Returns
    -------
    (y0, y1)
        y range spanned by the packing.  If a *height* was originally passed
        in, then for all alignments other than "baseline", a span of ``(0,
        height)`` is used without checking that it is actually large enough).
    descent
        The descent of the packing.
    offsets
        The bottom offsets of the boxes.
    """
    _api.check_in_list(
        ["baseline", "left", "top", "right", "bottom", "center"], align=align)
    return _offsetbox.aligned([(float(y0), float(y1)) for y0, y1 in yspans],
                              None if height is None else float(height), align)


class OffsetBox(Artist):
    """
    A simple container artist.

    The child artists are meant to be drawn at a relative position to its
    parent.

    Being an artist itself, all keyword arguments are passed on to `.Artist`.
    """

    def __init__(self, **kwargs):
        super().__init__()
        self._internal_update(kwargs)
        self._children = []
        self._offset = (0, 0)

    def _add_to_axes(self, axes, kind, autolim=True):
        return self._join_at_offset(axes)

    def _add_to_figure(self, figure, clip):
        return self._join_at_offset(figure)

    def _join_at_offset(self, parent):
        """Join *parent*, an axes or a figure, as a box pltrs (Rust) draws
        with its origin at the box's offset in display pixels (a plain
        offset box given to ``add_artist``); return self. A box whose
        offset a callable computes is placed where it says before every
        layout of a draw, as matplotlib calls it when it draws the box."""
        offset = (0., 0.) if callable(self._offset) else self._offset
        native, parts = _offsetbox.add_placed(
            parent._native, self._native, self._native_tree(),
            tuple(map(float, offset)))
        figure = parent.figure
        self._adopt_tree(iter(parts), figure)
        self._joined(self)
        self._placed = True
        _handles.adopt(self, native, figure)
        if callable(self._offset):
            from .axes._program_locators import follow
            follow(self, figure.figure)
            self._locate_by_program(figure.figure._get_renderer())
        return self

    def _locate_by_program(self, renderer):
        """Move the box where its callable offset puts it now (see
        `.axes._program_locators`)."""
        if callable(self._offset):
            offset = self.get_offset(self.get_bbox(renderer), renderer)
            _offsetbox.set_placed_offset(self._native, tuple(map(float, offset)))

    def _native_tree(self):
        """
        The description of the box and its children pltrs (Rust) lays out
        (see ``_native.offsetbox``), its artists as their loose natives;
        the boxes of this module describe themselves.
        """
        raise NotImplementedError(
            f"pltrs: an offset box of class {type(self).__name__} in a box "
            "added to a figure")

    def _adopt_tree(self, natives, figure):
        """
        Make the artists of the box, in the order of `_native_tree`, the
        objects of the artists the handles *natives* (an iterator) reach in
        *figure*, once the box joined it.
        """
        for child in self.get_children():
            _handles.adopt(child, next(natives), figure)

    def draw(self, renderer):
        """
        Update the location of children if necessary and draw them
        to the given *renderer*.
        """
        bbox, offsets = self._get_bbox_and_child_offsets(renderer)
        px, py = self.get_offset(bbox, renderer)
        for c, (ox, oy) in zip(self.get_visible_children(), offsets):
            c.set_offset((px + ox, py + oy))
            c.draw(renderer)

    def set_offset(self, xy):
        """
        Set the offset.

        Parameters
        ----------
        xy : (float, float) or callable
            The (x, y) coordinates of the offset in display units. These can
            either be given explicitly as a tuple (x, y), or by providing a
            function that converts the extent into the offset. This function
            must have the signature::

                def offset(width, height, xdescent, ydescent, renderer) \
-> (float, float)
        """
        self._offset = xy
        self._move_placed(xy)

    def _move_placed(self, xy):
        """Move the box to the offset *xy* where pltrs (Rust) draws it, for
        a box added to an axes or a figure itself."""
        if self.__dict__.get("_placed") and not callable(xy):
            _offsetbox.set_placed_offset(self._native, tuple(map(float, xy)))

    @_compat_get_offset
    def get_offset(self, bbox, renderer):
        """
        Return the offset as a tuple (x, y).

        The extent parameters have to be provided to handle the case where the
        offset is dynamically determined by a callable (see
        `~.OffsetBox.set_offset`).
        """
        return (
            self._offset(bbox.width, bbox.height, -bbox.x0, -bbox.y0, renderer)
            if callable(self._offset)
            else self._offset)

    def set_width(self, width):
        """Set the width of the box."""
        self.width = width

    def set_height(self, height):
        """Set the height of the box."""
        self.height = height

    def get_visible_children(self):
        r"""Return a list of the visible child `.Artist`\s."""
        return [c for c in self._children if c.get_visible()]

    def get_children(self):
        r"""Return a list of the child `.Artist`\s."""
        return self._children

    def get_bbox(self, renderer):
        """Return the bbox of the offsetbox, ignoring parent offsets."""
        bbox, offsets = self._get_bbox_and_child_offsets(renderer)
        return bbox

    def _drawing_areas(self):
        r"""The `DrawingArea`\s of the box's tree, in the order of
        `_native_tree`."""
        for child in self.get_children():
            if isinstance(child, OffsetBox):
                yield from child._drawing_areas()

    def _joined(self, owner):
        """Have the drawing areas of the child tree reach their native
        state through *owner*, the box added with the tree."""
        for index, area in enumerate(self._drawing_areas()):
            area._owner_box = (owner, index)


class PackerBase(OffsetBox):
    """The base of `VPacker` and `HPacker`."""

    def __init__(self, pad=0., sep=0., width=None, height=None,
                 align="baseline", mode="fixed", children=None):
        """
        Parameters
        ----------
        pad : float, default: 0.0
            The boundary padding in points.
        sep : float, default: 0.0
            The spacing between items in points.
        width, height : float, optional
            Width and height of the container box in pixels, calculated if
            *None*.
        align : {'top', 'bottom', 'left', 'right', 'center', 'baseline'}, \
default: 'baseline'
            Alignment of boxes.
        mode : {'fixed', 'expand', 'equal'}, default: 'fixed'
            The packing mode.
        children : list of `.Artist`
            The artists to pack.
        """
        super().__init__()
        self.height = height
        self.width = width
        self.sep = sep
        self.pad = pad
        self.mode = mode
        self.align = align
        self._children = children

    def _native_tree(self):
        kind = "vpacker" if isinstance(self, VPacker) else "hpacker"
        return (kind, float(self.pad), float(self.sep),
                None if self.width is None else float(self.width),
                None if self.height is None else float(self.height),
                self.align, self.mode,
                [child._native_tree() for child in self.get_children()])

    def _adopt_tree(self, natives, figure):
        for child in self.get_children():
            child._adopt_tree(natives, figure)


class VPacker(PackerBase):
    """
    VPacker packs its children vertically, automatically adjusting their
    relative positions at draw time.
    """

    def _get_bbox_and_child_offsets(self, renderer):
        """The bbox of the box and the offsets of its children."""
        dpicor = renderer.points_to_pixels(1.)
        pad = self.pad * dpicor
        sep = self.sep * dpicor

        if self.width is not None:
            for c in self.get_visible_children():
                if isinstance(c, PackerBase) and c.mode == "expand":
                    c.set_width(self.width)

        bboxes = [c.get_bbox(renderer) for c in self.get_visible_children()]
        (x0, x1), xoffsets = _get_aligned_offsets(
            [bbox.intervalx for bbox in bboxes], self.width, self.align)
        height, yoffsets = _get_packed_offsets(
            [bbox.height for bbox in bboxes], self.height, sep, self.mode)

        yoffsets = height - (yoffsets + [bbox.y1 for bbox in bboxes])
        ydescent = yoffsets[0]
        yoffsets = yoffsets - ydescent

        return (
            Bbox.from_bounds(x0, -ydescent, x1 - x0, height).padded(pad),
            [*zip(xoffsets, yoffsets)])


class HPacker(PackerBase):
    """
    HPacker packs its children horizontally, automatically adjusting their
    relative positions at draw time.
    """

    def _get_bbox_and_child_offsets(self, renderer):
        """The bbox of the box and the offsets of its children."""
        dpicor = renderer.points_to_pixels(1.)
        pad = self.pad * dpicor
        sep = self.sep * dpicor

        bboxes = [c.get_bbox(renderer) for c in self.get_visible_children()]
        if not bboxes:
            return Bbox.from_bounds(0, 0, 0, 0).padded(pad), []

        (y0, y1), yoffsets = _get_aligned_offsets(
            [bbox.intervaly for bbox in bboxes], self.height, self.align)
        width, xoffsets = _get_packed_offsets(
            [bbox.width for bbox in bboxes], self.width, sep, self.mode)

        x0 = bboxes[0].x0
        xoffsets -= ([bbox.x0 for bbox in bboxes] - x0)

        return (Bbox.from_bounds(x0, y0, width, y1 - y0).padded(pad),
                [*zip(xoffsets, yoffsets)])


class PaddedBox(OffsetBox):
    """
    A container to add a padding around an `.Artist`.

    Attributes
    ----------
    pad : float
        The padding in points.
    """

    def __init__(self, child, pad=0., *, draw_frame=False, patch_attrs=None):
        """
        Parameters
        ----------
        child : `~matplotlib.artist.Artist`
            The contained `.Artist`.
        pad : float, default: 0.0
            The padding in points. This will be scaled with the renderer dpi.
        draw_frame : bool
            Whether to draw the frame around the box.
        patch_attrs : dict or None
            Additional properties of the frame.
        """
        super().__init__()
        self.pad = pad
        self._children = [child]
        self._draw_frame = draw_frame
        self._patch_attrs = dict(patch_attrs or {})

    def _native_tree(self):
        from .patches import FancyBboxPatch
        return ("padded", float(self.pad), self._children[0]._native_tree(),
                bool(self._draw_frame),
                cbook.normalize_kwargs(self._patch_attrs, FancyBboxPatch))

    def _adopt_tree(self, natives, figure):
        from .patches import FancyBboxPatch
        self.patch = _handles.wrap(next(natives), FancyBboxPatch, figure)
        self._children[0]._adopt_tree(natives, figure)

    def _get_bbox_and_child_offsets(self, renderer):
        """The bbox of the box and the offset of its child."""
        pad = self.pad * renderer.points_to_pixels(1.)
        return (self._children[0].get_bbox(renderer).padded(pad), [(0, 0)])

    def update_frame(self, bbox, fontsize=None):
        """
        Fit the frame to *bbox*, its box style scaled by *fontsize*.

        pltrs: the frame is the patch of the box pltrs (Rust) draws, which
        exists once the box joined a figure; pltrs fits it again whenever
        it lays the figure out.
        """
        _update_frame(self, bbox, fontsize)
        self.stale = True

    def draw_frame(self, renderer):
        """Fit the frame to the box and draw it with *renderer*."""
        # update the location and size of the legend
        self.update_frame(self.get_window_extent(renderer))
        self.patch.draw(renderer)


class DrawingArea(OffsetBox):
    """
    The DrawingArea can contain any Artist as a child. The DrawingArea
    has a fixed width and height. The position of children relative to
    the parent is fixed. The children can be clipped at the
    boundaries of the parent.
    """

    def __init__(self, width, height, xdescent=0., ydescent=0., clip=False):
        """
        Parameters
        ----------
        width, height : float
            Width and height of the container box.
        xdescent, ydescent : float
            Descent of the box in x- and y-direction.
        clip : bool
            Whether to clip the children to the box.
        """
        super().__init__()
        self.width = width
        self.height = height
        self.xdescent = xdescent
        self.ydescent = ydescent
        self._clip_children = clip
        self.offset_transform = mtransforms.Affine2D()
        self.dpi_transform = mtransforms.Affine2D()
        #: The added box whose tree holds the area and the area's index
        #: there (None before the box is added).
        self._owner_box = None

    @property
    def clip_children(self):
        """
        If the children of this DrawingArea should be clipped
        by DrawingArea bounding box.
        """
        return self._clip_children

    @clip_children.setter
    def clip_children(self, val):
        self._clip_children = bool(val)
        if self._owner_box is not None:
            owner, index = self._owner_box
            _offsetbox.set_clip_children(owner._native, index,
                                         self._clip_children)

    def _drawing_areas(self):
        yield self

    def get_transform(self):
        """
        Return the `~matplotlib.transforms.Transform` applied to the children.
        """
        return self.dpi_transform + self.offset_transform

    def set_transform(self, t):
        """
        set_transform is ignored.
        """

    def set_offset(self, xy):
        """
        Set the offset of the container.

        Parameters
        ----------
        xy : (float, float)
            The (x, y) coordinates of the offset in display units.
        """
        self._offset = xy
        self.offset_transform.clear()
        self.offset_transform.translate(xy[0], xy[1])
        self._move_placed(xy)

    def get_offset(self):
        """Return offset of the container."""
        return self._offset

    def get_bbox(self, renderer):
        """Return the bbox of the area, ignoring parent offsets."""
        dpi_cor = renderer.points_to_pixels(1.)
        return Bbox.from_bounds(
            -self.xdescent * dpi_cor, -self.ydescent * dpi_cor,
            self.width * dpi_cor, self.height * dpi_cor)

    def add_artist(self, a):
        """Add an `.Artist` to the container box."""
        self._children.append(a)

    def _native_tree(self):
        return ("drawing", float(self.width), float(self.height),
                float(self.xdescent), float(self.ydescent),
                bool(self._clip_children),
                [child._native for child in self._children])

    def draw(self, renderer):
        # docstring inherited
        dpi_cor = renderer.points_to_pixels(1.)
        self.dpi_transform.clear()
        self.dpi_transform.scale(dpi_cor)
        for c in self._children:
            c.draw(renderer)


class TextArea(OffsetBox):
    """
    The TextArea is a container artist for a single Text instance.

    The text is placed at (0, 0) with baseline+left alignment, by default. The
    width and height of the TextArea instance is the width and height of its
    child text.
    """

    def __init__(self, s,
                 *,
                 textprops=None,
                 multilinebaseline=False,
                 ):
        """
        Parameters
        ----------
        s : str
            The text to be displayed.
        textprops : dict, default: {}
            Dictionary of keyword parameters to be passed to the `.Text`
            instance in the TextArea.
        multilinebaseline : bool, default: False
            Whether the baseline for multiline text is adjusted so that it
            is (approximately) center-aligned with single-line text.
        """
        if textprops is None:
            textprops = {}
        self._init_area(mtext.Text(0, 0, s, **textprops), multilinebaseline)

    def _init_area(self, text, multilinebaseline):
        """Make the area of the `.Text` *text*."""
        self._text = text
        super().__init__()
        self._children = [text]
        self.offset_transform = mtransforms.Affine2D()
        self._baseline_transform = mtransforms.Affine2D()
        self._multilinebaseline = multilinebaseline

    @classmethod
    def _of_text(cls, text, multilinebaseline):
        """The area of *text*, a `.Text` pltrs (Rust) laid out in a box."""
        area = cls.__new__(cls)
        area._init_area(text, multilinebaseline)
        return area

    def set_text(self, s):
        """Set the text of this area as a string."""
        self._text.set_text(s)

    def get_text(self):
        """Return the string representation of this area's text."""
        return self._text.get_text()

    def set_multilinebaseline(self, t):
        """
        Set multilinebaseline.

        If True, the baseline for multiline text is adjusted so that it is
        (approximately) center-aligned with single-line text.  This is used
        e.g. by the legend implementation so that single-line labels are
        baseline-aligned, but multiline labels are "center"-aligned with them.
        """
        self._multilinebaseline = t

    def get_multilinebaseline(self):
        """
        Get multilinebaseline.
        """
        return self._multilinebaseline

    def _native_tree(self):
        return ("text", self._text._native, bool(self._multilinebaseline))

    def set_transform(self, t):
        """
        set_transform is ignored.
        """

    def set_offset(self, xy):
        """
        Set the offset of the container.

        Parameters
        ----------
        xy : (float, float)
            The (x, y) coordinates of the offset in display units.
        """
        self._offset = xy
        self.offset_transform.clear()
        self.offset_transform.translate(xy[0], xy[1])
        self._move_placed(xy)

    def get_offset(self):
        """Return offset of the container."""
        return self._offset


def _laid_out_box(description, figure, classes):
    """
    The offset box of a box pltrs (Rust) laid out (a legend's, described
    by ``_native.legend.legend_box``), around the artists of *figure* it
    holds; *classes* maps the kinds of those artists to their classes.
    """
    kind, visible, *parts = description

    def artist(item):
        raw, artist_kind = item
        return _handles.wrap(raw, classes[artist_kind], figure)

    if kind == "text":
        text, multilinebaseline = parts
        box = TextArea._of_text(artist(text), multilinebaseline)
    elif kind == "drawing":
        (width, height, xdescent, ydescent), clip, artists = parts
        box = DrawingArea(width, height, xdescent, ydescent, clip)
        for child in artists:
            box.add_artist(artist(child))
    else:
        (pad, sep, width, height), (align, mode), children = parts
        packer = VPacker if kind == "vpacker" else HPacker
        box = packer(pad=pad, sep=sep, width=width, height=height,
                     align=align, mode=mode,
                     children=[_laid_out_box(child, figure, classes)
                               for child in children])
    box.set_visible(visible)
    return box


class AuxTransformBox(OffsetBox):
    """
    An OffsetBox with an auxiliary transform.

    All child artists are first transformed with *aux_transform*, then
    translated with an offset (the same for all children) so the bounding
    box of the children matches the drawn box.
    """

    def __init__(self, aux_transform):
        self.aux_transform = aux_transform
        super().__init__()
        self.offset_transform = mtransforms.Affine2D()
        # ref_offset_transform makes offset_transform always relative to the
        # lower-left corner of the bbox of its children.
        self.ref_offset_transform = mtransforms.Affine2D()

    def add_artist(self, a):
        """Add an `.Artist` to the container box."""
        self._children.append(a)

    def _native_tree(self):
        return ("aux", _live_transforms.to_native(self.aux_transform),
                [child._native for child in self._children])

    def get_transform(self):
        """Return the `.Transform` applied to the children."""
        return (self.aux_transform
                + self.ref_offset_transform
                + self.offset_transform)

    def set_transform(self, t):
        """
        set_transform is ignored.
        """

    def set_offset(self, xy):
        """
        Set the offset of the container.

        Parameters
        ----------
        xy : (float, float)
            The (x, y) coordinates of the offset in display units.
        """
        self._offset = xy
        self.offset_transform.clear()
        self.offset_transform.translate(xy[0], xy[1])
        self._move_placed(xy)

    def get_offset(self):
        """Return offset of the container."""
        return self._offset


class AnchoredOffsetbox(OffsetBox):
    """
    An OffsetBox placed according to location *loc*.

    AnchoredOffsetbox has a single child.  When multiple children are needed,
    use an extra OffsetBox to enclose them.  By default, the offset box is
    anchored against its parent Axes. You may explicitly specify the
    *bbox_to_anchor*.
    """

    # Location codes
    codes = {'upper right': 1,
             'upper left': 2,
             'lower left': 3,
             'lower right': 4,
             'right': 5,
             'center left': 6,
             'center right': 7,
             'lower center': 8,
             'upper center': 9,
             'center': 10,
             }

    def __init__(self, loc, *,
                 pad=0.4, borderpad=0.5,
                 child=None, prop=None, frameon=True,
                 bbox_to_anchor=None,
                 bbox_transform=None,
                 **kwargs):
        """
        Parameters
        ----------
        loc : str
            The box location.  Valid locations are
            'upper left', 'upper center', 'upper right',
            'center left', 'center', 'center right',
            'lower left', 'lower center', 'lower right'.
            For backward compatibility, numeric values are accepted as well.
        pad : float, default: 0.4
            Padding around the child as fraction of the fontsize.
        borderpad : float or (float, float), default: 0.5
            Padding between the offsetbox frame and the *bbox_to_anchor*.
        child : `.OffsetBox`
            The box that will be anchored.
        prop : `.FontProperties`
            This is only used as a reference for paddings. If not given,
            :rc:`legend.fontsize` is used.
        frameon : bool
            Whether to draw a frame around the box.
        bbox_to_anchor : `.BboxBase`, 2-tuple, or 4-tuple of floats
            Box that is used to position the legend in conjunction with *loc*.
        bbox_transform : None or :class:`matplotlib.transforms.Transform`
            The transform for the bounding box (*bbox_to_anchor*).
        **kwargs
            All other parameters are passed on to `.OffsetBox`.
        """
        kwargs.setdefault("zorder", 5)
        super().__init__(**kwargs)
        self.set_bbox_to_anchor(bbox_to_anchor, bbox_transform)
        self.set_child(child)
        if isinstance(loc, str):
            loc = _api.getitem_checked(self.codes, loc=loc)
        self.loc = loc
        self.borderpad = borderpad
        self.pad = pad
        self.frameon = frameon
        if prop is None:
            self.prop = FontProperties(size=rcParams["legend.fontsize"])
        else:
            self.prop = FontProperties._from_any(prop)
            if isinstance(prop, dict) and "size" not in prop:
                self.prop.set_size(rcParams["legend.fontsize"])

    def _add_to_axes(self, axes, kind, autolim=True):
        return self._join(axes)

    def _add_to_figure(self, figure, clip):
        return self._join(figure)

    def _native_anchor(self):
        """``bbox_to_anchor`` and ``bbox_transform`` as pltrs (Rust) takes
        them: the bounds of the box (a point for a box of no size) and the
        native transform of its coordinates (None: display pixels)."""
        bbox, transform = self._bbox_to_anchor, self._bbox_to_anchor_transform
        if bbox is None:
            return None, (None if transform is None
                          else _live_transforms.to_native(transform))
        spanning = getattr(bbox, "_spanning", None)
        if spanning is not None:
            bbox, transform = Bbox.unit(), spanning
        elif isinstance(bbox, TransformedBbox):
            bbox, transform = bbox._bbox, bbox._transform
        elif not isinstance(bbox, Bbox):
            bbox, transform = Bbox(bbox.get_points()), None
        x0, y0, width, height = bbox.bounds
        bounds = [x0, y0] if width == height == 0 else [x0, y0, width, height]
        return bounds, (None if transform is None
                        else _live_transforms.to_native(transform))

    def _native_options(self):
        """The settings of the box as pltrs (Rust) takes them."""
        bounds, transform = self._native_anchor()
        return {"loc": self.loc, "pad": float(self.pad),
                "borderpad": self.borderpad,
                "prop": FontManager._props(self.prop),
                "frameon": bool(self.frameon), "bbox_to_anchor": bounds,
                "bbox_transform": transform}

    def _native_tree(self):
        # The box inside another box: drawn at its anchor all the same.
        return ("anchored", self.get_child()._native_tree(),
                self._native_options())

    def _adopt_tree(self, natives, figure):
        from .patches import FancyBboxPatch
        self.patch = _handles.wrap(next(natives), FancyBboxPatch, figure)
        self.get_child()._adopt_tree(natives, figure)
        self._in_tree = True

    def get_window_extent(self, renderer=None):
        """Return the box's extent in display space (for a box inside
        another box's tree, its frame's: the box drawn at its anchor)."""
        if self.__dict__.get("_in_tree"):
            return self.patch.get_window_extent(renderer)
        return super().get_window_extent(renderer)

    def _join(self, parent):
        """Join *parent*, an axes or a figure, with the child tree: pltrs
        (Rust) anchors the box there; return self."""
        from .patches import FancyBboxPatch
        native, parts, frame = _offsetbox.add_anchored(
            parent._native, self._native, self.get_child()._native_tree(),
            self._native_options())
        figure = parent.figure
        self.get_child()._adopt_tree(iter(parts), figure)
        self.patch = _handles.wrap(frame, FancyBboxPatch, figure)
        self.get_child()._joined(self)
        return _handles.adopt(self, native, figure)

    def update_frame(self, bbox, fontsize=None):
        """
        Fit the frame to *bbox*, its box style scaled by *fontsize*.

        pltrs: the frame is the patch of the box pltrs (Rust) draws, which
        exists once the box joined a figure; pltrs fits it again whenever
        it lays the figure out.
        """
        _update_frame(self, bbox, fontsize)

    def set_child(self, child):
        """Set the child to be anchored."""
        self._child = child

    def get_child(self):
        """Return the child."""
        return self._child

    def get_children(self):
        """Return the list of children."""
        return [self._child]

    def get_bbox_to_anchor(self):
        """Return the bbox that the box is anchored to, if one was set."""
        transform = self._bbox_to_anchor_transform
        if self._bbox_to_anchor is None or transform is None:
            return self._bbox_to_anchor
        return TransformedBbox(self._bbox_to_anchor, transform)

    def set_bbox_to_anchor(self, bbox, transform=None):
        """
        Set the bbox that the box is anchored to.

        *bbox* can be a Bbox instance, a list of [left, bottom, width,
        height], or a list of [left, bottom] where the width and
        height will be assumed to be zero. The bbox will be
        transformed to display coordinate by the given transform.
        """
        if bbox is None or isinstance(bbox, BboxBase):
            self._bbox_to_anchor = bbox
        else:
            try:
                l = len(bbox)
            except TypeError as err:
                raise ValueError(f"Invalid bbox: {bbox}") from err
            if l == 2:
                bbox = [bbox[0], bbox[1], 0, 0]
            self._bbox_to_anchor = Bbox.from_bounds(*bbox)
        self._bbox_to_anchor_transform = transform


def _get_anchored_bbox(loc, bbox, parentbbox, pad_x, pad_y):
    """
    Return the (x, y) position of the *bbox* anchored at the *parentbbox* with
    the *loc* code with the *borderpad* and padding *pad_x*, *pad_y*.
    """
    c = [None, "NE", "NW", "SW", "SE", "E", "W", "E", "S", "N", "C"][loc]
    container = parentbbox.padded(-pad_x, -pad_y)
    return bbox.anchored(c, container=container).p0


class AnchoredText(AnchoredOffsetbox):
    """
    AnchoredOffsetbox with Text.
    """

    def __init__(self, s, loc, *, pad=0.4, borderpad=0.5, prop=None, **kwargs):
        """
        Parameters
        ----------
        s : str
            Text.
        loc : str
            Location code. See `AnchoredOffsetbox`.
        pad : float, default: 0.4
            Padding around the text as fraction of the fontsize.
        borderpad : float, default: 0.5
            Spacing between the offsetbox frame and the *bbox_to_anchor*.
        prop : dict, optional
            Dictionary of keyword parameters to be passed to the
            `~matplotlib.text.Text` instance contained inside AnchoredText.
        **kwargs
            All other parameters are passed to `AnchoredOffsetbox`.
        """
        if prop is None:
            prop = {}
        badkwargs = {'va', 'verticalalignment'}
        if badkwargs & set(prop):
            raise ValueError(
                'Mixing verticalalignment with AnchoredText is not supported.')

        self.txt = TextArea(s, textprops=prop)
        super().__init__(loc, pad=pad, borderpad=borderpad, child=self.txt,
                         prop=FontProperties._from_any(
                             {k: v for k, v in prop.items()
                              if k in ("family", "size", "style", "weight")}),
                         **kwargs)


class OffsetImage(OffsetBox):
    """
    Container artist for images: the data an offset box of the image's size
    times *zoom* shows.
    """

    def __init__(self, arr, *,
                 zoom=1,
                 cmap=None,
                 norm=None,
                 interpolation=None,
                 origin=None,
                 filternorm=True,
                 filterrad=4.0,
                 resample=False,
                 dpi_cor=True,
                 **kwargs
                 ):
        super().__init__()
        self._dpi_cor = dpi_cor
        self._image_options = dict(
            cmap=cmap, norm=norm, interpolation=interpolation, origin=origin,
            filternorm=filternorm, filterrad=filterrad, resample=resample,
            **kwargs)
        self.set_zoom(zoom)
        self.set_data(arr)

    def set_data(self, arr):
        """
        Set the image data: the box's image is made of it before the box
        joins a figure, and shows it once the box drew it.
        """
        from .image import BboxImage
        self._data = np.asarray(arr)
        image = getattr(self, "image", None)
        if image is not None and image.figure is not None:
            image.set_data(self._data)
        else:
            self.image = BboxImage._loose(self._data, **self._image_options)
            self._children = [self.image]
        self.stale = True

    def _native_tree(self):
        return ("image", self.image._native, float(self.get_zoom()),
                bool(self._dpi_cor))

    def get_data(self):
        """Return the image data."""
        return self._data

    def set_zoom(self, zoom):
        """Set the zoom factor."""
        self._zoom = zoom

    def get_zoom(self):
        """Return the zoom factor."""
        return self._zoom

    def get_offset(self):
        """Return offset of the container."""
        return self._offset

    def get_bbox(self, renderer):
        """Return the bbox of the image, ignoring parent offsets."""
        dpi_cor = renderer.points_to_pixels(1.) if self._dpi_cor else 1.
        zoom = self.get_zoom()
        data = self.get_data()
        ny, nx = data.shape[:2]
        w, h = dpi_cor * nx * zoom, dpi_cor * ny * zoom
        return Bbox.from_bounds(0, 0, w, h)


class AnnotationBbox(Artist, mtext._AnnotationBase):
    """
    Container for an `OffsetBox` referring to a specific position *xy*.

    Optionally an arrow pointing from the offsetbox to *xy* can be drawn.

    This is like `.Annotation`, but with `OffsetBox` instead of `.Text`.
    """

    def __str__(self):
        return f"AnnotationBbox({self.xy[0]:g},{self.xy[1]:g})"

    def __init__(self, offsetbox, xy, xybox=None, xycoords='data', boxcoords=None, *,
                 frameon=True, pad=0.4,  # FancyBboxPatch boxstyle.
                 annotation_clip=None,
                 box_alignment=(0.5, 0.5),
                 bboxprops=None,
                 arrowprops=None,
                 fontsize=None,
                 **kwargs):
        """
        Parameters
        ----------
        offsetbox : `OffsetBox`
        xy : (float, float)
            The point *(x, y)* to annotate. The coordinate system is determined
            by *xycoords*.
        xybox : (float, float), default: *xy*
            The position *(x, y)* to place the text at. The coordinate system
            is determined by *boxcoords*.
        xycoords : single or two-tuple of str or `.Artist` or `.Transform` or \
callable, default: 'data'
            The coordinate system that *xy* is given in.
        boxcoords : single or two-tuple of str or `.Artist` or `.Transform` \
or callable, default: value of *xycoords*
            The coordinate system that *xybox* is given in.
        frameon : bool, default: True
            By default, the text is surrounded by a white `.FancyBboxPatch`
            (accessible as the ``patch`` attribute of the `.AnnotationBbox`).
            If *frameon* is set to False, this patch is made invisible.
        annotation_clip: bool or None, default: None
            Whether to clip (i.e. not draw) the annotation when the annotation
            point *xy* is outside the Axes area.
        pad : float, default: 0.4
            Padding around the offsetbox.
        box_alignment : (float, float)
            A tuple of two floats for a vertical and horizontal alignment of
            the offset box w.r.t. the *boxcoords*.
            The lower-left corner is (0, 0) and upper-right corner is (1, 1).
        bboxprops : dict, optional
            A dictionary of properties to set for the annotation bounding box,
            for example *boxstyle* and *alpha*.
        arrowprops: dict, optional
            Arrow parameters passed to `.FancyArrowPatch` for drawing an arrow
            between the box and the annotated point *xy*.
        fontsize: float or str, optional
            Translated to points and passed as *mutation_scale* into
            `.FancyBboxPatch` to scale attributes of the box style (e.g. pad
            or rounding_size).
        **kwargs
            Other `AnnotationBbox` properties.
        """
        kwargs.setdefault("zorder", 3)
        super().__init__()
        self._draggable = None
        self._xy = self._xybox = None
        self.xy = xy
        self.xycoords = xycoords
        self._annotation_clip = annotation_clip
        self.offsetbox = offsetbox
        self.arrowprops = arrowprops.copy() if arrowprops is not None else None
        self.set_fontsize(fontsize)
        self.xybox = xybox if xybox is not None else xy
        self.boxcoords = boxcoords if boxcoords is not None else xycoords
        self._box_alignment = box_alignment
        self._frameon = frameon
        self._pad = pad
        self._bboxprops = dict(bboxprops or {})
        self._internal_update(kwargs)

    def _add_to_axes(self, axes, kind, autolim=True):
        return self._join(axes)

    def _add_to_figure(self, figure, clip):
        return self._join(figure)

    def _join(self, parent):
        """Join *parent*, an axes or a figure, with the offset box: pltrs
        (Rust) places it at *xybox* and draws the arrow; return self."""
        from ._patch_styles import _native_spec
        from .patches import BoxStyle, FancyBboxPatch
        from .text import _native_arrowprops, _native_coords
        figure = parent.figure
        bboxprops = cbook.normalize_kwargs(self._bboxprops, FancyBboxPatch)
        boxstyle = bboxprops.pop("boxstyle", None)
        arrowprops = None
        if self.arrowprops is not None:
            arrowprops = _native_arrowprops(
                {"arrowstyle": "simple", **self.arrowprops})
        native, parts, frame = _offsetbox.add_annotation_bbox(
            parent._native, self._native, self.offsetbox._native_tree(),
            tuple(map(float, self.xy)), {
                "xybox": tuple(map(float, self.xybox)),
                "xycoords": _native_coords(self.xycoords, figure),
                "boxcoords": _native_coords(self.boxcoords, figure,
                                            "boxcoords"),
                "frameon": bool(self._frameon), "pad": float(self._pad),
                "annotation_clip": self._annotation_clip,
                "box_alignment": tuple(map(float, self._box_alignment)),
                "boxstyle": (None if boxstyle is None
                             else _native_spec(boxstyle, BoxStyle)),
                "bboxprops": bboxprops, "arrowprops": arrowprops,
                "fontsize": float(self.get_fontsize())})
        self.offsetbox._adopt_tree(iter(parts), figure)
        self.patch = _handles.wrap(frame, FancyBboxPatch, figure)
        self.offsetbox._joined(self)
        return _handles.adopt(self, native, figure)

    def set_annotation_clip(self, b):
        """
        Set the annotation's clipping behavior.

        Parameters
        ----------
        b : bool or None
            - True: The annotation will be clipped when ``self.xy`` is
              outside the Axes.
            - False: The annotation will always be drawn.
            - None: The annotation will be clipped when ``self.xy`` is
              outside the Axes and ``self.xycoords == "data"``.
        """
        self._annotation_clip = b
        if self._figure is not None:
            _offsetbox.set_annotation_clip(self._native, b)
            self.stale = True

    def get_annotation_clip(self):
        """
        Return the annotation's clipping behavior.

        See `set_annotation_clip` for the meaning of return values.
        """
        if self._figure is not None:
            return _offsetbox.get_annotation_clip(self._native)
        return self._annotation_clip

    @property
    def xy(self):
        """The point *(x, y)* the box annotates, in *xycoords*."""
        return self._xy

    @xy.setter
    def xy(self, xy):
        self._xy = xy
        self._move(xy, None)

    @property
    def xybox(self):
        """The position *(x, y)* of the box, in *boxcoords*."""
        return self._xybox

    @xybox.setter
    def xybox(self, xybox):
        self._xybox = xybox
        self._move(None, xybox)

    def _move(self, xy, xybox):
        """Give the annotated point *xy* and the box position *xybox* (each
        kept when None) to the box pltrs (Rust) draws, once it has one."""
        if self._figure is None:
            return
        _offsetbox.set_annotation_points(
            self._native, *(None if point is None else tuple(map(float, point))
                            for point in (xy, xybox)))
        self.stale = True

    def update_positions(self, renderer):
        """
        Update pixel positions for the annotated point, the text, and the arrow.

        pltrs: places the frame (`patch`) and the parts of the offset box
        where pltrs (Rust) draws them in the figure's current layout.
        """
        if self._figure is not None:
            _offsetbox.update_annotation_positions(self._native)

    @property
    def xyann(self):
        """The position of the box (*xybox*)."""
        return self.xybox

    @xyann.setter
    def xyann(self, xyann):
        self.xybox = xyann

    @property
    def anncoords(self):
        """The coordinate system of the box position (*boxcoords*)."""
        return self.boxcoords

    @anncoords.setter
    def anncoords(self, coords):
        self.boxcoords = coords

    def get_children(self):
        """
        Return the list of children: the offset box and, once the box has
        joined an axes or a figure, its frame (pltrs (Rust) draws the arrow
        without an artist of its own).
        """
        patch = self.__dict__.get("patch")
        return [self.offsetbox, *([] if patch is None else [patch])]

    def set_fontsize(self, s=None):
        """
        Set the fontsize in points.

        If *s* is not given, reset to :rc:`legend.fontsize`.
        """
        if s is None:
            s = rcParams["legend.fontsize"]
        self.prop = FontProperties(size=s)

    def get_fontsize(self):
        """Return the fontsize in points."""
        return self.prop.get_size_in_points()


class DraggableBase:
    """
    Helper base class for a draggable artist (legend, offsetbox).

    Derived classes must override the following methods::

        def save_offset(self):
            '''
            Called when the object is picked for dragging; should save the
            reference position of the artist.
            '''

        def update_offset(self, dx, dy):
            '''
            Called during the dragging; (*dx*, *dy*) is the pixel offset from
            the point where the mouse drag started.
            '''

    Optionally, you may override the following method::

        def finalize_offset(self):
            '''Called when the mouse is released.'''
    """

    def __init__(self, ref_artist, use_blit=False):
        self.ref_artist = ref_artist
        if not ref_artist.pickable():
            ref_artist.set_picker(self._picker)
        self.got_artist = False
        self._use_blit = use_blit and self.canvas.supports_blit
        callbacks = self.canvas.callbacks
        self._disconnectors = [
            functools.partial(
                callbacks.disconnect, callbacks._connect_picklable(name, func))
            for name, func in [
                ("pick_event", self.on_pick),
                ("button_release_event", self.on_release),
                ("motion_notify_event", self.on_motion),
            ]
        ]

    @staticmethod
    def _picker(artist, mouseevent):
        # A custom picker to prevent dragging on mouse scroll events
        if mouseevent.name == "scroll_event":
            return False, {}
        return artist.contains(mouseevent)

    # A property, not an attribute, to maintain picklability.
    canvas = property(lambda self: self.ref_artist.get_figure(root=True).canvas)
    cids = property(lambda self: [
        disconnect.args[0] for disconnect in self._disconnectors[:2]])

    def on_motion(self, evt):
        if self._check_still_parented() and self.got_artist:
            dx = evt.x - self.mouse_x
            dy = evt.y - self.mouse_y
            self.update_offset(dx, dy)
            if self._use_blit:
                self.canvas.restore_region(self.background)
                self.ref_artist.draw(
                    self.ref_artist.get_figure(root=True)._get_renderer())
                self.canvas.blit()
            else:
                self.canvas.draw()

    def on_pick(self, evt):
        if self._check_still_parented():
            if evt.artist == self.ref_artist:
                self.mouse_x = evt.mouseevent.x
                self.mouse_y = evt.mouseevent.y
                self.save_offset()
                self.got_artist = True
            if self.got_artist and self._use_blit:
                self.ref_artist.set_animated(True)
                self.canvas.draw()
                fig = self.ref_artist.get_figure(root=False)
                self.background = self.canvas.copy_from_bbox(fig.bbox)
                self.ref_artist.draw(fig._get_renderer())
                self.canvas.blit()

    def on_release(self, event):
        if self._check_still_parented() and self.got_artist:
            self.finalize_offset()
            self.got_artist = False
            if self._use_blit:
                self.canvas.restore_region(self.background)
                self.ref_artist.draw(self.ref_artist.figure._get_renderer())
                self.canvas.blit()
                self.ref_artist.set_animated(False)

    def _check_still_parented(self):
        if self.ref_artist.get_figure(root=False) is None:
            self.disconnect()
            return False
        else:
            return True

    def disconnect(self):
        """Disconnect the callbacks."""
        for disconnector in self._disconnectors:
            disconnector()

    def save_offset(self):
        """Save the reference position of the artist (on a pick)."""

    def update_offset(self, dx, dy):
        """Move the artist by *dx*, *dy* pixels from where the drag started."""

    def finalize_offset(self):
        """Settle the artist where it was dropped (on the release)."""


class DraggableOffsetBox(DraggableBase):
    """Drag an offset box: its offset follows the mouse."""

    def __init__(self, ref_artist, offsetbox, use_blit=False):
        super().__init__(ref_artist, use_blit=use_blit)
        self.offsetbox = offsetbox

    def save_offset(self):
        offsetbox = self.offsetbox
        renderer = offsetbox.get_figure(root=True)._get_renderer()
        offset = offsetbox.get_offset(offsetbox.get_bbox(renderer), renderer)
        self.offsetbox_x, self.offsetbox_y = offset
        self.offsetbox.set_offset(offset)

    def update_offset(self, dx, dy):
        loc_in_canvas = self.offsetbox_x + dx, self.offsetbox_y + dy
        self.offsetbox.set_offset(loc_in_canvas)

    def get_loc_in_canvas(self):
        """The lower-left corner of the box on the canvas."""
        offsetbox = self.offsetbox
        renderer = offsetbox.get_figure(root=True)._get_renderer()
        bbox = offsetbox.get_bbox(renderer)
        ox, oy = offsetbox._offset
        loc_in_canvas = (ox + bbox.x0, oy + bbox.y0)
        return loc_in_canvas


class DraggableAnnotation(DraggableBase):
    """Drag an annotation: its text position follows the mouse."""

    def __init__(self, annotation, use_blit=False):
        super().__init__(annotation, use_blit=use_blit)
        self.annotation = annotation

    def save_offset(self):
        ann = self.annotation
        self.ox, self.oy = ann.get_transform().transform(ann.xyann)

    def update_offset(self, dx, dy):
        ann = self.annotation
        ann.xyann = ann.get_transform().inverted().transform(
            (self.ox + dx, self.oy + dy))
