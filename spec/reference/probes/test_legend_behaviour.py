"""Behaviour of legends, ported from matplotlib's own test suite
(``matplotlib/tests/test_legend.py``): which handles and labels
``legend()`` lists, explicit handles and labels, mismatched lengths,
figure legends, ``loc`` validation, label colours and automatic placement.

Area ``legend_behaviour``. One op replays a scenario (see
``probes/_scenario.py`` for the step language).

Ops
---
``legend_behaviour.run``  {case, steps, figure?, rc?, style?} -> one entry
    per step: the converted result of query steps, ``null`` for the others.

Cases
-----
Every case is named after the upstream test it mirrors; the
``TestLegendFunction`` and ``TestLegendFigureFunction`` tests, which check
the arguments ``Legend`` receives through a mock, are ported as the labels
of the legend made and the handles ``get_legend_handles_labels`` reports.
Placement tests compare the legend's window extent in pixels. A ``loc``
point of other than two numbers cannot be expressed in plt-rs (its
``LegendLoc::Point`` holds two), so plt-spec rejects those while
converting the arguments.

Left out: argument-type errors of the Python API (``TypeError`` for
mixed positional and keyword handles, handles passed as labels, a third
positional argument, ``shadow`` given as a string), ``loc`` given as a
``dict``, multi-column ``plot`` data with label lists, legend proxy-artist
properties (``legend_handles``, ``get_alignment``, ``_get_loc``), which
plt-rs does not expose, and the warnings themselves (only their effect on
the legend is recorded).
"""

import numpy as np

from probes._scenario import recorder
from spec_support import ProbeSet

probes = ProbeSet("legend_behaviour")
_case = recorder(probes)

SUBPLOT = ["fig", "subplots"]
RANGE = list(range(10))


def test_legend_entries():
    _case("TestLegendFunction.test_legend_no_args", [
        SUBPLOT, [0, "plot", RANGE, {"label": "hello world"}], [0, "legend"],
        ["@2", "get_texts"], [0, "get_legend_handles_labels"],
    ])
    _case("TestLegendFunction.test_legend_positional_handles_labels", [
        SUBPLOT, [0, "plot", RANGE], [0, "legend", ["@1"], ["hello world"]], ["@2", "get_texts"],
        [0, "get_legend_handles_labels"],
    ])
    _case("TestLegendFunction.test_legend_positional_labels_only", [
        SUBPLOT, [0, "plot", RANGE, {"label": "hello world"}], [0, "legend", ["foobar"]],
        ["@2", "get_texts"],
    ])
    _case("TestLegendFunction.test_legend_three_args", [
        SUBPLOT, [0, "plot", RANGE, {"label": "hello world"}],
        [0, "legend", ["@1"], ["foobar"], {"loc": "right"}], ["@2", "get_texts"],
    ])
    x = np.linspace(0, 1, 11)
    lines = [[0, "plot", x.tolist(), (k * x).tolist(), {"label": label}]
             for k, label in [(1, "x"), (2, "2x"), (3, "3x")]]
    _case("TestLegendFunction.test_legend_kwargs_handles_only", [
        SUBPLOT, *lines, [0, "legend", {"handles": ["@3", "@2"]}], ["@4", "get_texts"],
    ])
    _case("TestLegendFunction.test_legend_kwargs_labels_only", [
        SUBPLOT, [0, "plot", x.tolist(), x.tolist()], [0, "plot", x.tolist(), (2 * x).tolist()],
        [0, "legend", {"labels": ["x", "2x"]}], ["@3", "get_texts"],
        [0, "get_legend_handles_labels"],
    ])
    th = np.linspace(0, 2 * np.pi, 64)
    _case("TestLegendFunction.test_legend_kwargs_handles_labels", [
        SUBPLOT, [0, "plot", th.tolist(), np.sin(th).tolist(), {"label": "sin"}],
        [0, "plot", th.tolist(), np.cos(th).tolist(), {"label": "cos"}],
        [0, "legend", {"labels": ["a", "b"], "handles": ["@2", "@1"]}], ["@3", "get_texts"],
        [0, "get_legend_handles_labels"],
    ])
    _case("TestLegendFunction.test_legend_warns_on_unequal_number_of_handles_and_labels", [
        SUBPLOT, [0, "plot", [1, 2]], [0, "plot", [3, 4]],
        [0, "legend", ["@1", "@2"], ["only_one"]], ["@3", "get_texts"],
        [0, "legend", ["@1"], ["label_a", "label_b"]], ["@5", "get_texts"],
    ])
    _case("test_legend_handle_label_mismatch", [
        SUBPLOT, [0, "plot", RANGE], [0, "plot", RANGE],
        [0, "legend", {"handles": ["@1", "@2"], "labels": ["pl1", "pl2", "pl3"]}],
        ["@3", "get_texts"],
    ])
    _case("test_legend_nolabels_draw", [
        SUBPLOT, [0, "plot", [1, 2, 3]], [0, "legend"], [0, "get_legend.get_texts"],
        [0, "get_legend_handles_labels"],
    ])
    _case("test_empty_bar_chart_with_legend", [
        SUBPLOT, [0, "bar", [], [], {"label": "test"}], [0, "legend"], ["@2", "get_texts"],
    ])
    _case("test_reverse_legend_handles_and_labels", [
        SUBPLOT, *[[0, "plot", 1, 1, marker, {"label": label}]
                   for marker, label in [(".", "First label"), (",", "Second label"),
                                         ("o", "Third label")]],
        [0, "legend", {"reverse": True}], ["@4", "get_texts"],
    ])
    _case("test_legend_title_empty", [
        SUBPLOT, [0, "plot", RANGE, {"label": "mock data"}], [0, "legend"], ["@2", "get_title"],
        ["@2", "get_title.get_visible"],
    ])
    _case("test_legend_remove", [
        SUBPLOT, [0, "plot", RANGE], ["fig", "legend", ["@1"], ["test"]], ["@2", "remove"],
        ["fig", "legends"], [0, "legend", ["test"]], ["@5", "remove"], [0, "get_legend"],
    ])


def test_figure_legends():
    _case("TestLegendFigureFunction.test_legend_handle_label", [
        SUBPLOT, [0, "plot", RANGE], ["fig", "legend", ["@1"], ["hello world"]],
        ["@2", "get_texts"],
    ])
    _case("TestLegendFigureFunction.test_legend_no_args", [
        SUBPLOT, [0, "plot", RANGE, {"label": "hello world"}], ["fig", "legend"],
        ["@2", "get_texts"],
    ])
    _case("TestLegendFigureFunction.test_legend_label_arg", [
        SUBPLOT, [0, "plot", RANGE], ["fig", "legend", ["foobar"]], ["@2", "get_texts"],
    ])
    _case("TestLegendFigureFunction.test_legend_kw_args", [
        ["fig", "subplots", 1, 2], [0, "plot", RANGE, {"label": "first"}],
        [1, "plot", [2.0 * v for v in RANGE], {"label": "second"}],
        ["fig", "legend", {"loc": "right"}], ["@3", "get_texts"],
    ])


LOCATIONS = ["best", "upper right", "upper left", "lower left", "lower right", "right",
             "center left", "center right", "lower center", "upper center", "center"]


def test_legend_locations():
    labels = {"labels": ["mock data"]}
    for name, loc in [("tuple_1", [1.1]), ("tuple_3", [0.481, 0.4227, 0.4523]),
                      ("tuple_text", [0.481, "go blue"])]:
        _case(f"test_loc_invalid_tuple_exception/{name}", [SUBPLOT, [0, "legend",
                                                                     {"loc": loc, **labels}]])
    for loc in [[0.481, 0.442], [1, 2]]:
        _case(f"test_loc_valid_list/{loc}", [SUBPLOT, [0, "legend", {"loc": loc, **labels}],
                                             ["@1", "get_texts"]])
    _case("test_loc_invalid_list_exception", [SUBPLOT, [0, "legend",
                                                        {"loc": [1.1, 2.2, 3.3], **labels}]])
    for loc in [0, 1, 5, 10, 11, -1]:
        _case(f"test_loc_validation_numeric_value/{loc}",
              [SUBPLOT, [0, "legend", {"loc": loc, **labels}], ["@1", "get_texts"]])
    for loc in [*LOCATIONS, "wrong"]:
        _case(f"test_loc_validation_string_value/{loc}",
              [SUBPLOT, [0, "legend", {"loc": loc, **labels}], ["@1", "get_texts"]])
    for loc in ["outside right", "right"]:
        _case(f"test_fig_legend_set_loc/{loc}", [
            SUBPLOT, [0, "plot", RANGE, {"label": "test"}], ["fig", "legend"],
            ["@2", "set_loc", loc], ["@2", "get_texts"],
        ])


def test_label_colors():
    rows = [[0, "plot", RANGE, [k * v for v in RANGE], {"label": f"#{k}"}] for k in (1, 2, 3)]
    for name, color in [("red", "red"), ("none", "none"), ("gray", [0.5, 0.5, 0.5])]:
        _case(f"test_legend_labelcolor_single/{name}", [
            SUBPLOT, *rows, [0, "legend", {"labelcolor": color}], ["@4", "get_texts.get_color"],
        ])
    _case("test_legend_labelcolor_list", [
        SUBPLOT, *rows, [0, "legend", {"labelcolor": ["r", "g", "b"]}],
        ["@4", "get_texts.get_color"],
    ])
    for rule, keyword in [("linecolor", "color"), ("markeredgecolor", "markeredgecolor"),
                          ("markerfacecolor", "markerfacecolor")]:
        colored = [[0, "plot", RANGE, [k * v for v in RANGE],
                    {"label": f"#{k}", keyword: color}]
                   for k, color in [(1, "r"), (2, "g"), (3, "b")]]
        _case(f"test_legend_labelcolor_{rule}", [
            SUBPLOT, *colored, [0, "legend", {"labelcolor": rule}],
            ["@4", "get_texts.get_color"],
        ])
        _case(f"test_legend_labelcolor_rcparam_{rule}", [
            SUBPLOT, *colored, [0, "legend"], ["@4", "get_texts.get_color"],
        ], rc={"legend.labelcolor": rule})
    for rule, keyword in [("linecolor", "c"), ("markeredgecolor", "edgecolor")]:
        scattered = [[0, "scatter", RANGE, [k * v for v in RANGE],
                      {"label": f"#{k}", keyword: color}]
                     for k, color in [(1, "r"), (2, "g"), (3, "b")]]
        _case(f"test_legend_pathcollection_labelcolor_{rule}", [
            SUBPLOT, *scattered, [0, "legend", {"labelcolor": rule}],
            ["@4", "get_texts.get_color"],
        ])
    _case("test_legend_pathcollection_labelcolor_linecolor_cmap", [
        SUBPLOT, [0, "scatter", RANGE, RANGE, {"c": [float(v) for v in RANGE], "label": "#1"}],
        [0, "legend", {"labelcolor": "linecolor"}], ["@2", "get_texts.get_color"],
    ])


def _placed(axes_steps, figure):
    """One case's steps: every axes' steps (the last one makes its
    legend), a draw, then every legend's window extent in pixels."""
    steps, legends = [["fig", "subplots", 1, len(axes_steps)]], []
    for ax, ax_steps in enumerate(axes_steps):
        steps += [[ax, *step] for step in ax_steps]
        legends.append(f"@{len(steps) - 1}")
    return {"steps": [*steps, ["fig", "draw"], *[[leg, "get_window_extent"] for leg in legends]],
            "figure": figure, "tol": 1e-6}


def test_best_location():
    # matplotlib compares the legend extents of equivalent axes; the extents
    # in pixels are recorded instead.
    _case("test_legend_auto4", **_placed([
        [["set_title", histtype],
         ["hist", [0] + 5 * [9], {"bins": list(range(10)), "label": "Legend",
                                  "histtype": histtype}],
         ["legend", {"loc": "best"}]]
        for histtype in ["bar", "step", "stepfilled"]], {"figsize": [6.4, 2.4]}))
    _case("test_legend_loc_polycollection", **_placed([
        [["fill_between", [3, 4, 5], [1, 1, 1], [5, 5, 5],
          {"color": "gray", "alpha": 0.5, "label": "Shaded Area"}],
         ["set_xlim", 0, 6], ["set_ylim", -1, 5], ["legend", {"loc": loc}]]
        for loc in ["best", "lower left"]], {"figsize": [10, 5]}))
    _case("test_legend_text", **_placed([
        [["plot", [1, 2], [2, 1], {"label": "plot name"}],
         ["text", 1.5, 2, "some text blahblah", {"verticalalignment": "top"}],
         ["legend", {"loc": loc}]]
        for loc in ["best", "lower left"]], {"figsize": [10, 5]}))
