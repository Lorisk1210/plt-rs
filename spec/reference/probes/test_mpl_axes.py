"""matplotlib's own non-image tests of ``Axes``
(``matplotlib/tests/test_axes.py``, the tests defined before line 5585)
that no other probe area already covers.

Area ``mpl_axes``. matplotlib runs its tests in the ``classic`` style plus
``_classic_test_patch`` unless a test picks another one, so every probe
carries the ``style`` it runs in (a list of style names, applied with
``style.context`` on top of the reference configuration) and, when the
test changes rcParams, an ``rc`` dict applied inside it. Unless noted, an op
draws on ``ax = Figure().add_subplot()`` with an Agg canvas. Colours are
recorded as RGBA lists (``to_rgba``); legend labels as the label, or
``null`` when matplotlib hides it (empty or starting with ``_``); markers
as ``get_marker()`` with ``"None"`` for every spelling of no marker; line
styles by name (``"solid"``, ``"dashed"``, ``"dashdot"``, ``"dotted"``,
``"none"``). Data the upstream tests draw from ``np.random`` comes from
``spec_support.rng`` and is passed in the arguments.

Ops
---
``Axis.set_major_formatter`` / ``Axis.set_minor_formatter``  {axis,
    formatter} -> {before, after, label}: the ``isDefault_majfmt`` and
    ``isDefault_minfmt`` flags of the x and then the y axis before and after
    giving ``axis`` a ``"func"`` formatter (a function returning ``""``) or
    the ``"str"`` formatter ``"{x}_{pos}"``, and what the installed
    formatter makes of ``(0.5, 1)``
    (``test_funcformatter_auto_formatter``,
    ``test_strmethodformatter_auto_formatter``).
``Axes.twin.units``  {twin} -> after ``ax.plot(["0", "1"], ["a", "b"])``
    and ``ax2 = ax.twin<twin>()``: whether the shared axis of ``ax`` and of
    ``ax2`` has units, and ``ax2``'s conversion of the shared axis'
    categories in reverse order (``test_twin_units``).
``Axes.get_in_layout``  {twin} -> ``[ax, ax.twin<twin>()]``'s
    ``get_in_layout()`` (``test_twin_keeps_layout_participation...``).
``Axes.arrow``  {x, y, dx, dy, head_length} -> {xy, xlim, ylim}: the
    vertices of the arrow and the view limits (``test_arrow_empty``).
``Annotation.arrow_patch``  {arrowprops?} -> whether ``ax.annotate("foo",
    (0, 1), xytext=(2, 3), ...)`` has an arrow
    (``test_annotate_default_arrow``).
``Axes.hexbin``  {x, y, extent?, gridsize?, xscale?, yscale?, bins?, vmin?,
    vmax?} -> {offsets, clim} of the hexbin collection, or the error
    (``test_hexbin_bad_extents``, ``test_hexbin_log_offsets``,
    ``test_hexbin_log_clim``, with 4 hexagons across instead of 100 to keep
    the offsets short).
``Axes.hexbin.contains``  {x, y, extent, point} -> whether the hexbin
    collection made with ``picker=-1`` contains a mouse event at the display
    ``point`` (``test_hexbin_pickable``).
``Axes.imshow``  {data, norm, vmin?, vmax?} -> the colour limits of
    ``imshow(data, norm=Normalize(*norm), vmin=vmin, vmax=vmax)``, or the
    error (``test_imshow_norm_vminvmax``).
``Axes.pcolor``  {C, x?, y?} -> {datalim, count}: the data limits and the
    number of cells of ``pcolor([x, y,] C)``; ``x`` and ``y`` are 1-D or
    2-D (``test_pcolorargs_5205``).
``Axes.pcolormesh``  {C, x?, y?, shading?} -> the data limits after
    ``pcolormesh([x, y,] C, shading=shading)``, or the error
    (``test_pcolorargs``, ``test_pcolorflaterror``,
    ``test_samesizepcolorflaterror``).
``Axes.bar``  {x, height, color?, edgecolor?, alpha?, label?} -> {rects,
    faces, edges, labels, label, xlim, ylim}: ``[x, y, width, height]``,
    face and edge colour and legend label of every bar, the container's
    legend label and the view limits, or the error (``test_nan_bar_values``,
    ``test_bar_ticklabel_fail``, ``test_bar_color_none_alpha``,
    ``test_bar_edgecolor_none_alpha``, ``test_bar_labels``,
    ``test_bar_labels_length``).
``Axes.bar.timedelta``  {orientation, x, values, hours, err_hours?} ->
    {rects, xlim, ylim}: ``bar`` (``barh``) at the ISO datetimes ``x`` with
    widths (heights) of ``hours`` hours, one or one per bar, and an error
    of ``err_hours`` along the dates (``test_bar_timedelta``).
``Axes.broken_barh``  {start, hours, yrange} -> the data limits after
    ``broken_barh([(start, timedelta(hours=hours))], yrange)``
    (``test_bar_timedelta``).
``Axes.bar.datetime_start``  {orientation} -> whether the value axis
    labels its ticks with an ``AutoDateFormatter`` after bars from datetime
    starts with timedelta lengths (``test_bar_datetime_start``).
``Axes.grouped_bar``  {heights, positions?, tick_labels?, hatch?} ->
    {containers, hatches, removed}: the number of bar containers and of
    those in ``ax.containers``, each container's bar hatches, and how many
    remain in ``ax.containers`` after ``remove()``
    (``test_grouped_bar_return_value``, ``test_grouped_bar_hatch_sequence``).
``Axes.boxplot``  {data, rc?, ...keywords} -> {boxes, whiskers, caps,
    medians, fliers, means, xlim, xticks, xticklabels}: per part, the
    ``[xdata, ydata, marker, zorder]`` of each line (boxes as lines), then the
    view limits, ticks and tick labels, or the error. Keywords:
    ``positions``, ``widths``, ``capwidths``, ``usermedians``,
    ``conf_intervals``, ``showmeans``, ``manage_ticks``, ``zorder``; before
    that, ``xlim`` sets the x limits and the datasets ``before`` are
    boxplotted at ``before_positions`` (``test_boxplot_capwidths``, ``test_boxplot_bad_medians``,
    ``test_boxplot_bad_ci``, ``test_boxplot_zorder``,
    ``test_boxplot_marker_behavior``, ``test_manage_xticks``,
    ``test_boxplot_not_single``).
``Axes.bxp``  {data, positions?, widths?, capwidths?} -> the number of boxes
    of ``bxp(boxplot_stats(data), ...)`` on a log y axis, or the error
    (``test_bxp_bad_widths``, ``test_bxp_bad_positions``,
    ``test_bxp_bad_capwidths``; the bad widths and cap widths are two
    values for four boxes rather than one, which plt-rs reads as one value
    for every box).
``Axes.violinplot``  {data, positions?, widths?, quantiles?, facecolor?,
    linecolor?, showmeans?, showmedians?} -> {bodies, alphas, lines}: the
    face colour and alpha of each body and the edge colours of the
    ``cbars``, ``cmins``, ``cmaxes``, ``cmeans`` and ``cmedians`` lines that
    exist, or the error (``test_violinplot_bad_positions``,
    ``test_violinplot_bad_widths``, ``test_violinplot_bad_quantiles``,
    ``test_violinplot_outofrange_quantiles``,
    ``test_violinplot_color_sequence``).
``Axes.violinplot.set_facecolor``  {data, facecolor?} -> {face, alpha,
    after}: the first body's face colour and alpha, and its face colour
    after ``set_facecolor("red")`` (``test_violinplot_alpha``).
``Axes.violin``  {stats, positions, widths} -> the data limits of
    ``violin(stats, positions=..., widths=...)``; the statistics' ``coords``
    are numbers (``test_numeric_positions_with_float_widths_ok``, whose
    dates are given as ``date2num`` values).
``Axes.errorbar``  {x, y, xerr?, yerr?, fmt?, elinewidth?, elinestyle?,
    errorevery?, ...line keywords} -> {line, bars, caps}: the data line's
    ``{color, marker, linestyle, fillstyle, drawstyle}`` (``null`` without
    one), each bar collection's ``{colors, linestyles}`` (a line style as
    ``[offset, dashes]`` scaled by the line width, ``dashes`` ``null`` when
    solid), and the number of cap lines, or the error. Errors are a number,
    one per point or ``[lower, upper]`` lists; ``errorevery`` is ``[start,
    step]`` or one flag per point (``test_errorbar_nonefmt``,
    ``test_errorbar_line_specific_kwargs``, ``test_errorbar_every_invalid``, ``test_xerr_yerr_not_negative``,
    ``test_errorbar_linewidth_type``, ``test_errorbar_linestyle_type``).
``ErrorbarContainer.remove``  {x, y, xerr, yerr} -> ``[containers, lines,
    collections]``, the lengths of the axes' lists, after ``errorbar`` and
    after removing its container (``test_errorbar_remove``).
``Axes.errorbar.color_cycle``  {x} -> the colours of two error bar lines
    and a plotted line drawn after them (``test_errorbar_colorcycle``).
``Axes.stem``  {x, y, linefmt?, markerfmt?, basefmt?} -> {markers,
    markercolor, marker, markerstyle, linecolor, basecolor}: the head line's
    data, colour, marker and line style, the stems' colour and the
    baseline's colour (``test_stem_args``, ``test_stem_markerfmt``).
``Axes.stem.dates``  {x, y} -> {markers, baseline}: ``get_xydata()`` of
    the heads and of the baseline of a stem plot at the ISO datetimes ``x``
    (``test_stem_dates``).
``Axes.stem.baseline``  {projection} -> whether the baseline of a stem plot
    in axes of ``projection`` is split into more than 100 steps per segment
    (``test_stem_polar_baseline``).
``Axes.hist``  {x, bins?, histtype?, bottom?, stacked?, density?, color?,
    facecolor?, edgecolor?, label?} -> {n, xy, faces, edges, zorders,
    labels}: the counts, every step polygon's ``get_xy()`` (bar histograms:
    ``[]``), each patch's face and edge colour and z-order, and the first
    patch of each dataset's legend label (``test_hist_step_geometry``,
    ``test_hist_step_bottom_geometry``, ``test_hist_stacked_step_geometry``,
    ``test_hist_stacked_step_bottom_geometry``, ``test_hist_color_semantics``,
    ``test_hist_zorder``, ``test_hist_single_color_multiple_datasets``,
    ``test_hist_with_empty_input``, ``test_hist_unequal_bins_density``,
    ``test_hist_emptydata``, ``test_hist_labels``).
``Axes.hist.dates``  {data, stacked, bins?} -> {n, bins}: the counts and
    the bin edges (date numbers) of ``hist`` of the datasets of ISO
    datetimes ``data``, with bin edges given as datetimes
    (``test_hist_datetime_datasets``, ``test_hist_datetime_datasets_bins``,
    whose ``date2num`` and ``datetime64`` edges are the same values).
``Axes.get_legend_handles_labels``  {x, label} -> the legend labels after
    ``hist(x, label=label)`` (``test_hist_unused_labels``).
``Axes.hist.log``  {} -> the y limits, after a draw, of
    ``hist([[0], [0, 1]], 2, histtype="barstacked")`` and of
    ``hist([0, 0, 1], 2, histtype="barstacked")`` on log y axes
    (``test_hist_log_barstacked``).
``Axes.hist2d``  {x, y, density} -> the histogram of ``hist2d(x, y,
    density=density)`` (``test_hist2d_density``).
``Axes.stairs``  {values, edges} -> {xlim, ylim}, or the error
    (``test_stairs_empty``, ``test_stairs_invalid_nan``,
    ``test_stairs_invalid_mismatch``).
``StepPatch.set_data``  {values, edges, new_edges} -> the error of
    ``stairs(values, edges).set_data(edges=new_edges)``, or ``null``
    (``test_stairs_invalid_update``, ``test_stairs_invalid_update2``).
``Figure.legend``  {} -> the legend texts of ``fig.legend()`` after a
    stack plot labelled ``"a"`` in each of two subfigures, after
    ``draw_without_rendering`` (``test_stackplot_subfig_legend``).
``Axes.scatter``  {x, y, s?, c?, color?, edgecolors?, edgecolor?, marker?,
    fillstyle?, linewidths?, norm?, vmin?, vmax?} -> {faces, edges,
    linewidths} after ``update_scalarmappable()``, or the error. ``c`` is a
    colour string, a list of colour strings and RGB(A) rows, or a list of
    numbers (mapped values); ``color`` and the edge keywords are colours
    (``color`` colours the edges too unless they are given); ``norm`` is
    ``[vmin, vmax]`` of a ``Normalize`` (``TestScatter.test_scatter_color``,
    ``test_scatter_unfilled``, ``test_scatter_unfillable``,
    ``test_scatter_size_arg_size``, ``test_scatter_edgecolor_RGB``,
    ``test_scatter_norm_vminvmax``, ``test_scatter_c``,
    ``test_scatter_linewidths``, ``test_parse_scatter_color_args``,
    ``test_parse_scatter_color_args_edgecolors``,
    ``test_parse_scatter_color_args_error``).
``Axes.eventplot``  {positions, colors?, alpha?} -> {colors, alphas}: each
    collection's colour and alpha, or the error. ``colors`` is a colour
    string or a list of colour strings and RGB(A) lists
    (``test_eventplot_colors``, ``test_eventplot_alpha``).
``MollweideAxes.transProjection``  {lonlat?, xy?} -> the forward transform of
    the ``[lon, lat]`` points, or the inverse transform of the ``[x, y]``
    points, of ``add_subplot(projection="mollweide")``
    (``test_mollweide_forward_inverse_closure``,
    ``test_mollweide_inverse_forward_closure``: the grids are 15 degrees and
    0.125 apart instead of one degree and 1/500, and both directions are
    compared with matplotlib's values rather than with a round trip).
``Figure.savefig``  {rc} -> the size in pixels of ``ax.plot([0, 1], [0,
    1])`` saved at 80 dpi as raw RGBA under ``rc`` (``test_tick_space_size_0``).
``pyplot.sca``  {} -> after ``fig1, ax1 = plt.subplots()``, a second
    ``plt.subplots()`` and ``plt.sca(ax1)``: whether ``ax1`` is ``gca()`` and
    ``fig1`` is ``gcf()`` (``test_pyplot_axes``).

Covered elsewhere: ``test_get_labels``, ``test_label_shift``,
``test_twinx_cla``, ``test_twin_inherit_autoscale_setting``,
``test_twin_respects_position_after_set_position``,
``test_twin_stays_aligned_after_constrained_layout``, ``test_inverted_cla``,
``test_autoscale_tight``, ``test_autoscale_log_shared``,
``test_use_sticky_edges``, ``test_arrow_in_view``, ``test_inverted_limits``,
``test_hist2d_autolimits``, ``test_log_scales``, ``test_log_scales_no_data``,
``test_log_scales_invalid`` and ``test_errorbar_log_autoscale_order_independent``
(area ``axes_behaviour``). ``test_limits_empty_data``, a
``check_figures_equal`` test, is ported as figure cases (group
``mpl_tests_axes``).

Left out: tests of Python mechanics: ``test_repr`` (``repr``),
``test_subclass_clear_cla`` and ``test_cla_not_redefined_internally``
(subclass hooks), ``test_nargs_stem``, ``test_nargs_legend`` and
``test_nargs_pcolorfast`` (argument counts), ``test_annotate_signature``
(signatures), ``test_spy_invalid_kwargs``,
``test_scatter_singular_plural_arguments`` and the ``edgecolor`` /
``edgecolors`` precedence of ``test_parse_scatter_color_args_edgecolors``
(keywords and their aliases, which typed options rule out),
``test_errorbar_mapview_kwarg`` and ``test_structured_data`` (dict views,
structured arrays and ``data=``), ``test_marker_as_markerstyle`` (a
``MarkerStyle`` object where a marker code is expected),
``test_as_mpl_axes_api`` (the ``_as_mpl_axes`` protocol),
``test_hist_barstacked_bottom_unchanged`` (mutating the caller's array),
``test_pcolormesh_underflow_error``, ``test_pcolorargs_with_read_only`` and
``test_hist_float16`` (numpy error states, read-only masks and float16);
pandas inputs (``test_boxplot_dates_pandas``, ``test_pcolor_regression``,
``test_bar_pandas``, ``test_bar_pandas_indexed``, ``test_pandas_minimal_plot``,
``test_grouped_bar_dataframe``); warnings, which plt-rs does not emit
(``test_plot_format_kwarg_redundant``, ``test_stairs_no_baseline_fill_warns``,
``test_scatter_color_warning``, ``test_parse_c_facecolor_warning_direct``,
``test_scatter_c_facecolor_warning_integration``, the non-monotonic grids of
``test_pcolorargs`` and the ``[0.5] * 3`` case of ``test_scatter_c``); and
inputs Rust's types rule out: 2-D data (``test_fill_between_input``,
``test_fill_betweenx_input``, ``test_errorbar_shape``, the 2-D heads of
``test_stem_args``, the nested and ragged lists of
``test_boxplot_bad_medians`` and ``test_boxplot_bad_ci``), ``None``
(``test_xerr_yerr_not_none``, the ``None`` width of
``test_scatter_linewidths`` and colour of ``test_eventplot_colors``),
numbers as labels (``0`` and ``[0]`` in ``test_hist_labels``), masked
coordinates (``test_pcolorargs``), timedeltas and datetimes where plt-rs
takes numbers (``test_hist_timedelta_raises``, the timedelta error of
``test_xerr_yerr_not_negative``, ``test_datetime_positions_with_datetime64``,
``test_datetime_positions_with_float_widths_raises``,
``test_datetime_positions_with_scalar_float_width_raises``,
``test_mixed_positions_datetime_and_numeric_raises``), strings, tuples and
lists in place of typed values (``s="foo"`` in
``test_scatter_size_arg_size``, the ``errorevery`` tuples and string of
``test_errorbar_every_invalid``, ``color=[0.1, 0.2]`` in
``test_scatter_color``, the lists mixing numbers and strings in
``test_scatter_c``, the list ``elinewidth`` of
``test_errorbar_linewidth_type``, the projection name of
``test_invalid_projection``) and the call signatures of ``test_stem_args``
without positions or with a positional format. Also left out:
``test_hexbin_string_norm`` (a norm named by a string together with
``vmin`` and ``vmax``: plt-rs's colour options take a norm object, which
excludes limits) and ``test_stackplot_facecolor`` (plt-rs's stack plots take
their fill colours once, as ``colors``).
"""

import datetime
import io

import matplotlib as mpl
import matplotlib.colors as mcolors
import matplotlib.dates as mdates
import matplotlib.pyplot as plt
import numpy as np
from matplotlib.backend_bases import MouseEvent
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.cbook import boxplot_stats
from matplotlib.figure import Figure
from matplotlib.patches import Polygon

from spec_support import ProbeSet, rng

probes = ProbeSet("mpl_axes")

CLASSIC = ["classic", "_classic_test_patch"]


def _styled(fn):
    """``fn`` evaluated inside ``style.context(style)`` and ``rc_context(rc)``."""
    def evaluate(style, rc=None, **kw):
        with mpl.style.context(style), mpl.rc_context(rc or {}):
            return fn(**kw)

    return evaluate


def probe(op, fn, style=CLASSIC, rc=None, tol=None, **args):
    if rc is not None:
        args["rc"] = rc
    return probes.probe(op, _styled(fn), tol=tol, style=style, **args)


def _figure():
    fig = Figure()
    FigureCanvasAgg(fig)
    return fig


def _axes(**kw):
    return _figure().add_subplot(**kw)


def _rgba(color):
    return list(mcolors.to_rgba(color))


def _rgba_list(colors):
    return mcolors.to_rgba_array(colors).tolist()


def _legend_label(label):
    return label if label and not label.startswith("_") else None


LINESTYLE_NAMES = {"-": "solid", "--": "dashed", "-.": "dashdot", ":": "dotted"}


def _marker(line):
    """``get_marker()``, ``"None"`` for every spelling of no marker."""
    marker = line.get_marker()
    return "None" if marker in ("", " ", "None", None) else marker


def _linestyle(line):
    """``get_linestyle()`` by name, ``"none"`` for no line."""
    return LINESTYLE_NAMES.get(line.get_linestyle(), "none")


def _color_arg(value):
    """A JSON colour: a string, or an RGB(A) list or ``[name, alpha]`` as a
    tuple."""
    return tuple(value) if isinstance(value, list) else value


def _colors_arg(value):
    if isinstance(value, list):
        return [_color_arg(v) for v in value]
    return value


# --- axis formatters -----------------------------------------------------------------

def _formatter_flags(ax):
    return [ax.xaxis.isDefault_majfmt, ax.xaxis.isDefault_minfmt,
            ax.yaxis.isDefault_majfmt, ax.yaxis.isDefault_minfmt]


def _set_formatter(which):
    def run(axis, formatter):
        ax = _axes()
        before = _formatter_flags(ax)
        target = getattr(ax, f"{axis}axis")
        fmt = (lambda x, pos: "") if formatter == "func" else "{x}_{pos}"
        getattr(target, f"set_{which}_formatter")(fmt)
        installed = getattr(target, f"get_{which}_formatter")()
        return {"before": before, "after": _formatter_flags(ax), "label": installed(0.5, 1)}

    return run


def test_funcformatter_auto_formatter():
    probe("Axis.set_major_formatter", _set_formatter("major"), axis="x", formatter="func")


def test_strmethodformatter_auto_formatter():
    probe("Axis.set_minor_formatter", _set_formatter("minor"), axis="y", formatter="str")


# --- twins ---------------------------------------------------------------------------

def _twin_units(twin):
    ax = _axes()
    ax.plot(["0", "1"], ["a", "b"])
    ax2 = getattr(ax, f"twin{twin}")()
    axis, axis2 = getattr(ax, f"{twin}axis"), getattr(ax2, f"{twin}axis")
    categories = ["1", "0"] if twin == "x" else ["b", "a"]
    return [axis.units is not None, axis2.units is not None,
            axis2.convert_units(categories)]


def test_twin_units():
    for twin in ["x", "y"]:
        probe("Axes.twin.units", _twin_units, twin=twin)


def _in_layout(twin):
    ax = _axes()
    ax2 = getattr(ax, f"twin{twin}")()
    return [ax.get_in_layout(), ax2.get_in_layout()]


def test_twin_keeps_layout_participation_for_layout_managed_axes():
    for twin in ["x", "y"]:
        probe("Axes.get_in_layout", _in_layout, twin=twin)


# --- arrows and annotations ----------------------------------------------------------

def _arrow(x, y, dx, dy, head_length):
    ax = _axes()
    arrow = ax.arrow(x, y, dx, dy, head_length=head_length)
    return {"xy": arrow.get_xy(), "xlim": ax.get_xlim(), "ylim": ax.get_ylim()}


def test_arrow_empty():
    probe("Axes.arrow", _arrow, x=0, y=0, dx=0, dy=0, head_length=0)


def _annotate_arrow(**kw):
    ax = _axes()
    return ax.annotate("foo", (0, 1), xytext=(2, 3), **kw).arrow_patch is not None


def test_annotate_default_arrow():
    probe("Annotation.arrow_patch", _annotate_arrow)
    probe("Annotation.arrow_patch", _annotate_arrow, arrowprops={})


# --- hexbin --------------------------------------------------------------------------

def _hexbin(x, y, **kw):
    ax = _axes()
    if "extent" in kw:
        kw["extent"] = tuple(kw["extent"])
    hexes = ax.hexbin(x, y, **kw)
    return {"offsets": hexes.get_offsets(), "clim": hexes.get_clim()}


def test_hexbin_bad_extents():
    x, y = (np.arange(20) / 20).reshape((2, 10)).tolist()
    for extent in [[1, 0, 0, 1], [0, 1, 1, 0]]:
        assert isinstance(probe("Axes.hexbin", _hexbin, x=x, y=y, extent=extent), ValueError)


def test_hexbin_log_offsets():
    x = np.geomspace(1, 100, 500).tolist()
    probe("Axes.hexbin", _hexbin, x=x, y=x, xscale="log", yscale="log", gridsize=2)


def test_hexbin_log_clim():
    x, y = np.arange(200).reshape((2, 100)).tolist()
    probe("Axes.hexbin", _hexbin, x=x, y=y, gridsize=4, bins="log", vmin=2, vmax=100)


def _hexbin_contains(x, y, extent, point):
    ax = _axes()
    hexes = ax.hexbin(x, y, extent=tuple(extent), picker=-1)
    event = MouseEvent("button_press_event", ax.figure.canvas, *point)
    return bool(hexes.contains(event)[0])


def test_hexbin_pickable():
    x, y = (np.arange(200) / 200).reshape((2, 100)).tolist()
    probe("Axes.hexbin.contains", _hexbin_contains, x=x, y=y, extent=[.1, .3, .6, .7],
          point=[400, 300])


# --- images and meshes ---------------------------------------------------------------

def _imshow(data, norm, **kw):
    ax = _axes()
    return ax.imshow(data, norm=mcolors.Normalize(*norm), **kw).get_clim()


def test_imshow_norm_vminvmax():
    probe("Axes.imshow", _imshow, data=[[1, 2], [3, 4]], norm=[-10, 10])
    probe("Axes.imshow", _imshow, data=[[1, 2], [3, 4]], norm=[-10, 10], vmin=0, vmax=5)


def _mesh_args(C, x, y):
    return [np.asarray(C, float)] if x is None else [np.asarray(x, float),
                                                     np.asarray(y, float),
                                                     np.asarray(C, float)]


def _pcolor(C, x=None, y=None):
    ax = _axes()
    cells = ax.pcolor(*_mesh_args(C, x, y))
    return {"datalim": ax.dataLim.get_points(), "count": cells.get_array().size}


def _grid_5205():
    x = [-1.5, -1.0, -0.5, 0.0, 0.5, 1.0, 1.5]
    y = [-1.5, -1.25, -1.0, -0.75, -0.5, -0.25, 0, 0.25, 0.5, 0.75, 1.0, 1.25, 1.5]
    X, Y = np.meshgrid(x, y)
    return x, y, X, Y, np.hypot(X, Y)


def test_pcolorargs_5205():
    x, y, X, Y, Z = _grid_5205()
    probe("Axes.pcolor", _pcolor, C=Z.tolist())
    probe("Axes.pcolor", _pcolor, x=x, y=y, C=Z[:-1, :-1].tolist())
    probe("Axes.pcolor", _pcolor, x=X.tolist(), y=Y.tolist(), C=Z[:-1, :-1].tolist())


def _pcolormesh(C, x=None, y=None, **kw):
    ax = _axes()
    ax.pcolormesh(*_mesh_args(C, x, y), **kw)
    return ax.dataLim.get_points()


def test_pcolorargs():
    n = 12
    x = np.linspace(-1.5, 1.5, n)
    y = np.linspace(-1.5, 1.5, n * 2)
    X, Y = np.meshgrid(x, y)
    Z = np.hypot(X, Y) / 5
    small = Z[:-1, :-1].tolist()
    for args in [dict(x=y.tolist(), y=x.tolist(), C=Z.tolist()),
                 dict(x=X.tolist(), y=Y.tolist(), C=Z.T.tolist()),
                 dict(x=x.tolist(), y=y.tolist(), C=small, shading="gouraud"),
                 dict(x=X.tolist(), y=Y.tolist(), C=small, shading="gouraud")]:
        assert isinstance(probe("Axes.pcolormesh", _pcolormesh, **args), TypeError)
    nan_x = x.copy()
    nan_x[0] = np.nan
    assert isinstance(probe("Axes.pcolormesh", _pcolormesh, x=nan_x.tolist(), y=y.tolist(),
                            C=small), ValueError)
    probe("Axes.pcolormesh", _pcolormesh, x=x.tolist(), y=y.tolist(), C=small)
    probe("Axes.pcolormesh", _pcolormesh, x=x.tolist(), y=y.tolist(), C=Z.tolist())


def test_pcolorflaterror():
    Z = rng(19680801).normal(27).reshape(3, 9).tolist()
    assert isinstance(probe("Axes.pcolormesh", _pcolormesh, x=list(range(9)), y=list(range(3)),
                            C=Z, shading="flat"), TypeError)


def test_samesizepcolorflaterror():
    x, y = np.meshgrid(np.arange(5), np.arange(3))
    assert isinstance(probe("Axes.pcolormesh", _pcolormesh, x=x.tolist(), y=y.tolist(),
                            C=(x + y).tolist(), shading="flat"), TypeError)


# --- bars ----------------------------------------------------------------------------

def _bar(x, height, **kw):
    ax = _axes()
    bars = ax.bar(x, height, **kw)
    return {
        "rects": [[r.get_x(), r.get_y(), r.get_width(), r.get_height()] for r in bars],
        "faces": [_rgba(r.get_facecolor()) for r in bars],
        "edges": [_rgba(r.get_edgecolor()) for r in bars],
        "labels": [_legend_label(r.get_label()) for r in bars],
        "label": _legend_label(bars.get_label()),
        "xlim": ax.get_xlim(),
        "ylim": ax.get_ylim(),
    }


def test_nan_bar_values():
    probe("Axes.bar", _bar, x=[0, 1], height=[np.nan, 4])


def test_bar_ticklabel_fail():
    probe("Axes.bar", _bar, x=[], height=[])


def test_bar_color_none_alpha():
    probe("Axes.bar", _bar, x=[1, 2], height=[2, 4], alpha=0.3, color="none", edgecolor="r")


def test_bar_edgecolor_none_alpha():
    probe("Axes.bar", _bar, x=[1, 2], height=[2, 4], alpha=0.3, color="r", edgecolor="none")


def test_bar_labels():
    for x, width, label in [
            ("x", 1, "x"),
            (["a", "b", "c"], [10, 20, 15], ["A", "B", "C"]),
            (["a", "b", "c"], [10, 20, 15], ["R", "Y", "_nolegend_"]),
            (["a", "b", "c"], [10, 20, 15], "bars")]:
        probe("Axes.bar", _bar, x=x, height=width, label=label)


def test_bar_labels_length():
    for label in [["X", "Y", "Z"], ["X"]]:
        assert isinstance(probe("Axes.bar", _bar, x=["x", "y"], height=[1, 2], label=label),
                          ValueError)


def _dt(text):
    return datetime.datetime.fromisoformat(text)


def _bar_timedelta(orientation, x, values, hours, err_hours=None):
    ax = _axes()
    widths = [datetime.timedelta(hours=h) for h in hours]
    width = widths[0] if len(widths) == 1 else widths
    kw = {}
    if err_hours is not None:
        kw["xerr" if orientation == "vertical" else "yerr"] = datetime.timedelta(hours=err_hours)
    positions = [_dt(day) for day in x]
    if orientation == "vertical":
        bars = ax.bar(positions, values, width=width, **kw)
    else:
        bars = ax.barh(positions, values, height=width, **kw)
    return {
        "rects": [[r.get_x(), r.get_y(), r.get_width(), r.get_height()] for r in bars],
        "xlim": ax.get_xlim(),
        "ylim": ax.get_ylim(),
    }


def _broken_barh_dates(start, hours, yrange):
    ax = _axes()
    ax.broken_barh([(_dt(start), datetime.timedelta(hours=hours))], tuple(yrange))
    return ax.dataLim.get_points()


def test_bar_timedelta():
    day = ["2018-01-01T00:00:00"]
    probe("Axes.bar.timedelta", _bar_timedelta, orientation="vertical", x=day, values=[1.0],
          hours=[3])
    probe("Axes.bar.timedelta", _bar_timedelta, orientation="vertical", x=day, values=[1.0],
          hours=[3], err_hours=2)
    probe("Axes.bar.timedelta", _bar_timedelta, orientation="horizontal", x=day, values=[1],
          hours=[3])
    probe("Axes.bar.timedelta", _bar_timedelta, orientation="horizontal", x=day, values=[1],
          hours=[3], err_hours=2)
    probe("Axes.bar.timedelta", _bar_timedelta, orientation="horizontal", x=day * 2,
          values=[1, 1.5], hours=[3])
    probe("Axes.bar.timedelta", _bar_timedelta, orientation="horizontal", x=day * 2,
          values=[1, 1.5], hours=[1, 2])
    probe("Axes.broken_barh", _broken_barh_dates, start=day[0], hours=1, yrange=[10, 20])


def _bar_datetime_start(orientation):
    start = np.array([np.datetime64("2012-01-01"), np.datetime64("2012-02-01"),
                      np.datetime64("2012-01-15")])
    stop = np.array([np.datetime64("2012-02-07"), np.datetime64("2012-02-13"),
                     np.datetime64("2012-02-12")])
    ax = _axes()
    if orientation == "vertical":
        ax.bar([0, 1, 3], height=stop - start, bottom=start)
        return isinstance(ax.yaxis.get_major_formatter(), mdates.AutoDateFormatter)
    ax.barh([0, 1, 3], width=stop - start, left=start)
    return isinstance(ax.xaxis.get_major_formatter(), mdates.AutoDateFormatter)


def test_bar_datetime_start():
    for orientation in ["vertical", "horizontal"]:
        probe("Axes.bar.datetime_start", _bar_datetime_start, orientation=orientation)


def _grouped_bar(heights, **kw):
    ax = _axes()
    ret = ax.grouped_bar(heights, **kw)
    containers = ret.bar_containers
    result = {
        "containers": [len(containers), sum(c in ax.containers for c in containers)],
        "hatches": [[r.get_hatch() for r in c] for c in containers],
    }
    ret.remove()
    result["removed"] = sum(c in ax.containers for c in containers)
    return result


def test_grouped_bar_return_value():
    probe("Axes.grouped_bar", _grouped_bar, heights=[[1, 2, 3], [11, 12, 13]],
          tick_labels=["A", "B", "C"])


def test_grouped_bar_hatch_sequence():
    probe("Axes.grouped_bar", _grouped_bar, heights=[[1, 2], [2, 3], [3, 4]], positions=[0, 1],
          hatch=["//", "xx", ".."])


# --- box and violin plots ------------------------------------------------------------

def _line_desc(line):
    x, y = line.get_data()
    return [np.asarray(x, float), np.asarray(y, float), _marker(line), line.get_zorder()]


def _boxplot(data, before=None, before_positions=None, **kw):
    ax = _axes()
    if "xlim" in kw:
        ax.set_xlim(*kw.pop("xlim"))
    if before is not None:
        ax.boxplot(before, positions=before_positions)
    parts = ax.boxplot(data, **kw)
    ax.figure.canvas.draw()
    result = {key: [_line_desc(line) for line in parts[key]]
              for key in ["boxes", "whiskers", "caps", "medians", "fliers", "means"]}
    result["xlim"] = ax.get_xlim()
    result["xticks"] = ax.get_xticks()
    result["xticklabels"] = [t.get_text() for t in ax.get_xticklabels()]
    return result


BOX_X = np.hstack([-25, np.linspace(-7, 7, 140), 25]).tolist()


def test_boxplot_capwidths():
    data = rng(5).uniform(15).reshape(5, 3).T.tolist()
    for kw in [dict(capwidths=[0.3, 0.2, 0.1], widths=[0.1, 0.2, 0.3]),
               dict(capwidths=[0.3, 0.2, 0.1], widths=0.2),
               dict(capwidths=[0.3, 0.2, 0.1]),
               dict(capwidths=0.5, widths=[0.1, 0.2, 0.3]),
               dict(capwidths=0.5, widths=0.2),
               dict(capwidths=0.5),
               dict(widths=[0.1, 0.2, 0.3]),
               dict(widths=0.2),
               {}]:
        probe("Axes.boxplot", _boxplot, data=data, **kw)


def test_boxplot_bad_medians():
    assert isinstance(probe("Axes.boxplot", _boxplot, data=[BOX_X], usermedians=[1, 2]),
                      ValueError)


def test_boxplot_bad_ci():
    assert isinstance(probe("Axes.boxplot", _boxplot, data=[BOX_X, BOX_X],
                            conf_intervals=[[1, 2]]), ValueError)


def test_boxplot_zorder():
    x = list(range(10))
    probe("Axes.boxplot", _boxplot, data=[x])
    probe("Axes.boxplot", _boxplot, data=[x], zorder=10)


def test_boxplot_marker_behavior():
    test_data = list(range(100))
    test_data[-1] = 150
    probe("Axes.boxplot", _boxplot, data=[test_data], showmeans=True,
          rc={"lines.marker": "s", "boxplot.flierprops.marker": "o",
              "boxplot.meanprops.marker": "^"})


def test_manage_xticks():
    gen = rng(0)
    y1, y2 = gen.normal(20, 10, 3).tolist(), gen.normal(20, 3, 1).tolist()
    probe("Axes.boxplot", _boxplot, data=[y1, y2], xlim=[0, 4], positions=[1, 2],
          manage_ticks=False)


def test_boxplot_not_single():
    gen = rng(6)
    first = [gen.uniform(100).tolist()]
    probe("Axes.boxplot", _boxplot, before=first, before_positions=[3],
          data=[gen.uniform(100).tolist()], positions=[5])


def _bxp(data, **kw):
    ax = _axes()
    ax.set_yscale("log")
    mpl.rcParams["boxplot.boxprops.linewidth"] = mpl.rcParams["lines.linewidth"]
    return len(ax.bxp(boxplot_stats(data), **kw)["boxes"])


BXP_DATA = np.exp(rng(937).normal(4 * 37, 1.25, 1.0)).reshape(4, 37).tolist()


def test_bxp_bad_widths():
    assert isinstance(probe("Axes.bxp", _bxp, data=BXP_DATA, widths=[1, 2]), ValueError)


def test_bxp_bad_positions():
    assert isinstance(probe("Axes.bxp", _bxp, data=BXP_DATA, positions=[2, 3]), ValueError)


def test_bxp_bad_capwidths():
    assert isinstance(probe("Axes.bxp", _bxp, data=BXP_DATA, capwidths=[1, 2]), ValueError)


VIOLIN_LINES = ["cbars", "cmins", "cmaxes", "cmeans", "cmedians"]


def _violinplot(data, **kw):
    ax = _axes()
    for key in ["facecolor", "linecolor"]:
        if key in kw:
            kw[key] = _colors_arg(kw[key])
    parts = ax.violinplot(data, **kw)
    return {
        "bodies": [_rgba_list(b.get_facecolor()) for b in parts["bodies"]],
        "alphas": [b.get_alpha() for b in parts["bodies"]],
        "lines": {k: _rgba_list(parts[k].get_edgecolor()) for k in VIOLIN_LINES if k in parts},
    }


def _normal_sets(seed, count, size=100, scale=1.0):
    gen = rng(seed)
    return [gen.normal(size, 0.0, scale).tolist() for _ in range(count)]


def test_violinplot_bad_positions():
    assert isinstance(probe("Axes.violinplot", _violinplot, data=_normal_sets(855654600, 4),
                            positions=list(range(5))), ValueError)


def test_violinplot_bad_widths():
    assert isinstance(probe("Axes.violinplot", _violinplot, data=_normal_sets(280109889, 4),
                            positions=list(range(4)), widths=[1, 2, 3]), ValueError)


def test_violinplot_bad_quantiles():
    assert isinstance(probe("Axes.violinplot", _violinplot, data=_normal_sets(544003745, 1),
                            quantiles=[[0.1, 0.2], [0.5, 0.7]]), ValueError)


def test_violinplot_outofrange_quantiles():
    data = _normal_sets(888194417, 1)
    for quantiles in [[[0.1, 0.2, 0.3, 1.05]], [[-0.05, 0.2, 0.3, 0.75]]]:
        assert isinstance(probe("Axes.violinplot", _violinplot, data=data, quantiles=quantiles),
                          ValueError)


def test_violinplot_color_sequence():
    gen = rng(19680801)
    data = [sorted(gen.normal(100, 0.0, std).tolist()) for std in range(1, 5)]
    probe("Axes.violinplot", _violinplot, data=data, positions=list(range(4)),
          facecolor=["k", "r", ["b", 0.5], ["g", 0.2]],
          linecolor=[["y", 0.4], "b", "m", ["k", 0.8]],
          showmeans=True, showextrema=True, showmedians=True)


def _violin_alpha(data, **kw):
    ax = _axes()
    if "facecolor" in kw:
        kw["facecolor"] = _color_arg(kw["facecolor"])
    body = ax.violinplot(data, positions=[1], **kw)["bodies"][0]
    face, alpha = _rgba_list(body.get_facecolor()), body.get_alpha()
    body.set_facecolor("red")
    return {"face": face, "alpha": alpha, "after": _rgba_list(body.get_facecolor())}


def test_violinplot_alpha():
    data = _normal_sets(3, 1)
    probe("Axes.violinplot.set_facecolor", _violin_alpha, style=["default"], data=data)
    probe("Axes.violinplot.set_facecolor", _violin_alpha, style=["default"], data=data,
          facecolor=["blue", 0.3])


VIOLIN_STATS = [
    {"coords": [19398.0, 19495.0, 19514.0], "vals": [1.2, 2.8, 1.5], "mean": 1.84,
     "median": 1.5, "min": 1.2, "max": 2.8, "quantiles": [1.2, 1.5, 2.8]},
    {"coords": [19398.0, 19495.0, 19514.0], "vals": [0.8, 1.1, 0.9], "mean": 0.94,
     "median": 0.9, "min": 0.8, "max": 1.1, "quantiles": [0.8, 0.9, 1.1]},
]


def _violin(stats, positions, widths):
    ax = _axes()
    as_dates = [dict(s, coords=mdates.num2date(s["coords"])) for s in stats]
    ax.violin(as_dates, positions=positions, widths=widths)
    return ax.dataLim.get_points()


def test_numeric_positions_with_float_widths_ok():
    probe("Axes.violin", _violin, stats=VIOLIN_STATS, positions=[1.0, 2.0], widths=[0.5, 1.0])


# --- error bars ----------------------------------------------------------------------

def _line_style(line):
    return {"color": _rgba(line.get_color()), "marker": _marker(line),
            "linestyle": _linestyle(line), "fillstyle": line.get_fillstyle(),
            "drawstyle": line.get_drawstyle()}


def _errorbar(x, y, **kw):
    ax = _axes()
    if "errorevery" in kw and isinstance(kw["errorevery"], list) and len(kw["errorevery"]) == 2 \
            and all(isinstance(v, int) and not isinstance(v, bool) for v in kw["errorevery"]):
        kw["errorevery"] = tuple(kw["errorevery"])
    line, caps, bars = ax.errorbar(x, y, **kw)
    return {
        "line": None if line is None else _line_style(line),
        "bars": [{"colors": _rgba_list(b.get_color()),
                  "linestyles": [[off, None if dashes is None else list(dashes)]
                                 for off, dashes in b.get_linestyle()]} for b in bars],
        "caps": len(caps),
    }


def test_errorbar_nonefmt():
    x = list(range(5))
    probe("Axes.errorbar", _errorbar, x=x, y=x, xerr=1, yerr=1, fmt="none")


def test_errorbar_line_specific_kwargs():
    x = list(range(5))
    probe("Axes.errorbar", _errorbar, x=x, y=x, xerr=1, yerr=1, ls="None", marker="s",
          fillstyle="full", drawstyle="steps-mid", dash_capstyle="round",
          dash_joinstyle="miter", solid_capstyle="butt", solid_joinstyle="bevel")


def test_errorbar_every_invalid():
    x = np.linspace(0, 1, 15)
    y = x * (1 - x)
    yerr = (y / 6).tolist()
    x, y = x.tolist(), y.tolist()
    probe("Axes.errorbar", _errorbar, x=x, y=y, yerr=yerr, errorevery=[1, 2])
    assert isinstance(probe("Axes.errorbar", _errorbar, x=x, y=y, yerr=yerr,
                            errorevery=[False, True]), ValueError)


def test_xerr_yerr_not_negative():
    for kw in [dict(xerr=[[-0.5], [1]], yerr=[[-0.5], [1]]), dict(xerr=[[-0.5], [1]]),
               dict(yerr=[[-0.5], [1]])]:
        assert isinstance(probe("Axes.errorbar", _errorbar, x=[0], y=[0], **kw), ValueError)


def test_errorbar_linewidth_type():
    probe("Axes.errorbar", _errorbar, x=[1, 2, 3], y=[1, 2, 3], yerr=[1, 2, 3], elinewidth=1)


def test_errorbar_linestyle_type():
    probe("Axes.errorbar", _errorbar, x=[1, 2, 3], y=[1, 2, 3], yerr=[1, 2, 3], elinestyle="--")


def _errorbar_remove(x, y, xerr, yerr):
    ax = _axes()
    bars = ax.errorbar(x, y, xerr=xerr, yerr=yerr)

    def counts():
        return [len(ax.containers), len(ax.lines), len(ax.collections)]

    before = counts()
    bars.remove()
    return [before, counts()]


def test_errorbar_remove():
    x = list(range(5))
    probe("ErrorbarContainer.remove", _errorbar_remove, x=x, y=x, xerr=1, yerr=1)


def _errorbar_cycle(x):
    ax = _axes()
    y = 2 * np.asarray(x)
    e1, _, _ = ax.errorbar(x, y, c=None)
    e2, _, _ = ax.errorbar(x, 2 * y, c=None)
    ln1, = ax.plot(x, 4 * y)
    return [_rgba(e1.get_color()), _rgba(e2.get_color()), _rgba(ln1.get_color())]


def test_errorbar_colorcycle():
    probe("Axes.errorbar.color_cycle", _errorbar_cycle, x=list(range(10)))


# --- stem plots ----------------------------------------------------------------------

def _stem(x, y, **kw):
    ax = _axes()
    stems = ax.stem(x, y, **kw)
    return {
        "markers": [list(map(float, d)) for d in stems.markerline.get_data()],
        "markercolor": _rgba(stems.markerline.get_color()),
        "marker": _marker(stems.markerline),
        "markerstyle": _linestyle(stems.markerline),
        "linecolor": _rgba_list(stems.stemlines.get_color()),
        "basecolor": _rgba(stems.baseline.get_color()),
    }


STEM_X, STEM_Y = [1, 3, 5], [9, 8, 7]


def test_stem_args():
    for kw in [{}, dict(linefmt="r--"), dict(linefmt="r--", basefmt="b--")]:
        probe("Axes.stem", _stem, x=STEM_X, y=STEM_Y, **kw)
    probe("Axes.stem", _stem, x=[0, 1, 2], y=STEM_Y)


def test_stem_markerfmt():
    for kw in [{}, dict(markerfmt="x"), dict(markerfmt="rx"),
               dict(linefmt="r"), dict(linefmt="rx"), dict(linefmt="r", markerfmt="x"),
               dict(linefmt="r", markerfmt="g"), dict(linefmt="r", markerfmt="gx"),
               dict(linefmt="r", markerfmt=" "), dict(linefmt="r", markerfmt="")]:
        probe("Axes.stem", _stem, x=STEM_X, y=STEM_Y, **kw)


def _stem_dates(x, y):
    ax = _axes()
    stems = ax.stem([datetime.datetime.fromisoformat(day) for day in x], y)
    return {"markers": stems.markerline.get_xydata(), "baseline": stems.baseline.get_xydata()}


def test_stem_dates():
    probe("Axes.stem.dates", _stem_dates, x=["2013-09-28T11:00:00", "2013-09-28T12:00:00"],
          y=[100, 200])


def _stem_baseline(projection):
    ax = _axes(projection=projection)
    x = np.linspace(1.57, 3.14, 10)
    y = np.linspace(0, 1, 10)
    return ax.stem(x, y, bottom=0.5).baseline.get_path()._interpolation_steps > 100


def test_stem_polar_baseline():
    probe("Axes.stem.baseline", _stem_baseline, projection="polar")


# --- histograms ----------------------------------------------------------------------

def _hist(x, **kw):
    ax = _axes()
    n, _, patches = ax.hist(x, **kw)
    n = np.asarray(n)
    groups = [patches] if n.ndim == 1 else list(patches)
    flat = [p for group in groups for p in group]
    return {
        "n": n,
        "xy": [p.get_xy() for p in flat if isinstance(p, Polygon)],
        "faces": [_rgba(p.get_facecolor()) for p in flat],
        "edges": [_rgba(p.get_edgecolor()) for p in flat],
        "zorders": [p.get_zorder() for p in flat],
        "labels": [_legend_label(group[0].get_label()) for group in groups if len(group)],
    }


HIST_BINS = [0, 1, 2, 3]
HIST_1 = [0, 0, 1, 1, 1, 2]
HIST_2 = [0, 1, 2]


def test_hist_step_geometry():
    for histtype in ["step", "stepfilled"]:
        probe("Axes.hist", _hist, x=HIST_1, bins=HIST_BINS, histtype=histtype)


def test_hist_step_bottom_geometry():
    for histtype in ["step", "stepfilled"]:
        probe("Axes.hist", _hist, x=HIST_1, bins=HIST_BINS, bottom=[1, 2, 1.5],
              histtype=histtype)


def test_hist_stacked_step_geometry():
    for histtype in ["step", "stepfilled"]:
        probe("Axes.hist", _hist, x=[HIST_1, HIST_2], bins=HIST_BINS, stacked=True,
              histtype=histtype)


def test_hist_stacked_step_bottom_geometry():
    for histtype in ["step", "stepfilled"]:
        probe("Axes.hist", _hist, x=[HIST_1, HIST_2], bins=HIST_BINS, stacked=True,
              bottom=[1, 2, 1.5], histtype=histtype)


def test_hist_color_semantics():
    for histtype in ["stepfilled", "step"]:
        for kw in [dict(color="r", facecolor="y", edgecolor="g"), dict(color="r", edgecolor="g"),
                   dict(color="r", facecolor="y"), dict(facecolor="y", edgecolor="g"),
                   dict(color="r"), dict(facecolor="y"), dict(edgecolor="g"), {}]:
            probe("Axes.hist", _hist, x=[1, 2, 3], histtype=histtype, **kw)


def test_hist_zorder():
    for histtype in ["bar", "step", "stepfilled"]:
        probe("Axes.hist", _hist, x=[1, 2], histtype=histtype)


def test_hist_single_color_multiple_datasets():
    probe("Axes.hist", _hist, x=[[0, 1, 2], [3, 4, 5]], color="k")


def test_hist_with_empty_input():
    for data in [[], [[]], [[], []]]:
        probe("Axes.hist", _hist, x=data)


def test_hist_unequal_bins_density():
    t = rng(57483).normal(100).tolist()
    probe("Axes.hist", _hist, x=t, bins=[-3, -1, -0.5, 0, 1, 5], density=True)


def test_hist_emptydata():
    probe("Axes.hist", _hist, x=[[], list(range(10)), list(range(10))], histtype="step")


def test_hist_labels():
    for label in [None, "0", "00"]:
        probe("Axes.hist", _hist, x=[0, 1], label=label)


def _hist_dates(data, stacked, bins=None):
    ax = _axes()
    kw = {} if bins is None else {"bins": [_dt(edge) for edge in bins]}
    n, edges, _ = ax.hist([[_dt(day) for day in row] for row in data], stacked=stacked, **kw)
    return {"n": n, "bins": edges}


def test_hist_datetime_datasets():
    data = [["2017-01-01", "2017-01-01"], ["2017-01-01", "2017-01-02"]]
    for stacked in [True, False]:
        probe("Axes.hist.dates", _hist_dates, data=data, stacked=stacked)


def test_hist_datetime_datasets_bins():
    data = [["2019-01-05", "2019-01-11", "2019-02-01", "2019-03-01"],
            ["2019-01-11", "2019-02-05", "2019-02-18", "2019-03-01"]]
    for stacked in [True, False]:
        probe("Axes.hist.dates", _hist_dates, data=data, bins=["2019-01-01", "2019-02-01",
                                                               "2019-03-01"], stacked=stacked)


def _hist_labels(x, label):
    ax = _axes()
    ax.hist(x, label=label)
    return ax.get_legend_handles_labels()[1]


def test_hist_unused_labels():
    probe("Axes.get_legend_handles_labels", _hist_labels, x=[[1, 2, 3]],
          label=["values", "unused", "also unused"])


def _hist_log():
    fig = _figure()
    axs = fig.subplots(2)
    axs[0].hist([[0], [0, 1]], 2, histtype="barstacked")
    axs[0].set_yscale("log")
    axs[1].hist([0, 0, 1], 2, histtype="barstacked")
    axs[1].set_yscale("log")
    fig.canvas.draw()
    return [axs[0].get_ylim(), axs[1].get_ylim()]


def test_hist_log_barstacked():
    probe("Axes.hist.log", _hist_log)


def _hist2d(x, y, density):
    return _axes().hist2d(x, y, density=density)[0]


def test_hist2d_density():
    gen = rng(2)
    probe("Axes.hist2d", _hist2d, x=gen.uniform(100).tolist(), y=gen.uniform(100).tolist(),
          density=True)


# --- stairs and stack plots ----------------------------------------------------------

def _stairs(values, edges, **kw):
    ax = _axes()
    ax.stairs(values, edges, **kw)
    return {"xlim": ax.get_xlim(), "ylim": ax.get_ylim()}


def test_stairs_empty():
    probe("Axes.stairs", _stairs, values=[], edges=[42])


def test_stairs_invalid_nan():
    assert isinstance(probe("Axes.stairs", _stairs, values=[1, 2], edges=[0, np.nan, 1]),
                      ValueError)


def test_stairs_invalid_mismatch():
    assert isinstance(probe("Axes.stairs", _stairs, values=[1, 2], edges=[0, 1]), ValueError)


def _stairs_update(values, edges, new_edges):
    _axes().stairs(values, edges).set_data(edges=new_edges)


def test_stairs_invalid_update():
    assert isinstance(probe("StepPatch.set_data", _stairs_update, values=[1, 2], edges=[0, 1, 2],
                            new_edges=[1, np.nan, 2]), ValueError)


def test_stairs_invalid_update2():
    assert isinstance(probe("StepPatch.set_data", _stairs_update, values=[1, 2], edges=[0, 1, 2],
                            new_edges=[0, 1, 2, 3, 4]), ValueError)


def _subfig_legend():
    fig = _figure()
    for subfig in fig.subfigures(nrows=1, ncols=2):
        ax = subfig.subplots(nrows=1, ncols=1)
        ax.stackplot([3, 4], [[1, 2]], labels=["a"])
    legend = fig.legend()
    fig.draw_without_rendering()
    return [t.get_text() for t in legend.get_texts()]


def test_stackplot_subfig_legend():
    probe("Figure.legend", _subfig_legend)


# --- scatter -------------------------------------------------------------------------

def _scatter(x, y, **kw):
    ax = _axes()
    for key in ["c", "color", "edgecolors", "edgecolor"]:
        if key in kw:
            kw[key] = _colors_arg(kw[key])
    if "norm" in kw:
        kw["norm"] = mcolors.Normalize(*kw["norm"])
    if "fillstyle" in kw:
        kw["marker"] = mpl.markers.MarkerStyle(kw.get("marker", "o"), kw.pop("fillstyle"))
    coll = ax.scatter(x, y, **kw)
    coll.update_scalarmappable()
    return {"faces": _rgba_list(coll.get_facecolors()), "edges": _rgba_list(coll.get_edgecolors()),
            "linewidths": coll.get_linewidths()}


def test_scatter_color():
    assert isinstance(probe("Axes.scatter", _scatter, x=[1, 2, 3], y=[1, 2, 3], color=[1, 2, 3]),
                      ValueError)


def test_scatter_unfilled():
    probe("Axes.scatter", _scatter, x=[0, 1, 2], y=[1, 3, 2], c=["0.1", "0.3", "0.5"],
          marker="o", fillstyle="none", linewidths=[1.1, 1.2, 1.3])


def test_scatter_unfillable():
    probe("Axes.scatter", _scatter, style=["default"], x=[0, 1, 2], y=[1, 3, 2],
          c=["0.1", "0.3", "0.5"], marker="x", linewidths=[1.1, 1.2, 1.3])


def test_scatter_size_arg_size():
    x = [0, 1, 2, 3]
    for args in [dict(x=x, y=x, s=x[1:]), dict(x=x[1:], y=x[1:], s=x)]:
        assert isinstance(probe("Axes.scatter", _scatter, **args), ValueError)


def test_scatter_edgecolor_RGB():
    probe("Axes.scatter", _scatter, x=[1, 2, 3], y=[1, np.nan, np.nan], edgecolor=[1, 0, 0])
    probe("Axes.scatter", _scatter, x=[1, 2, 3, 4], y=[1, np.nan, np.nan, 1],
          edgecolor=[1, 0, 0, 1])


def test_scatter_norm_vminvmax():
    x = [1, 2, 3]
    probe("Axes.scatter", _scatter, x=x, y=x, c=x, norm=[-10, 10])
    assert isinstance(probe("Axes.scatter", _scatter, x=x, y=x, c=x, norm=[-10, 10], vmin=0,
                            vmax=5), ValueError)


SCATTER_C_CASES = [
    "0.5", ["rgby"], "red", "none", None, ["r", "g", "b", "none"], "jaune", ["jaune"], ["jaune"] * 4,
    [0.5] * 4, [0.5] * 5, ["0.5", "0.4", "0.6", "0.7"], ["0.5", "red", "0.6", "C5"],
    [[1, 0, 0]], [[1, 0, 0]] * 3, [[1, 0, 0]] * 4, [[1, 0, 0]] * 5,
    [[1, 0, 0, 0.5]], [[1, 0, 0, 0.5]] * 3, [[1, 0, 0, 0.5]] * 4, [[1, 0, 0, 0.5]] * 5,
    [[1, 0, 0, 0.5]] * 3 + [[1, 0, 0]],
    [[1, 0, 0, 0.5], "red", "0.0"], [[1, 0, 0, 0.5], "red", "0.0", "C5"],
    [[1, 0, 0, 0.5], "red", "0.0", "C5", [0, 1, 0]],
    [[1, 0, 0, 0.5], "red", "jaune"], [[1, 0, 0, 0.5], "red", "0.0", "jaune"],
    [[1, 0, 0, 0.5], "red", "0.0", "C5", "jaune"],
]


def test_scatter_c():
    x = [0, 1, 2, 3]
    for c in SCATTER_C_CASES:
        probe("Axes.scatter", _scatter, x=x, y=x, edgecolors="black",
              **({} if c is None else {"c": c}))


def test_scatter_linewidths():
    x = list(range(5))
    for i in range(3):
        probe("Axes.scatter", _scatter, x=x, y=[i] * 5, c=f"C{i}", marker="x", s=100,
              linewidths=i + 1)


def test_parse_scatter_color_args():
    x = [0, 1]
    for kw in [{}, dict(c="r"), dict(color="b"), dict(color=["b", "g"])]:
        probe("Axes.scatter", _scatter, x=x, y=x, **kw)


def test_parse_scatter_color_args_edgecolors():
    x = [0, 1]
    for kw in [{}, dict(c="b"), dict(edgecolors="r"), dict(edgecolors=["r", "g"]),
               dict(edgecolors="face"), dict(edgecolors="none"), dict(color="r"),
               dict(color="r", edgecolors="g")]:
        probe("Axes.scatter", _scatter, x=x, y=x, **kw)


def test_parse_scatter_color_args_error():
    assert isinstance(probe("Axes.scatter", _scatter, x=[0, 1], y=[0, 1],
                            c=[[0.1, 0.2, 0.7], [0.2, 0.4, 1.4]]), ValueError)


# --- event plots ---------------------------------------------------------------------

def _eventplot(positions, **kw):
    ax = _axes()
    if "colors" in kw:
        kw["colors"] = _colors_arg(kw["colors"])
    collections = ax.eventplot(positions, **kw)
    return {"colors": [_rgba_list(c.get_color()) for c in collections],
            "alphas": [c.get_alpha() for c in collections]}


def test_eventplot_colors():
    data = [[0], [1], [2], [3]]
    for colors in ["0.5", ["tab:orange", "tab:pink", "tab:cyan", "bLacK"],
                   ["red", [0, 1, 0], [1, 0, 1, 0.5]]]:
        probe("Axes.eventplot", _eventplot, positions=data[:len(colors)] if isinstance(
            colors, list) else data, colors=colors)


def test_eventplot_alpha():
    data = [[0, 2, 4], [1, 3, 5, 7]]
    probe("Axes.eventplot", _eventplot, positions=data, alpha=0.7)
    probe("Axes.eventplot", _eventplot, positions=data, alpha=[0.5, 0.7])
    assert isinstance(probe("Axes.eventplot", _eventplot, positions=data,
                            alpha=[0.5, 0.7, 0.9]), ValueError)
    assert isinstance(probe("Axes.eventplot", _eventplot, positions=[[0, 2, 4]],
                            alpha=[0.5, 0.7]), ValueError)


# --- projections, saving, dates ------------------------------------------------------

def _mollweide(lonlat=None, xy=None):
    trans = _axes(projection="mollweide").transProjection
    if lonlat is not None:
        return trans.transform(np.asarray(lonlat, float))
    return trans.inverted().transform(np.asarray(xy, float))


def test_mollweide_forward_inverse_closure():
    lon = np.radians(np.arange(-180, 181, 15))
    lat = np.radians(np.arange(-75, 76, 15))
    lon, lat = np.meshgrid(lon, lat)
    probe("MollweideAxes.transProjection", _mollweide, tol=1e-9,
          lonlat=np.column_stack([lon.ravel(), lat.ravel()]).tolist())


def test_mollweide_inverse_forward_closure():
    x = np.linspace(0, 1, 9)
    x, y = np.meshgrid(x, x)
    probe("MollweideAxes.transProjection", _mollweide, tol=1e-9,
          xy=np.column_stack([x.ravel(), y.ravel()]).tolist())


def _savefig_raw():
    fig = _figure()
    fig.add_subplot().plot([0, 1], [0, 1])
    buffer = io.BytesIO()
    fig.savefig(buffer, dpi=80, format="raw")
    width, height = fig.canvas.get_width_height()
    return [width, height, len(buffer.getvalue()) // 4]


def test_tick_space_size_0():
    probe("Figure.savefig", _savefig_raw, rc={"font.size": 0})


def _sca_figures():
    plt.close("all")
    try:
        fig1, ax1 = plt.subplots()
        plt.subplots()
        plt.sca(ax1)
        return [ax1 is plt.gca(), fig1 is plt.gcf()]
    finally:
        plt.close("all")


def test_pyplot_axes():
    probe("pyplot.sca", _sca_figures)
