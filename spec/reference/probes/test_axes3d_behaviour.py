"""Behaviour of 3-D axes, ported from the non-image tests of
``mpl_toolkits/mplot3d/tests/test_axes3d.py`` that the ``axes3d`` area
(mouse navigation, ``format_coord``, aspect) does not already cover.

Area ``axes3d_behaviour``. Every op draws on ``Figure()`` (6.4 x 4.8 in at
100 dpi) with an Agg canvas and ``ax = fig.add_subplot(projection="3d")``.
Limits are ``[[x0, x1], [y0, y1], [z0, z1]]``.

Ops
---
``axes3d.limits``  {scene} -> [limits, ...]
    The limits after each step of a scene:

    ``aspect_datalim``  ``ax.plot([0, 1], [0, 2], [0, 3])``, then
        ``set_aspect("equal", adjustable="datalim")`` and back to ``"auto"``,
        drawing after each step (``test_set_aspect_datalim_restores_limits``).
    ``aspect_datalim_untouched``  ``ax.plot([1, 2], [3, 4], [5, 6])``,
        ``set_aspect("equal", adjustable="datalim")``, ``set_xlim(-50, 50)``,
        ``set_aspect("auto", adjustable="datalim")``: the axis set by hand
        keeps its limits, the others come back
        (``test_set_aspect_datalim_restores_untouched_axes``).
    ``lines3d`` / ``polys3d`` / ``lines3d_ragged``  ``add_collection3d`` of a
        ``Line3DCollection`` or ``Poly3DCollection`` (autolim)
        (``test_line3dCollection_autoscaling``,
        ``test_poly3dCollection_autoscaling``,
        ``test_line3dcollection_autolim_ragged``).
    ``autoscale``  ``margins(x=0, y=.1, z=.2)`` and ``plot([0, 1], [0, 1],
        [0, 1])``; ``autoscale(False)``, ``set_autoscalez_on(True)`` and
        ``plot([0, 2], [0, 2], [0, 2])``; ``autoscale(axis="x")`` and the same
        plot (``test_autoscale``, without its private ``_view_margin``).
    ``shared_retick``  two axes sharing z, both ``plot([0, 1], [0, 1], [0,
        2])``, then ``ax1.set_zticks([-0.5, 0, 2, 2.5])``: the limits of
        both (``test_shared_axes_retick``).
    ``scale:<name>``  an empty axes with all three scales set to ``name``
        (``test_scale3d_default_limits``).

``axes3d.zbounds``  {} -> [{"zlim", "zbound", "inverted"}, ...]
    ``set_zlim(0, 1)``, ``set_zbound((0, 2))``, ``invert_zaxis()``,
    ``set_zbound(upper=1)``, ``set_zbound(lower=2)``
    (``test_inverted_zaxis``).
``axes3d.set_limit``  {axis, side, value} -> limits
    ``set_<axis>lim3d(**{side: value})``: the limits, or an error for
    non-finite values (``test_invalid_axes_limits``, ``test_set_zlim``).
``axes3d.errors``  {case} -> true
    Errors raised by ``set_proj_type("persp", focal_length=0)``
    (``focal_zero``), ``plot_wireframe(..., rstride=0, cstride=0)``
    (``wireframe_strides``) and ``margins(-1)`` (``margin``), against
    valid calls (``focal_half``, ``wireframe_ok``, ``margin_ok``)
    (``test_axes3d_focal_length_checks``, ``test_wireframe3dzerostrideraises``,
    ``test_margins_errors``).
``axes3d.margins``  {} -> [[x, y, z], ...]
    ``margins()`` after ``margins(0.2)``, ``margins(0.1, 0.2, 0.3)``,
    ``margins(x=0)``, ``margins(y=0.1)`` and ``margins(z=0)``
    (``test_margins``, ``test_margin_getters``).
``axes3d.box_aspect``  {vertical_axis} -> [a, b, c]
    ``view_init(0, 0, 0, vertical_axis=...)``, a draw and
    ``set_box_aspect(None)``: the box aspect rolled to the vertical axis
    (``_roll_to_vertical(_box_aspect)``, the edge lengths the camera sees,
    always 4:4:3; ``test_set_box_aspect_vertical_axis`` checks the stored
    order, which pltrs keeps private).
``axes3d.scatter_nonfinite``  {} -> {"finite_offsets", "aligned"}
    ``ax.scatter(x, y, z, color=four colours)`` with a NaN or infinite
    coordinate in three of the points: the number of points drawn, and
    whether there are as many face colours as offsets
    (``test_scatter_masked_color``; matplotlib deletes the non-finite
    points, pltrs keeps them undrawn, the colours aligned either way).

``axes3d.ticks``  {scene} -> {"x": axis ticks, "y": ..., "z": ...}
    After a draw, each axis' ``get_ticklocs()``, ``[t.get_text() for t in
    get_ticklabels()]`` and the same with ``minor=True``, as ``{"locs",
    "labels", "minor_locs", "minor_labels"}`` (``get_zticks`` and
    ``get_zticklabels`` for z; every location the locator gives, inside the
    view or not, minor ones on major ticks left out). Scenes:

    ``default``  an empty axes.
    ``inset``  ``set_zlim(0.05, 0.95)``: ticks outside the view.
    ``minor``  ``set_xlim(0, 1)``, ``xaxis.set_minor_locator(
        MultipleLocator(0.1))`` and ``yaxis.set_tick_params(labelright=True)``
        (the second label side of the y axis: its labels come twice).
    ``log``  ``set_zscale("log")`` and ``set_zlim(1, 1000)``.
    ``dates``  ``plot([0, 1], [0, 1], [19000, 19010])``, then
        ``zaxis_date()``.
    ``dates_zlim``  ``set_zlim(19000, 19003)``, then ``zaxis_date()``.
    ``dates_kept_formatter``  ``zaxis.set_major_formatter(NullFormatter())``,
        then ``zaxis_date()``: the date locator, the formatter set by hand.
``axes3d.axis_position``  {elev, azim, roll, inverted?} -> [x, y, z]
    ``view_init(elev, azim, roll)`` (after ``invert_xaxis()`` when
    ``inverted``), a draw and ``get_axis_position()``.
``axes3d.format_data``  {axis, value, scene} -> str
    ``format_<axis>data(value)`` after a draw (``format_xdata``,
    ``format_ydata``, ``format_zdata``), for the scenes ``default``,
    ``log`` and ``dates`` of ``axes3d.ticks``.

Left out: ``test_axes3d_repr`` and ``test_invalid_line_data`` (Python
API), ``test_bar3d_*`` and ``test_ax3d_tickcolour`` (colours drawn, figure
goldens), ``test_proj_transform`` and ``test_world`` (private projection
helpers), ``test_quaternion``, ``test_rotate``, ``test_pan``,
``test_toolbar_zoom_pan``, ``test_format_coord``,
``test_ctrl_rotation_snaps_to_5deg`` and the 3-D scale navigation tests
(the ``axes3d`` area), ``test_mixedsamplesraises`` (strides and counts are
one enum in pltrs), ``test_unautoscale`` (``auto=`` of ``set_xlim``),
``test_inverted_cla``, ``test_axes3d_set_aspect_deperecated_params``
(deprecations), ``test_axes3d_focal_length_checks``' orthographic case
(an orthographic projection has no focal length in pltrs), the
tight-bbox and offset-text tests, and smoke tests.
"""

import numpy as np
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.figure import Figure
from matplotlib.ticker import MultipleLocator, NullFormatter
from mpl_toolkits.mplot3d import art3d

from spec_support import ProbeSet

probes = ProbeSet("axes3d_behaviour")


def _axes():
    fig = Figure()
    FigureCanvasAgg(fig)
    return fig, fig.add_subplot(projection="3d")


def _lims(ax):
    return [list(ax.get_xlim()), list(ax.get_ylim()), list(ax.get_zlim())]


def _limits(scene):
    fig, ax = _axes()
    observed = []
    if scene == "aspect_datalim":
        ax.plot([0, 1], [0, 2], [0, 3])
        for aspect in [None, "equal", "auto"]:
            if aspect is not None:
                ax.set_aspect(aspect, adjustable="datalim")
            fig.canvas.draw()
            observed.append(_lims(ax))
    elif scene == "aspect_datalim_untouched":
        ax.plot([1, 2], [3, 4], [5, 6])
        observed.append(_lims(ax))
        ax.set_aspect("equal", adjustable="datalim")
        ax.set_xlim(-50, 50)
        ax.set_aspect("auto", adjustable="datalim")
        observed.append(_lims(ax))
    elif scene in ("lines3d", "lines3d_ragged"):
        lines = ([[(0, 0, 0), (1, 4, 2)], [(1, 1, 3), (2, 0, 2)], [(1, 0, 4), (1, 4, 5)]]
                 if scene == "lines3d" else
                 [[(0, 0, 0), (1, 1, 1), (2, 2, 2)], [(0, 1, 0), (1, 2, 1)],
                  [(1, 0, 1), (2, 1, 2), (3, 2, 3), (4, 3, 4)]])
        ax.add_collection3d(art3d.Line3DCollection(lines), autolim=True)
        observed.append(_lims(ax))
    elif scene == "polys3d":
        ax.add_collection3d(art3d.Poly3DCollection([np.array([[0, 0, 0], [1, 1, 3],
                                                              [1, 0, 4]])]))
        observed.append(_lims(ax))
    elif scene == "autoscale":
        ax.margins(x=0, y=.1, z=.2)
        ax.plot([0, 1], [0, 1], [0, 1])
        observed.append(_lims(ax))
        ax.autoscale(False)
        ax.set_autoscalez_on(True)
        ax.plot([0, 2], [0, 2], [0, 2])
        observed.append(_lims(ax))
        ax.autoscale(axis="x")
        ax.plot([0, 2], [0, 2], [0, 2])
        observed.append(_lims(ax))
    elif scene == "shared_retick":
        ax2 = fig.add_subplot(212, projection="3d", sharez=ax)
        ax.plot([0, 1], [0, 1], [0, 2])
        ax2.plot([0, 1], [0, 1], [0, 2])
        ax.set_zticks([-0.5, 0, 2, 2.5])
        observed += [_lims(ax), _lims(ax2)]
    else:
        scale = scene.removeprefix("scale:")
        ax.set_xscale(scale)
        ax.set_yscale(scale)
        ax.set_zscale(scale)
        fig.canvas.draw()
        observed.append(_lims(ax))
    return observed


def _zbounds():
    fig, ax = _axes()
    steps = [lambda: ax.set_zlim(0, 1), lambda: ax.set_zbound((0, 2)), ax.invert_zaxis,
             lambda: ax.set_zbound(upper=1), lambda: ax.set_zbound(lower=2)]
    observed = []
    for step in steps:
        step()
        observed.append({"zlim": list(ax.get_zlim()), "zbound": list(ax.get_zbound()),
                         "inverted": bool(ax.zaxis_inverted())})
    return observed


def _set_limit(axis, side, value):
    fig, ax = _axes()
    getattr(ax, f"set_{axis}lim3d")(**{side: value})
    return _lims(ax)


def _errors(case):
    fig, ax = _axes()
    if case.startswith("focal"):
        ax.set_proj_type("persp", focal_length=0 if case == "focal_zero" else 0.5)
    elif case.startswith("wireframe"):
        x, y = np.meshgrid(np.linspace(-1, 1, 5), np.linspace(-1, 1, 5))
        stride = 0 if case == "wireframe_strides" else 1
        ax.plot_wireframe(x, y, x * y, rstride=stride, cstride=stride)
    else:
        ax.margins(-1 if case == "margin" else -0.4)
    return True


def _margins():
    fig, ax = _axes()
    observed = []
    for args, kwargs in [((0.2,), {}), ((0.1, 0.2, 0.3), {}), ((), {"x": 0}),
                         ((), {"y": 0.1}), ((), {"z": 0})]:
        ax.margins(*args, **kwargs)
        observed.append([float(m) for m in ax.margins()])
    return observed


def _box_aspect(vertical_axis):
    fig, ax = _axes()
    ax.view_init(elev=0, azim=0, roll=0, vertical_axis=vertical_axis)
    fig.canvas.draw()
    ax.set_box_aspect(None)
    return ax._roll_to_vertical(ax._box_aspect).tolist()


def _scatter_nonfinite():
    fig, ax = _axes()
    path3d = ax.scatter([np.nan, 1, 2, 1], [0, np.inf, 2, 1], [0, 1, -np.inf, 1],
                        color=[[0.0, 0.0, 0.0, 1]] * 4)
    fig.canvas.draw()
    offsets = np.asarray(path3d.get_offsets())
    colors = super(type(path3d), path3d).get_facecolors()
    return {"finite_offsets": int(np.isfinite(offsets).all(axis=1).sum()),
            "aligned": len(offsets) == len(colors)}


def _tick_scene(scene):
    fig, ax = _axes()
    if scene == "inset":
        ax.set_zlim(0.05, 0.95)
    elif scene == "minor":
        ax.set_xlim(0, 1)
        ax.xaxis.set_minor_locator(MultipleLocator(0.1))
        ax.yaxis.set_tick_params(labelright=True)
    elif scene == "log":
        ax.set_zscale("log")
        ax.set_zlim(1, 1000)
    elif scene == "dates":
        ax.plot([0, 1], [0, 1], [19000, 19010])
        ax.zaxis_date()
    elif scene == "dates_zlim":
        ax.set_zlim(19000, 19003)
        ax.zaxis_date()
    elif scene == "dates_kept_formatter":
        ax.zaxis.set_major_formatter(NullFormatter())
        ax.zaxis_date()
    else:
        assert scene == "default", scene
    fig.canvas.draw()
    return ax


def _ticks(scene):
    ax = _tick_scene(scene)
    observed = {}
    for name, axis in [("x", ax.xaxis), ("y", ax.yaxis), ("z", ax.zaxis)]:
        observed[name] = {
            "locs": list(axis.get_ticklocs()),
            "labels": [t.get_text() for t in axis.get_ticklabels()],
            "minor_locs": list(axis.get_ticklocs(minor=True)),
            "minor_labels": [t.get_text() for t in axis.get_ticklabels(minor=True)],
        }
    assert observed["z"]["locs"] == list(ax.get_zticks())
    return observed


def _axis_position(elev, azim, roll, inverted=False):
    fig, ax = _axes()
    if inverted:
        ax.invert_xaxis()
    ax.view_init(elev, azim, roll)
    fig.canvas.draw()
    return [bool(high) for high in ax.get_axis_position()]


def _format_data(axis, value, scene):
    ax = _tick_scene(scene)
    return getattr(ax, f"format_{axis}data")(value)


TICK_SCENES = ["default", "inset", "minor", "log", "dates", "dates_zlim", "dates_kept_formatter"]


SCENES = ["aspect_datalim", "aspect_datalim_untouched", "lines3d", "polys3d",
          "lines3d_ragged", "autoscale", "shared_retick",
          *[f"scale:{s}" for s in ["linear", "log", "symlog", "logit", "asinh"]]]


def test_limits():
    for scene in SCENES:
        probes.probe("axes3d.limits", _limits, scene=scene)
    probes.probe("axes3d.zbounds", _zbounds)


def test_set_limits():
    for axis, sides in [("x", ["left", "right"]), ("y", ["bottom", "top"]),
                        ("z", ["bottom", "top"])]:
        for side in sides:
            for value in [np.inf, np.nan, 2.0]:
                probes.probe("axes3d.set_limit", _set_limit, axis=axis, side=side, value=value)


def test_errors_and_margins():
    for case in ["focal_zero", "focal_half", "wireframe_strides", "wireframe_ok", "margin",
                 "margin_ok"]:
        probes.probe("axes3d.errors", _errors, case=case)
    probes.probe("axes3d.margins", _margins)


def test_box_aspect_and_scatter():
    for vertical_axis in ["x", "y", "z"]:
        probes.probe("axes3d.box_aspect", _box_aspect, vertical_axis=vertical_axis)
    probes.probe("axes3d.scatter_nonfinite", _scatter_nonfinite)


def test_ticks_and_readout():
    for scene in TICK_SCENES:
        probes.probe("axes3d.ticks", _ticks, scene=scene)
    for elev, azim, roll in [(30, -60, 0), (-30, 120, 0), (10, 45, 0), (60, 200, 30),
                             (90, -90, 0), (-75, 10, -20)]:
        probes.probe("axes3d.axis_position", _axis_position, elev=elev, azim=azim, roll=roll)
    probes.probe("axes3d.axis_position", _axis_position, elev=30, azim=-60, roll=0,
                 inverted=True)
    for scene, values in [("default", [0.123456, -0.5, 1234.5678, 3.14159e-5]),
                          ("log", [0.123456, 42.0, 999.9]), ("dates", [19003.25])]:
        for axis in ["x", "y", "z"]:
            for value in values:
                probes.probe("axes3d.format_data", _format_data, axis=axis, value=value,
                             scene=scene)
    assert _axis_position(30, -60, 0) == [False, True, False]
