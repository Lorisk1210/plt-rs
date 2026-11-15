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
import numbers
import weakref

import numpy as np

from . import _docstring
from . import (_api, _handles, _live_transforms, _named, _offsetbox_trees,
               _python_drawing, _unusable, _values, cbook)
from ._annotation_kept import native_coords
from . import rcParams
from . import text as mtext
from . import transforms as mtransforms
from ._native import artist as _artist, offsetbox as _offsetbox
from ._drawn_with import DrawnWith
from ._offsetbox_trees import (
    PACK_ALIGNS, PACK_MODES, TreeOwner, TreeSetting, Unusable, adopt_part,
    artist_handle, box_tree, children, choice, follow, joinable, number,
    optional_number)
from ._program_classes import takes_from_program
from .artist import Artist
from .font_manager import FontManager, FontProperties
from .transforms import Bbox, BboxBase, Transform, TransformedBbox


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
    widths = [float(w) for w in widths]
    if _taken_as_given(total):
        # matplotlib's passes the total through when it packs tight, and
        # fails where it computes with it as Python does.
        if mode == "expand" and len(widths) > 1:
            sep = (total - sum(map(np.float64, widths))) / (len(widths) - 1)
        elif mode == "equal":
            sep = total / len(widths) - max(widths)
        return total, _offsetbox.packed(widths, None, float(sep), mode)[1]
    return _offsetbox.packed(widths, None if total is None else float(total),
                             float(sep), mode)


def _taken_as_given(length):
    """Whether *length*, a total length of a packer, is one matplotlib's
    computes with as it is given: one that is no number."""
    return length is not None and not isinstance(length, numbers.Real)


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
    if _taken_as_given(height) and align != "baseline":
        # matplotlib's spans the height as given, and fails where it computes
        # with it as Python does.
        if align in ["left", "bottom"]:
            offsets = [-y0 for y0, y1 in yspans]
        elif align in ["right", "top"]:
            offsets = [height - y1 for y0, y1 in yspans]
        else:
            offsets = [(height - (y1 - y0)) * .5 - y0 for y0, y1 in yspans]
        return (0, height), offsets
    return _offsetbox.aligned([(float(y0), float(y1)) for y0, y1 in yspans],
                              None if height is None else float(height), align)


class OffsetBox(TreeOwner, DrawnWith, Artist):
    """
    A simple container artist.

    The child artists are meant to be drawn at a relative position to its
    parent.

    Being an artist itself, all keyword arguments are passed on to `.Artist`.
    """

    def set_figure(self, fig):
        """
        Set the `.Figure` for the `.OffsetBox` and all its children.

        Parameters
        ----------
        fig : `~matplotlib.figure.Figure`
        """
        super().set_figure(fig)
        if fig is not None:
            # The box and those inside it have the figure, as matplotlib's
            # gives it them, down to the texts of text areas, which measure
            # with its dpi (a box a program makes and draws in its own
            # `draw`, matplotlib-scalebar's).
            for box in self._boxes():
                box.__dict__.pop("_joined_late", None)
                box._figure_given = fig

    @Artist.axes.setter
    def axes(self, ax):
        # The box's children are in its Axes, as matplotlib sets them.
        Artist.axes.fset(self, ax)
        for c in self.get_children():
            if c is not None:
                c.axes = ax

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    #: matplotlib's draw of a box reads no transform of its own (see
    #: `._python_drawing.evaluate_given_transform`).
    _draw_reads_transform = False

    #: Whether the boxes of the class hold artists rather than boxes (a
    #: drawing area, a text area, an offset image, an aux-transform box).
    _leaf = False

    #: The artist pltrs (Rust) laid the box out for (a legend, for its
    #: ``_legend_box``; None for other boxes), which the box is on and which
    #: takes what the program sets on the box (``_laid_out_box_changed``)
    #: and where it moves it (``_laid_out_box_moved``).
    _laid_out_for = None

    #: The box's place in the tree pltrs (Rust) lays out (see
    #: `._offsetbox_trees`): the native handle of the box added with the
    #: tree and the box's index in it, None outside any tree.
    _node = None

    #: The description of the box as its class (of this package) describes
    #: it to pltrs (Rust) (see ``_native.offsetbox``), a method of the
    #: classes of this module; None for a class describing none.
    _package_tree = None

    def __init__(self, **kwargs):
        self._native = _offsetbox.new_box()
        super().__init__()
        self._internal_update(kwargs)
        # Clipping has not been implemented in the OffsetBox family, so
        # disable the clip flag for consistency. It can always be turned back
        # on to zero effect.
        self.set_clip_on(False)
        self._children = []
        self._offset = (0, 0)

    @classmethod
    def _sizes_itself(cls):
        """Whether the boxes of the class size and draw themselves, which
        pltrs (Rust) does not lay out: a subclass of the program's describing
        no tree (``_package_tree``), or one overriding how matplotlib's boxes
        size, place or draw each other (`._offsetbox_trees.PROGRAM_PROTOCOL`).
        """
        return (cls._package_tree is OffsetBox._package_tree
                or any(takes_from_program(cls, name)
                       for name in _offsetbox_trees.PROGRAM_PROTOCOL))

    def _native_tree(self):
        """
        The description of the box and its children pltrs (Rust) lays out
        (see ``_native.offsetbox``), its artists as their natives. A box of
        the program's (see `_sizes_itself`) is a box pltrs (Rust) lays out
        with the extent its ``get_bbox`` gives and that its ``draw`` draws,
        after its ``set_offset`` was given where the box went, as
        matplotlib's parents do.
        """
        if self._sizes_itself():
            return ("program", self.get_visible(), self._native)
        return self._package_tree()

    def _changed(self):
        """Hand pltrs (Rust) the tree holding the box again, as matplotlib
        reads the box's settings again whenever it draws it."""
        owner = self.__dict__.get("_tree_owner")
        if owner is None and "_tree_handle" in self.__dict__:
            owner = self
        if owner is not None:
            owner._sync_tree()
        if self._laid_out_for is not None:
            self._laid_out_for._laid_out_box_changed(self)
        self.stale = True

    @_docstring.copy(Artist.set_visible)
    def set_visible(self, b):
        # docstring inherited
        if not b:
            # A hidden box keeps the offset its parent last gave it.
            self._laid_out_offset()
        super().set_visible(b)
        self._changed()

    def _add_to_axes(self, axes, kind, autolim=True):
        if self._comes_back(axes):
            return Artist._add_to_axes(self, axes, kind, autolim)
        if self._sizes_itself():
            _python_drawing.paint(self)
            super()._add_to_axes(axes, kind, autolim)
            self._join_children(axes.figure)
            follow(self)
            return self
        # matplotlib's `add_artist` sets the Axes of the box and its children
        # first.
        self.axes = axes
        return self._join_at_offset(axes)

    def _add_to_figure(self, figure, clip):
        if self._comes_back(figure):
            return Artist._add_to_figure(self, figure, clip)
        if self._sizes_itself():
            _python_drawing.paint(self)
            super()._add_to_figure(figure, clip)
            self._join_children(figure)
            follow(self)
            return self
        return self._join_at_offset(figure)

    def _join_at_offset(self, parent):
        """Join *parent*, an axes or a figure, as a box pltrs (Rust) draws
        with its origin at the box's offset in display pixels (a plain
        offset box given to ``add_artist``); return self. A box whose
        offset a callable computes is placed where it says before every
        layout of a draw, as matplotlib calls it when it draws the box."""
        self._join_placed(_offsetbox.add_placed, parent._native, parent.figure)
        if callable(self._offset):
            self._locate_by_program(parent.figure.figure._get_renderer())
        return self

    def _join_placed(self, add, parent, figure):
        """Join *figure* as a box placed at its offset that *add* adds to
        the native *parent* (``_native.offsetbox.add_placed`` or
        ``add_owned``)."""
        offset = (0., 0.) if callable(self._offset) else self._offset
        described = self._tree_to_join()
        native, parts = add(parent, self._native, described,
                            tuple(map(float, offset)))
        _handles.adopt(self, native, figure)
        self._own_tree(parts, figure, native, described)

    def _join_children(self, figure, holder=None):
        """
        Have the boxes of this box of the program's (see `_sizes_itself`)
        join *figure* as the boxes it draws itself, as matplotlib's parent
        sets its children's offsets and draws them: each box of the
        package's among its children becomes a box placed at its offset
        that the box of the program's, or its *holder* (the native of the
        nearest one in the figure), owns and draws where its ``draw`` draws
        it; a box of the program's among them hands its own on. A subclass
        of the package's leaves (a `DrawingArea` overriding ``draw``) gets
        the box its class describes so, which the class' methods measure,
        place and draw.
        """
        holder = self._native if holder is None else holder
        if self._leaf:
            if "_tree_handle" not in self.__dict__:
                offset = (0., 0.) if callable(self._offset) else self._offset
                described = joinable(self._package_tree)
                native, parts = _offsetbox.add_owned(
                    holder, None, described, tuple(map(float, offset)))
                self._own_tree(parts, figure, native, described)
            return
        for child in self.get_children():
            if not isinstance(child, OffsetBox):
                continue
            if child._sizes_itself():
                if child.get_figure(root=True) is None:
                    child.set_figure(figure)
                child._join_children(figure, holder)
            elif child.__dict__.get("_tree_owner") is None:
                child._join_placed(_offsetbox.add_owned, holder, figure)

    def _locate_by_program(self, renderer):
        """Bring the box up to date before a layout of a draw (see
        `.axes._program_locators`): its tree as it is now, the boxes among
        the children of a box of the program's, and its offset where a
        callable puts it."""
        if self.__dict__.get("_decoupled"):
            # Removed or cleared: matplotlib's draw no longer reaches it.
            return
        self._sync_tree()
        figure = self.get_figure(root=True)
        if self._sizes_itself() and figure is not None:
            self._join_children(figure)
        if callable(self._offset) and self._places_itself():
            offset = self.get_offset(self.get_bbox(renderer), renderer)
            _offsetbox.set_placed_offset(self._tree_handle, tuple(map(float, offset)))

    def _places_itself(self):
        """Whether the box is a box pltrs (Rust) draws with its origin at its
        offset (`set_offset`): added on its own, or drawn by a box of the
        program's."""
        return "_tree_handle" in self.__dict__ and self._tree_root() is self

    def _adopt_root(self, natives, figure):
        # The box itself is the root of its tree: its package's parts.
        self._adopt_package_parts(natives, figure)

    def _adopt_tree(self, natives, figure):
        """
        Make the artists of the box, in the order of `_native_tree`, the
        objects of the artists the handles *natives* (an iterator) reach in
        *figure*, once the box joined it (the box itself for a box of the
        program's, which then has the boxes among its children join it).
        """
        if not self._sizes_itself():
            self._adopt_package_parts(natives, figure)
            return
        if adopt_part(self, next(natives), figure):
            _python_drawing.paint(self)
        self._laid_out_by_parent = True
        self._join_children(figure)
        follow(self)

    def _adopt_package_parts(self, natives, figure):
        """`_adopt_tree` of the box as the package's class describes it."""
        for child in self.get_children():
            adopt_part(child, next(natives), figure)

    def _place_as_laid_out(self):
        """Give a box of the program's in the tree of a box pltrs (Rust)
        lays out the offset it was placed at, before its ``draw`` runs, as
        matplotlib's parent does."""
        if self.__dict__.get("_laid_out_by_parent"):
            offset = _offsetbox.program_box_offset(self._native)
            if offset is not None:
                self.set_offset(tuple(map(np.float64, offset)))

    def _boxes(self):
        """The box and the boxes of its tree pltrs (Rust) lays out, in tree
        order."""
        yield self
        if not self._sizes_itself() and not self._leaf:
            try:
                kids = list(self.get_children())
            except TypeError:
                # Children matplotlib cannot iterate (see `_offsetbox_trees`).
                return
            for child in kids:
                if isinstance(child, OffsetBox):
                    yield from child._boxes()

    def _joined(self, owner, handle):
        """Have the boxes of the tree of this box know their place in the
        tree pltrs (Rust) laid out under the native *handle*, which *owner*
        added: their figure, Axes and staleness are then its. A box of the
        program's owning a tree of its own keeps that."""
        for index, box in enumerate(self._boxes()):
            if box.__dict__.get("_tree_handle", handle) is handle:
                box._tree_owner = owner
                box._node = (handle, index)

    def _synced_node(self):
        """The box's place in the tree pltrs (Rust) lays out (see
        ``_node``), the tree handed again first if it changed."""
        owner = self.__dict__.get("_tree_owner")
        if owner is not None:
            if owner.__dict__.get("_decoupled"):
                # Removed or cleared, its owner keeps the tree apart:
                # measured with matplotlib's algorithms, at the offsets the
                # last draw gave (`TreeOwner._keep_drawn_state`).
                return None
            owner._sync_tree()
            if owner._tree_unusable():
                # Measured with matplotlib's algorithms, as pltrs (Rust)
                # cannot take the tree.
                return None
        return self._node

    def _laid_out_bbox(self, renderer):
        """The extent of the box where pltrs (Rust) lays it out, relative to
        its origin; None for a box outside any tree, or one of the
        program's not deriving from a box of the package's."""
        if self._sizes_itself():
            handle = self.__dict__.get("_tree_handle")
            node = None if handle is None else (handle, 0)
        else:
            node = self._synced_node()
        if node is None:
            return None
        self._first_steps_with(renderer)
        self._measure_program_boxes(renderer)
        with mtext._measured_by_given(renderer, self):
            return Bbox.from_bounds(*_offsetbox.node_extent(*node))

    #: Whether matplotlib's measure of the box (its
    #: ``_get_bbox_and_child_offsets``) starts converting points with the
    #: renderer, as its packers and padded boxes do.
    _converts_points_first = False

    def _first_steps_with(self, renderer):
        """Use *renderer* as matplotlib's measure of the box first does,
        failing where it fails for what is no renderer (None included)."""
        if self._converts_points_first:
            renderer.points_to_pixels(1.)

    def _measure_program_boxes(self, renderer):
        """Have the boxes of the program's inside this box measure
        themselves (their ``get_bbox``), as matplotlib's parents ask them
        when they are measured."""
        boxes = [box for box in self._boxes()
                 if box is not self and box.__dict__.get("_laid_out_by_parent")]
        if boxes and renderer is None:
            renderer = self.get_figure(root=True)._get_renderer()
        for box in boxes:
            _offsetbox.set_program_extent(
                box._native, tuple(map(float, box.get_bbox(renderer).bounds)))

    def _laid_out_offset(self):
        """Where pltrs (Rust) placed the box's origin when it last drew the
        box holding it, for a box inside a tree another box added, which
        becomes the box's offset as matplotlib's parent sets it then; None
        otherwise, before that draw, and for a box its parent does not
        place (a hidden one), which keeps its offset."""
        if self._sizes_itself() or self.__dict__.get("_tree_owner") is self:
            return None
        node = self._synced_node()
        if node is None:
            return None
        draws = _artist.draw_count(node[0])
        if not draws or self.__dict__.get("_offset_given_at") == draws:
            # Not placed yet, or given an offset by the program since.
            return None
        origin = _offsetbox.node_origin(*node)
        if origin is None:
            return None
        # matplotlib's parents compute offsets with numpy.
        self._offset = tuple(map(np.float64, origin))
        return self._offset

    def _offset_given(self):
        """Note that the program gave the box its offset since the box
        holding it was last drawn: matplotlib's box keeps it until its
        parent places it again, as it draws it."""
        node = self._node
        if node is not None and node[0] is not None:
            self._offset_given_at = _artist.draw_count(node[0])

    def get_window_extent(self, renderer=None):
        # docstring inherited
        if renderer is None:
            renderer = self.get_figure(root=True)._get_renderer()
        bbox = self.get_bbox(renderer)
        try:  # Some subclasses redefine get_offset to take no args.
            px, py = self.get_offset(bbox, renderer)
        except TypeError:
            px, py = self.get_offset()
        return bbox.translated(px, py)

    def draw(self, renderer):
        """
        Update the location of children if necessary and draw them
        to the given *renderer*.
        """
        _unusable.check(self)
        bbox, offsets = self._get_bbox_and_child_offsets(renderer)
        px, py = self.get_offset(bbox, renderer)
        for c, (ox, oy) in zip(self.get_visible_children(), offsets):
            c.set_offset((px + ox, py + oy))
            c.draw(renderer)
        self.stale = False

    def _check_drawing(self, renderer):
        """Reach what matplotlib's draw of the box reaches once it measured
        the box, failing as matplotlib's does on what is no artist (in a
        tree pltrs (Rust) cannot take, see `._offsetbox_trees`): the boxes
        inside it."""
        if self._leaf:
            return
        for child in self.get_visible_children():
            check = getattr(child, "_check_drawing", None)
            if check is not None:
                check(renderer)

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
        self._offset_given()
        self._move_placed(xy)
        if self._laid_out_for is not None:
            self._laid_out_for._laid_out_box_moved(xy)
        self.stale = True

    def _move_placed(self, xy):
        """Move the box to the offset *xy* where pltrs (Rust) draws it, for
        a box drawn at its offset (see `_places_itself`)."""
        if not callable(xy) and self._places_itself():
            _offsetbox.set_placed_offset(self._tree_handle, tuple(map(float, xy)))

    @_compat_get_offset
    def get_offset(self, bbox, renderer):
        """
        Return the offset as a tuple (x, y).

        The extent parameters have to be provided to handle the case where the
        offset is dynamically determined by a callable (see
        `~.OffsetBox.set_offset`).

        Parameters
        ----------
        bbox : `.Bbox`
        renderer : `.RendererBase` subclass
        """
        laid_out = self._laid_out_offset()
        if laid_out is not None:
            return laid_out
        return (
            self._offset(bbox.width, bbox.height, -bbox.x0, -bbox.y0, renderer)
            if callable(self._offset)
            else self._offset)

    def set_width(self, width):
        """
        Set the width of the box.

        Parameters
        ----------
        width : float
        """
        self.width = width
        self.stale = True

    def set_height(self, height):
        """
        Set the height of the box.

        Parameters
        ----------
        height : float
        """
        self.height = height
        self.stale = True

    def get_visible_children(self):
        r"""Return a list of the visible child `.Artist`\s."""
        return [c for c in self._children if c.get_visible()]

    def get_children(self):
        r"""Return a list of the child `.Artist`\s."""
        return self._children

    def contains(self, mouseevent):
        """
        Delegate the mouse event contains-check to the children.

        As a container, the `.OffsetBox` does not respond itself to
        mouseevents.

        Parameters
        ----------
        mouseevent : `~matplotlib.backend_bases.MouseEvent`

        Returns
        -------
        contains : bool
            Whether any values are within the radius.
        details : dict
            An artist-specific dictionary of details of the event context,
            such as which points are contained in the pick radius. See the
            individual Artist subclasses for details.

        See Also
        --------
        .Artist.contains
        """
        return super().contains(mouseevent)

    def get_bbox(self, renderer):
        """Return the bbox of the offsetbox, ignoring parent offsets."""
        laid_out = self._laid_out_bbox(renderer)
        if laid_out is not None:
            return laid_out
        bbox, offsets = self._get_bbox_and_child_offsets(renderer)
        return bbox

    def _drawer(self):
        # The artist added with the tree draws it (see `._drawn_with`); the
        # box that added it draws itself.
        owner = self.__dict__.get("_tree_owner")
        return None if owner is self else owner

    def _get_figure(self, root):
        """Return the `.Figure` or `.SubFigure` the box is on: that of the
        artist added with its tree, for a box inside another, which keeps it
        once that artist is removed or cleared, as matplotlib's keeps the
        figure the artist gave it, or that of the artist it was laid out
        for."""
        owner = self._drawer()
        if owner is None:
            owner = self._laid_out_for
        if self._figure is None and owner is not None:
            return owner._home_figure(root=root)
        return super()._get_figure(root)

    def _loose_axes(self):
        # A box laid out for an artist is in that artist's Axes. A box
        # inside another is in the Axes it was given (an Axes listing it, see
        # `.axes._foreign_children`), else in the Axes of the box that added
        # the tree; inside an artist's tree, in none.
        if self._laid_out_for is not None:
            return self._laid_out_for.axes
        owner = self._drawer()
        given = super()._loose_axes()
        if owner is None or given is not None:
            return given
        return owner.axes if isinstance(owner, OffsetBox) else None


class PackerBase(OffsetBox):
    _converts_points_first = True

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

            - 'fixed' packs the given `.Artist`\\s tight with *sep* spacing.
            - 'expand' uses the maximal available space to distribute the
              artists with equal spacing in between.
            - 'equal': Each artist an equal fraction of the available space
              and is left-aligned (or top-aligned) therein.

        children : list of `.Artist`
            The artists to pack.

        Notes
        -----
        *pad* and *sep* are in points and will be scaled with the renderer
        dpi, while *width* and *height* are in pixels.
        """
        super().__init__()
        self.height = height
        self.width = width
        self.sep = sep
        self.pad = pad
        self.mode = mode
        self.align = align
        self._children = children

    pad = TreeSetting()
    sep = TreeSetting()
    width = TreeSetting()
    height = TreeSetting()
    align = TreeSetting()
    mode = TreeSetting()

    def _package_tree(self):
        kind = "vpacker" if isinstance(self, VPacker) else "hpacker"
        return (kind, self.get_visible(), number(self.pad), number(self.sep),
                optional_number(self.width), optional_number(self.height),
                choice(self.align, PACK_ALIGNS), choice(self.mode, PACK_MODES),
                [box_tree(child) for child in children(self)])

    def _adopt_package_parts(self, natives, figure):
        for child in self.get_children():
            child._adopt_tree(natives, figure)


class VPacker(PackerBase):
    """
    VPacker packs its children vertically, automatically adjusting their
    relative positions at draw time.

    .. code-block:: none

       +---------+
       | Child 1 |
       | Child 2 |
       | Child 3 |
       +---------+
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

    .. code-block:: none

       +-------------------------------+
       | Child 1    Child 2    Child 3 |
       +-------------------------------+
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

    The `.PaddedBox` contains a `.FancyBboxPatch` that is used to visualize
    it when rendering.

    .. code-block:: none

       +----------------------------+
       |                            |
       |                            |
       |                            |
       | <--pad--> Artist           |
       |             ^              |
       |            pad             |
       |             v              |
       +----------------------------+

    Attributes
    ----------
    pad : float
        The padding in points.
    patch : `.FancyBboxPatch`
        When *draw_frame* is True, this `.FancyBboxPatch` is made visible and
        creates a border around the box.
    """

    pad = TreeSetting()
    _converts_points_first = True

    def __init__(self, child, pad=0., *, draw_frame=False, patch_attrs=None):
        """
        Parameters
        ----------
        child : `~matplotlib.artist.Artist`
            The contained `.Artist`.
        pad : float, default: 0.0
            The padding in points. This will be scaled with the renderer dpi.
            In contrast, *width* and *height* are in *pixels* and thus not
            scaled.
        draw_frame : bool
            Whether to draw the contained `.FancyBboxPatch`.
        patch_attrs : dict or None
            Additional parameters passed to the contained `.FancyBboxPatch`.
        """
        from .patches import FancyBboxPatch
        super().__init__()
        self.pad = pad
        self._children = [child]
        # matplotlib's square frame without padding, white with a black
        # edge in its look, shown with *draw_frame*.
        if patch_attrs is not None:
            # matplotlib's frame is made, then updated with them.
            patch_attrs.items  # noqa: B018
        self.patch = FancyBboxPatch._from_native(_offsetbox.new_padded_frame(
            bool(draw_frame),
            cbook.normalize_kwargs(dict(patch_attrs or {}), FancyBboxPatch)), None)

    def _package_tree(self):
        if _unusable.keeps(self.patch):
            # A frame matplotlib's draw fails on (a mutation scale that is
            # no number).
            raise Unusable
        return ("padded", self.get_visible(), number(self.pad),
                box_tree(self._children[0]), self.patch._native)

    def _check_drawing(self, renderer):
        # docstring inherited: matplotlib's draw draws the frame, then the
        # child.
        if self.patch.get_visible():
            _unusable.check(self.patch)
        super()._check_drawing(renderer)

    def _adopt_package_parts(self, natives, figure):
        adopt_part(self.patch, next(natives), figure)
        self._children[0]._adopt_tree(natives, figure)

    def _get_bbox_and_child_offsets(self, renderer):
        """The bbox of the box and the offset of its child."""
        pad = self.pad * renderer.points_to_pixels(1.)
        return (self._children[0].get_bbox(renderer).padded(pad), [(0, 0)])

    def draw(self, renderer):
        # docstring inherited
        _unusable.check(self)
        bbox, offsets = self._get_bbox_and_child_offsets(renderer)
        px, py = self.get_offset(bbox, renderer)
        for c, (ox, oy) in zip(self.get_visible_children(), offsets):
            c.set_offset((px + ox, py + oy))
        self.draw_frame(renderer)
        for c in self.get_visible_children():
            c.draw(renderer)
        self.stale = False

    def update_frame(self, bbox, fontsize=None):
        # pltrs: pltrs (Rust) fits the frame of a box it lays out again
        # whenever it lays the figure out.
        _update_frame(self, bbox, fontsize)
        self.stale = True

    def draw_frame(self, renderer):
        # update the location and size of the legend
        self.update_frame(self.get_window_extent(renderer))
        self.patch.draw(renderer)


class _FollowsLayout:
    """
    A leaf box whose transforms matplotlib sets as it draws it: its
    ``offset_transform`` (the parent's ``set_offset``), and what the
    box's own draw sets (`_followed_layout`). They read where pltrs (Rust)
    last laid the box out, once it did.
    """

    @property
    def offset_transform(self):
        """The translation to the box's offset (`set_offset`)."""
        self._follow_layout()
        return self._offset_transform

    @offset_transform.setter
    def offset_transform(self, transform):
        self._offset_transform = transform

    def _follow_layout(self):
        """Give the box's transforms what matplotlib's draw gives them where
        pltrs (Rust) last laid the box out."""
        offset = self._laid_out_offset()
        if offset is not None:
            self._offset_transform.clear().translate(*offset)
            self._followed_layout()

    def _followed_layout(self):
        """Set what the box's own draw sets, once laid out."""


class DrawingArea(_FollowsLayout, OffsetBox):
    """
    The DrawingArea can contain any Artist as a child. The DrawingArea
    has a fixed width and height. The position of children relative to
    the parent is fixed. The children can be clipped at the
    boundaries of the parent.
    """

    _leaf = True
    width = TreeSetting()
    height = TreeSetting()
    xdescent = TreeSetting()
    ydescent = TreeSetting()

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

    @property
    def dpi_transform(self):
        """The scaling of points to pixels the area's draw sets."""
        self._follow_layout()
        return self._dpi_transform

    @dpi_transform.setter
    def dpi_transform(self, transform):
        self._dpi_transform = transform

    def _followed_layout(self):
        # matplotlib's draw scales points to the renderer's pixels.
        self._dpi_transform.clear().scale(self.get_figure(root=True).dpi / 72)

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
        self._changed()

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
        self._offset_given()
        self.offset_transform.clear()
        self.offset_transform.translate(xy[0], xy[1])
        self._move_placed(xy)
        self.stale = True

    def get_offset(self):
        """Return offset of the container."""
        laid_out = self._laid_out_offset()
        return self._offset if laid_out is None else laid_out

    def get_bbox(self, renderer):
        dpi_cor = renderer.points_to_pixels(1.)
        return Bbox.from_bounds(
            -self.xdescent * dpi_cor, -self.ydescent * dpi_cor,
            self.width * dpi_cor, self.height * dpi_cor)

    def add_artist(self, a):
        """Add an `.Artist` to the container box."""
        self._children.append(a)
        self._changed()
        # matplotlib's asks whether the artist has a transform, failing on
        # what is no artist (pltrs (Rust) places the area's artists).
        a.is_transform_set()

    def _package_tree(self):
        return ("drawing", self.get_visible(), number(self.width),
                number(self.height), number(self.xdescent), number(self.ydescent),
                bool(self._clip_children),
                [artist_handle(child) for child in self._children])

    def _check_drawing(self, renderer):
        # docstring inherited: matplotlib's draw reads the clip of each
        # artist it clips to the area, then draws it.
        for child in self._children:
            for name in ("clipbox", "draw") if self._clip_children else ("draw",):
                getattr(child, name)

    def draw(self, renderer):
        # Draw the children, where pltrs (Rust) places them: in points from
        # the area's offset, clipped to the area if it clips them.
        _unusable.check(self)
        dpi_cor = renderer.points_to_pixels(1.)
        self.dpi_transform.clear()
        self.dpi_transform.scale(dpi_cor)
        for c in self._children:
            c.draw(renderer)
        self.stale = False


class TextArea(_FollowsLayout, OffsetBox):
    """
    The TextArea is a container artist for a single Text instance.

    The text is placed at (0, 0) with baseline+left alignment, by default. The
    width and height of the TextArea instance is the width and height of its
    child text.
    """

    _leaf = True

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
        """Make the area of the `.Text` *text*, which it places through
        ``offset_transform + _baseline_transform``, as matplotlib does."""
        self._area_text = text
        super().__init__()
        self._children = [text]
        self.offset_transform = mtransforms.Affine2D()
        self._baseline = mtransforms.Affine2D()
        # Kept on the Python side: pltrs (Rust) gives the text the
        # translation these make whenever it lays the area out.
        text._given_transform = self._offset_transform + self._baseline
        self._multilinebaseline = multilinebaseline

    @classmethod
    def _of_text(cls, text, multilinebaseline):
        """The area of *text*, a `.Text` pltrs (Rust) laid out in a box."""
        area = cls.__new__(cls)
        area._init_area(text, multilinebaseline)
        return area

    @property
    def _text(self):
        """The `.Text` of the area."""
        self._follow_layout()
        return self._area_text

    @_text.setter
    def _text(self, text):
        self._area_text = text

    @property
    def _baseline_transform(self):
        """The shift of multi-line text centred on the baseline, which
        `get_bbox` computes."""
        self._follow_layout()
        return self._baseline

    @_baseline_transform.setter
    def _baseline_transform(self, transform):
        self._baseline = transform

    def _followed_layout(self):
        # matplotlib's draw measures the area (`get_bbox`), which shifts
        # its text.
        self._set_baseline()

    def _measured_dpi(self):
        """The dpi matplotlib measures the text at where pltrs (Rust) does
        not: that of the figure the area's box was given (a box a program
        makes and draws in its own `draw`, matplotlib-scalebar's) for a
        text in no figure, else None (its figure's)."""
        given = self.__dict__.get("_figure_given")
        if given is None or self._area_text.get_figure(root=False) is not None:
            return None
        return float(given.get_figure(root=True).dpi)

    def _set_baseline(self):
        """Shift the text as matplotlib's `get_bbox` does: multi-line text
        centred on the baseline with *multilinebaseline*, else not."""
        shift = _offsetbox.text_area_baseline(
            self._area_text._native, bool(self._multilinebaseline),
            self._measured_dpi())
        self._baseline.clear()
        if shift:
            self._baseline.translate(0, shift)

    def set_text(self, s):
        """Set the text of this area as a string."""
        self._area_text.set_text(s)
        self.stale = True

    def get_text(self):
        """Return the string representation of this area's text."""
        return self._area_text.get_text()

    def set_multilinebaseline(self, t):
        """
        Set multilinebaseline.

        If True, the baseline for multiline text is adjusted so that it is
        (approximately) center-aligned with single-line text.  This is used
        e.g. by the legend implementation so that single-line labels are
        baseline-aligned, but multiline labels are "center"-aligned with them.
        """
        self._multilinebaseline = t
        self._changed()

    def get_multilinebaseline(self):
        """
        Get multilinebaseline.
        """
        return self._multilinebaseline

    def _package_tree(self):
        if self.__dict__.get("_joined_late"):
            # No figure to measure the text at, as matplotlib's.
            raise Unusable
        return ("text", self.get_visible(), self._area_text._native,
                bool(self._multilinebaseline))

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
        self._offset_given()
        self.offset_transform.clear()
        self.offset_transform.translate(xy[0], xy[1])
        self._move_placed(xy)
        self.stale = True

    def get_offset(self):
        """Return offset of the container."""
        laid_out = self._laid_out_offset()
        return self._offset if laid_out is None else laid_out

    def get_bbox(self, renderer):
        # Return the bbox of the text, ignoring parent offsets: its extent, at
        # least as tall as ``"lp"`` above its baseline, multi-line text
        # centred on the baseline with *multilinebaseline* (pltrs (Rust)
        # measures it, as it lays the area out).
        laid_out = self._laid_out_bbox(renderer)
        if laid_out is None and (
                (self._area_text.get_figure(root=True) is None and self._drawer() is None
                 and "_figure_given" not in self.__dict__)
                or self.__dict__.get("_joined_late")):
            # matplotlib measures the text at the dpi of the area's figure (a
            # box of a tree has its owner's, unless it joined the tree late,
            # see `._offsetbox_trees.TreeOwner._note_late_boxes`),
            raise AttributeError("'NoneType' object has no attribute 'dpi'")
        # with *renderer*, whose metrics it looks up in a weak cache
        # (``_get_text_metrics_function``), which refuses what cannot be
        # referred to weakly.
        weakref.ref(renderer)
        if laid_out is not None:
            return laid_out
        self._set_baseline()
        return Bbox.from_bounds(*_offsetbox.text_area_extent(
            self._area_text._native, bool(self._multilinebaseline),
            self._measured_dpi()))

    def draw(self, renderer):
        # docstring inherited
        _unusable.check(self)
        self._area_text.draw(renderer)
        self.stale = False


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


class AuxTransformBox(_FollowsLayout, OffsetBox):
    """
    An OffsetBox with an auxiliary transform.

    All child artists are first transformed with *aux_transform*, then
    translated with an offset (the same for all children) so the bounding
    box of the children matches the drawn box.  (In other words, adding an
    arbitrary translation to *aux_transform* has no effect as it will be
    cancelled out by the later offsetting.)

    `AuxTransformBox` is similar to `.DrawingArea`, except that the extent of
    the box is not predetermined but calculated from the window extent of its
    children, and the extent of the children will be calculated in the
    transformed coordinate.
    """

    _leaf = True

    def __init__(self, aux_transform):
        self.aux_transform = aux_transform
        super().__init__()
        self.offset_transform = mtransforms.Affine2D()
        # ref_offset_transform makes offset_transform always relative to the
        # lower-left corner of the bbox of its children.
        self.ref_offset_transform = mtransforms.Affine2D()

    def add_artist(self, a):
        """Add an `.Artist` to the container box."""
        # matplotlib's leaves an artist without a figure (``figure`` None)
        # without one, as it does not give the box's to those it adds.
        self._children.append(a)
        # (Asked before the transform, which may bring the artist into the
        # figure; what is no artist fails on the transform, as matplotlib's.)
        figureless = (self.get_figure(root=False) is not None
                      and hasattr(a, "set_transform") and a.get_figure() is None)
        a.set_transform(self.get_transform())
        if figureless:
            a._no_figure = True
        self._changed()

    def _package_tree(self):
        return ("aux", self.get_visible(),
                _live_transforms.to_native(self.aux_transform),
                [artist_handle(child) for child in self._children])

    def _adopt_package_parts(self, natives, figure):
        # docstring inherited; pltrs (Rust) places the children of a box it
        # lays out, which then read back the transform it gives them.
        super()._adopt_package_parts(natives, figure)
        for child in self._children:
            child.__dict__.pop("_given_transform", None)
            if child.__dict__.get("_no_figure"):
                _artist.set_figureless(child._native, True)

    def get_bbox(self, renderer):
        # Return the bbox of the children under the aux transform alone, its
        # lower left corner at the origin: where pltrs (Rust) lays the box
        # out, else as matplotlib measures it, which moves the children's
        # lower left corner to the origin (``ref_offset_transform``).
        laid_out = self._laid_out_bbox(renderer)
        if laid_out is not None:
            return laid_out
        offset = self.offset_transform.get_matrix()
        self.ref_offset_transform.clear()
        self.offset_transform.clear()
        union = Bbox.union([c.get_window_extent(renderer) for c in self._children])
        self.ref_offset_transform.translate(-union.x0, -union.y0)
        self.offset_transform.set_matrix(offset)
        return Bbox.from_bounds(0, 0, union.width, union.height)

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
        self._offset_given()
        self.offset_transform.clear()
        self.offset_transform.translate(xy[0], xy[1])
        self._move_placed(xy)
        self.stale = True

    def get_offset(self):
        """Return offset of the container."""
        laid_out = self._laid_out_offset()
        return self._offset if laid_out is None else laid_out

    def draw(self, renderer):
        # Draw the children, where pltrs (Rust) places them: through the aux
        # transform, then from the box's offset.
        _unusable.check(self)
        for c in self._children:
            if (c.__dict__.get("_no_figure") and isinstance(c, mtext.Text)
                    and c.get_visible() and c.get_text()):
                # matplotlib's lays the text out at the resolution of the
                # figure it does not have.
                raise AttributeError("'NoneType' object has no attribute 'dpi'")
            c.draw(renderer)
        self.stale = False


def _class_zorder(box, kwargs, zorder):
    """Give *box* the z-order *zorder* matplotlib's class declares (a class
    attribute, set through no setter), unless it is given one or its class
    of the program's declares its own."""
    if "zorder" not in kwargs and not takes_from_program(type(box), "zorder"):
        _artist.set_zorder(box._native, float(zorder))


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
            See the parameter *loc* of `.Legend` for details.
        pad : float, default: 0.4
            Padding around the child as fraction of the fontsize.
        borderpad : float or (float, float), default: 0.5
            Padding between the offsetbox frame and the *bbox_to_anchor*.
            If a float, the same padding is used for both x and y.
            If a tuple of two floats, it specifies the (x, y) padding.

            .. versionadded:: 3.11
               The *borderpad* parameter now accepts a tuple of (x, y) paddings.
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

        Notes
        -----
        See `.Legend` for a detailed description of the anchoring mechanism.
        """
        super().__init__(**kwargs)
        _class_zorder(self, kwargs, 5)
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
        from .patches import FancyBboxPatch
        # The frame: matplotlib's square box without padding, white with a
        # black edge in its look, which pltrs (Rust) draws once the box is
        # added and fits to the box whenever it lays the box out.
        self.patch = FancyBboxPatch._from_native(
            _offsetbox.new_frame(bool(frameon)), None)
        self.patch.set_mutation_scale(self.prop.get_size_in_points())

    def _add_to_axes(self, axes, kind, autolim=True):
        if self._comes_back(axes):
            return Artist._add_to_axes(self, axes, kind, autolim)
        # matplotlib's `add_artist` sets the Axes of the box and its children
        # first: the box anchors to it.
        self.axes = axes
        return self._join(axes)

    def _add_to_figure(self, figure, clip):
        if self._comes_back(figure):
            return Artist._add_to_figure(self, figure, clip)
        return self._join(figure)

    def _native_anchor(self):
        """``bbox_to_anchor`` and ``bbox_transform`` as pltrs (Rust) takes
        them: the bounds of the box (a point for a box of no size) and the
        native transform of its coordinates (None: display pixels);
        `Unusable` for a transform that is none, which matplotlib fails on
        as it anchors the box (`TransformedBbox`)."""
        bbox, transform = self._bbox_to_anchor, self._bbox_to_anchor_transform
        if transform is not None and not isinstance(transform, Transform):
            raise Unusable
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

    #: The names under which the box keeps what pltrs (Rust) cannot take:
    #: its tree, and its anchoring.
    _problems = ("tree", "anchor")

    def _native_options(self):
        """The settings of the box as pltrs (Rust) takes them; `Unusable`
        for one it cannot take, which matplotlib computes with as it
        measures the box: a padding no number, a location no code, no box
        to anchor to (that of no Axes)."""
        if not (isinstance(self.loc, numbers.Integral) and 1 <= self.loc <= 10):
            raise Unusable
        if self._bbox_to_anchor is None and self.axes is None:
            raise Unusable
        try:
            borderpad = tuple(map(number, self.borderpad))
        except TypeError:
            borderpad = number(self.borderpad)
        bounds, transform = self._native_anchor()
        return {"loc": self.loc, "pad": number(self.pad),
                "borderpad": borderpad,
                "prop": FontManager._props(self.prop),
                "frameon": bool(self.frameon), "bbox_to_anchor": bounds,
                "bbox_transform": transform}

    def _options_to_join(self):
        """The settings the box joins a figure with: its own, or
        matplotlib's defaults without a frame while pltrs (Rust) cannot take
        them (the box then fails to draw, as matplotlib's does)."""
        try:
            return self._native_options()
        except Unusable:
            return {"loc": 1, "pad": 0.4, "borderpad": 0.5,
                    "prop": FontManager._props(self.prop), "frameon": False,
                    "bbox_to_anchor": None, "bbox_transform": None}

    def _package_tree(self):
        # The box inside another box: drawn at its anchor all the same.
        return ("anchored", self.get_visible(), box_tree(self.get_child()),
                self._native_options(), self.patch._native)

    def _adopt_package_parts(self, natives, figure):
        adopt_part(self.patch, next(natives), figure)
        self.get_child()._adopt_tree(natives, figure)

    def _tree_root(self):
        # An anchored box added to an axes or a figure owns its child's tree.
        return self.get_child()

    def _adopt_root(self, natives, figure):
        self.get_child()._adopt_tree(natives, figure)

    def _sync_tree(self):
        # The tree, then the settings matplotlib reads again at every draw.
        super()._sync_tree()
        if "_tree_handle" in self.__dict__ and _python_drawing.figure_changeable():
            try:
                options = self._native_options()
            except Unusable:
                self._keep_problem("anchor")
                return
            _unusable.drop(self, "anchor")
            if options != self._synced_options:
                _offsetbox.set_anchored_options(self._tree_handle, options)
                self._synced_options = options

    def _fail_as_drawn(self, _kept):
        # docstring inherited: matplotlib's draw measures the box where it
        # anchors it, then the child it draws.
        renderer = _python_drawing.drawing_renderer(self)
        OffsetBox.get_window_extent(self, renderer)
        check = getattr(self.get_child(), "_check_drawing", None)
        if check is not None:
            check(renderer)

    # (Named as matplotlib's, which the class inherits from `OffsetBox`.)
    @_named.named("OffsetBox.get_window_extent")
    def get_window_extent(self, renderer=None):
        """Return the box's extent in display space (for a box inside
        another box's tree, its frame's: the box drawn at its anchor), the
        boxes of the program's inside measured as they are now."""
        self._check_figure_renderer(renderer)
        if renderer is not None:
            # What matplotlib's `get_bbox` asks of the renderer first.
            renderer.points_to_pixels(self.prop.get_size_in_points())
        owner = self.__dict__.get("_tree_owner", self)
        owner._sync_tree()
        if owner._tree_unusable() or (owner is self and "_tree_handle" not in self.__dict__):
            # matplotlib's measure, failing where matplotlib's does on what
            # pltrs (Rust) cannot take, and of a box added nowhere (one a
            # program makes and draws in its own `draw`).
            return OffsetBox.get_window_extent(self, renderer)
        self.get_child()._measure_program_boxes(renderer)
        if self._synced_node() is not None:
            return self.patch._laid_out_extent(renderer)
        self._sync_tree()
        return self._laid_out_extent(renderer)

    def _join(self, parent):
        """Join *parent*, an axes or a figure, with the child tree: pltrs
        (Rust) anchors the box there; return self."""
        options = self._options_to_join()
        described = self._tree_to_join()
        native, parts, frame = _offsetbox.add_anchored(
            parent._native, self._native, described, options, self.patch._native)
        figure = parent.figure
        _handles.adopt(self.patch, frame, figure)
        _handles.adopt(self, native, figure)
        self._synced_options = options
        self._own_tree(parts, figure, native, described)
        self._sync_tree()
        _python_drawing.attach(self)
        return self

    def _locate_by_program(self, renderer):
        # docstring inherited; the child then goes where a ``get_offset``
        # of the program's puts it, as matplotlib's ``draw`` asks it.
        super()._locate_by_program(renderer)
        if takes_from_program(type(self), "get_offset") and not self.__dict__.get("_decoupled"):
            offset = self.get_offset(self.get_bbox(renderer), renderer)
            _offsetbox.set_anchored_offset(self._tree_handle,
                                           tuple(map(float, offset)))

    def get_bbox(self, renderer):
        fontsize = renderer.points_to_pixels(self.prop.get_size_in_points())
        pad = self.pad * fontsize
        return self.get_child().get_bbox(renderer).padded(pad)

    @_compat_get_offset
    def get_offset(self, bbox, renderer):
        fontsize_in_pixels = renderer.points_to_pixels(self.prop.get_size_in_points())
        try:
            borderpad_x, borderpad_y = self.borderpad
        except TypeError:
            borderpad_x = self.borderpad
            borderpad_y = self.borderpad
        pad_x_pixels = borderpad_x * fontsize_in_pixels
        pad_y_pixels = borderpad_y * fontsize_in_pixels
        bbox_to_anchor = self.get_bbox_to_anchor()
        x0, y0 = _get_anchored_bbox(
            self.loc,
            Bbox.from_bounds(0, 0, bbox.width, bbox.height),
            bbox_to_anchor,
            pad_x_pixels,
            pad_y_pixels
        )
        return x0 - bbox.x0, y0 - bbox.y0

    def draw(self, renderer):
        # Draw the frame and the child with *renderer*: where pltrs (Rust)
        # draws the box, for a box added to an axes or a figure; else at its
        # anchor, as matplotlib's does.
        if not self.get_visible():
            return
        if "_tree_handle" in self.__dict__:
            Artist.draw(self, renderer)
            return
        bbox = self.get_window_extent(renderer)
        fontsize = renderer.points_to_pixels(self.prop.get_size_in_points())
        self.update_frame(bbox, fontsize)
        self.patch.draw(renderer)
        px, py = self.get_offset(self.get_bbox(renderer), renderer)
        self.get_child().set_offset((px, py))
        self.get_child().draw(renderer)
        self.stale = False

    def update_frame(self, bbox, fontsize=None):
        # pltrs: the frame is the patch of the box pltrs (Rust) draws, which
        # exists once the box joined a figure; pltrs fits it again whenever it
        # lays the figure out.
        _update_frame(self, bbox, fontsize)

    def set_child(self, child):
        """Set the child to be anchored."""
        self._child = child
        if child is not None:
            child.axes = self.axes
        self._changed()

    def get_child(self):
        """Return the child."""
        return self._child

    def get_children(self):
        """Return the list of children."""
        return [self._child]

    def get_bbox_to_anchor(self):
        """Return the bbox that the box is anchored to."""
        if self._bbox_to_anchor is None:
            return self.axes.bbox
        transform = self._bbox_to_anchor_transform
        if transform is None:
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
    Container artist for images.

    Image data is displayed using `.BboxImage`. This image is meant to be positioned
    relative to a parent artist.

    Parameters
    ----------
    arr: array-like or `PIL.Image.Image`
        The data to be color-coded. The interpretation depends on the
        shape:

        - (M, N) `~numpy.ndarray` or masked array: values to be colormapped
        - (M, N, 3): RGB array
        - (M, N, 4): RGBA array

    zoom: float, default: 1
        zoom factor:

        - no zoom: factor =1
        - zoom in: factor > 1
        - zoom out: 0< factor < 1

    cmap : str or `~matplotlib.colors.Colormap`, default: :rc:`image.cmap`
        The Colormap instance or registered colormap name used to map scalar
        data to colors. This parameter is ignored if X is RGB(A).

    norm : str or `~matplotlib.colors.Normalize`, default: None
        Maps luminance to 0-1. This parameter is ignored if X is RGB(A).

    interpolation : str, default: :rc:`image.interpolation`
        Supported values are 'none', 'auto', 'nearest', 'bilinear',
        'bicubic', 'spline16', 'spline36', 'hanning', 'hamming', 'hermite',
        'kaiser', 'quadric', 'catrom', 'gaussian', 'bessel', 'mitchell',
        'sinc', 'lanczos', 'blackman'.

    origin : {'upper', 'lower'}, default: :rc:`image.origin`
        Place the [0, 0] index of the array in the upper left or lower left
        corner of the Axes. The convention 'upper' is typically used for
        matrices and images.

    filternorm : bool, default: True
        A parameter for the antigrain image resize filter
        (see the antigrain documentation).
        If filternorm is set, the filter normalizes integer values and corrects
        the rounding errors. It doesn't do anything with the source floating
        point values, it corrects only integers according to the rule of 1.0
        which means that any sum of pixel weights must be equal to 1.0. So,
        the filter function must produce a graph of the proper shape.

    filterrad : float > 0, default: 4
        The filter radius for filters that have a radius parameter, i.e. when
        interpolation is one of: 'sinc', 'lanczos' or 'blackman'.

    resample : bool, default: False
        When True, use a full resampling method. When False, only resample when
        the output image is larger than the input image.

    dpi_cor: bool, default: True
        Correct for the backend DPI setting

    **kwargs : `.BboxImage` properties

    """

    _leaf = True

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
        from .image import BboxImage
        self._data = np.asarray(arr)
        image = getattr(self, "image", None)
        if image is not None and image.figure is not None:
            image.set_data(self._data)
        else:
            self.image = BboxImage._loose(self._data, **self._image_options)
            self._children = [self.image]
        self.stale = True

    def _package_tree(self):
        if np.ndim(self.get_data()) < 2:
            # matplotlib's sizes the box by the rows and columns of the data.
            raise Unusable
        return ("image", self.get_visible(), self.image._native,
                number(self.get_zoom()), bool(self._dpi_cor))

    def get_data(self):
        return self._data

    def get_children(self):
        return super().get_children()

    def set_zoom(self, zoom):
        self._zoom = zoom
        self._changed()

    def get_zoom(self):
        return self._zoom

    def get_offset(self):
        """Return offset of the container."""
        # Return offset of the container: where the box holding it placed it
        # when it was last drawn (pltrs (Rust) lays the box out), the offset
        # given otherwise.
        laid_out = self._laid_out_offset()
        return self._offset if laid_out is None else laid_out

    def draw(self, renderer):
        # docstring inherited
        _unusable.check(self)
        self.image.draw(renderer)
        self.stale = False

    def get_window_extent(self, renderer=None):
        """Return the image's box in display space: its size at its zoom,
        at its offset (matplotlib's)."""
        if renderer is None:
            figure = self.image.get_figure(root=True)
            renderer = None if figure is None else figure._get_renderer()
        return self.get_bbox(renderer).translated(*self.get_offset())

    def get_bbox(self, renderer):
        dpi_cor = renderer.points_to_pixels(1.) if self._dpi_cor else 1.
        zoom = self.get_zoom()
        data = self.get_data()
        ny, nx = data.shape[:2]
        w, h = dpi_cor * nx * zoom, dpi_cor * ny * zoom
        return Bbox.from_bounds(0, 0, w, h)


class AnnotationBbox(TreeOwner, Artist, mtext._AnnotationBase):
    """
    Container for an `OffsetBox` referring to a specific position *xy*.

    Optionally an arrow pointing from the offsetbox to *xy* can be drawn.

    This is like `.Annotation`, but with `OffsetBox` instead of `.Text`.
    """

    def set_figure(self, fig):
        # docstring inherited: matplotlib's override has none.
        super().set_figure(fig)

    # matplotlib's draw of the class is not decorated with
    # `allow_rasterization`: asking to rasterize it warns.
    _ignores_rasterization = True

    #: matplotlib's draw of the box reads no transform of its own (see
    #: `._python_drawing.evaluate_given_transform`).
    _draw_reads_transform = False

    def get_window_extent(self, renderer=None):
        # Return the annotation's box in display space, where pltrs (Rust)
        # places it in the figure as it is now (matplotlib places the box when
        # it is measured, without laying out the figure).
        return self._placed_extent(renderer)

    def get_tightbbox(self, renderer=None):
        return self._placed_extent(renderer)

    def contains(self, mouseevent):
        # (pltrs (Rust) hit-tests the child box while the box is shown.)
        return super().contains(mouseevent)

    def draw(self, renderer):
        # docstring inherited
        super().draw(renderer)

    def _placed_extent(self, renderer):
        """The box placed in the figure as it is now, measured with
        *renderer* (by default its figure's); a box outside any figure has
        no renderer to be measured with, as in matplotlib."""
        self._check_figure_renderer(None)
        self._sync_tree()
        self.update_positions(self.get_figure(root=True)._get_renderer()
                              if renderer is None else renderer)
        from ._annotation_arrow import check_assigned
        check_assigned(self)
        return self._extent_as_is()

    def _tree_root(self):
        # The annotation box owns the tree of its offset box.
        return self.offsetbox

    def _adopt_root(self, natives, figure):
        # docstring inherited: matplotlib's annotation box gives the boxes
        # of its tree no Axes, so the artists inside them keep the one the
        # program gave them (none), where pltrs (Rust) holds them in the
        # annotation box's.
        super()._adopt_root(natives, figure)
        for box in self.offsetbox._boxes():
            if box._leaf:
                for artist in box.get_children():
                    artist._axes_as_given = True

    #: The names under which the box keeps what pltrs (Rust) cannot take:
    #: its tree, the annotated point, the position of the box and their
    #: coordinate systems.
    _problems = ("tree", "xy", "xybox", "coords")

    def _fail_as_drawn(self, _kept):
        # docstring inherited: matplotlib's draw checks the annotated point,
        # then places the box (``update_positions``) and draws it.
        renderer = _python_drawing.drawing_renderer(self)
        if not self._check_xy(renderer):
            return
        ox0, oy0 = self._get_xy(renderer, self.xybox, self.boxcoords)
        bbox = self.offsetbox.get_bbox(renderer)
        fw, fh = self._box_alignment
        self.offsetbox.set_offset(
            (ox0 - fw*bbox.width - bbox.x0, oy0 - fh*bbox.height - bbox.y0))
        bbox = self.offsetbox.get_window_extent(renderer)
        renderer.points_to_pixels(self.get_fontsize())
        if self.arrowprops:
            arrow_begin = bbox.p0 + bbox.size * self._arrow_relpos
            arrow_end = self._get_position_xy(renderer)
            if not np.isfinite([*arrow_begin, *arrow_end]).all():
                self._connect_arrow(arrow_begin, arrow_end, renderer)
        self.offsetbox._check_drawing(renderer)

    def _connect_arrow(self, begin, end, renderer):
        """Fail as matplotlib's arrow from *begin* to *end* (display points,
        not all finite) fails as it draws: its connection style joins them
        and clips the path by the frame (or *patchA*) and the shrinking; a
        path the NaNs leave empty has no first segment to clip."""
        arrow = self.arrow_patch
        dpi_cor = renderer.points_to_pixels(1.)
        arrow.get_connectionstyle()(
            begin, end, patchA=self.arrowprops.get("patchA", self.patch),
            patchB=arrow.patchB, shrinkA=arrow.shrinkA * dpi_cor,
            shrinkB=arrow.shrinkB * dpi_cor)

    def _locate_by_program(self, renderer):
        """Hand pltrs (Rust) the offset box as it is now before a layout of
        a draw (see `.axes._program_locators`), as matplotlib reads it
        again whenever it draws it."""
        self._sync_tree()

    def __str__(self):
        return f"AnnotationBbox({self.xy[0]:g},{self.xy[1]:g})"

    @_docstring.interpd
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
            The coordinate system that *xy* is given in. See the parameter
            *xycoords* in `.Annotation` for a detailed description.

        boxcoords : single or two-tuple of str or `.Artist` or `.Transform` \
or callable, default: value of *xycoords*
            The coordinate system that *xybox* is given in. See the parameter
            *textcoords* in `.Annotation` for a detailed description.

        frameon : bool, default: True
            By default, the text is surrounded by a white `.FancyBboxPatch`
            (accessible as the ``patch`` attribute of the `.AnnotationBbox`).
            If *frameon* is set to False, this patch is made invisible.

        annotation_clip: bool or None, default: None
            Whether to clip (i.e. not draw) the annotation when the annotation
            point *xy* is outside the Axes area.

            - If *True*, the annotation will be clipped when *xy* is outside
              the Axes.
            - If *False*, the annotation will always be drawn.
            - If *None*, the annotation will be clipped when *xy* is outside
              the Axes and *xycoords* is 'data'.

        pad : float, default: 0.4
            Padding around the offsetbox.

        box_alignment : (float, float)
            A tuple of two floats for a vertical and horizontal alignment of
            the offset box w.r.t. the *boxcoords*.
            The lower-left corner is (0, 0) and upper-right corner is (1, 1).

        bboxprops : dict, optional
            A dictionary of properties to set for the annotation bounding box,
            for example *boxstyle* and *alpha*.  See `.FancyBboxPatch` for
            details.

        arrowprops: dict, optional
            Arrow properties, see `.Annotation` for description.

        fontsize: float or str, optional
            Translated to points and passed as *mutation_scale* into
            `.FancyBboxPatch` to scale attributes of the box style (e.g. pad
            or rounding_size).  The name is chosen in analogy to `.Text` where
            *fontsize* defines the mutation scale as well.  If not given,
            :rc:`legend.fontsize` is used.  See `.Text.set_fontsize` for valid
            values.

        **kwargs
            Other `AnnotationBbox` properties.  See `.AnnotationBbox.set` for
            a list.
        """
        super().__init__()
        self._draggable = None
        self._xy = self._xybox = None
        x, y = xy  # matplotlib unpacks the point as it makes the box
        self.xy = x, y
        self.xycoords = xycoords
        self._annotation_clip = annotation_clip
        self.offsetbox = offsetbox
        self.arrowprops = arrowprops.copy() if arrowprops is not None else None
        self.set_fontsize(fontsize)
        self.xybox = xybox if xybox is not None else xy
        self.boxcoords = boxcoords if boxcoords is not None else xycoords
        self._box_alignment = box_alignment
        self._arrow_relpos = (None if arrowprops is None
                              else self.arrowprops.pop("relpos", (0.5, 0.5)))
        self._frameon = frameon
        self._pad = pad
        if bboxprops and not hasattr(bboxprops, "keys"):
            # matplotlib's frame refuses it (``self.patch.set(**bboxprops)``).
            from .patches import FancyBboxPatch
            FancyBboxPatch.set(**bboxprops)
        self._bboxprops = dict(bboxprops or {})
        _class_zorder(self, kwargs, 3)
        self._internal_update(kwargs)

    def _add_to_axes(self, axes, kind, autolim=True):
        if self._comes_back(axes):
            return Artist._add_to_axes(self, axes, kind, autolim)
        return self._join(axes)

    def _add_to_figure(self, figure, clip):
        if self._comes_back(figure):
            return Artist._add_to_figure(self, figure, clip)
        return self._join(figure)

    def _join(self, parent):
        """Join *parent*, an axes or a figure, with the offset box: pltrs
        (Rust) places it at *xybox* and draws the arrow; return self."""
        from ._patch_styles import _native_spec
        from .patches import BoxStyle, FancyBboxPatch
        from .text import _native_arrowprops
        figure = parent.figure
        bboxprops = cbook.normalize_kwargs(self._bboxprops, FancyBboxPatch)
        boxstyle = bboxprops.pop("boxstyle", None)
        arrowprops = None
        if self.arrowprops is not None:
            arrowprops = _native_arrowprops(
                {"arrowstyle": "simple", **self.arrowprops,
                 "relpos": self._arrow_relpos})
        described = self._tree_to_join()
        native, parts, frame = _offsetbox.add_annotation_bbox(
            parent._native, self._native, described,
            self._joining_point("xy", parent), {
                "xybox": self._joining_point("xybox", parent),
                **self._joining_coords(figure),
                "frameon": bool(self._frameon), "pad": float(self._pad),
                "annotation_clip": self._annotation_clip,
                "box_alignment": tuple(map(float, self._box_alignment)),
                "boxstyle": (None if boxstyle is None
                             else _native_spec(boxstyle, BoxStyle)),
                "bboxprops": bboxprops, "arrowprops": arrowprops,
                "fontsize": float(self.get_fontsize())})
        self.patch = _handles.wrap(frame, FancyBboxPatch, figure)
        _handles.adopt(self, native, figure)
        self._own_tree(parts, figure, native, described)
        for name in ("xy", "xybox"):
            self._move(name)
        self._hold_coords(figure)
        _python_drawing.attach(self)
        loose = self.__dict__.pop("_loose_arrow", None)
        if loose is not None:
            # What the program changed of the arrow before the box joined.
            from ._annotation_arrow import AnnotationBboxArrow, adopt
            adopt(self, loose, AnnotationBboxArrow)
        return self

    @property
    def arrow_patch(self):
        """The arrow from the box to the annotated point, a
        `.FancyArrowPatch` (None without *arrowprops*): once the box joined
        a figure, a mirror of the arrow pltrs (Rust) draws (see
        `._annotation_arrow`)."""
        return _values.assigned(self, "arrow_patch", self._arrow_patch)

    @arrow_patch.setter
    def arrow_patch(self, patch):
        from . import _annotation_arrow
        _annotation_arrow.assign(self, patch, self._take_arrow)

    #: What matplotlib's ``update_positions`` first calls on the arrow.
    _first_arrow_call = "set_positions"

    def _take_arrow(self, patch):
        """Make *patch* the arrow: its look the one pltrs (Rust) draws once
        the box joined a figure, the loose arrow until then."""
        if "patch" not in self.__dict__:
            self._loose_arrow = patch
            return
        from ._annotation_arrow import AnnotationBboxArrow, adopt
        adopt(self, patch, AnnotationBboxArrow)

    def _arrow_patch(self):
        """The arrow as pltrs (Rust) draws it (see `arrow_patch`)."""
        if self.arrowprops is None:
            return None
        if "patch" not in self.__dict__:
            # Not joined yet (the frame comes with joining): the arrow is
            # matplotlib's patch until then, whose look pltrs (Rust) takes
            # when the box joins.
            loose = self.__dict__.get("_loose_arrow")
            if loose is None:
                from .patches import FancyArrowPatch
                loose = self._loose_arrow = FancyArrowPatch(
                    (0, 0), (1, 1), **self.arrowprops)
            return loose
        from ._annotation_arrow import AnnotationBboxArrow, mirror
        return mirror(self, AnnotationBboxArrow)

    def _set_native_annotation_clip(self, b):
        """Keep the clipping behavior *b*, and give it to pltrs (Rust)'s
        box once it is in a figure."""
        self._annotation_clip = b
        if self._figure is not None:
            _offsetbox.set_annotation_clip(self._native, b)
            self.stale = True

    def _native_annotation_clip(self):
        """The clipping behavior: pltrs (Rust)'s, once the box is in a
        figure."""
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
        self._move("xy")

    @property
    def xybox(self):
        """The position *(x, y)* of the box, in *boxcoords*."""
        return self._xybox

    @xybox.setter
    def xybox(self, xybox):
        self._xybox = xybox
        self._move("xybox")

    def _move(self, name):
        """Give the point *name* (``"xy"`` or ``"xybox"``) to the box pltrs
        (Rust) draws, once it has one; matplotlib keeps any value and fails
        on one no pair of numbers when it draws the box (`_fail_as_drawn`),
        and so does the box while it keeps one."""
        if self._figure is None:
            return
        try:
            point = self._native_point(name)
        except Unusable:
            self._keep_problem(name)
            return
        if self.arrowprops and not np.isfinite(point).all():
            # The arrow drawn from or to it fails to connect (see
            # `_connect_arrow`).
            self._keep_problem(name)
        else:
            _unusable.drop(self, name)
        _offsetbox.set_annotation_points(
            self._native, *(point if part == name else None for part in ("xy", "xybox")))
        self.stale = True

    def _native_point(self, name, parent=None):
        """The point *name* (``"xy"`` or ``"xybox"``) as pltrs (Rust) takes
        it, a pair of floats, its ``"data"`` coordinates in units (dates,
        say) converted by *parent* or the box's Axes, as matplotlib's
        ``_get_xy`` converts them; `Unusable` for any other value."""
        coords = self.xycoords if name == "xy" else self.boxcoords
        axes = self.axes if parent is None else parent
        try:
            x, y = getattr(self, name)
            if hasattr(axes, "convert_xunits"):
                xcoords, ycoords = coords if isinstance(coords, tuple) else (coords, coords)
                if cbook._str_equal(xcoords, "data"):
                    x = axes.convert_xunits(x)
                if cbook._str_equal(ycoords, "data"):
                    y = axes.convert_yunits(y)
            return (number(x), number(y))
        except (TypeError, ValueError):
            raise Unusable from None

    def _native_coords(self, figure):
        """The coordinate systems of the box in *figure* as pltrs (Rust)
        takes them, None for those it cannot take."""
        return {name: native_coords(getattr(self, name), figure)
                for name in ("xycoords", "boxcoords")}

    def _joining_coords(self, figure):
        """The coordinate systems the box joins *figure* with: ``'data'``
        for those pltrs (Rust) cannot take (`_hold_coords` then keeps
        them)."""
        return {name: "data" if native is None else native
                for name, native in self._native_coords(figure).items()}

    def _hold_coords(self, figure):
        """Keep the coordinate systems of the box as a problem while pltrs
        (Rust) cannot take one: matplotlib places the points with them as
        it draws the box (`_fail_as_drawn`)."""
        if None in self._native_coords(figure).values():
            self._keep_problem("coords")
        else:
            _unusable.drop(self, "coords")

    def _joining_point(self, name, parent):
        """The point *name* the box joins *parent* (an Axes or a figure)
        with: its own, or the origin while pltrs (Rust) cannot take it
        (`_move` then keeps it)."""
        try:
            return self._native_point(name, parent)
        except Unusable:
            return (0.0, 0.0)

    def update_positions(self, renderer):
        """Update pixel positions for the annotated point, the text, and the arrow."""
        # pltrs: places the frame (`patch`) and the parts of the offset box
        # where pltrs (Rust) draws them in the figure as it is now (not laid
        # out first, as in matplotlib), which fails as there as it measures
        # the offset box with *renderer*.
        if self._figure is not None:
            self.offsetbox.get_bbox(renderer)
            _offsetbox.update_annotation_positions(self._native)

    @property
    def xyann(self):
        return self.xybox

    @xyann.setter
    def xyann(self, xyann):
        self.xybox = xyann

    @property
    def anncoords(self):
        return self.boxcoords

    @anncoords.setter
    def anncoords(self, coords):
        self.boxcoords = coords

    def get_children(self):
        patch = self.__dict__.get("patch")
        arrow = self.arrow_patch
        return [self.offsetbox, *([] if patch is None else [patch]),
                *([] if arrow is None else [arrow])]

    def set_fontsize(self, s=None):
        """
        Set the fontsize in points.

        If *s* is not given, reset to :rc:`legend.fontsize`.
        """
        if s is None:
            s = rcParams["legend.fontsize"]
        self.prop = FontProperties(size=s)
        if self._figure is not None:
            _offsetbox.set_annotation_fontsize(
                self._native, float(self.get_fontsize()))
        self.stale = True

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

    In the current implementation of `.DraggableLegend` and
    `DraggableAnnotation`, `update_offset` places the artists in display
    coordinates, and `finalize_offset` recalculates their position in axes
    coordinate and set a relevant attribute.
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
        pass

    def update_offset(self, dx, dy):
        pass

    def finalize_offset(self):
        pass


class DraggableOffsetBox(DraggableBase):
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
        offsetbox = self.offsetbox
        renderer = offsetbox.get_figure(root=True)._get_renderer()
        bbox = offsetbox.get_bbox(renderer)
        ox, oy = offsetbox._offset
        loc_in_canvas = (ox + bbox.x0, oy + bbox.y0)
        return loc_in_canvas


class DraggableAnnotation(DraggableBase):
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


# What matplotlib's module also has from its imports, which programs
# import from it.
from . import artist as martist, path as mpath  # noqa: E402, F401
from .image import BboxImage  # noqa: E402, F401
from .patches import (  # noqa: E402, F401
    bbox_artist as mbbox_artist, FancyArrowPatch, FancyBboxPatch)
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
