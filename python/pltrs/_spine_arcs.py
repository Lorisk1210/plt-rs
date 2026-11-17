"""
What the arc and circle spines read back as, matplotlib's way: a polar
Axes' ``'polar'`` and ``'inner'`` spines, a geographic Axes' ``'geo'``
spine and the arcs and circles a program makes (`.Spine.arc_spine`,
`.Spine.circular_spine`) keep a path in their
own unit coordinates (`.Path.arc`, `.Path.unit_circle`), which a draw or a
measure replaces for an arc of a side (``_adjust_location``), and a patch
transform scaling it to their centre and size (``_recompute_transform``).
pltrs (Rust) lays them out and draws them in display space; this module
answers the getters only.
"""

import numpy as np

from .path import Path
from .transforms import Affine2D, IdentityTransform


def mark(spine, name, axes):
    """Have *spine*, the spine *name* of *axes*, read back as matplotlib's
    does: a polar Axes' ``'polar'`` and ``'inner'`` spines as the arcs
    matplotlib makes over the whole circle (`.Spine.arc_spine`, of radius
    0.5 and 0), a geographic Axes' ``'geo'`` spine as the circle around
    the map (`.Spine.circular_spine`, its axis the latitudes')."""
    if "_outline_kind" in spine.__dict__:
        # Marked when its Axes first listed it.
        return
    if axes.name == "polar" and name in ("polar", "inner"):
        spine.__dict__.update(_outline_kind="arc", _arc_path=Path.arc(0, 360),
                              _arc_center=(0.5, 0.5),
                              _arc_size=1.0 if name == "polar" else 0.0)
    elif name == "geo":
        spine.__dict__.update(_outline_kind="circle", _arc_path=Path.unit_circle(),
                              _arc_center=(0.5, 0.5), _arc_size=1.0,
                              _spine_type="circle", _axis=axes.yaxis)


def reads_back(spine):
    """Whether *spine* is an arc or a circle as matplotlib's reads back:
    a polar Axes' arc, or an arc or circle the program shaped."""
    return bool(spine.__dict__.get("_outline_kind")) or (
        "_shape" in spine.__dict__ and spine._patch_type in ("arc", "circle"))


def path(spine):
    """The spine's ``_path``: the one it was made with until a draw or a
    measure of an arc of a side replaces it by the arc of its bounds, else
    of the view's angles (matplotlib's ``_adjust_location``)."""
    if spine.spine_type in ("bottom", "top") and spine._patch_type == "arc":
        draws = spine._draws()
        if spine.__dict__.get("_arc_draws", (0, 0)) != draws:
            spine.__dict__["_arc_draws"] = draws
            _adjust(spine)
    return spine.__dict__[_path_key(spine)]


def _path_key(spine):
    """Where the spine keeps its arc's path: a polar Axes' spine, linear
    to pltrs (Rust), apart from the path of its linear face."""
    return "_arc_path" if spine.__dict__.get("_outline_kind") else "_kept_path"


def _adjust(spine):
    """matplotlib's ``_adjust_location`` of an arc of a side: its path the
    arc of its bounds (else the view's angles), and for the inner arc its
    size the diameter its radial view leaves."""
    axes = spine.axes
    bounds = spine.get_bounds()
    low, high = axes.viewLim.intervalx if bounds is None else bounds
    direction = getattr(axes, "get_theta_direction", lambda: 1)()
    offset = getattr(axes, "get_theta_offset", lambda: 0)()
    low, high = low * direction + offset, high * direction + offset
    if low > high:
        low, high = high, low
    spine.__dict__[_path_key(spine)] = Path.arc(np.rad2deg(low), np.rad2deg(high))
    if spine.spine_type == "bottom":
        transform = IdentityTransform() if spine.axis is None else spine.axis.get_transform()
        rmin, rmax = transform.transform(axes.viewLim.intervaly)
        get_rorigin = getattr(axes, "get_rorigin", None)
        rorigin = rmin if get_rorigin is None else transform.transform(get_rorigin())
        spine.__dict__["_arc_size"] = (rmin - rorigin) / (rmax - rorigin)


def patch_transform(spine):
    """The transform scaling the spine's unit path to its centre and size
    (matplotlib's ``_recompute_transform``)."""
    path(spine)
    if spine.__dict__.get("_outline_kind"):
        center, size = spine.__dict__["_arc_center"], spine.__dict__["_arc_size"]
    else:
        center, size = spine._shape
        size = spine.__dict__.get("_arc_size", size)
        center = (spine.convert_xunits(center[0]), spine.convert_yunits(center[1]))
    return Affine2D().scale(size * 0.5, size * 0.5).translate(*center)


def artist_transform(spine, given):
    """The transform the spine's patch is drawn through: the one *given*
    (None: none was), else a polar Axes' wedge in its Axes for its arcs,
    else the Axes' own, as matplotlib's `set_patch_arc` and
    `set_patch_circle` set it."""
    if given is not None:
        return given
    axes = spine.axes
    if spine.__dict__.get("_outline_kind") and axes.name == "polar":
        return axes.transWedge + axes.transAxes
    return axes.transAxes


def window_extent(spine):
    """The extent of a polar Axes' arc pltrs (Rust) gives none of (the
    inner arc of no radius): its path through its transform, as
    matplotlib's measures it."""
    return spine.get_path().get_extents(spine.get_transform())
