"""Behaviour of patches, ported from the non-image tests of matplotlib's
``lib/matplotlib/tests/test_patches.py``.

Area ``patch_behaviour``.

Ops
---
``patch.geometry``  {shape, steps, queries} -> {query: value}
    Builds a standalone patch (no axes, so its transform is its patch
    transform and its coordinates are its own), applies ``steps`` in order
    and answers ``queries``.

    ``shape`` is ``{"kind": ..., **params}``:

    * ``rectangle``  ``Rectangle(xy, width, height, angle=0,
      rotation_point="xy")``; ``rotation_point`` is ``"xy"``, ``"center"``
      or ``[x, y]``
    * ``ellipse``  ``Ellipse(xy, width, height, angle=0)``
    * ``circle``  ``Circle(xy, radius)``
    * ``polygon``  ``Polygon(xy, closed)``
    * ``wedge``  ``Wedge(center, r, theta1, theta2, width=None)``
    * ``arrow``  ``Arrow(x, y, dx, dy, width=1)``
    * ``fancy_arrow``  ``FancyArrow(x, y, dx, dy, width, head_width,
      head_length, shape="full", overhang=0)``

    ``steps`` are ``[name, value]``: ``set_angle``, ``set_xy`` (polygons),
    ``set_center``, ``set_radius``, ``set_width``, ``set_theta1``,
    ``set_theta2`` (wedges) and ``set_data`` (arrows; ``value`` holds the
    keyword arguments).

    ``queries``: ``corners`` (``get_corners()``), ``center``
    (``get_center()``), ``vertices`` / ``co_vertices`` (ellipses), ``verts``
    (``get_verts()``: the first polygon of the placed outline), ``xy``
    (``Polygon.get_xy()``), ``extents`` (``get_extents()``), ``path``
    (``get_transform().transform_path(get_path())`` as ``{vertices,
    codes}``) and ``["contains", points]`` (``[contains_point(p), ...]``,
    within the default radius: none, as the patch has no edge).

    Mirrors ``test_Polygon_close``, ``test_corner_center``,
    ``test_ellipse_vertices``, ``test_rotate_rect``, ``test_negative_rect``,
    ``test_wedge_movement``, ``test_contains_point``,
    ``test_contains_points``, ``test_fancyarrow_setdata``,
    ``test_arrow_set_data``, ``test_degenerate_polygon`` and
    ``test_empty_verts``.

``patch.style``  {kw, steps, rcparams} -> {facecolor, edgecolor, hatchcolor,
    capstyle, joinstyle, edgegapcolor, dashed}
    ``Rectangle((0, 0), 1, 1, **kw)`` followed by the setters in ``steps``,
    all under ``rc_context(rcparams)`` (matplotlib reads
    ``patch.edgecolor`` again whenever it falls back to it for the hatch
    colour, pltrs when the patch is made) (``[name, value]`` for ``set_edgecolor``,
    ``set_hatchcolor``, ``set_facecolor``, ``set_edgegapcolor`` and
    ``set_linestyle``, a name or ``[offset, [on, off, ...]]``); colours as RGBA (``get_facecolor()``,
    ``get_edgecolor()``, ``get_hatchcolor()``, ``get_edgegapcolor()`` or
    ``null``), ``dashed`` is ``_has_dashed_edge()``.
    Mirrors ``test_patch_color_none``, ``test_default_capstyle``,
    ``test_default_joinstyle``, ``test_patch_hatchcolor_inherit_logic``,
    ``test_patch_hatchcolor_fallback_logic``,
    ``test_facecolor_none_force_edgecolor_false``,
    ``test_facecolor_none_force_edgecolor_true``,
    ``test_facecolor_none_edgecolor_force_edgecolor``,
    ``test_patch_edgegapcolor_getter_setter``,
    ``test_patch_edgegapcolor_init`` and ``test_patch_has_dashed_edge``.

Left out: ``test_patch_str`` (Python ``repr``), the datetime and units
tests, ``test_fancyarrow_shape_error`` (the arrow shape is an enum in
pltrs, so an unknown one cannot be given), ``test_color_override_warning`` (a warning), ``test_default_antialiased``
and ``test_default_linestyle`` (setting ``None`` to restore a default is
Python API; pltrs setters take values), ``test_boxstyle_errors`` (the
``fancy`` area parses box styles), ``test_patch_linestyle_accents`` and
``test_patch_zero_linewidth_dashed_uses_solid_gc_dashes`` (smoke tests
drawing only), ``test_empty_fancyarrow`` (empty array arguments) and
``test_patch_edgegapcolor_update_from`` (``update_from`` copies Python
attributes). An edge gap colour of ``"none"`` is left out too: matplotlib
tells it from ``None`` (no gap colour) although both draw nothing, while
pltrs uses ``"none"`` for no gap colour.
"""

import matplotlib as mpl
import matplotlib.colors as mcolors
import numpy as np
from matplotlib.patches import (Arrow, Circle, Ellipse, FancyArrow, Polygon, Rectangle,
                                Wedge)

from spec_support import ProbeSet

probes = ProbeSet("patch_behaviour")


def _shape(kind, **params):
    if kind == "rectangle":
        point = params.get("rotation_point", "xy")
        return Rectangle(params["xy"], params["width"], params["height"],
                         angle=params.get("angle", 0.0),
                         rotation_point=tuple(point) if isinstance(point, list) else point)
    if kind == "ellipse":
        return Ellipse(params["xy"], params["width"], params["height"],
                       angle=params.get("angle", 0.0))
    if kind == "circle":
        return Circle(params["xy"], params["radius"])
    if kind == "polygon":
        return Polygon(np.asarray(params["xy"], float).reshape(-1, 2), closed=params["closed"])
    if kind == "wedge":
        return Wedge(params["center"], params["r"], params["theta1"], params["theta2"],
                     width=params.get("width"))
    if kind == "arrow":
        return Arrow(params["x"], params["y"], params["dx"], params["dy"],
                     width=params.get("width", 1.0))
    if kind == "fancy_arrow":
        return FancyArrow(params["x"], params["y"], params["dx"], params["dy"],
                          width=params["width"], head_width=params["head_width"],
                          head_length=params["head_length"],
                          shape=params.get("shape", "full"),
                          overhang=params.get("overhang", 0))
    raise ValueError(kind)


def _step(patch, name, value):
    if name == "set_data":
        patch.set_data(**value)
    else:
        getattr(patch, name)(value)


def _path(path):
    return {"vertices": path.vertices.tolist(),
            "codes": None if path.codes is None else path.codes.tolist()}


def _query(patch, query):
    if isinstance(query, list):
        name, points = query
        return [bool(patch.contains_point(p)) for p in points]
    if query == "corners":
        return patch.get_corners().tolist()
    if query == "center":
        return list(patch.get_center())
    if query == "vertices":
        return [list(v) for v in patch.get_vertices()]
    if query == "co_vertices":
        return [list(v) for v in patch.get_co_vertices()]
    if query == "verts":
        verts = patch.get_verts()
        return [list(v) for v in verts]
    if query == "xy":
        return patch.get_xy().tolist()
    if query == "extents":
        return patch.get_extents().get_points().tolist()
    if query == "path":
        return _path(patch.get_transform().transform_path(patch.get_path()))
    raise ValueError(query)


def _geometry(shape, steps, queries):
    patch = _shape(**shape)
    for name, value in steps:
        _step(patch, name, value)
    return {str(q if isinstance(q, str) else q[0]): _query(patch, q) for q in queries}


def _rgba(color):
    return list(mcolors.to_rgba(color))


def _style(kw, steps, rcparams):
    with mpl.rc_context(rcparams):
        rect = Rectangle((0, 0), 1, 1, **kw)
        for name, value in steps:
            if name == "set_linestyle" and isinstance(value, list):
                value = (value[0], tuple(value[1]))
            getattr(rect, name)(value)
        gap = rect.get_edgegapcolor()
        return {"facecolor": _rgba(rect.get_facecolor()),
                "edgecolor": _rgba(rect.get_edgecolor()),
                "hatchcolor": _rgba(rect.get_hatchcolor()),
                "capstyle": rect.get_capstyle(), "joinstyle": rect.get_joinstyle(),
                "edgegapcolor": None if gap is None else _rgba(gap),
                "dashed": bool(rect._has_dashed_edge())}


RECT = {"kind": "rectangle", "xy": [10, 20], "width": 1, "height": 2}
ELLIPSE = {"kind": "ellipse", "xy": [10.5, 21], "width": 1, "height": 2}
OPEN = [[0, 0], [0, 1], [1, 1]]
CLOSED = OPEN + [[0, 0]]
WEDGE = {"kind": "wedge", "center": [0, 0], "r": 5, "theta1": 0, "theta2": 45, "width": 2}
ALL = ["verts", "extents", "path"]

GEOMETRIES = [
    (RECT, [], ["corners", "center", *ALL]),
    (RECT, [["set_angle", 90]], ["corners", "center", *ALL]),
    (RECT, [["set_angle", 33]], ["corners", "center", *ALL]),
    ({**RECT, "rotation_point": "center", "angle": 45}, [], ["corners", "center", *ALL]),
    ({**RECT, "rotation_point": [0, 0], "angle": 30}, [], ["corners", "center", "verts"]),
    ({"kind": "rectangle", "xy": [1.0, 2.0], "width": 2, "height": 3, "angle": 30.0}, [],
     ["verts"]),
    ({"kind": "rectangle", "xy": [-3, -2], "width": 3, "height": 2}, [], ["verts"]),
    ({"kind": "rectangle", "xy": [0, 0], "width": -3, "height": -2}, [], ["verts", "extents"]),
    (ELLIPSE, [], ["corners", "center", "vertices", "co_vertices", *ALL]),
    (ELLIPSE, [["set_angle", 90]], ["corners", "center", "vertices", "co_vertices"]),
    (ELLIPSE, [["set_angle", 33]], ["corners", "vertices", "co_vertices"]),
    ({"kind": "ellipse", "xy": [0, 0], "width": 0, "height": 0}, [],
     ["vertices", "co_vertices"]),
    ({"kind": "ellipse", "xy": [0, 0], "width": 2, "height": 1, "angle": 30}, [],
     ["vertices", "co_vertices", "extents"]),
    ({"kind": "ellipse", "xy": [2.252, -10.859], "width": 2.265, "height": 1.98,
      "angle": 68.78}, [], ["vertices", "co_vertices"]),
    ({"kind": "ellipse", "xy": [0.5, 0.5], "width": 0.5, "height": 1.0}, [],
     [["contains", [[0.0, 0.5], [0.2, 0.5], [0.25, 0.5], [0.5, 0.5], [0.74, 0.5],
                    [0.5, 0.99], [0.5, 1.01]]]]),
    ({"kind": "circle", "xy": [1, 2], "radius": 3}, [], ["extents", ["contains", [[1, 2],
                                                                                 [3.9, 2],
                                                                                 [4.1, 2]]]]),
    ({"kind": "polygon", "xy": OPEN, "closed": True}, [], ["xy", "verts"]),
    ({"kind": "polygon", "xy": OPEN, "closed": True}, [["set_xy", OPEN]], ["xy"]),
    ({"kind": "polygon", "xy": CLOSED, "closed": False}, [], ["xy", "verts"]),
    ({"kind": "polygon", "xy": CLOSED, "closed": False}, [["set_xy", CLOSED]], ["xy"]),
    ({"kind": "polygon", "xy": OPEN, "closed": False}, [["set_xy", OPEN]], ["xy"]),
    ({"kind": "polygon", "xy": CLOSED, "closed": True}, [["set_xy", CLOSED]], ["xy"]),
    ({"kind": "polygon", "xy": [[0, 0], [0, 0]], "closed": False}, [], ["xy"]),
    ({"kind": "polygon", "xy": [[0, 0]], "closed": True}, [], ["xy", "extents"]),
    ({"kind": "polygon", "xy": [], "closed": True}, [], ["verts"]),
    (WEDGE, [], ["path"]),
    (WEDGE, [["set_center", [1, 1]]], ["path"]),
    (WEDGE, [["set_radius", 8]], ["path"]),
    (WEDGE, [["set_width", 3]], ["path"]),
    (WEDGE, [["set_theta1", 30], ["set_theta2", 50]], ["path"]),
    ({**WEDGE, "width": None}, [["set_theta2", 270]], ["path", "extents"]),
    ({"kind": "arrow", "x": 2, "y": 0, "dx": 0, "dy": 10}, [], ["verts"]),
    ({"kind": "arrow", "x": 2, "y": 0, "dx": 0, "dy": 10},
     [["set_data", {"x": .5, "dx": 3, "dy": 8, "width": 1.2}]], ["verts"]),
    ({"kind": "fancy_arrow", "x": 0, "y": 0, "dx": 10, "dy": 10, "width": .5,
      "head_width": 1, "head_length": 5}, [], ["verts"]),
    ({"kind": "fancy_arrow", "x": 0, "y": 0, "dx": 10, "dy": 10, "width": .5,
      "head_width": 1, "head_length": 5},
     [["set_data", {"x": 1, "y": 1, "dx": 15, "dy": 15, "width": 2, "head_width": 2,
                    "head_length": 1}]], ["verts"]),
    ({"kind": "fancy_arrow", "x": 0, "y": 0, "dx": 3, "dy": 1, "width": .2,
      "head_width": .6, "head_length": .9, "shape": "left", "overhang": .3}, [], ["verts"]),
]


def test_geometry():
    for shape, steps, queries in GEOMETRIES:
        probes.probe("patch.geometry", _geometry, shape=shape, steps=steps, queries=queries)


STYLES = [
    ({"facecolor": "none", "alpha": 1}, [], {}),
    ({}, [], {}),
    ({"hatch": "//", "ec": "red", "hatchcolor": "yellow"}, [], {"hatch.color": "edge"}),
    ({"hatch": "//"}, [["set_edgecolor", "orange"]], {"hatch.color": "edge"}),
    ({"hatch": "//"}, [["set_edgecolor", "orange"], ["set_hatchcolor", "cyan"]],
     {"hatch.color": "edge"}),
    ({"hatch": "//"}, [["set_hatchcolor", "purple"], ["set_edgecolor", "green"]],
     {"hatch.color": "edge"}),
    ({"hatch": "//", "hatchcolor": "green"}, [], {}),
    ({"hatch": "//", "hatchcolor": "green"}, [], {"hatch.color": "blue"}),
    ({"hatch": "//", "hatchcolor": "green", "edgecolor": "red"}, [], {"hatch.color": "yellow"}),
    ({"hatch": "//"}, [], {"hatch.color": "blue"}),
    ({"hatch": "//", "edgecolor": "red"}, [], {"hatch.color": "blue"}),
    ({"hatch": "//", "edgecolor": "red"}, [], {"hatch.color": "edge"}),
    ({"hatch": "//"}, [], {}),
    ({"facecolor": "none"}, [], {"patch.force_edgecolor": False}),
    ({"facecolor": "none"}, [], {"patch.force_edgecolor": True}),
    ({"facecolor": "none"}, [], {"patch.force_edgecolor": False, "patch.edgecolor": "red"}),
    ({"facecolor": "none"}, [], {"patch.force_edgecolor": True, "patch.edgecolor": "red"}),
    ({"fill": False}, [], {}),
    ({"fill": False, "facecolor": "green"}, [], {}),
    ({"edgegapcolor": "blue"}, [], {}),
    ({}, [["set_edgegapcolor", "red"]], {}),
    ({}, [["set_linestyle", "--"]], {}),
    ({}, [["set_linestyle", ":"]], {}),
    ({}, [["set_linestyle", "-."]], {}),
    ({}, [["set_linestyle", [0, [2, 2, 10, 2]]]], {}),
    ({"alpha": 0.5, "facecolor": "red", "edgecolor": "blue"}, [], {}),
]


def test_styles():
    for kw, steps, rc in STYLES:
        probes.probe("patch.style", _style, kw=kw, steps=steps, rcparams=rc)
