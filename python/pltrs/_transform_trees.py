"""
matplotlib's transform and bbox trees of figures, subfigures and Axes,
over the values pltrs (Rust) holds.

A figure's trees are matplotlib's: ``fig.transFigure`` is a
`.BboxTransformTo` of ``fig.bbox``, a `.TransformedBbox` of
``fig.bbox_inches`` through ``fig.dpi_scale_trans``; an Axes' ``transData``
is ``transScale + (transLimits + transAxes)``, its ``bbox`` a
`.TransformedBbox` of its ``_position`` through its figure's
``transSubfigure``. Each object is made once, the first time one of them
is read (`TreeAttribute`), and kept, so that ``ax.transData is
ax.transData`` and the trees compose, invert, freeze and invalidate as
matplotlib's do.

The leaves the trees rest on are pltrs (Rust)'s: an Axes' position, view
and data limits and scales, a subfigure's box, the figure's size and
resolution (``_native.transforms.tree_leaves``). They are read when the
trees are built, and pltrs (Rust) hands those that changed to `_sync` at
the end of every native call that changed them (``_native.transforms.watch``),
which sets them as matplotlib sets its own, invalidating what is built on
them. A program setting a leaf itself (``ax.viewLim.set(...)``) sets the
value pltrs (Rust) holds (`_LeafWatcher`).

pltrs (Rust) resolves these transforms itself when it is handed one
(``set_transform``, ``transform=``): the nodes standing for a transform it
has carry it as ``_live`` (see `._live_transforms.to_native`), and the
boxes of an Axes or figure the transform spanning them, ``_spanning``.
"""

import contextlib
import functools
import threading
import weakref

import numpy as np

from . import _lazy_imports, rcParams
from ._native import artist as _artist, axis as _axis
from ._native import figure as _figure, transforms as _transforms
from ._program_classes import takes_from_program
from .transforms import (Affine2D, Bbox, BboxTransformFrom, BboxTransformTo,
                         IdentityTransform, ScaledTranslation, TransformNode,
                         TransformWrapper, TransformedBbox,
                         blended_transform_factory)


_transform_replay = _lazy_imports.module(globals(), "._transform_replay")
_tree_validity = _lazy_imports.module(globals(), "._tree_validity")


def follow_evaluations(fig):
    """Have pltrs (Rust) hand what the draws of the root figure *fig*
    evaluate of matplotlib's graph to `._transform_replay`, which it does
    only while a leaf of the graph is wild (see there)."""
    _transforms.set_evaluation_hook(
        fig._native, functools.partial(_evaluated, weakref.ref(fig)))


def _evaluated(figure_ref, *evaluation):
    """One evaluation of the graph by a draw of the figure *figure_ref*."""
    fig = figure_ref()
    if fig is not None:
        _transform_replay.evaluated(fig, *evaluation)


def patch_added(axes, patch):
    """matplotlib's ``add_patch`` of *patch* to *axes* evaluates the graph
    (``_update_patch_limits``): replayed while a leaf of it, or the
    patch's own transform, is wild."""
    if _transforms.replays(patch._native):
        _transform_replay.patch_limits_updated(axes, patch)


def patches_added(axes, patches):
    """`patch_added` for each of *patches*, which a plotting method of
    *axes* made (pltrs (Rust) added them)."""
    if _transforms.replays(axes._native):
        for patch in patches:
            patch_added(axes, patch)


def relimmed(axes, visible_only):
    """matplotlib's ``relim`` of *axes* takes every patch (every visible
    one with *visible_only*) into the data limits again, as `patch_added`
    does."""
    if _transforms.replays(axes._native):
        for patch in axes.patches:
            if not visible_only or patch.get_visible():
                patch_added(axes, patch)


def measured(artist):
    """matplotlib's ``get_window_extent`` of *artist* (a text, a legend)
    evaluates the graph: replayed while a leaf of its transforms is
    wild."""
    if artist._figure is not None and _transforms.replays(artist._native):
        _transform_replay.artist_measured(artist)


def radial_ticks_placed(axes, count):
    """matplotlib's ``_update_ticks`` of the radial axis of the polar
    *axes* computes the angles of its *count* ticks with numpy: replayed
    while a leaf of the graph is wild."""
    if _transforms.replays(axes._native):
        _transform_replay.radial_ticks_placed(axes, count)


class _Syncs(threading.local):
    """The syncs from pltrs (Rust) running on each thread."""

    #: How many are running: a leaf they set is not handed back to it.
    count = 0


_syncs = _Syncs()


def _syncing():
    """Whether a sync from pltrs (Rust) is running on this thread."""
    return _syncs.count > 0


@contextlib.contextmanager
def setting_leaves():
    """Set leaves to pltrs (Rust)'s values meanwhile, on this thread (not
    handed back)."""
    _syncs.count += 1
    try:
        yield
    finally:
        _syncs.count -= 1


class TreeAttribute:
    """
    An attribute of the trees of an Axes or a figure (``transData``,
    ``bbox``...): reading one the first time builds the trees with the
    owner's ``_build_transform_trees``, which sets them all as plain
    attributes, read directly after that (and set by assignment, as
    matplotlib's projections do).
    """

    def __init__(self, doc=None):
        self.__doc__ = doc

    def __set_name__(self, owner, name):
        self._name = name

    def __get__(self, obj, objtype=None):
        if obj is None:
            return self
        if "_transform_trees" not in obj.__dict__:
            obj._build_transform_trees()
        try:
            return obj.__dict__[self._name]
        except KeyError:
            raise AttributeError(
                f"{type(obj).__name__!r} object has no attribute "
                f"{self._name!r}") from None


#: The attributes trees are made of, which do not pickle (pltrs (Rust)'s
#: state does, from which an unpickled object builds them again).
_TREE_KEYS = frozenset([
    "_transform_trees", "_leaf_watchers", "_tree_owners", "_tree_scales",
    "_view_sets",
    "bbox_inches", "dpi_scale_trans", "bbox", "figbbox", "transFigure",
    "transSubfigure", "bbox_relative", "_position", "_originalPosition",
    "dataLim", "_viewLim", "transScale", "transLimits", "transAxes",
    "transData", "_xaxis_transform", "_yaxis_transform",
    "_rectilinear_axis_transforms",
])


#: The key under which the pickled state of an object says which nodes of
#: its trees had their values computed (see `._tree_validity`).
_VALIDITY = "_tree_validity"


def without_trees(state):
    """The pickled *state* of an object, without its trees: with which of
    their nodes had their values instead (`._tree_validity.of`)."""
    if "_transform_trees" not in state:
        return state
    kept = {key: value for key, value in state.items()
            if key not in _TREE_KEYS and key not in state["_transform_trees"]}
    kept[_VALIDITY] = _tree_validity.of(state, _TREE_KEYS)
    return kept


def _restore_validity(d, dpi_scale_trans):
    """Have the trees among the attributes *d* of an unpickled object,
    just built, hold what the pickled one's held
    (`._tree_validity.restore`); nothing for an object that was made."""
    valid = d.pop(_VALIDITY, None)
    if valid is not None:
        _tree_validity.restore(d, _TREE_KEYS, valid, dpi_scale_trans)


class _LeafWatcher(TransformNode):
    """
    A parent of a leaf, which a change of the leaf invalidates: unless
    the change comes from pltrs (Rust), it hands the leaf's new value to
    pltrs (Rust) (*push*, given the leaf).
    """

    def __init__(self, leaf, push):
        super().__init__()
        self._push = push
        self.set_children(leaf)
        leaf.get_points()  # Valid, so that its next change reaches here.

    def _invalidate_internal(self, level, invalidating_node):
        if not _syncing():
            self._push(invalidating_node)
        # Valid again, so that its next change reaches here too.
        invalidating_node.get_points()

    def __reduce__(self):
        # Pickled with a leaf it watches, it unpickles as a plain node.
        return (TransformNode, ())


def _watch_leaf(owner, leaf, push):
    """Hand pltrs (Rust) the changes the program makes to *leaf* of
    *owner*."""
    owner.__dict__.setdefault("_leaf_watchers", []).append(
        _LeafWatcher(leaf, push))


def _points(extents):
    """The points ``[[x0, y0], [x1, y1]]`` of *extents*."""
    return np.array(extents, dtype=float).reshape(2, 2)


def set_extents(bbox, extents):
    """Give the leaf *bbox* the *extents* ``(x0, y0, x1, y1)``,
    invalidating what is built on it when they differ."""
    points = _points(extents)
    if not np.array_equal(bbox._points, points, equal_nan=True):
        bbox._points = points
        bbox.invalidate()


def set_matrix(affine, matrix):
    """Give the leaf *affine* the *matrix*, invalidating what is built on
    it when it differs (NaN counting as itself: a leaf that stays NaN
    invalidates nothing)."""
    if not np.array_equal(affine.get_matrix(), matrix, equal_nan=True):
        affine.set_matrix(np.asarray(matrix, dtype=float))


def build_root(fig):
    """Build the trees of the root figure *fig* (matplotlib's
    ``Figure.__init__``) and follow its leaves."""
    d = fig.__dict__
    d["_transform_trees"] = ()
    dpi, size, _, _ = _transforms.tree_leaves(fig._native, [], [])
    d["bbox_inches"] = Bbox.from_bounds(0, 0, *size)
    d["dpi_scale_trans"] = Affine2D().scale(dpi)
    d["bbox"] = TransformedBbox(d["bbox_inches"], d["dpi_scale_trans"])
    d["figbbox"] = d["bbox"]
    d["transFigure"] = BboxTransformTo(d["bbox"])
    d["transSubfigure"] = d["transFigure"]
    d["_tree_owners"] = weakref.WeakValueDictionary()
    d["dpi_scale_trans"]._live = _transforms.native(("inches",))
    d["transFigure"]._live = _transforms.trans_figure(fig._native)
    # The figure's own box, which a print cut to a window replaces rather
    # than moves (`_follow_print_window`): what is anchored to it stays.
    own = BboxTransformTo(d["bbox"])
    own._live = _transforms.figure_box(fig._native)
    d["bbox"]._spanning = own
    _watch_leaf(fig, d["bbox_inches"], functools.partial(_push_size, fig))
    _transforms.watch(fig._native, functools.partial(_sync, weakref.ref(fig)))
    _restore_validity(d, d["dpi_scale_trans"])


def build_subfigure(sub):
    """Build the trees of the subfigure *sub* (matplotlib's
    ``SubFigure.__init__``) and follow its box."""
    parent = sub._parent
    root = sub._figure
    d = sub.__dict__
    d["_transform_trees"] = ()
    d["dpi_scale_trans"] = parent.dpi_scale_trans
    d["transFigure"] = parent.transFigure
    _, _, frames, _ = _transforms.tree_leaves(root._native, [sub._native], [])
    d["bbox_relative"] = Bbox.null()
    if frames:
        set_extents(d["bbox_relative"], frames[0][1])
    d["figbbox"] = parent.figbbox
    d["bbox"] = TransformedBbox(d["bbox_relative"], parent.transSubfigure)
    d["transSubfigure"] = BboxTransformTo(d["bbox"])
    d["transSubfigure"]._live = _transforms.trans_figure(sub._native)
    d["bbox"]._spanning = d["transSubfigure"]
    root._tree_owners[_artist.id(sub._native)] = sub
    _transforms.watch_frame(sub._native)
    _restore_validity(d, d["dpi_scale_trans"])


def build_axes(ax, as_made=False):
    """
    Build the trees of the Axes *ax* (matplotlib's ``Axes.set_figure``
    and the ``_set_lim_and_transforms`` of its projection) and follow
    their leaves. *as_made* builds them from the boxes matplotlib's
    ``__init__`` builds them from (a new view, no data, and for an Axes
    of a grid the unit box, which its cell replaces after), then gives
    them the Axes' own, for a projection class of the program's, whose
    ``_set_lim_and_transforms`` may read them.
    """
    # Its (sub)figure's, also for an Axes removed from it, which keeps
    # measuring there as matplotlib's keeps its trees.
    fig = ax._home_figure()
    root = ax._figure
    d = ax.__dict__
    d["_transform_trees"] = ()
    to_figure = fig.transSubfigure
    _, _, _, axes = _transforms.tree_leaves(root._native, [], [ax._native])
    d["_position"] = Bbox.unit()
    d["_originalPosition"] = Bbox.unit()
    d["bbox"] = TransformedBbox(d["_position"], to_figure)
    d["dataLim"] = Bbox.null()
    d["_viewLim"] = Bbox.unit()
    d["transScale"] = TransformWrapper(IdentityTransform())
    leaves = axes[0] if axes else None
    if leaves is not None and not as_made:
        _set_axes_boxes(ax, leaves)
    elif leaves is not None and ax.get_subplotspec() is None:
        # An Axes made at a rectangle has it from the start.
        position, original = leaves[1]
        set_extents(d["_position"], position)
        set_extents(d["_originalPosition"], original)
    made_before = set(d)
    ax._set_lim_and_transforms()
    # What the projection's trees are made of besides (not pickled).
    d["_transform_trees"] = tuple(sorted(set(d) - made_before))
    if leaves is not None and as_made:
        _set_axes_boxes(ax, leaves)
    ax._update_transScale()
    if leaves is not None:
        d["_tree_scales"] = leaves[4]
        ax._apply_projection_leaves(leaves[5])
    # pltrs (Rust) resolves these as the Axes' own (its projection's).
    for key, native in (("transData", _transforms.trans_data),
                        ("transAxes", _transforms.trans_axes)):
        transform = d.get(key)
        if transform is not None and not hasattr(transform, "_live"):
            transform._live = native(ax._native)
    d["bbox"]._spanning = d["transAxes"]
    _watch_leaf(ax, d["_viewLim"], functools.partial(_push_limits, ax, "view"))
    _watch_leaf(ax, d["dataLim"], functools.partial(_push_limits, ax, "data"))
    # Also an Axes outside the figure as its trees are built (one its
    # class' constructor builds, which joins the figure later under its
    # id), under the id its leaves come with (a 3-D Axes' plane's).
    root._tree_owners[_transforms.watch_axes(ax._native)] = ax
    _restore_validity(d, fig.dpi_scale_trans)


def _set_axes_boxes(ax, leaves, positions=True):
    """Give the boxes of *ax* the values of its *leaves* (its positions
    too with *positions*)."""
    d = ax.__dict__
    _, (position, original), (view, view_sets), (data, minpos), _, _ = leaves
    if positions:
        set_extents(d["_position"], position)
        set_extents(d["_originalPosition"], original)
    set_extents(d["_viewLim"], view)
    if d.setdefault("_view_sets", view_sets) != view_sets:
        # matplotlib's ``set_xlim`` and ``set_view_interval`` invalidate
        # what is built on the view limits whatever they set them to.
        d["_view_sets"] = view_sets
        d["_viewLim"].invalidate()
    data_lim = d["dataLim"]
    set_extents(data_lim, data)
    data_lim._minpos = np.array(minpos, dtype=float)


def _apply_axes(ax, leaves):
    """Give the trees of *ax* its new *leaves*: an Axes placed in another
    (an inset, a secondary axis) that left it keeps the position its host
    placed it at, as matplotlib's keeps the one its locator set at its
    last draw, where pltrs (Rust) has its place in the host's fractions."""
    d = ax.__dict__
    _set_axes_boxes(ax, leaves, positions=not d.get("_left_host"))
    if d.get("_tree_scales") != leaves[4]:
        d["_tree_scales"] = leaves[4]
        ax._update_transScale()
    ax._apply_projection_leaves(leaves[5])


def _sync(figure_ref, leaves):
    """The *leaves* of the root figure *figure_ref* that changed, from
    pltrs (Rust): ``(dpi, size, frames, axes)``."""
    fig = figure_ref()
    if fig is None:
        return
    dpi, size, frames, axes = leaves
    d = fig.__dict__
    owners = d["_tree_owners"]
    with setting_leaves():
        set_matrix(d["dpi_scale_trans"], Affine2D().scale(dpi).get_matrix())
        set_extents(d["bbox_inches"], (0.0, 0.0, *size))
        for frame_id, relative in frames:
            sub = owners.get(frame_id)
            if sub is not None:
                set_extents(sub.__dict__["bbox_relative"], relative)
        for leaves in axes:
            ax = owners.get(leaves[0])
            if ax is not None:
                _apply_axes(ax, leaves)


def _push_size(fig, bbox_inches):
    """The size the program gave ``fig.bbox_inches``, for pltrs (Rust)."""
    (x0, y0), (x1, y1) = bbox_inches.get_points()
    _figure.set_size_inches(fig._native, float(x1 - x0), float(y1 - y0))


def _push_limits(ax, which, bbox):
    """The view or data limits the program gave the box of *ax*, for
    pltrs (Rust), as they are (matplotlib assigns the box's points)."""
    (x0, y0), (x1, y1) = bbox.get_points()
    setter = (_axis.set_view_interval if which == "view"
              else _axis.set_data_interval)
    setter(ax._native, "x", float(x0), float(x1), True)
    setter(ax._native, "y", float(y0), float(y1), True)


def sync(fig):
    """Bring the trees of the root figure *fig* up to date now (Python
    code run in the middle of a draw reads those of the laid-out
    figure), cut to the window a print of it is cut to, if any."""
    if "_transform_trees" in fig.__dict__:
        _transforms.sync(fig._native)
        _follow_print_window(fig)


def _follow_print_window(fig):
    """
    While pltrs (Rust) prints the root figure *fig* cut to a window (a
    tight box, a ``bbox_inches`` given), its trees as matplotlib's print
    sets them (``_tight_bbox.adjust_bbox``): ``transFigure`` moved by the
    window's corner, and so every transform through it, and ``fig.bbox``
    of the window's size; put back once the print is over.
    """
    d = fig.__dict__
    window = _figure.print_window(fig._native)
    kept = d.get("_kept_for_window")
    if kept is not None and (window is None or kept[0] != window):
        _, d["bbox"], boxout = d.pop("_kept_for_window")
        d["transFigure"]._boxout = boxout
        d["transFigure"].invalidate()
        kept = None
    if window is None or kept is not None:
        return
    x0, y0, x1, y1 = window
    scale = d["dpi_scale_trans"].frozen()
    trans_figure = d["transFigure"]
    d["_kept_for_window"] = (window, d["bbox"], trans_figure._boxout)
    (left, bottom), size = scale.transform([(x0, y0), d["bbox_inches"].size])
    trans_figure._boxout = Bbox.from_bounds(-left, -bottom, *size)
    trans_figure.invalidate()
    d["bbox"] = TransformedBbox(Bbox.from_bounds(0, 0, x1 - x0, y1 - y0), scale)
    d["bbox"]._spanning = trans_figure


class FigureTrees:
    """The trees of a (sub)figure (a mixin of `.FigureBase`)."""

    bbox_inches = TreeAttribute("The box of the figure in inches.")
    dpi_scale_trans = TreeAttribute(
        "The transform from inches to display coordinates.")
    bbox = TreeAttribute("The (sub)figure's box in display coordinates.")
    figbbox = TreeAttribute("The box of the root figure in display "
                            "coordinates.")
    transFigure = TreeAttribute(
        "The transform from fractions of the root figure to display "
        "coordinates.")
    transSubfigure = TreeAttribute(
        "The transform from fractions of this (sub)figure to display "
        "coordinates.")
    bbox_relative = TreeAttribute(
        "The box of a subfigure in the fractions of its parent.")

    def _build_transform_trees(self):
        if self is self._figure:
            build_root(self)
        else:
            build_subfigure(self)

    def __getstate__(self):
        return without_trees(super().__getstate__())


class AxesTrees:
    """The trees of an Axes (a mixin of `~.axes._base._AxesBase`)."""

    bbox = TreeAttribute("The Axes' box in display coordinates.")
    dataLim = TreeAttribute("The bounding box of the data the Axes "
                            "autoscales to.")
    _viewLim = TreeAttribute()
    _position = TreeAttribute()
    _originalPosition = TreeAttribute()
    transScale = TreeAttribute()
    transLimits = TreeAttribute()
    transAxes = TreeAttribute("The transform from Axes fractions to display "
                              "coordinates.")
    transData = TreeAttribute("The transform from data to display "
                              "coordinates.")
    _xaxis_transform = TreeAttribute()
    _yaxis_transform = TreeAttribute()

    def _build_transform_trees(self, as_made=False):
        build_axes(self, as_made)

    def __getstate__(self):
        return without_trees(super().__getstate__())

    def _set_lim_and_transforms(self):
        """
        Set the *_xaxis_transform*, *_yaxis_transform*, *transScale*,
        *transData*, *transLimits* and *transAxes* transformations.

        .. note::

            This method is primarily used by rectilinear projections of the
            `~matplotlib.axes.Axes` class, and is meant to be overridden by
            new kinds of projection Axes that need different transformations
            and limits. (See `~matplotlib.projections.polar.PolarAxes` for an
            example.)
        """
        self.transAxes = BboxTransformTo(self.bbox)

        # Transforms the x and y axis separately by a scale factor.
        # It is assumed that this part will have non-linear components
        # (e.g., for a log scale).
        self.transScale = TransformWrapper(IdentityTransform())

        # An affine transformation on the data, generally to limit the
        # range of the axes
        self.transLimits = BboxTransformFrom(
            TransformedBbox(self._viewLim, self.transScale))

        # The parentheses are important for efficiency here -- they
        # group the last two (which are usually affines) separately
        # from the first (which, with log-scaling can be non-affine).
        self.transData = self.transScale + (self.transLimits + self.transAxes)

        self._xaxis_transform = blended_transform_factory(
            self.transData, self.transAxes)
        self._yaxis_transform = blended_transform_factory(
            self.transAxes, self.transData)
        self._xaxis_transform._live = _transforms.trans_xaxis(self._native)
        self._yaxis_transform._live = _transforms.trans_yaxis(self._native)
        # What a subclass leaves of these is drawn as rectilinear Axes are
        # (`.projections._program`).
        self._rectilinear_axis_transforms = (self._xaxis_transform,
                                             self._yaxis_transform)

    def _update_transScale(self):
        """Have *transScale* follow the scales of the axes (once the trees
        are built, which read them then)."""
        if "_transform_trees" in self.__dict__:
            self.transScale.set(blended_transform_factory(
                self.xaxis.get_transform(), self.yaxis.get_transform()))

    def _apply_projection_leaves(self, leaves):
        """Give the trees of the projection the *leaves* pltrs (Rust)
        has (polar and geographic Axes override it)."""

    def get_xaxis_text1_transform(self, pad_points):
        """
        Returns
        -------
        transform : Transform
            The transform used for drawing x-axis labels, which will add
            *pad_points* of padding (in points) between the axis and the label.
            The x-direction is in data coordinates and the y-direction is in
            axis coordinates
        valign : {'center', 'top', 'bottom', 'baseline', 'center_baseline'}
            The text vertical alignment.
        halign : {'center', 'left', 'right'}
            The text horizontal alignment.

        Notes
        -----
        This transformation is primarily used by the `~matplotlib.axis.Axis`
        class, and is meant to be overridden by new kinds of projections that
        may need to place axis elements in different locations.
        """
        labels_align = rcParams["xtick.alignment"]
        return (self.get_xaxis_transform(which='tick1') +
                ScaledTranslation(0, -1 * pad_points / 72,
                                  self.get_figure(root=False).dpi_scale_trans),
                "top", labels_align)

    def get_xaxis_text2_transform(self, pad_points):
        """
        Returns
        -------
        transform : Transform
            The transform used for drawing secondary x-axis labels, which will
            add *pad_points* of padding (in points) between the axis and the
            label.  The x-direction is in data coordinates and the y-direction
            is in axis coordinates
        valign : {'center', 'top', 'bottom', 'baseline', 'center_baseline'}
            The text vertical alignment.
        halign : {'center', 'left', 'right'}
            The text horizontal alignment.

        Notes
        -----
        This transformation is primarily used by the `~matplotlib.axis.Axis`
        class, and is meant to be overridden by new kinds of projections that
        may need to place axis elements in different locations.
        """
        labels_align = rcParams["xtick.alignment"]
        return (self.get_xaxis_transform(which='tick2') +
                ScaledTranslation(0, pad_points / 72,
                                  self.get_figure(root=False).dpi_scale_trans),
                "bottom", labels_align)

    def get_yaxis_text1_transform(self, pad_points):
        """
        Returns
        -------
        transform : Transform
            The transform used for drawing y-axis labels, which will add
            *pad_points* of padding (in points) between the axis and the label.
            The x-direction is in axis coordinates and the y-direction is in
            data coordinates
        valign : {'center', 'top', 'bottom', 'baseline', 'center_baseline'}
            The text vertical alignment.
        halign : {'center', 'left', 'right'}
            The text horizontal alignment.

        Notes
        -----
        This transformation is primarily used by the `~matplotlib.axis.Axis`
        class, and is meant to be overridden by new kinds of projections that
        may need to place axis elements in different locations.
        """
        labels_align = rcParams["ytick.alignment"]
        return (self.get_yaxis_transform(which='tick1') +
                ScaledTranslation(-1 * pad_points / 72, 0,
                                  self.get_figure(root=False).dpi_scale_trans),
                labels_align, "right")

    def get_yaxis_text2_transform(self, pad_points):
        """
        Returns
        -------
        transform : Transform
            The transform used for drawing secondary y-axis labels, which will
            add *pad_points* of padding (in points) between the axis and the
            label.  The x-direction is in axis coordinates and the y-direction
            is in data coordinates
        valign : {'center', 'top', 'bottom', 'baseline', 'center_baseline'}
            The text vertical alignment.
        halign : {'center', 'left', 'right'}
            The text horizontal alignment.

        Notes
        -----
        This transformation is primarily used by the `~matplotlib.axis.Axis`
        class, and is meant to be overridden by new kinds of projections that
        may need to place axis elements in different locations.
        """
        labels_align = rcParams["ytick.alignment"]
        return (self.get_yaxis_transform(which='tick2') +
                ScaledTranslation(pad_points / 72, 0,
                                  self.get_figure(root=False).dpi_scale_trans),
                labels_align, "left")

    def _default_axis_scale(self, axis, name):
        """The scale object of *axis*, named *name*, when none was given
        from Python (pltrs (Rust) set it, and *transScale* reads it):
        matplotlib's default of that scale, or the function scale of a
        colorbar's long axis."""
        from .scale import FuncScale, scale_factory
        colorbar = getattr(self, "_colorbar", None)
        if name == "function" and colorbar is not None:
            return FuncScale(axis, colorbar._scale_functions())
        return scale_factory(name, axis)

    def _unstale_viewLim(self):
        """Apply the autoscaling pending (matplotlib's
        ``_unstale_viewLim``), through the ``autoscale_view`` of an Axes
        class of the program's (cartopy's ``GeoAxes``), unless pltrs (Rust)
        is handing its leaves (laying the figure out), when they are what
        it works with."""
        if not _syncing():
            from ._native import axes as _axes, axes_base as _axes_base
            if takes_from_program(type(self), "autoscale_view"):
                scalex, scaley = _axes_base.take_stale_autoscale(self._native)
                if scalex or scaley:
                    self.autoscale_view(scalex=scalex, scaley=scaley)
            _axes.get_xlim(self._native)

    def _unstale_by_program(self):
        """Bring the view limits up to date through the ``autoscale_view``
        of an Axes class of the program's overriding it before they are
        read, as matplotlib's getters do (`_unstale_viewLim`); pltrs (Rust)
        brings those of the other Axes up to date itself."""
        if takes_from_program(type(self), "autoscale_view"):
            self._unstale_viewLim()

    @property
    def viewLim(self):
        """The view limits as `.Bbox` in data coordinates."""
        self._unstale_viewLim()
        return self._viewLim
