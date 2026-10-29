"""matplotlib's own non-image tests of collections
(``matplotlib/tests/test_collections.py``) that the
``collection_behaviour`` area leaves out.

Area ``mpl_collections``. Every op runs in the style matplotlib's test
fixture applies (``classic`` plus ``_classic_test_patch``) unless its
``style`` is ``mpl20`` (tests marked ``mpl20`` or ``default``).
``np.random`` draws come from ``spec_support.rng``. Collections are added
to ``plt.subplots()`` axes, which pltrs collections always live in.

Ops
---
``EventCollection.get``  {steps} -> {"positions", "orientation",
    "horizontal", "linelength", "lineoffset", "segments", "linestyle",
    "linewidth", "color", "colors"} of the collection of
    ``generate_EventCollection_plot`` (positions ``[0, 1, 2, 3, 5, 8, 13,
    21]``, horizontal, offset 1, length 0.5, width 2, red, solid) after the
    calls in ``steps``, ``[name, argument]`` pairs: ``set_positions``,
    ``add_positions``, ``switch_orientation``, ``set_orientation``,
    ``set_linelength``, ``set_lineoffset``, ``set_linestyle`` (a name or an
    ``[offset, [on, off]]`` pattern), ``set_linewidth`` and ``set_color``.
    ``linestyle`` is ``getp(coll, "linestyle")``: ``[offset, pattern]``
    pairs scaled by the width (``test__EventCollection__*``).
``LineCollection.norm``  {} -> ``[coll.norm(2), coll.norm(2)]`` for a line
    collection mapping ``arange(4)``, before and after ``set_array(arange(4)
    + 5)``: the norm keeps its limits (``test_collection_norm_autoscale``).
``Collection.get_datalim``  {scene} -> ``get_datalim(ax.transData)``:

    ``null``  ``[[x0, y0], [x1, y1]]`` of an empty ``PathCollection``
        (``test_null_collection_datalim``: the null box).
    ``no_offsets``  the same for ``PathCollection([Path([(0, 0), (1,
        0)])])``, whose path is in data coordinates
        (``test_no_offsets_datalim``).
    ``offsets_late``  {"null_differs", "late_matches"}: circles of size 2
        without offsets, with ``offsets=(10, 10)``, and given them by
        ``set_offsets`` afterwards (``test_set_offsets_late``).
``PathCollection.get_linestyle``  {} -> {"linestyles", "linewidths",
    "linestyles_3", "linewidths_3"}: ``get_linestyle()`` and
    ``get_linewidth()`` of an empty path collection given two solid styles
    and widths ``[1, 2, 3]``, then three solid styles (``test_lslw_bcast``:
    cycled to their common length).
``Collection.set_linestyle``  {ls} -> ``get_linestyle()`` after
    ``set_linestyle(ls)``, or the error for an unknown style
    (``test_set_wrong_linestyle``; outside any axes in matplotlib, as an
    empty ``Collection()`` cannot be added to one).
``Axes.scatter``  {ls} -> ``get_linestyle()`` of ``ax.scatter([0, 1, 2],
    [0, 1, 2], linestyle=ls)`` once the figure is saved as PDF
    (``test_scatter_empty_linestyle_pdf``, ``test_linestyle_single_dashes``).
``PathCollection.get_capstyle``, ``PathCollection.get_joinstyle``  {init,
    set} -> the style of ``PathCollection([], capstyle=init)`` (``None``:
    not given), after ``set_capstyle(set)`` when ``set`` is not ``None``
    (``test_capstyle``, ``test_joinstyle``).
``LineCollection.get``  {} -> {"linewidth", "edgecolor", "zorder",
    "facecolor", "facecolor_none"} of ``LineCollection(None,
    linewidth=2.2, edgecolor="r", zorder=3, facecolors=[0, 1, 0, 1])`` and
    the face colours of ``LineCollection(None, facecolor=None)``
    (``test_LineCollection_args``; outside any axes in matplotlib, which
    cannot add a collection without segments to one).
``LineCollection.get_segments``  {} -> whether a 256-point segment reads
    back unchanged (``test_get_segments``).
``Collection.get_offset_transform``  {} -> whether a skew given at
    creation and one set afterwards read back alike
    (``test_set_offset_transform``).
``EllipseCollection.get``  {} -> {"widths", "heights", "angles",
    "set_widths", "set_heights", "set_angles"}: one ellipse of 2 x 3 at 45°
    at ten random offsets, then twenty random axes and angles
    (``test_EllipseCollection_setter_getter``).
``QuadMesh.get_cursor_data``  {array} -> what is under the cell ``(0.5,
    0.5)`` of ``pcolormesh(x * x.T)``, ``x = arange(4)``, without an array
    (``mesh.set_array(None)``) or with ``ones`` (``test_quadmesh_cursor_data``).
``QuadMesh.get_coordinates``  {} -> the vertex grid of ``pcolormesh`` on the
    mesh of ``[0, 1, 2]`` x ``[2, 4, 6]`` (``test_quadmesh_get_coordinates``).
``QuadMesh.set_array``  {shading} -> the arrays read back after setting
    ``arange`` values then ``ones`` on a flat or Gouraud mesh
    (``test_quadmesh_set_array``); ``shading: "wrong_size"`` sets 54 values
    on a 7 x 10 mesh, an error (``test_quadmesh_set_array_validation``).
``Collection.get_facecolors``  {func, scene} -> the face colours of a
    ``pcolor``/``pcolormesh``:

    ``vmin_vmax``  ``[[1]]`` in ``plasma`` through ``Normalize(0, 1)``,
        then with the norm's limits moved to (1, 2)
        (``test_quadmesh_vmin_vmax``).
    ``alpha_array``  ``arange(9).reshape(3, 3)`` given ``alpha=z / 8`` when
        made, and given ``set_alpha(z / 8)`` (``test_quadmesh_alpha_array``).
    ``alpha_validation``  ``arange(12).reshape(3, 4)`` given two alphas: an
        error (``test_alpha_validation``).
``Collection.color_logic``  {func} -> ``[{"faces", "edges"}, ...]``: the
    colours of a ``pcolor``/``pcolormesh`` of ``arange(12).reshape(3, 4)``
    after each step of ``test_color_logic`` (in the ``default`` style):
    edges and faces given, set, reset to their defaults (``None``), the
    array removed and given back, and faces given as one colour or as RGB
    or RGBA rows, which the mapping overrides until the array is removed.
``PolyQuadMesh.get_paths``  {scene} -> {"paths", "faces", "mesh_faces"}:
    the cells ``pcolor`` keeps and their colours where a corner (``x``,
    ``y``) or a value (``z``) of the first cell is masked, and the colours of
    the other cells of the unmasked ``pcolormesh``
    (``test_polyquadmesh_masked_vertices_array``, before its arrays change).
``Collection.get_hatch_linewidth``  {set} -> the hatch line width of a
    hatched ``PolyCollection`` made under ``hatch.linewidth = 2`` or given
    ``set_hatch_linewidth(2)`` (``test_hatch_linewidth``).
``FillBetweenPolyCollection``  {case} -> the error of a ``where`` of the
    wrong size or curves of unequal sizes
    (``test_fill_between_poly_collection_raise``).
``PathCollection.legend_elements``  {scene} -> {"labels", "colors",
    "alphas", "markersizes", "markerfacecolors"} of the lines a legend draws
    for the handles of ``sc.legend_elements(...)``, ``sc = ax.scatter(x,
    y, c=c, s=s, cmap="jet", marker="o", linewidths=0)`` with ten random
    points, colours in 0..4 and sizes in 10..299
    (``test_pathcollection_legend_elements``): ``colors`` (``fmt="{x:g}"``),
    ``sizes`` (``prop="sizes", alpha=0.5, color="red"``), ``func``
    (``prop="sizes", num=4, fmt="{x:.2f}", func=2x``) and ``locator``
    (``num=MaxNLocator(nbins=9, min_n_ticks=8, steps=...)``); ``inverse``
    scatters fifty points of size ``5 / c`` and asks for the sizes of
    ``[0.2, 0.4, 0.6, 0.8]`` through ``func=5 / s``
    (``test_legend_inverse_size_label_relationship``).

Covered elsewhere: the image and ``check_figures_equal`` tests (cases of
``mpl_collections``); ``test_add_collection``, ``test_picking``,
``test_quadmesh_contains*``, ``test_quadmesh_cursor_data_multiple_points``,
``test_scatter_alpha_array``, the ``pcolormesh`` cases of
``test_quadmesh_vmin_vmax`` and ``test_quadmesh_alpha_array``,
``test_blended_collection_autolim``,
``test_singleton_autolim``, ``test_autolim_with_zeros`` and the hatch colour
tests (``collection_behaviour``); ``test_quiver_limits`` and
``test_barb_limits`` (``quiver_behaviour``).

Left out: ``test_array_dimensions`` and the 2-D and RGB(A) arrays of
``test_quadmesh_set_array_validation`` (pltrs arrays are flat lists of
values), the new arrays of ``test_polyquadmesh_masked_vertices_array``
(pltrs's ``pcolor`` makes plain polygons, which keep their number when the
array changes),
``test_set_offset_units`` (``set_offsets`` takes data floats), the
``pcolor`` case of ``test_quadmesh_get_coordinates`` (no mesh behind
``pcolor`` in pltrs), ``test_pandas_indexing``,
``test_EventCollection_nosort``, ``test_collection_set_verts_array``,
``test_collection_set_array`` and ``test_check_offsets_dtype`` (Python
containers and dtypes), the direction and dimension errors of
``test_fill_between_poly_collection_raise`` (typed in pltrs),
``test_draw_path_collection_no_hatchcolor`` and
``test_third_party_backend_hatchcolors_arg_fallback`` (calls into Python
renderers).
"""

import io

import matplotlib as mpl
import matplotlib.collections as mcollections
import matplotlib.colors as mcolors
import matplotlib.path as mpath
import matplotlib.pyplot as plt
import matplotlib.transforms as mtransforms
import numpy as np
from matplotlib.collections import (EventCollection, FillBetweenPolyCollection,
                                    LineCollection, PolyCollection)

from probes._mpl_tests import prober
from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_collections")
probe = prober(probes)


def _rgba(colors):
    return mcolors.to_rgba_array(colors).tolist()


def _dashes(styles):
    return [[float(offset), None if pattern is None else [float(v) for v in pattern]]
            for offset, pattern in styles]


# --- event collections --------------------------------------------------------


def _event_collection():
    coll = EventCollection(np.array([0., 1., 2., 3., 5., 8., 13., 21.]),
                           orientation="horizontal", lineoffset=1, linelength=.5,
                           linewidth=2, color=[1, 0, 0, 1], linestyle="solid",
                           antialiased=True)
    fig, ax = plt.subplots()
    ax.add_collection(coll)
    return coll


def event_op(steps):
    coll = _event_collection()
    for name, *argument in steps:
        value = argument[0] if argument else None
        if name in ("switch_orientation",):
            coll.switch_orientation()
        elif name == "set_linestyle" and isinstance(value, list):
            coll.set(linestyle=(value[0], tuple(value[1])))
        elif name in ("set_linestyle", "set_linewidth"):
            coll.set(**{name[4:]: value})
        elif name == "set_color":
            coll.set_color(np.array(value))
        else:
            getattr(coll, name)(value)
    return {"positions": [float(p) for p in coll.get_positions()],
            "orientation": coll.get_orientation(),
            "horizontal": coll.is_horizontal(),
            "linelength": coll.get_linelength(),
            "lineoffset": coll.get_lineoffset(),
            "segments": [seg.tolist() for seg in coll.get_segments()],
            "linestyle": _dashes(plt.getp(coll, "linestyle")),
            "linewidth": float(plt.getp(coll, "linewidth")),
            "color": _rgba([coll.get_color()])[0],
            "colors": _rgba(coll.get_colors())}


EVENT_STEPS = {
    "get_props": [],
    "set_positions": [["set_positions", [0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89]]],
    "add_positions": [["switch_orientation"], ["add_positions", 34],
                      ["switch_orientation"]],
    "append_positions": [["append_positions", 89]],
    "extend_positions": [["extend_positions", [55, 89]]],
    "switch_orientation": [["switch_orientation"]],
    "switch_orientation_2x": [["switch_orientation"], ["switch_orientation"]],
    "set_orientation": [["set_orientation", "vertical"]],
    "set_linelength": [["set_linelength", 15]],
    "set_lineoffset": [["set_lineoffset", -5.0]],
    "set_linestyle": [["set_linestyle", "dashed"]],
    "set_linestyle_tuple": [["set_linestyle", [0, [3.7, 1.6]]]],
    "set_linewidth": [["set_linewidth", 5]],
    "set_color": [["set_color", [0, 1, 1, 1]]],
}


def _probe_events(*names):
    for name in names:
        probe("EventCollection.get", event_op, style="mpl20", steps=EVENT_STEPS[name])


def test__EventCollection__get_props():
    _probe_events("get_props")


def test__EventCollection__set_positions():
    _probe_events("set_positions")


def test__EventCollection__add_positions():
    _probe_events("add_positions")


def test__EventCollection__append_positions():
    _probe_events("append_positions")


def test__EventCollection__extend_positions():
    _probe_events("extend_positions")


def test__EventCollection__switch_orientation():
    _probe_events("switch_orientation")


def test__EventCollection__switch_orientation_2x():
    _probe_events("switch_orientation_2x")


def test__EventCollection__set_orientation():
    _probe_events("set_orientation")


def test__EventCollection__set_linelength():
    _probe_events("set_linelength")


def test__EventCollection__set_lineoffset():
    _probe_events("set_lineoffset")


def test__EventCollection__set_prop():
    _probe_events("set_linestyle", "set_linestyle_tuple", "set_linewidth")


def test__EventCollection__set_color():
    _probe_events("set_color")


# --- norms, limits, line styles ----------------------------------------------


def norm_op():
    fig, ax = plt.subplots()
    coll = LineCollection(np.arange(24).reshape((4, 3, 2)), array=np.arange(4))
    ax.add_collection(coll)
    before = coll.norm(2)
    coll.set_array(np.arange(4) + 5)
    return [float(before), float(coll.norm(2))]


def test_collection_norm_autoscale():
    assert probe("LineCollection.norm", norm_op) == [2 / 3, 2 / 3]


def datalim_op(scene):
    fig, ax = plt.subplots()
    if scene == "null":
        coll = mcollections.PathCollection([])
    elif scene == "no_offsets":
        coll = mcollections.PathCollection([mpath.Path([(0, 0), (1, 0)])])
    else:
        null = mcollections.CircleCollection(sizes=[2])
        init = mcollections.CircleCollection(sizes=[2], offsets=(10, 10))
        late = mcollections.CircleCollection(sizes=[2])
        late.set_offsets((10, 10))
        bounds = []
        for coll in (null, init, late):
            ax.add_collection(coll)
            bounds.append(coll.get_datalim(ax.transData).bounds)
        return {"null_differs": bounds[0] != bounds[1],
                "late_matches": bounds[1] == bounds[2]}
    ax.add_collection(coll)
    return coll.get_datalim(ax.transData).get_points().tolist()


def test_datalim():
    probe("Collection.get_datalim", datalim_op, scene="null")
    probe("Collection.get_datalim", datalim_op, scene="no_offsets")
    assert probe("Collection.get_datalim", datalim_op, scene="offsets_late") == {
        "null_differs": True, "late_matches": True}


def lslw_op():
    fig, ax = plt.subplots()
    col = mcollections.PathCollection([])
    ax.add_collection(col)
    col.set_linestyles(["-", "-"])
    col.set_linewidths([1, 2, 3])
    out = {"linestyles": _dashes(col.get_linestyles()),
           "linewidths": [float(w) for w in col.get_linewidths()]}
    col.set_linestyles(["-", "-", "-"])
    out.update(linestyles_3=_dashes(col.get_linestyles()),
               linewidths_3=[float(w) for w in col.get_linewidths()])
    return out


def test_lslw_bcast():
    probe("PathCollection.get_linestyle", lslw_op, style="mpl20")


def set_linestyle_op(ls):
    c = mcollections.Collection()
    c.set_linestyle(ls)
    return _dashes(c.get_linestyle())


def test_set_wrong_linestyle():
    assert isinstance(probe("Collection.set_linestyle", set_linestyle_op, ls="fuzzy"),
                      ValueError)
    probe("Collection.set_linestyle", set_linestyle_op, ls="dashed")


def scatter_op(ls):
    fig, ax = plt.subplots()
    ls = (ls[0], tuple(ls[1])) if isinstance(ls, list) else ls
    sc = ax.scatter([0, 1, 2], [0, 1, 2], linestyle=ls)
    fig.savefig(io.BytesIO(), format="pdf")
    return _dashes(sc.get_linestyle())


def test_scatter_linestyles():
    for ls in ("", " ", "none", [0.0, [2.0, 2.0]]):
        probe("Axes.scatter", scatter_op, ls=ls)


def _join_cap(which):
    def run(init, set):
        fig, ax = plt.subplots()
        col = mcollections.PathCollection([], **({which: init} if init else {}))
        ax.add_collection(col)
        if set is not None:
            getattr(col, f"set_{which}")(set)
        return getattr(col, f"get_{which}")()

    return run


def test_capstyle_joinstyle():
    for which, first, second in (("capstyle", "round", "butt"),
                                 ("joinstyle", "round", "miter")):
        op = f"PathCollection.get_{which}"
        assert probe(op, _join_cap(which), style="mpl20", init=None, set=None) is None
        assert probe(op, _join_cap(which), style="mpl20", init=first, set=None) == first
        assert probe(op, _join_cap(which), style="mpl20", init=first, set=second) == second


def line_collection_op():
    lc = LineCollection(None, linewidth=2.2, edgecolor="r", zorder=3,
                        facecolors=[0, 1, 0, 1])
    plain = LineCollection(None, facecolor=None)
    return {"linewidth": float(lc.get_linewidth()[0]), "edgecolor": _rgba(lc.get_edgecolor()),
            "zorder": lc.get_zorder(), "facecolor": _rgba(lc.get_facecolor()),
            "facecolor_none": _rgba(plain.get_facecolor())}


def test_line_collection_args():
    probe("LineCollection.get", line_collection_op)


def segments_op():
    fig, ax = plt.subplots()
    segments = np.tile(np.linspace(0, 1, 256), (2, 1)).T
    lc = LineCollection([segments])
    ax.add_collection(lc)
    readback, = lc.get_segments()
    return bool(np.all(segments == readback))


def test_get_segments():
    assert probe("LineCollection.get_segments", segments_op)


def offset_transform_op():
    fig, ax = plt.subplots()
    skew = mtransforms.Affine2D().skew(2, 2)
    init = mcollections.Collection(offsets=[(0, 0)], offset_transform=skew)
    late = mcollections.Collection(offsets=[(0, 0)])
    late.set_offset_transform(skew)
    ax.add_collection(init)
    ax.add_collection(late)
    return skew == init.get_offset_transform() == late.get_offset_transform()


def test_set_offset_transform():
    assert probe("Collection.get_offset_transform", offset_transform_op)


def ellipses_op():
    stream = rng(0)
    fig, ax = plt.subplots()
    ec = mcollections.EllipseCollection(widths=(2,), heights=(3,), angles=(45,),
                                        offsets=stream.uniform(20).reshape(10, 2) * 10,
                                        units="x", offset_transform=ax.transData)
    out = {"widths": ec.get_widths().tolist(), "heights": ec.get_heights().tolist(),
           "angles": ec.get_angles().tolist()}
    ax.add_collection(ec)
    ec.set(widths=stream.uniform(20) * 2, heights=stream.uniform(20) * 3,
           angles=stream.uniform(20) * 180)
    out.update(set_widths=ec.get_widths().tolist(), set_heights=ec.get_heights().tolist(),
               set_angles=ec.get_angles().tolist())
    return out


def test_ellipse_collection_setter_getter():
    probe("EllipseCollection.get", ellipses_op)


# --- meshes -------------------------------------------------------------------


def cursor_op(array):
    x = np.arange(4)
    fig, ax = plt.subplots()
    mesh = ax.pcolormesh(x[:, None] * x[None, :])
    if array == "none":
        mesh.set_array(None)
    else:
        mesh.set_array(np.ones((4, 4)))
    fig.draw_without_rendering()
    x, y = mesh.get_transform().transform((0.5, 0.5))
    event = mpl.backend_bases.MouseEvent("motion_notify_event", fig.canvas, x, y)
    data = mesh.get_cursor_data(event)
    return None if data is None else data.tolist()


def test_quadmesh_cursor_data():
    assert probe("QuadMesh.get_cursor_data", cursor_op, array="none") is None
    assert probe("QuadMesh.get_cursor_data", cursor_op, array="ones") == [1.0]


def coordinates_op():
    xx, yy = np.meshgrid([0, 1, 2], [2, 4, 6])
    fig, ax = plt.subplots()
    return ax.pcolormesh(xx, yy, np.ones((2, 2))).get_coordinates().tolist()


def test_quadmesh_get_coordinates():
    probe("QuadMesh.get_coordinates", coordinates_op)


def set_array_op(shading):
    fig, ax = plt.subplots()
    if shading == "wrong_size":
        coll = ax.pcolormesh(np.arange(11), np.arange(8), np.zeros((7, 10)))
        coll.set_array(np.arange(54))
        return None
    cells = 9 if shading == "flat" else 16
    z = np.arange(cells).reshape((3, 3) if shading == "flat" else (4, 4))
    coll = ax.pcolormesh(np.arange(4), np.arange(4), np.ones(z.shape), shading=shading)
    out = []
    for values in (z, np.ones(cells)):
        coll.set_array(values)
        fig.canvas.draw()
        out.append(np.ravel(coll.get_array()).tolist())
    return out


def test_quadmesh_set_array():
    probe("QuadMesh.set_array", set_array_op, shading="flat")
    probe("QuadMesh.set_array", set_array_op, shading="gouraud")
    assert isinstance(probe("QuadMesh.set_array", set_array_op, shading="wrong_size"),
                      ValueError)


def facecolors_op(func, scene):
    fig, ax = plt.subplots()
    pcolor = getattr(ax, func)
    if scene == "vmin_vmax":
        cmap = mpl.colormaps["plasma"]
        norm = mcolors.Normalize(vmin=0, vmax=1)
        coll = pcolor([[1]], cmap=cmap, norm=norm)
        fig.canvas.draw()
        before = _rgba(coll.get_facecolors())
        norm.vmin, norm.vmax = 1, 2
        fig.canvas.draw()
        return [before, _rgba(coll.get_facecolors())]
    if scene == "alpha_array":
        z = np.arange(9).reshape((3, 3))
        alpha = z / z.max()
        coll1 = pcolor(np.arange(4), np.arange(4), z, alpha=alpha)
        coll2 = pcolor(np.arange(4), np.arange(4), z)
        coll2.set_alpha(alpha)
        fig.canvas.draw()
        return [_rgba(coll1.get_facecolors()), _rgba(coll2.get_facecolors())]
    pc = pcolor(np.arange(12).reshape((3, 4)))
    pc.set_alpha([0.5, 0.6])
    pc.update_scalarmappable()
    return _rgba(pc.get_facecolors())


def test_mesh_facecolors():
    for func in ("pcolor", "pcolormesh"):
        probe("Collection.get_facecolors", facecolors_op, func=func, scene="vmin_vmax")
        probe("Collection.get_facecolors", facecolors_op, func=func, scene="alpha_array")
        assert isinstance(probe("Collection.get_facecolors", facecolors_op, func=func,
                                scene="alpha_validation"), ValueError)


def color_logic_op(func):
    pcfunc = getattr(plt, func)
    z = np.arange(12).reshape(3, 4)
    out = []

    def record(pc):
        pc.update_scalarmappable()
        out.append({"faces": _rgba(pc.get_facecolor()), "edges": _rgba(pc.get_edgecolor())})

    record(pcfunc(z, edgecolors="red", facecolors="none"))
    pc = pcfunc(z)
    pc.set_facecolor("none")
    pc.set_edgecolor("red")
    record(pc)
    pc.set_alpha(0.5)
    record(pc)
    pc.set_alpha(None)
    record(pc)
    pc.set_edgecolor(None)
    record(pc)
    pc.set_facecolor(None)
    record(pc)
    pc.set_array(None)
    record(pc)
    pc.set_array(z)
    record(pc)
    record(pcfunc(z, edgecolors=(1, 0, 0), facecolors=(0, 1, 0)))
    for channels in (3, 4):
        pc = pcfunc(z, edgecolors=(1, 0, 0), facecolors=np.ones((12, channels)))
        record(pc)
        pc.set_array(None)
        record(pc)
    return out


def test_color_logic():
    for func in ("pcolor", "pcolormesh"):
        probe("Collection.color_logic", color_logic_op, style="mpl20", func=func)


def masked_vertices_op(scene):
    xx, yy = np.meshgrid([0, 1, 2], [0, 1, 2, 3])
    zz = (xx * yy)[:-1, :-1]
    quadmesh = plt.pcolormesh(xx, yy, zz)
    quadmesh.update_scalarmappable()
    corner = (xx == 0) & (yy == 0)
    if scene == "x":
        polymesh = plt.pcolor(np.ma.masked_where(corner, xx), yy, zz)
    elif scene == "y":
        polymesh = plt.pcolor(xx, np.ma.masked_where(corner, yy), zz)
    else:
        polymesh = plt.pcolor(np.ma.masked_where(corner[:-1, :-1], zz))
    polymesh.update_scalarmappable()
    return {"paths": len(polymesh.get_paths()), "faces": _rgba(polymesh.get_facecolor()),
            "mesh_faces": _rgba(quadmesh.get_facecolor()[1:, :])}


def test_polyquadmesh_masked_vertices_array():
    for scene in ("x", "y", "z"):
        res = probe("PolyQuadMesh.get_paths", masked_vertices_op, scene=scene)
        assert res["paths"] == 5 and res["faces"] == res["mesh_faces"]


def hatch_linewidth_op(set):
    fig, ax = plt.subplots()
    polygons = [[(0.1, 0.1), (0.1, 0.4), (0.4, 0.4), (0.4, 0.1)],
                [(0.6, 0.6), (0.6, 0.9), (0.9, 0.9), (0.9, 0.6)]]
    if set:
        coll = PolyCollection(polygons, hatch="x")
        coll.set_hatch_linewidth(2.0)
    else:
        with mpl.rc_context({"hatch.linewidth": 2.0}):
            coll = PolyCollection(polygons, hatch="x")
    ax.add_collection(coll)
    return coll.get_hatch_linewidth()


def test_hatch_linewidth():
    for set in (True, False):
        assert probe("Collection.get_hatch_linewidth", hatch_linewidth_op, set=set) == 2.0


def fill_between_op(case):
    t = np.linspace(0, 16)
    if case == "where":
        f1 = np.sin(t)
        FillBetweenPolyCollection("x", t, f1, f1 + 0.2, where=[False] * 3)
    else:
        f1 = np.asarray([1, 2])
        FillBetweenPolyCollection("y", t, f1, f1 + 0.2)


def test_fill_between_poly_collection_raise():
    for case in ("where", "sizes"):
        assert isinstance(probe("FillBetweenPolyCollection", fill_between_op, case=case),
                          ValueError)


# --- legend elements ----------------------------------------------------------


def _legend_looks(ax, handles, labels):
    lines = ax.legend(handles, labels).legend_handles
    return {"labels": labels,
            "colors": _rgba([line.get_color() for line in lines]),
            "alphas": [line.get_alpha() for line in lines],
            "markersizes": [float(line.get_markersize()) for line in lines],
            "markerfacecolors": _rgba([line.get_markerfacecolor() for line in lines])}


def legend_elements_op(scene):
    fig, ax = plt.subplots()
    if scene == "inverse":
        stream = rng(19680801)
        x, y = stream.uniform(50), stream.uniform(50)
        sc = ax.scatter(x, y, s=5 / (1 - stream.uniform(50)))
        return _legend_looks(ax, *sc.legend_elements(prop="sizes", num=[0.2, 0.4, 0.6, 0.8],
                                                    func=lambda s: 5 / s))
    stream = rng(19680801)
    x, y = stream.uniform(10), stream.uniform(10)
    c = stream.integers(10, 0, 5)
    s = stream.integers(10, 10, 300)
    sc = ax.scatter(x, y, c=c, s=s, cmap="jet", marker="o", linewidths=0)
    if scene == "colors":
        kwargs = {"fmt": "{x:g}"}
    elif scene == "sizes":
        kwargs = {"prop": "sizes", "alpha": 0.5, "color": "red"}
    elif scene == "func":
        kwargs = {"prop": "sizes", "num": 4, "fmt": "{x:.2f}", "func": lambda x: 2 * x}
    else:
        kwargs = {"num": mpl.ticker.MaxNLocator(nbins=9, min_n_ticks=9 - 1,
                                               steps=[1, 2, 2.5, 3, 5, 6, 8, 10])}
    return _legend_looks(ax, *sc.legend_elements(**kwargs))


def test_pathcollection_legend_elements():
    res = probe("PathCollection.legend_elements", legend_elements_op, scene="colors")
    assert res["labels"] == ["0", "1", "2", "3", "4"]
    res = probe("PathCollection.legend_elements", legend_elements_op, scene="sizes")
    assert all(alpha == 0.5 for alpha in res["alphas"])
    res = probe("PathCollection.legend_elements", legend_elements_op, scene="func")
    np.testing.assert_array_almost_equal(
        res["markersizes"], np.sqrt(np.array(res["labels"], float) / 2))
    assert len(probe("PathCollection.legend_elements", legend_elements_op,
                     scene="locator")["labels"]) == 9


def test_legend_inverse_size_label_relationship():
    res = probe("PathCollection.legend_elements", legend_elements_op, scene="inverse")
    np.testing.assert_array_almost_equal(
        [5 / size**2 for size in res["markersizes"]], [0.2, 0.4, 0.6, 0.8], decimal=1)
