"""Behaviour of the axes API, ported from matplotlib's own test suite
(``matplotlib/tests/test_axes.py``, ``test_subplots.py``,
``test_figure.py``): autoscaling, margins and sticky edges, explicit limits
and bounds, shared, twinned and removed axes, aspect and positions, labels,
titles and tick sides, ticks, subplot grids and figure-level axes
management.

Area ``axes_behaviour``. One op replays a scenario (see
``probes/_scenario.py`` for the step language).

Ops
---
``axes_behaviour.run``  {case, steps, figure?, rc?, style?} -> one entry per
    step: the converted result of query steps, ``null`` for the others.

Cases
-----
Every case is named after the upstream test it mirrors (``/variant`` for
parametrisations and for the parts of a test that build several
figures). Upstream assertions comparing two values (``check_figures_equal``
and the like) are ported by recording both. Adaptations:

* random data comes from ``spec_support.rng`` (``test_hist2d_autolimits``,
  ``test_shared_axes_autoscale``, ``test_inverted_cla``);
* ``test_tick_label_update`` labels the ticks with ``set_ticklabels``
  instead of a Python ``FuncFormatter``;
* ``test_remove_shared_axes_relim`` sets the limits of a remaining axes:
  plt-rs handles of removed artists are invalid;
* ``test_twin_with_aspect`` compares positions after a draw rather than
  ``bbox`` before it, which only shows the original positions;
* ``test_zoom_inset`` uses a 4 x 4 mesh over the same square and leaves out
  ``indicate_inset_zoom``;
* ``test_limits_after_scroll_zoom/limits``, ``test_xylim_changed_shared``
  and ``test_centered_bar_label_label_beyond_limits`` record the limits the
  upstream set-up makes (scroll zoom goes through a private method, which
  the ``navigation`` area covers; callbacks are Python functions);
* ``test_shared`` and the ``label_outer`` tests read tick label visibility
  from ``get_tick_params()`` and the offset text, as ``get_xticklabels``
  lists only the visible labels; the integer sharing modes ``0``/``1`` and
  the colorbar and layout-engine variants of ``test_label_outer`` are left
  out;
* ``test_twin_respects_position_after_set_position/*-twinned_first``
  extends the upstream test to ``set_position`` after twinning;
* of ``test_axis_method_errors``, ``test_margins_errors`` and
  ``test_add_subplot_invalid`` only the value errors are ported;
* of ``test_axisbelow``, ``test_remove_overlapping_locs`` and
  ``test_date_empty`` only the getters are ported (the drawing, the minor
  tick filtering and the second epoch are covered elsewhere);
  ``test_remove_overlapping_locs`` uses the setter rather than ``plt.setp``.

The cases ``relim_in_autoscale/<method>`` mirror no upstream test. They
record which artists ``relim`` counts (``Artist._get_in_autoscale``, set by
``add_line``, ``add_patch``, ``add_image`` and ``add_collection`` but not by
``add_artist`` or ``add_collection(..., autolim=False)``): the data limits
after ``<method>``, after ``relim()``, and after ``relim()`` with a line
added. ``relim_in_autoscale/colorbar`` does the same on the axes of a
colorbar with extensions. Nor do the cases
``twin_fixed_data_ratio/<projection>/<twin>``: ``twinx`` and ``twiny`` of
polar and geographic axes, which raise the ``ValueError`` of
``set_adjustable("datalim")`` on axes overriding ``get_data_ratio``.

Left out: tests of Python calling conventions and types (``TypeError`` for
argument counts and mixes, subclass hooks such as ``axes_class``,
``SubplotBase``, ``cla`` overrides, units, pandas, pickling), private API
(``titleOffsetTrans``, ``_set_view_from_bbox``, ``_shared_axes``),
``add_axes``/``add_subplot`` of an existing axes object, tight bounding
boxes of decorations (covered by the figure cases), secondary axes and bar
labels.
"""

import itertools

import numpy as np

from probes._scenario import recorder
from spec_support import ProbeSet, rng

probes = ProbeSet("axes_behaviour")
_case = recorder(probes)


def _lims(*axes):
    return [step for ax in axes for step in ([ax, "get_xlim"], [ax, "get_ylim"])]


SUBPLOT = ["fig", "subplots"]


# --- autoscaling, margins and sticky edges ---------------------------------------------

def test_autoscale():
    _case("test_autoscale_tight", [
        SUBPLOT, [0, "plot", [1, 2, 3, 4]],
        [0, "autoscale", {"enable": True, "axis": "x", "tight": False}],
        [0, "autoscale", {"enable": True, "axis": "y", "tight": True}],
        *_lims(0), [0, "get_autoscalex_on"], [0, "get_autoscaley_on"], [0, "get_autoscale_on"],
        [0, "autoscale", {"enable": None}],
        *_lims(0), [0, "get_autoscalex_on"], [0, "get_autoscaley_on"], [0, "get_autoscale_on"],
    ])
    x = np.arange(100, dtype=float).tolist()
    _case("test_autoscale_log_shared", [
        ["fig", "subplots", 2, 1, {"sharex": True}],
        [0, "loglog", x, x], [1, "semilogx", x, x],
        [0, "autoscale", {"tight": True}], [1, "autoscale", {"tight": True}],
        ["fig", "draw"], *_lims(0, 1),
    ])
    _case("test_use_sticky_edges", [
        SUBPLOT, [0, "imshow", [[0, 1], [2, 3]], {"origin": "lower"}], *_lims(0),
        [0, "use_sticky_edges", False], [0, "autoscale"], *_lims(0),
        [0, "use_sticky_edges", True], [0, "autoscale"], *_lims(0),
    ])
    _case("test_arrow_in_view", [SUBPLOT, [0, "arrow", 1, 1, 1, 1], *_lims(0)])
    _case("test_autoscale_tiny_sticky", [SUBPLOT, [0, "bar", 0, 1e-9], ["fig", "draw"],
                                         [0, "get_ylim"]])
    _case("test_vline_limit", [["fig", "gca"], [0, "axvline", 0.5],
                               [0, "plot", [-0.1, 0, 0.2, 0.1]], *_lims(0)])
    gen = rng(17)
    x, y = gen.uniform(100).tolist(), gen.uniform(100).tolist()
    _case("test_hist2d_autolimits", [["fig", "add_subplot"], [0, "hist2d", x, y], *_lims(0),
                                     [0, "get_autoscale_on"]])
    xs = np.arange(0, 4 * np.pi, 0.05)
    _case("test_fill_between_axes_limits", [
        SUBPLOT, [0, "plot", xs.tolist(), (0.1 * np.sin(xs)).tolist()], *_lims(0),
        [0, "axhline", 0.075], *_lims(0),
    ])
    _case("test_multiplot_autoscale", [
        ["fig", "subplots", 2, 1, {"sharex": "all"}],
        [0, "plot", [18000, 18250, 18500, 18750], [2, 3, 2, 3]], [1, "axhspan", -5, 5],
        *_lims(0, 1),
    ])
    _case("test_bezier_autoscale", [
        SUBPLOT,
        [0, "add_patch", {"new": "PathPatch",
                          "args": [[[-1, 0], [0, -1], [1, 0], [1, 0]], [1, 3, 3, 79]]}],
        [0, "autoscale"], *_lims(0),
    ])


def test_axes_margins():
    zeros = np.zeros((10, 10)).tolist()
    for name, call in [("plot", ["plot", [0, 1, 2, 3]]), ("bar", ["bar", [0, 1, 2, 3], [1, 1, 1, 1]]),
                       ("barh", ["barh", [0, 1, 2, 3], [1, 1, 1, 1]]), ("pcolor", ["pcolor", zeros]),
                       ("pcolorfast", ["pcolorfast", zeros]), ("hist", ["hist", list(range(10))]),
                       ("imshow", ["imshow", zeros])]:
        _case(f"test_axes_margins/{name}", [SUBPLOT, [0, *call], [0, "get_xbound"],
                                            [0, "get_ybound"]])


def test_margins():
    data = [1, 10]
    for name, args in [("one", [1]), ("two", [0.5, 2]), ("keywords", [{"x": -0.2, "y": 0.5}])]:
        _case(f"test_margins/{name}", [SUBPLOT, [0, "plot", data], [0, "margins", *args],
                                       [0, "get_xmargin"], [0, "get_ymargin"], *_lims(0)])
    _case("test_margin_getters", [["fig", "add_subplot"], [0, "margins", 0.2, 0.3],
                                  [0, "get_xmargin"], [0, "get_ymargin"]])
    _case("test_set_margin_updates_limits", [SUBPLOT, [0, "plot", [1, 2], [1, 2]],
                                             [0, "set", {"xscale": "log", "xmargin": 0}],
                                             [0, "get_xlim"]])
    for name, args in [("negative", [-1]), ("negative_y", [1, -1]),
                       ("negative_x_keyword", [{"x": -1}]), ("negative_y_keyword", [{"y": -1}])]:
        _case(f"test_margins_errors/{name}", [["fig", "add_subplot"], [0, "margins", *args]])
    _case("test_log_margins", [SUBPLOT, [0, "set_xmargin", 0.05],
                               [0, "semilogx", [10, 100], [10, 100]], [0, "get_xlim"]],
          rc={"axes.autolimit_mode": "data"})


def test_relim():
    _case("test_relim_visible_only", [
        ["fig", "add_subplot"], [0, "plot", [0.0, 10.0], [0.0, 10.0]], *_lims(0),
        [0, "plot", [-10.0, 20.0], [-10.0, 30.0]], *_lims(0),
        ["@4", "set_visible", False], *_lims(0),
        [0, "relim", {"visible_only": True}], [0, "autoscale_view"], *_lims(0),
    ])
    _case("test_relim_collection", [
        SUBPLOT, [0, "scatter", [1, 2, 3], [4, 5, 6]], [0, "relim"],
        [0, "dataLim"], [0, "dataLim.minpos"],
        ["@1", "set_offsets", [[10, 20], [30, 40]]], [0, "relim"],
        [0, "dataLim"], [0, "dataLim.minpos"],
        [0, "plot", [0, 1], [0, 1]], ["@1", "set_visible", False],
        [0, "relim", {"visible_only": True}], [0, "dataLim"], [0, "dataLim.minpos"],
    ])
    _case("test_relim_collection_log_scale", [
        SUBPLOT, [0, "set_xscale", "log"], [0, "set_yscale", "log"],
        [0, "scatter", [1e-3, 1e-2, 1e-1], [1e1, 1e2, 1e3]],
        ["@3", "set_offsets", [[1e1, 1e4], [1e2, 1e5]]], [0, "relim"],
        [0, "dataLim"], [0, "dataLim.minpos"],
    ])
    xs = np.linspace(0, 10, 50)
    _case("test_relim_collection_autoscale_view", [
        SUBPLOT, [0, "scatter", [], []],
        ["@1", "set_offsets", np.column_stack((xs, np.sin(xs))).tolist()],
        [0, "relim"], [0, "autoscale_view"], *_lims(0),
    ])


def test_relim_in_autoscale():
    def relimmed(ax):
        return [[ax, "dataLim"], [ax, "relim"], [ax, "dataLim"],
                [ax, "plot", [-3, -2], [-3, -2]], [ax, "relim"], [ax, "dataLim"]]

    grid = [[1, 2], [3, 4]]
    z = [[0, 1, 2], [1, 2, 3], [2, 3, 4]]
    xs, ys = [0, 1, 2, 3, 4], [0, 2, 1, 3, 4]
    tri = [[0, 1, 0, 1], [0, 0, 1, 1], [1, 2, 3, 4]]
    patch = {"new": "PathPatch", "args": [[[1, 1], [3, 1], [3, 2], [1, 1]], [1, 2, 2, 79]]}
    for name, call in [
        ("hlines", ["hlines", [1, 2], 0, 3]),
        ("vlines", ["vlines", [1, 2], 0, 3]),
        ("eventplot", ["eventplot", [[1, 2, 4], [3, 5]]]),
        ("hexbin", ["hexbin", xs, ys, {"gridsize": 3}]),
        ("hexbin_marginals", ["hexbin", xs, ys, {"gridsize": 3, "marginals": True}]),
        ("pcolor", ["pcolor", grid]),
        ("pcolormesh", ["pcolormesh", grid]),
        ("pcolorfast_quadmesh", ["pcolorfast", [[0, 1, 3], [0.5, 1.5, 3.5]],
                                 [[0, 0, 0], [2, 2, 2]], [[1, 2]]]),
        ("pcolorfast_image", ["pcolorfast", grid]),
        ("pcolorfast_pcolorimage", ["pcolorfast", [0, 1, 3], [0, 2, 3], grid]),
        ("contour", ["contour", z]),
        ("contourf", ["contourf", z]),
        ("tripcolor", ["tripcolor", *tri]),
        ("tripcolor_gouraud", ["tripcolor", *tri, {"shading": "gouraud"}]),
        ("imshow", ["imshow", grid]),
        ("axline", ["axline", [0, 0], [1, 2]]),
        ("add_patch", ["add_patch", patch]),
        ("add_artist", ["add_artist", patch]),
    ]:
        _case(f"relim_in_autoscale/{name}", [SUBPLOT, [0, *call], *relimmed(0)])
    _case("relim_in_autoscale/colorbar", [
        SUBPLOT, [0, "pcolormesh", grid], ["fig", "colorbar", "@1", {"extend": "both"}],
        *[["@2", f"ax.{method}", *args] for _, method, *args in relimmed(None)],
    ])


# --- explicit limits and bounds ---------------------------------------------------------

def test_limits():
    data = [[-5, -3, 2, 4], [1, 2, -3, 5]]
    _case("test_inverted_limits/x", [SUBPLOT, [0, "invert_xaxis"], [0, "plot", *data], *_lims(0)])
    _case("test_inverted_limits/y", [SUBPLOT, [0, "invert_yaxis"], [0, "plot", *data], *_lims(0)])
    _case("test_inverted_limits/log", [SUBPLOT, [0, "set_yscale", "log"], [0, "set_ylim", 10, 1],
                                       [0, "get_ylim"]])
    _case("test_set_xy_bound", [
        ["fig", "add_subplot"],
        [0, "set_xbound", 2.0, 3.0], [0, "get_xbound"], [0, "get_xlim"],
        [0, "set_xbound", {"upper": 4.0}], [0, "get_xbound"], [0, "get_xlim"],
        [0, "set_xbound", {"lower": 3.0}], [0, "get_xbound"], [0, "get_xlim"],
        [0, "set_ybound", 2.0, 3.0], [0, "get_ybound"], [0, "get_ylim"],
        [0, "set_ybound", {"upper": 4.0}], [0, "get_ybound"], [0, "get_ylim"],
        [0, "set_ybound", {"lower": 3.0}], [0, "get_ybound"], [0, "get_ylim"],
    ])
    _case("test_log_scales_invalid", [
        SUBPLOT, [0, "set_xscale", "log"], [0, "set_xlim", -1, 10], [0, "get_xlim"],
        [0, "set_yscale", "log"], [0, "set_ylim", -1, 10], [0, "get_ylim"],
    ])
    for name, call in [("x_nan", ["set_xlim", float("nan")]), ("x_inf", ["set_xlim", float("inf")]),
                       ("y_nan", ["set_ylim", float("nan")]), ("y_inf", ["set_ylim", float("inf")])]:
        _case(f"test_invalid_axis_limits/{name}", [SUBPLOT, [0, "plot", [0, 1], [0, 1]],
                                                   [0, *call]])
    _case("test_axis_extent_arg", [SUBPLOT, [0, "axis", [5, 10, 15, 20]], *_lims(0)])
    _case("test_axis_extent_arg2", [SUBPLOT, [0, "axis", {"xmin": 5, "xmax": 10, "ymin": 15,
                                                          "ymax": 20}], *_lims(0)])
    _case("test_set_ticks_inverted", [SUBPLOT, [0, "invert_xaxis"], [0, "set_xticks", [0.3, 0.7]],
                                      [0, "get_xlim"], [0, "set_xticks", [-1]], [0, "get_xlim"]])
    x = np.arange(100)
    y = np.linspace(-0.1, 0.1, 100)
    for axis in "xy":
        for auto in [None, True, False]:
            _case(f"test_unautoscale/{axis}-{auto}", [
                SUBPLOT, [0, "scatter", y.tolist(), x.tolist()],
                [0, f"set_{axis}lim", [-0.5, 0.5], {"auto": auto}],
                [0, f"get_autoscale{axis}_on"], ["fig", "draw"], [0, f"get_{axis}lim"],
            ])
    _case("test_limits_after_scroll_zoom/limits", [
        SUBPLOT, [0, "set_xlim", [-0.5, 0.5]], [0, "set_ylim", {"ymin": -1, "ymax": 2}], *_lims(0),
    ])
    bars = []
    for label, value, left in [("a", 10, 0), ("b", 20, 10), ("c", 50, 30)]:
        bars += [[0, "barh", "col", value, {"label": label, "left": left}],
                 [0, "bar_label", f"@{len(bars) + 1}", {"label_type": "center"}]]
    _case("test_centered_bar_label_label_beyond_limits", [
        SUBPLOT, *bars, [0, "set_xlim", None, 20], ["fig", "draw"], *_lims(0),
        [0, "get_autoscalex_on"], [0, "get_autoscaley_on"],
    ])


# --- shared, twinned and removed axes ---------------------------------------------------

def test_sharing():
    _case("test_empty_shared_subplots", [
        ["fig", "subplots", 1, 2, {"sharex": True, "sharey": True}],
        [0, "plot", [1, 2, 3], [2, 4, 6]], *_lims(1),
    ])
    _case("test_shared_scale", [
        ["fig", "subplots", 2, 2, {"sharex": True, "sharey": True}],
        [0, "set_xscale", "log"], [0, "set_yscale", "log"],
        *[[i, f"get_{a}scale"] for i in range(4) for a in "xy"],
        [3, "set_xscale", "linear"], [3, "set_yscale", "linear"],
        *[[i, f"get_{a}scale"] for i in range(4) for a in "xy"],
    ])
    _case("test_shared_axes_retick", [
        ["fig", "subplots", 2, 2, {"sharex": "all", "sharey": "all"}],
        *[[i, "plot", [0, 2], "o-"] for i in range(4)],
        [0, "set_xticks", [-0.5, 0, 1, 1.5]], *[[i, "get_xlim"] for i in range(4)],
        [0, "set_yticks", [-0.5, 0, 2, 2.5]], *[[i, "get_ylim"] for i in range(4)],
    ])
    x = list(range(1, 10))
    _case("test_shared_axes_clear_scale", [
        ["fig", "subplots", 1, 2, {"sharey": True}], [0, "loglog", x, x], [1, "loglog", x, x],
        [0, "clear"], *[[i, f"get_{a}scale"] for i in range(2) for a in "xy"],
    ])
    levels = np.arange(-80, 90, 40).tolist()
    gen = rng(3)
    z = [gen.uniform(len(levels)).tolist() for _ in levels]
    _case("test_shared_axes_autoscale", [
        ["fig", "subplots", 2, 1, {"sharex": True, "sharey": True}],
        [0, "set_xlim", -1000, 1000], [0, "set_ylim", -1000, 1000],
        [0, "contour", levels, levels, z], [1, "contour", levels, levels, z],
        [0, "get_autoscalex_on"], [1, "get_autoscalex_on"],
        [0, "get_autoscaley_on"], [1, "get_autoscaley_on"], *_lims(0, 1),
    ])
    z = np.arange(15).reshape(3, 5).tolist()
    _case("test_sticky_shared_axes/leader", [
        ["fig", "add_subplot", 211], ["fig", "add_subplot", 212, {"sharex": "@ax0"}],
        [1, "pcolormesh", z], *_lims(0, 1),
    ])
    _case("test_sticky_shared_axes/follower", [
        ["fig", "add_subplot", 212], ["fig", "add_subplot", 211, {"sharex": "@ax0"}],
        [0, "pcolormesh", z], *_lims(0, 1),
    ])
    _case("test_remove_shared_axes_relim", [
        ["fig", "subplots", 2, 2, {"sharex": "all", "sharey": "all"}], [1, "get_xlim"],
        [0, "remove"], [1, "set_xlim", 0, 5], [1, "get_xlim"], [2, "get_xlim"],
    ])
    for name, setup in [("gca", [["fig", "gca"]]), ("subplots", [SUBPLOT]),
                        ("subplots_shared", [["fig", "subplots", 2, 2,
                                              {"sharex": "all", "sharey": "all"}]]),
                        ("add_axes", [["fig", "add_axes", [0.1, 0.1, 0.8, 0.8]]])]:
        for axis, twin in [("x", "twinx"), ("y", "twiny")]:
            twin_step = len(setup)
            _case(f"test_remove_shared_axes/{name}-{axis}", [
                *setup, [0, twin], [f"@{twin_step}", "remove"], [0, f"set_{axis}lim", 0, 15],
                [0, f"get_{axis}ticks"],
            ])
    _case("test_xylim_changed_shared", [
        ["fig", "subplots", 2, 1, {"sharex": True, "sharey": True}],
        [0, "set", {"xlim": [1, 3], "ylim": [2, 4]}], *_lims(1),
    ])


def test_twins():
    _case("test_twinx_cla", [
        SUBPLOT, [0, "twinx"], [1, "twiny"], ["fig", "draw"],
        [1, "xaxis.get_visible"], [1, "patch.get_visible"],
        [1, "cla"], [2, "cla"],
        *[[i, attr] for i in [1, 2, 0]
          for attr in ["xaxis.get_visible", "patch.get_visible", "yaxis.get_visible"]],
    ])
    _case("test_twin_inherit_autoscale_setting", [
        SUBPLOT, [0, "twinx"], [0, "set_autoscalex_on", False], [0, "twinx"],
        [1, "get_autoscalex_on"], [2, "get_autoscalex_on"],
        [0, "twiny"], [0, "set_autoscaley_on", False], [0, "twiny"],
        [3, "get_autoscaley_on"], [4, "get_autoscaley_on"],
    ])
    for twin in ["twinx", "twiny"]:
        _case(f"test_twin_respects_position_after_set_position/{twin}", [
            SUBPLOT, [0, "set_position", [0.2, 0.2, 0.5, 0.5]], [0, twin],
            *[[i, "get_position", {"original": original}] for original in [True, False]
              for i in range(2)],
        ])
        _case(f"test_twin_with_aspect/{twin}", [
            SUBPLOT, [0, twin], [0, "set_aspect", 5], [1, "set_aspect", 2], ["fig", "draw"],
            [0, "get_position"], [1, "get_position"],
        ])
    _case("test_twinx_knows_limits", [
        SUBPLOT, [0, "axvspan", 1, 2], [0, "twinx"], [1, "plot", [0, 0.5], [1, 2]],
        [1, "viewLim"], [0, "viewLim"],
    ])
    _case("test_twinx_knows_limits/control", [
        SUBPLOT, [0, "axvspan", 1, 2], [0, "plot", [0, 0.5], [1, 2]], [0, "viewLim"],
    ])
    patch_visibility = [[i, "patch.get_visible"] for i in range(3)]
    _case("test_twinning_patch_visibility_default", [SUBPLOT, [0, "twinx"], *patch_visibility[:2]])
    _case("test_twinning_patch_visibility_respects_delta_zorder", [
        SUBPLOT, [0, "twinx", {"delta_zorder": -1}], [0, "get_zorder"], [1, "get_zorder"],
        *patch_visibility[:2],
    ])
    _case("test_twinning_patch_visibility_multiple_twins_same_zorder", [
        SUBPLOT, [0, "twinx"], [0, "twinx"], *patch_visibility,
    ])
    _case("test_twinning_patch_visibility_updates_for_new_bottom", [
        SUBPLOT, [0, "twinx"], [0, "twinx", {"delta_zorder": -1}], *patch_visibility,
    ])
    _case("test_twinning_patch_visibility_updates_after_set_zorder", [
        SUBPLOT, [0, "twinx"], *patch_visibility[:2], [1, "set_zorder", -1],
        *patch_visibility[:2],
    ])
    for projection in ["aitoff", "hammer", "lambert", "mollweide", "polar"]:
        for twin in ["twinx", "twiny"]:
            _case(f"twin_fixed_data_ratio/{projection}/{twin}", [
                ["fig", "add_subplot", {"projection": projection}], [0, twin],
            ])


def test_twins_and_layout():
    for twin in ["twinx", "twiny"]:
        _case(f"test_twin_stays_aligned_after_constrained_layout/{twin}", [
            SUBPLOT, [0, "set_position", [0.2, 0.2, 0.5, 0.5]], [0, twin], ["fig", "draw"],
            [0, "get_position"], [1, "get_position"],
        ], figure={"layout": "constrained"})
        _case(f"test_twin_respects_position_after_set_position/{twin}-twinned_first", [
            SUBPLOT, [0, twin], [0, "set_position", [0.2, 0.2, 0.5, 0.5]],
            *[[i, "get_position", {"original": original}] for original in [True, False]
              for i in range(2)],
        ])
    grid = [1.0, 2.0, 3.0, 4.0, 5.0]
    _case("test_zoom_inset", [
        SUBPLOT, [0, "pcolormesh", grid, grid, np.ones((4, 4)).tolist()],
        [0, "set_aspect", 1.0], [0, "apply_aspect"],
        [0, "inset_axes", [0.7, 0.7, 0.35, 0.35]],
        ["@4", "set_xlim", 1.5, 2.15], ["@4", "set_ylim", 2, 2.5], ["@4", "set_aspect", 1.0],
        ["fig", "draw"], ["@4", "get_position"], [0, "get_position"],
    ])


def test_inverted_cla():
    img = rng(5).uniform(100).reshape(10, 10).tolist()
    x = np.linspace(0, 2 * np.pi, 100)
    cos = [x.tolist(), np.cos(x).tolist()]
    inverted = [[0, "xaxis.get_inverted"], [0, "yaxis.get_inverted"]]
    _case("test_inverted_cla/image_then_line", [
        ["fig", "gca"], *inverted, [0, "imshow", img], *inverted,
        [0, "cla"], [0, "plot", *cos], *inverted,
        [0, "cla"], [0, "imshow", img], [0, "autoscale"], *inverted,
    ])
    _case("test_inverted_cla/shared", [
        ["fig", "add_subplot", 211], ["fig", "add_subplot", 212, {"sharey": "@ax0"}],
        [0, "yaxis.set_inverted", True], [1, "yaxis.get_inverted"],
        [1, "plot", *cos], [0, "cla"], [1, "yaxis.get_inverted"],
        [1, "cla"], [0, "imshow", img], [1, "plot", *cos], [1, "cla"],
        [0, "yaxis.get_inverted"], [1, "yaxis.get_inverted"],
    ])


# --- aspect and positions ---------------------------------------------------------------

def test_aspect():
    for adjustable in ["box", "datalim"]:
        _case(f"test_shared_with_aspect_1/{adjustable}", [
            ["fig", "subplots", {"nrows": 2, "sharex": True}],
            [0, "set_aspect", 2, {"adjustable": adjustable, "share": True}],
            [1, "get_aspect"], [1, "get_adjustable"],
        ])
        _case(f"test_shared_with_aspect_1/{adjustable}-unshared", [
            ["fig", "subplots", {"nrows": 2, "sharex": True}],
            [0, "set_aspect", 2, {"adjustable": adjustable}], [1, "get_aspect"],
        ])
        _case(f"test_shared_with_aspect_3/{adjustable}", [
            ["fig", "subplots", {"nrows": 2, "sharey": True}],
            [0, "set_aspect", 2, {"adjustable": adjustable}],
            [1, "set_aspect", 0.5, {"adjustable": adjustable}],
            [0, "plot", [1, 2], [3, 4]], [1, "plot", [3, 4], [1, 2]], ["fig", "draw"],
            *_lims(0, 1), [0, "get_position"], [1, "get_position"],
            [0, "get_aspect"], [1, "get_aspect"],
        ])
    _case("test_shared_with_aspect_2", [
        ["fig", "subplots", {"nrows": 2, "sharex": True, "sharey": True}],
        [0, "set_aspect", 2, {"share": True}],
        [0, "plot", [1, 2], [3, 4]], [1, "plot", [3, 4], [1, 2]], ["fig", "draw"], *_lims(0, 1),
    ])
    _case("test_shared_aspect_error", [
        ["fig", "subplots", 1, 2, {"sharex": True, "sharey": True}], [0, "axis", "equal"],
        ["fig", "draw"],
    ])
    for name, value in [("negative", -1), ("zero", 0), ("inf", float("inf")),
                        ("negative_inf", float("-inf"))]:
        _case(f"test_set_aspect_negative/{name}", [SUBPLOT, [0, "set_aspect", value]])
    _case("test_square_plot", [
        SUBPLOT, [0, "plot", [0, 1, 2, 3], [1.0, 3.0, 5.0, 7.0], "mo"], [0, "axis", "square"],
        *_lims(0), [0, "get_aspect"],
        [0, "get_position", {"original": True}], [0, "get_position", {"original": False}],
    ])
    _case("test_aspect_nonlinear_adjustable_box", [
        ["fig", "add_subplot"], [0, "plot", [0.4, 0.6], [0.4, 0.6]],
        [0, "set", {"xscale": "log", "xlim": [1, 10], "yscale": "logit",
                    "ylim": [1 / 11, 1 / 1001], "aspect": 1, "adjustable": "box"}],
        [0, "margins", 0], [0, "get_position"],
    ], figure={"figsize": [10, 10]})
    _case("test_aspect_nonlinear_adjustable_datalim", [
        ["fig", "add_axes", [0.1, 0.1, 0.8, 0.8]], [0, "plot", [0.4, 0.6], [0.4, 0.6]],
        [0, "set", {"xscale": "log", "xlim": [1, 100], "yscale": "logit",
                    "ylim": [1 / 101, 1 / 11], "aspect": 1, "adjustable": "datalim"}],
        [0, "margins", 0], [0, "apply_aspect"], *_lims(0),
    ], figure={"figsize": [10, 10]})
    _case("test_box_aspect", [
        SUBPLOT, [0, "twinx"], [1, "plot", [12, 344]], [0, "set_box_aspect", 1],
        [0, "get_box_aspect"], ["fig", "draw"], [0, "get_position"], [1, "get_position"],
    ])
    _case("test_box_aspect/reference", [
        SUBPLOT, [0, "margins", 0], [0, "plot", [0, 2], [6, 8]],
        [0, "set_aspect", "equal", {"adjustable": "box"}], ["fig", "draw"], [0, "get_position"],
    ])
    for name, order in [("position_first", [[0, "set_position", [0.1, 0.1, 0.9, 0.2]],
                                            ["fig", "draw"], [0, "set_box_aspect", 1.0]]),
                        ("aspect_first", [[0, "set_box_aspect", 1.0], ["fig", "draw"],
                                          [0, "set_position", [0.1, 0.1, 0.9, 0.2]]])]:
        _case(f"test_box_aspect_custom_position/{name}", [
            SUBPLOT, *order, ["fig", "draw"], [0, "get_position"],
        ])
    _case("test_set_position", [
        SUBPLOT, [0, "set_aspect", 3.0],
        [0, "set_position", [0.1, 0.1, 0.4, 0.4], {"which": "both"}], [0, "get_position"],
        [0, "set_aspect", 2.0],
        [0, "set_position", [0.1, 0.1, 0.4, 0.4], {"which": "original"}], [0, "get_position"],
        [0, "set_aspect", 3.0],
        [0, "set_position", [0.1, 0.1, 0.4, 0.4], {"which": "active"}], [0, "get_position"],
    ])
    _case("test_sharing_does_not_link_positions", [
        ["fig", "add_subplot", 221], ["fig", "add_axes", [0.6, 0.6, 0.3, 0.3], {"sharex": "@ax0"}],
        [1, "get_position"], ["fig", "subplots_adjust", {"left": 0}], [1, "get_position"],
        [0, "get_position"],
    ])


# --- labels, titles and tick sides ------------------------------------------------------

def test_decorations():
    _case("test_get_labels", [SUBPLOT, [0, "set_xlabel", "x label"], [0, "set_ylabel", "y label"],
                              [0, "get_xlabel"], [0, "get_ylabel"]])
    shifts = [step for axis, locs in [("x", ["left", "center", "right", "center"]),
                                      ("y", ["top", "center", "bottom", "center"])]
              for loc in locs
              for step in [[0, f"set_{axis}label", "Test label", {"loc": loc}],
                           [0, f"{axis}axis.label.get_horizontalalignment"]]]
    _case("test_label_shift", [SUBPLOT, *shifts])
    _case("test_title_location_roundtrip", [
        SUBPLOT, [0, "set_title", "aardvark"], [0, "set_title", "left", {"loc": "left"}],
        [0, "set_title", "right", {"loc": "right"}],
        *[[0, "get_title", {"loc": loc}] for loc in ["left", "right", "center"]],
    ], rc={"axes.titlelocation": "center"})
    _case("test_title_location_roundtrip/get_foo", [SUBPLOT, [0, "get_title", {"loc": "foo"}]])
    _case("test_title_location_roundtrip/set_foo", [SUBPLOT, [0, "set_title", "fail",
                                                                {"loc": "foo"}]])
    sides = [[0, f"{a}axis.get_tick_params", {"which": which}] for a in "xy"
             for which in ["major", "minor"]]
    _case("test_rc_tick", [["fig", "add_subplot", 1, 1, 1], *sides],
          rc={"xtick.bottom": False, "xtick.top": True, "ytick.left": True, "ytick.right": False})
    _case("test_rc_major_minor_tick", [["fig", "add_subplot", 1, 1, 1], *sides],
          rc={"xtick.top": True, "ytick.right": True, "xtick.bottom": True, "ytick.left": True,
              "xtick.minor.bottom": False, "xtick.major.bottom": False,
              "ytick.major.left": False, "ytick.minor.left": False})
    for name, call in [("left_xmin", ["set_xlim", {"left": 0, "xmin": 1}]),
                       ("right_xmax", ["set_xlim", {"right": 0, "xmax": 1}]),
                       ("bottom_ymin", ["set_ylim", {"bottom": 0, "ymin": 1}]),
                       ("top_ymax", ["set_ylim", {"top": 0, "ymax": 1}])]:
        _case(f"test_axis_method_errors/{name}", [SUBPLOT, [0, *call]])
    x = (10 ** np.array([18, 18.1, 18.2, 18.3])).tolist()
    y, yerr = [100, 80, 60, 30], [10, 10, 10, 10]
    _case("test_errorbar_log_autoscale_order_independent", [
        ["fig", "subplots", 2], [0, "set_xscale", "log"], [0, "set_yscale", "log"],
        [0, "errorbar", x, y, {"yerr": yerr}], [1, "errorbar", x, y, {"yerr": yerr}],
        [1, "set_xscale", "log"], [1, "set_yscale", "log"], *_lims(0, 1),
    ])


# --- ticks ------------------------------------------------------------------------------

def test_ticks():
    ys = np.log(np.linspace(0.1, 100)).tolist()
    _case("test_log_scales", [
        SUBPLOT, [0, "plot", ys], [0, "set_yscale", "log", {"base": 5.5}], [0, "invert_yaxis"],
        [0, "set_xscale", "log", {"base": 9.0}], ["fig", "draw"],
        [0, "get_xticks"], [0, "get_xticklabels"], [0, "get_yticks"], [0, "get_yticklabels"],
        [0, "get_xticks", {"minor": True}], [0, "get_yticks", {"minor": True}],
    ])
    _case("test_log_scales_no_data", [
        SUBPLOT, [0, "set", {"xscale": "log", "yscale": "log"}],
        [0, "xaxis.set_major_locator", {"new": "MultipleLocator", "args": [1]}], *_lims(0),
    ])
    _case("test_adjust_numtick_aspect", [
        SUBPLOT, [0, "locator_params", {"axis": "y", "nbins": "auto"}],
        [0, "set_xlim", 0, 1000], [0, "set_aspect", "equal"], ["fig", "draw"], [0, "get_yticks"],
        [0, "set_ylim", 0, 1000], ["fig", "draw"], [0, "get_yticks"],
    ])
    _case("test_auto_numticks", [
        ["fig", "subplots", 4, 4],
        *[[i, f"get_{a}ticks"] for i in range(16) for a in "xy"],
    ])
    _case("test_auto_numticks_log", [
        SUBPLOT, [0, "loglog", [1e-20, 1e5], [1e-16, 10]], [0, "get_xticks"], [0, "get_yticks"],
    ], rc={"axes.autolimit_mode": "round_numbers"})
    _case("test_subsampled_ticklabels", [
        SUBPLOT, [0, "plot", list(range(10))],
        [0, "xaxis.set_ticks", (np.arange(10) + 0.1).tolist()],
        [0, "locator_params", {"nbins": 5}],
        [0, "xaxis.set_ticklabels", list("bcdefghijk")], ["fig", "draw"],
        [0, "get_xticks"], [0, "get_xticklabels"],
    ])
    _case("test_get_xticklabel", [
        SUBPLOT, [0, "plot", list(range(10))], ["fig", "draw"],
        [0, "get_xticklabels"], [0, "get_yticklabels"],
    ], style="classic")
    _case("test_tick_label_update", [
        SUBPLOT, [0, "set_xticks", [-1, 0, 1, 2, 3]], [0, "set_xlim", -0.5, 2.5],
        [0, "xaxis.set_ticklabels", ["", "", "unit value", "", ""]],
        ["fig", "draw"], [0, "get_xticklabels"],
    ])
    for xscale in ["symlog", "log"]:
        for yscale in ["symlog", "log"]:
            _case(f"test_minorticks_on/{xscale}-{yscale}", [
                SUBPLOT, [0, "plot", [1, 2, 3, 4]], [0, "set_xscale", xscale],
                [0, "set_yscale", yscale], [0, "minorticks_on"], ["fig", "draw"],
                [0, "get_xticks", {"minor": True}], [0, "get_yticks", {"minor": True}],
            ])


# --- axis getters (test_axes.py, test_axis.py, test_ticker.py, test_dates.py) ----------

def test_axis_getters():
    settings = [False, "line", True]
    _case("test_axisbelow", [
        ["fig", "subplots", 1, 3, {"sharex": True, "sharey": True}],
        *[step for i, setting in enumerate(settings)
          for step in ([i, "set_axisbelow", setting], [i, "get_axisbelow"])],
    ])
    positions = [[0, "xaxis.get_ticks_position"], [0, "yaxis.get_ticks_position"]]
    _case("test_get_tick_position_rcParams", [["fig", "add_subplot"], *positions],
          rc={"xtick.top": True, "xtick.labeltop": True, "xtick.bottom": False,
              "xtick.labelbottom": False, "ytick.right": True, "ytick.labelright": True,
              "ytick.left": False, "ytick.labelleft": False})
    _case("test_get_tick_position_tick_top_tick_right", [
        ["fig", "add_subplot"], [0, "xaxis.tick_top"], [0, "yaxis.tick_right"], *positions,
    ])
    _case("test_get_tick_position_tick_params", [
        ["fig", "add_subplot"],
        [0, "tick_params", {"top": True, "labeltop": True, "bottom": False,
                            "labelbottom": False, "right": True, "labelright": True,
                            "left": False, "labelleft": False}],
        *positions,
    ])
    _case("test_remove_overlapping_locs", [
        SUBPLOT, [0, "xaxis.get_remove_overlapping_locs"],
        [0, "xaxis.set_remove_overlapping_locs", False], [0, "xaxis.get_remove_overlapping_locs"],
    ])
    _case("test_date_empty", [SUBPLOT, [0, "xaxis_date"], ["fig", "draw"], [0, "get_xlim"]])


# --- subplot grids: sharing and outer labels (test_subplots.py) ------------------------

SHARE_MODES = [False, True, "all", "none", "row", "col"]


def _joined(n):
    return [[i, f"get_shared_{a}_axes.joined", f"@ax{i}", f"@ax{j}"]
            for a in "xy" for i in range(n) for j in range(i + 1, n)]


def _tick_labels(n):
    return [[i, f"{a}axis.{query}"] for i in range(n) for a in "xy"
            for query in ["get_tick_params", "offsetText.get_visible"]]


def _axis_labels(n):
    return [[i, query] for i in range(n) for query in ["get_xlabel", "get_ylabel"]]


def test_subplots_sharing():
    _case("test_shared/default", [["fig", "subplots", 2, 2], *_joined(4), *_tick_labels(4)])
    for sharex, sharey in itertools.product(SHARE_MODES, repeat=2):
        _case(f"test_shared/{sharex}-{sharey}", [
            ["fig", "subplots", 2, 2, {"sharex": sharex, "sharey": sharey}],
            *_joined(4), *_tick_labels(4),
        ])
    for keyword in ["sharex", "sharey"]:
        _case(f"test_exceptions/{keyword}", [["fig", "subplots", 2, 2, {keyword: "blah"}]])
    _case("test_shared_and_moved/y", [
        ["fig", "subplots", 1, 2, {"sharey": True}], *_tick_labels(2),
        [1, "yaxis.tick_left"], *_tick_labels(2),
    ])
    _case("test_shared_and_moved/x", [
        ["fig", "subplots", 2, 1, {"sharex": True}], *_tick_labels(2),
        [1, "xaxis.tick_bottom"], *_tick_labels(2),
    ])
    for top, bottom, left, right in itertools.product([True, False], repeat=4):
        _case(f"test_subplots_hide_ticklabels/{top}-{bottom}-{left}-{right}", [
            ["fig", "subplots", 3, 3, {"sharex": True, "sharey": True}],
            *[[i, f"{a}axis.get_tick_params"] for i in range(9) for a in "xy"],
        ], rc={"xtick.labeltop": top, "xtick.labelbottom": bottom, "ytick.labelleft": left,
               "ytick.labelright": right})


def test_label_outer():
    labelled = [[i, "set", {"xlabel": "foo", "ylabel": "bar"}] for i in range(4)]
    for remove in [True, False]:
        _case(f"test_label_outer/{remove}", [
            ["fig", "subplots", 2, 2, {"sharex": True, "sharey": True}], *labelled,
            *[[i, "label_outer", {"remove_inner_ticks": remove}] for i in range(4)],
            *_tick_labels(4), *_axis_labels(4),
        ])
    spans = [[[0, 1], [0, 2]], [[1, 3], [0, 1]], [[1, 2], [2, 3]], [[2, 3], [1, 2]]]
    _case("test_label_outer_span", [
        *[["fig", "add_subplot", {"new": "SubplotSpec", "args": [3, 3, *span]}] for span in spans],
        *labelled, *[[i, "label_outer"] for i in range(4)], *_tick_labels(4), *_axis_labels(4),
    ])
    _case("test_label_outer_non_gridspec", [
        ["fig", "add_axes", [0, 0, 1, 1]], *labelled[:1], [0, "label_outer"], *_tick_labels(1),
        *_axis_labels(1),
    ])
    for xpos, ypos in itertools.product(["bottom", "top"], ["left", "right"]):
        _case(f"test_subplots_hide_axislabels/{xpos}-{ypos}", [
            ["fig", "subplots", 3, 3, {"sharex": True, "sharey": True}],
            *[step for i in range(9) for step in [
                [i, "set", {"xlabel": "foo", "ylabel": "bar"}],
                [i, "xaxis.set_label_position", xpos], [i, "yaxis.set_label_position", ypos],
                [i, "label_outer"]]],
            *_axis_labels(9),
        ])


def test_subplot_positions():
    for widths, heights in itertools.product([None, [1, 3, 2]], [None, [1, 2]]):
        _case(f"test_width_and_height_ratios/{widths}-{heights}", [
            ["fig", "subplots", 2, 3, {"width_ratios": widths, "height_ratios": heights}],
            *[[i, "get_position"] for i in range(6)],
        ])


# --- figures (test_figure.py) ---------------------------------------------------------

def test_figure_axes():
    for name, args in [("no_columns", [2, 0, 1]), ("no_rows", [0, 2, 1]), ("index_0", [2, 2, 0]),
                       ("index_5", [2, 2, 5]), ("code_42", [42]), ("code_1000", [1000])]:
        _case(f"test_add_subplot_invalid/{name}", [["fig", "add_subplot", *args]])
    _case("test_add_subplot_twotuple", [
        *[["fig", "add_subplot", 3, 2, {"new": "tuple", "args": span}]
          for span in [[3, 5], [4, 6], [3, 6], [4, 5]]],
        *[[i, "get_position"] for i in range(4)],
    ])
    _case("test_add_subplot_twotuple/reversed",
          [["fig", "add_subplot", 3, 2, {"new": "tuple", "args": [6, 3]}]])
    _case("test_axes_remove", [["fig", "subplots", 2, 2], [3, "remove"], ["fig", "get_axes"]])
    for method in ["clear", "clf"]:
        _case(f"test_figure_clear/{method}", [
            ["fig", "add_subplot", 111], ["fig", method], ["fig", "get_axes"],
            *[["fig", "add_subplot", 2, 1, i] for i in [1, 2]], ["fig", method],
            ["fig", "get_axes"],
        ])
    params = ["left", "right", "bottom", "top", "wspace", "hspace"]
    _case("test_suplots_adjust_incremental", [
        ["fig", "subplots_adjust", {"left": 0}], ["fig", "subplots_adjust", {"right": 1}],
        ["fig", "subplotpars.left"], ["fig", "subplotpars.right"],
    ])
    _case("test_clf_subplotpars", [
        ["fig", "subplots_adjust", {"left": 0.135, "right": 0.91, "bottom": 0.12, "top": 0.89,
                                    "wspace": 0.21, "hspace": 0.21}],
        *[["fig", f"subplotpars.{key}"] for key in params], ["fig", "clf"],
        *[["fig", f"subplotpars.{key}"] for key in params],
    ])
