"""
matplotlib's evaluations of its transform graph, replayed where pltrs
(Rust) does the work.

matplotlib's transforms compute lazily: a node of the graph makes its
matrix with numpy when an artist's ``draw`` (or ``add_patch``, or a
measurement) first asks for it after a leaf changed, and keeps it until
the next change, or makes it at every call
(``CompositeGenericTransform.get_affine``). Where a leaf is no ordinary
number (view limits without width, an infinite rectangle), that
arithmetic warns (``RuntimeWarning: invalid value encountered in dot``),
as often as nodes are computed. pltrs (Rust) draws from its own state
and computes none of them, so the package evaluates the graph it keeps
(`.transforms`, over pltrs (Rust)'s leaves: `._transform_trees`) at the
same moments, with the calls matplotlib's code makes there, in its
order: the nodes then compute and cache as matplotlib's do, and their
arithmetic (numpy's own in `.transforms`, pltrs (Rust)'s with numpy's
warnings in ``_native.transforms``) warns as matplotlib's does.

Only the graph is evaluated, on no data: paths and points go through
the transforms empty, so that what the non-affine parts say of the data
(a logarithm of a negative coordinate) stays with pltrs (Rust), which
maps the data.

Nothing is replayed while every leaf is tame (a finite number of
ordinary size, a view with some width): pltrs (Rust) hands the draws of
a figure to `evaluated` (``_native.transforms.set_evaluation_hook``)
only while a leaf of the figure's graph is wild, or for a patch whose
own transform is, and the other sites ask it first
(``_native.transforms.replays``; `._transform_trees.patch_added` and its
kin), so that this module loads for such figures only. Skipping the evaluations of a tame
graph leaves the caches as if never asked, which is where matplotlib's
are once a leaf turned wild: its change invalidated every node built on
it.
"""

import contextlib
import functools
import weakref

import numpy as np

from ._native import archive as _archive, artist as _artist

#: No points: what goes through the transforms in place of the data.
_NO_POINTS = np.empty((0, 2))

#: The modules of the classes whose draws are replayed (`_artist_drawn`).
_DRAWN_MODULES = frozenset(["lines", "text", "collections", "patches", "spines"])

#: The `.TransformedPath` each line drawn keeps, with its transform
#: (matplotlib's ``Line2D._transformed_path``).
_line_paths = weakref.WeakKeyDictionary()


@functools.cache
def _no_path():
    """The path of no points."""
    from .path import Path
    return Path(_NO_POINTS)


def evaluated(figure, kind, artist_id, axis):
    """One evaluation of the graph by a draw of the root *figure* (see
    ``pltrs::transforms::Evaluation``): the artist *artist_id* drawn, the
    axes *artist_id* starting its draw, its background patch drawn, or
    its *axis*, ``(name, ticks, label)``."""
    from . import _handles, _transform_trees
    _transform_trees.sync(figure)
    artist = figure._artists.get(artist_id)
    if artist is None:
        # One pltrs (Rust) made that the program never asked for (a
        # spine): its object is made now, if it is of a kind drawn here.
        native = _archive.handle(figure._native, artist_id)
        module = _handles._CLASS_MODULES.get(_artist.class_name(native))
        if kind != "draw" or module not in _DRAWN_MODULES:
            return
        artist = _handles.wrap_any(native, figure)
    with _spines_placed(artist if kind != "draw" else getattr(artist, "axes", None)):
        if kind == "axis":
            _axis_drawn(artist, *axis)
        elif kind == "axes":
            if artist.name == "polar":
                _polar_drawn(artist)
        elif kind == "background":
            _patch_drawn(artist.patch)
        else:
            _artist_drawn(artist)


@contextlib.contextmanager
def _spines_placed(axes):
    """
    Have the spines of *axes* (if it is an Axes) count as placed meanwhile.

    Asking a spine that was given no position for its transform places it,
    which makes the ticks of its axis anew
    (`.spines.Spine._ensure_position_is_set`): matplotlib's does so where
    its axis first makes a tick, long before a draw reads the transform,
    so reading it here must leave the ticks alone.
    """
    spines = getattr(axes, "spines", None)
    unplaced = [spine for spine in (spines.values() if spines is not None else ())
                if getattr(spine, "_position_set", True) is False]
    for spine in unplaced:
        spine._position_set = True
    try:
        yield
    finally:
        for spine in unplaced:
            spine._position_set = False


def _artist_drawn(artist):
    """matplotlib's ``draw`` of *artist*, as far as it evaluates
    transforms."""
    from .collections import Collection
    from .lines import Line2D
    from .patches import Patch
    from .text import Text
    if isinstance(artist, Patch):
        _patch_drawn(artist)
    elif isinstance(artist, Line2D):
        _line_drawn(artist)
    elif isinstance(artist, Text):
        _text_drawn(artist)
    elif isinstance(artist, Collection):
        _collection_drawn(artist)


def _clip_read(artist):
    """``GraphicsContextBase.get_clip_path``, which a renderer asks for
    each time it draws for *artist*: its clip path through its transform,
    refused with a log message when it is not finite."""
    from .backend_bases import GraphicsContextBase
    clip_path = artist.get_clip_path() if artist.get_clip_on() else None
    if clip_path is not None:
        gc = GraphicsContextBase()
        gc.set_clip_path(clip_path)
        gc.get_clip_path()


def _patch_drawn(patch):
    """``Patch.draw``: the patch's transform made (a spine's once it is
    placed along its axis), its path taken through the non-affine part,
    then the affine part, and the path drawn in the patch's clip path."""
    from .spines import Spine
    transform = (_spine_transform(patch) if isinstance(patch, Spine)
                 else patch.get_transform())
    transform.transform_path_non_affine(_no_path())
    transform.get_affine()
    _clip_read(patch)


def _spine_transform(spine):
    """
    The transform ``Patch.draw`` makes for *spine* once
    ``Spine._adjust_location`` placed it.

    The arcs around and at the centre of polar Axes are matplotlib's
    circles of the wedge's box, sized (the one at the centre from the
    radial limits and origin, with numpy), through ``transWedge`` and
    ``transAxes``; pltrs (Rust) draws them in data coordinates, which is
    what their mirrors answer.
    """
    from .transforms import Affine2D
    axes = spine.axes
    arc = getattr(axes, "name", None) == "polar" and spine.spine_type in ("top", "bottom")
    if not arc:
        return spine.get_transform()
    size = 1.0
    if spine.spine_type == "bottom":
        rmin, rmax = axes._viewLim.intervaly
        rorigin = axes.get_rorigin()
        size = (rmin - rorigin) / (rmax - rorigin)
    return (Affine2D().scale(size * 0.5, size * 0.5).translate(0.5, 0.5)
            + (axes.transWedge + axes.transAxes))


def _drawn_parts(line):
    """Whether ``Line2D.draw`` draws the line of *line* and its
    markers."""
    marker = line.get_marker()
    return (line.get_linestyle() not in ("None", "none", " ", ""),
            marker not in ("None", "none", " ", "", None)
            and line.get_markersize() > 0)


def _line_drawn(line):
    """``Line2D.draw``: the line's path through its transform (kept until
    the transform changes), then the affine part once for the line and
    once for the markers, each drawn in the line's clip path."""
    from .transforms import TransformedPath
    transform = line.get_transform()
    kept = _line_paths.get(line)
    if kept is None or kept[0] is not transform:
        kept = _line_paths[line] = (
            transform, TransformedPath(_no_path(), transform))
    line_drawn, markers_drawn = _drawn_parts(line)
    if line_drawn:
        kept[1].get_transformed_path_and_affine()
        _clip_read(line)
    if markers_drawn:
        kept[1].get_transformed_points_and_affine()
        _clip_read(line)


def _moved_line_drawn(transform, path=None):
    """``Line2D.draw`` of a line whose data were set since its last draw
    (a tick's): its path and its points through the non-affine part of
    *transform* anew, then the affine part. *path* is the line's where
    the data go through too (see `_axis_parts`)."""
    path = _no_path() if path is None else path
    transform.transform_path_non_affine(path)
    transform.transform_non_affine(path.vertices)
    transform.get_affine()


def _position_mapped(transform, position=_NO_POINTS):
    """A position through *transform* (``Text.draw``,
    ``Text.get_window_extent``): *position* where the data go through
    too."""
    transform.transform(position)


def _text_drawn(text):
    """``Text.draw``: the text's position through its transform, again to
    place its box."""
    if text.get_text() == "":
        return
    transform = text.get_transform()
    _position_mapped(transform)
    if text.get_bbox_patch() is not None:
        _position_mapped(transform)


def _collection_drawn(collection):
    """``Collection.draw``: the non-affine parts of the transform and of
    the offset transform (``_prepare_points``), then both as the renderer
    takes them: the transform frozen, the offset transform as a matrix,
    in the collection's clip path."""
    transform = collection.get_transform()
    offset_transform = collection.get_offset_transform()
    if not transform.is_affine:
        transform.transform_path_non_affine(_no_path())
        transform = transform.get_affine()
    if not offset_transform.is_affine:
        offset_transform.transform_non_affine(_NO_POINTS)
        offset_transform = offset_transform.get_affine()
    transform.frozen()
    offset_transform.__array__()
    _clip_read(collection)


#: How many steps matplotlib's gridlines are cut into where a projection
#: bends them (``GRIDLINE_INTERPOLATION_STEPS``).
_GRIDLINE_STEPS = 180


def _axis_parts(axes, name, tick):
    """
    The transforms and data of the parts of *tick* of axis *name* of
    *axes*: the gridline's ``(transform, path)``, those of the marks on
    each side, and the labels' ``(transform, position)`` on each side.

    The data are left out (None for a path, no points for a position)
    but for polar Axes: there the transforms' non-affine part depends on
    the Axes' own settings (the angle of the radial labels), and pltrs
    (Rust) says nothing of what numpy's arithmetic makes of them.
    """
    from .path import Path
    get_transform = getattr(axes, f"get_{name}axis_transform")
    transforms = (get_transform(which="grid"),
                  [get_transform(which="tick1"), get_transform(which="tick2")],
                  [getattr(axes, f"get_{name}axis_text{side}_transform")(tick[4])[0]
                   for side in (1, 2)])
    if axes.name != "polar":
        return ((transforms[0], None),
                [(transform, None) for transform in transforms[1]],
                [(transform, _NO_POINTS) for transform in transforms[2]])
    loc = tick[5]
    ends = [(loc, 0), (loc, 1)] if name == "x" else [(0, loc), (1, loc)]
    gridline = Path(np.array(ends, float))
    gridline._interpolation_steps = _GRIDLINE_STEPS
    return ((transforms[0], gridline),
            [(transform, Path(np.array([end], float)))
             for transform, end in zip(transforms[1], ends)],
            [(transform, np.array(end, float))
             for transform, end in zip(transforms[2], ends)])


def _label_extents(ticks, parts):
    """``Axis._get_ticklabel_bboxes``: the first label of every tick shown
    measured, then the second (*parts* holding each tick's, see
    `_axis_parts`)."""
    for side in (0, 1):
        for tick, (_, _, labels) in zip(ticks, parts):
            if tick[2][side]:
                _position_mapped(*labels[side])


def _ticks_updated(axes, name):
    """``Axis._update_ticks``: each tick the axis has placed, a radial
    tick of polar Axes with the angles of its marks and labels
    (`_radial_tick_placed`)."""
    if axes.name == "polar" and name == "y":
        axis = axes.yaxis
        for _ in range(sum(len(axis.get_ticklocs(minor=minor)) for minor in (False, True))):
            _radial_tick_placed(axes)


def _radial_tick_placed(axes):
    """``RadialTick.update_position``: the angles of the tick's marks and
    labels from the theta limits (the position of the radial labels on a
    full circle), in numpy's arithmetic."""
    from .projections.polar import _is_full_circle_deg
    thetamin, thetamax = axes.get_thetamin(), axes.get_thetamax()
    direction = axes.get_theta_direction()
    offset = np.rad2deg(axes.get_theta_offset())
    if _is_full_circle_deg(thetamin, thetamax):
        angle = (axes.get_rlabel_position() * direction + offset) % 360 - 90
    else:
        angle = (thetamin * direction + offset) % 360 - 90
        np.deg2rad(angle if direction > 0 else angle + 180)
    (angle + 90) % 180 - 90
    angle = (thetamax * direction + offset) % 360 - 90
    np.deg2rad(angle if direction > 0 else angle + 180)
    (angle + 90) % 180 - 90


def _axis_drawn(axes, name, ticks, label):
    """``Axis.draw`` of axis *name* of *axes*, drawing *ticks* (each
    ``(gridline, marks, labels, text, pad, loc)``): the ticks placed,
    their labels measured, each tick drawn (its gridline, in the axis'
    clip path, its marks, its labels with text), the axis label placed
    past the labels of the ticks
    placed and measured again and the spine on its side (unless the
    program gave it coordinates), then drawn with *label*."""
    axis = getattr(axes, f"{name}axis")
    parts = [_axis_parts(axes, name, tick) for tick in ticks]
    _ticks_updated(axes, name)
    _label_extents(ticks, parts)
    for (gridline, marked, shown, text, _, _), (grid, marks, labels) in zip(ticks, parts):
        if gridline:
            _moved_line_drawn(*grid)
            _clip_read(axis)
        for side in (0, 1):
            if marked[side]:
                _moved_line_drawn(*marks[side])
        for side in (0, 1):
            if shown[side] and text:
                _position_mapped(*labels[side])
    mirror = axis._axis_label
    if "_label_transform" not in axis.__dict__:
        _ticks_updated(axes, name)
        _label_extents(ticks, parts)
        spine = axes.spines.get(axis.get_label_position())
        if spine is not None:
            spine.get_transform().transform_path(_no_path())
    if label:
        _position_mapped(mirror._transform_of(mirror.text))


def _polar_drawn(axes):
    """``PolarAxes.draw`` before it draws its children: the theta and
    radial limits read, the wedge fitted to them through ``transWedge``
    and the transform of the radial labels compared with the one the
    limits ask for."""
    from . import _background_patch
    from .patches import Wedge
    from .projections.polar import _is_full_circle_deg
    thetamin, thetamax = np.rad2deg(axes._realViewLim.intervalx)
    if thetamin > thetamax:
        thetamin, thetamax = thetamax, thetamin
    origin = axes._originViewLim
    scale = axes.yaxis.get_transform()
    rmin, rmax = ((scale.transform(axes._realViewLim.intervaly)
                   - scale.transform(origin.y0)) * np.sign(origin.y1 - origin.y0))
    if isinstance(_background_patch.axes_patch(axes), Wedge):
        center = axes.transWedge.transform((0.5, 0.5))
        edge, _ = axes.transWedge.transform((1, 0))
        radius = edge - center[0]
        radius - min(radius * (rmax - rmin) / rmax, radius)
    if _is_full_circle_deg(thetamin, thetamax):
        along = axes._r_label_position + axes.transData
    else:
        along = axes._yaxis_transform
    axes._yaxis_text_transform != along  # noqa: B015


def patch_limits_updated(axes, patch):
    """``_AxesBase._update_patch_limits``, where ``add_patch`` and
    ``relim`` take *patch* into the data limits of *axes*: the patch's
    transform made, searched for the data transform, and what comes
    before it applied."""
    from .patches import Rectangle
    if (isinstance(patch, Rectangle)
            and not patch.get_width() and not patch.get_height()):
        return
    transform = patch.get_transform()
    updatex, updatey = transform.contains_branch_seperately(axes.transData)
    if not (updatex or updatey):
        return
    if axes.name != "rectilinear":
        if updatex:
            transform == axes.get_yaxis_transform()  # noqa: B015
        if updatey:
            transform == axes.get_xaxis_transform()  # noqa: B015
    (transform - axes.transData).transform(_NO_POINTS)


def radial_ticks_placed(axes, count):
    """``Axis._update_ticks`` of the radial axis of the polar *axes*,
    placing *count* ticks, where the program asks for its ticks (a draw
    places them as it replays the axis)."""
    for _ in range(count):
        _radial_tick_placed(axes)
