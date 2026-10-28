"""matplotlib's own non-image tests of ``Axes``
(``matplotlib/tests/test_axes.py``) that no other probe area already
covers.

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
    the offsets short; ``test_pathological_hexbin``).
``Axes.norm_by_name``  {plot, values, norm, vmin?, vmax?, samples} -> {vmin,
    vmax, normalized}: the limits of the norm of what ``plot`` makes of the
    four ``values`` with ``norm`` given by its scale name and the limits
    given, and what that norm makes of ``samples`` (``null`` where it
    masks one). ``plot`` is ``"hexbin"`` (the values as ``C`` at the
    corners of the unit square, each in a hexagon of its own),
    ``"imshow"`` and ``"pcolormesh"`` (the values as a 2 x 2 grid) or
    ``"scatter"`` (the values as ``c`` of four points)
    (``test_hexbin_string_norm``).
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
``Axes.bar``  {x, height, color?, facecolor?, edgecolor?, alpha?, label?,
    bottom?} -> {rects, faces, edges, labels, label, xlim, ylim}: ``[x, y,
    width, height]``, face and edge colour and legend label of every bar,
    the container's legend label and the view limits, or the error
    (``test_nan_bar_values``, ``test_bar_ticklabel_fail``,
    ``test_bar_color_none_alpha``, ``test_bar_edgecolor_none_alpha``,
    ``test_bar_labels``, ``test_bar_labels_length``,
    ``test_bar_color_precedence``, ``test_bar_shape_mismatch``,
    ``test_bar_broadcast_args``, ``test_bar_leading_nan``).
``Axes.bar.timedelta``  {orientation, x, values, hours, err_hours?} ->
    {rects, xlim, ylim}: ``bar`` (``barh``) at the ISO datetimes ``x`` with
    widths (heights) of ``hours`` hours, one or one per bar, and an error
    of ``err_hours`` along the dates (``test_bar_timedelta``).
``Axes.broken_barh``  {start, hours, yrange} -> the data limits after
    ``broken_barh([(start, timedelta(hours=hours))], yrange)``
    (``test_bar_timedelta``, ``test_broken_barh_timedelta``).
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
    ``conf_intervals``, ``showmeans``, ``manage_ticks``, ``zorder``,
    ``tick_labels``; before
    that, ``xlim`` sets the x limits and the datasets ``before`` are
    boxplotted at ``before_positions`` (``test_boxplot_capwidths``, ``test_boxplot_bad_medians``,
    ``test_boxplot_bad_ci``, ``test_boxplot_zorder``,
    ``test_boxplot_marker_behavior``, ``test_manage_xticks``,
    ``test_boxplot_not_single``, ``test_boxplot_tick_labels``).
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
    ``test_violinplot_color_sequence``, ``test_violin_point_mass``,
    ``test_violinplot_empty_dataset``).
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
    after removing its container; ``null`` errors are not given
    (``test_errorbar_remove``, ``test_errorbar_inputs_shotgun`` without
    the arrays that repeat its lists).
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
    ``test_hist_emptydata``, ``test_hist_labels``,
    ``test_length_one_hist``).
``Axes.hist.dates``  {data, stacked, bins?} -> {n, bins}: the counts and
    the bin edges (date numbers) of ``hist`` of the datasets of ISO
    datetimes ``data``, with bin edges given as datetimes
    (``test_hist_datetime_datasets``, ``test_hist_datetime_datasets_bins``,
    whose ``date2num`` and ``datetime64`` edges are the same values).
``Axes.get_legend_handles_labels``  {x, label} -> the legend labels after
    ``hist(x, label=label)`` (``test_hist_unused_labels``,
    ``test_unicode_hist_label``).
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
    ``test_parse_scatter_color_args_error``, ``test_color_length_mismatch``,
    ``test_scatter_empty_data``, ``test_scatter_color_repr_error``).
``Axes.eventplot``  {positions, colors?, alpha?, lineoffsets?,
    linelengths?, linewidths?, linestyles?} -> {colors, alphas}: each
    collection's colour and alpha, or the error. ``colors`` is a colour
    string or a list of colour strings and RGB(A) lists
    (``test_eventplot_colors``, ``test_eventplot_alpha``,
    ``test_eventplot_errors``, whose numbers for colours and line styles
    are two names here: only the lengths count).
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
``Axes.plot``  {y, x?, fmt?, color?, linestyle?} -> the ``{color, marker,
    linestyle}`` of each line of ``plot([x,] y[, fmt], ...)``, or the
    error; ``y`` holds numbers or categories (``test_plot_format_errors``,
    whose ``"string"`` is the category list ``["string"]``,
    ``test_plot_format``, ``test_plot_errors``, ``test_empty_line_plots``,
    ``test_none_kwargs``, ``test_color_None``).
``Axes.axline``  {xy1, xy2?, slope?, xscale?, yscale?} -> ``true`` once
    ``axline(xy1, xy2, slope=slope)`` on axes of these scales is drawn, or
    the error (``test_axline_args``).
``Axes.vlines`` / ``Axes.hlines``  {rc} -> the colours of ``vlines(0.5, 0,
    1)`` (``hlines``) (``test_vlines_default``, ``test_hlines_default``).
``Axes.fill_between.cycle``  {} -> ``[faces, edges]`` of three
    ``fill_between(range(3), range(3))``, three ``fill_betweenx``, and
    ``fill_between`` with ``facecolor``, ``color`` and twice ``edgecolor``
    ``"k"`` (``test_fillbetween_cycle`` without the keyword aliases).
``Axes.stackplot``  {facecolor?, facecolors?, colors?} -> {faces, next}:
    the face colours of the two layers of ``stackplot(x, 1.0 * x, 2.0 * x
    + 1, ...)`` over ``x = np.linspace(0, 10, 10)``, given the keywords
    present, and the colour of a ``plot`` drawn next
    (``test_stackplot_facecolor``).
``Axes.bar.color_cycle``  {} -> five times the colour of ``plot(range(3))``
    and the face colours of ``bar(range(3), range(3))`` drawn after it
    (``test_bar_color_cycle``).
``Axes.eventplot.segments``  {positions, orientation?, colors?} -> each
    collection's segments after a draw (``test_empty_eventplot``,
    ``test_eventplot_orientation``).
``Axes.eventplot.legend``  {positions, label} -> the legend texts after
    ``eventplot(positions, label=label)`` (``test_eventplot_legend``).
``Axes.pie``  {x, explode?, labels?, radius?, normalize?, autopct?,
    textprops?} -> {wedges, texts, pct}: ``[theta1, theta2]`` of each wedge,
    and each label and percentage label as ``{text, ha, va, rotation,
    rotation_mode, size, color}`` (labels only when given), or the error
    (``test_pie_textprops``, ``test_pie_get_negative_values``,
    ``test_pie_invalid_explode``, ``test_pie_invalid_labels``,
    ``test_pie_invalid_radius``, ``test_normalize_kwarg_pie``,
    ``test_pie_non_finite_values``, ``test_pie_all_zeros``).
``Axes.pie_label``  {x, labels, startangle?, distance?, rotate?,
    alignment?} -> the texts of ``pie_label(pie(x, startangle=...),
    labels, ...)`` described as for ``Axes.pie``, or the error; ``labels``
    is a template or a list (``test_pie_label_formatter``,
    ``test_pie_label_auto_align``, ``test_pie_label_fail``).
``Axes.bar_label``  {x, values, orientation?, invert?, yerr?, label_type?,
    fmt?, labels?, padding?} -> {labels, ylim}: ``{text, xy, xyann, ha,
    va}`` of each label of ``bar_label`` on ``bar(x, values)`` (``barh``),
    after inverting the axes listed in ``invert``, and the y limits, or the
    error (the ``test_bar_label_location_*`` tests, ``test_bar_label_fmt``
    with templates, ``test_bar_label_labels``, ``test_bar_label_nan_ydata``
    and ``test_bar_label_nan_ydata_inverted``, which only reads whether y
    is inverted and so is the same probe, ``test_bar_label_padding``,
    ``test_nan_barlabels``, ``test_int_fmt_bar_label``).
``Axes.barh``  {y, width, left?, height?} -> ``[x, y, width, height]`` of
    each bar (``test_bar_broadcast_args``, ``test_bar_leading_nan``).
``Axes.bar.errorbar``  {x, height, yerr, capsize, zorder} -> {bars, caps,
    barlines}: the z-order of the bars, cap lines and bar line collections
    (``test_bar_errbar_zorder``).
``Axes.broken_barh.paths``  {xranges, yrange, align?} -> {extents, xlim,
    ylim}: each path's extent and the view limits; ``yrange``'s position
    may be a category (``test_broken_barh_empty``,
    ``test_broken_barh_align``, ``test_broken_barh_categorical``).
``Axes.hist.bins``  {x, bins?, range?, density?} -> {n, bins}: the counts
    and bin edges; ``bins`` may name an estimator (``test_hist_auto_bins``,
    ``test_hist_nan_data``, ``test_hist_range_and_density``).
``Axes.errorbar.caps``  {x, y, yerr, ecolor?, fmt?, rc?} -> {line, caps,
    barlines}: whether there is a data line, ``[markeredgecolor,
    markersize, markeredgewidth]`` of each cap line and the line widths of
    each bar collection (``test_caps_color``, ``test_caps_no_ecolor``,
    ``test_errorbar_uses_rcparams``).
``Axes.legend.errorbar``  {} -> the legend texts of two empty error bars
    labelled ``"empty y"`` and ``"empty x"`` (``test_empty_errorbar_legend``).
``Axes.tick_params``  {calls?, draw?, rc?} -> after each ``tick_params(**call)``
    (and a draw): the state of the first major and minor tick of each axis
    (keys ``xmajor``, ``xminor``, ``ymajor``, ``yminor``), as ``{size,
    width, pad, direction, color, markeredgecolor, labelsize, labelcolor,
    labelrotation, rotation_mode, labelfamily, tick1On, tick2On, label1On,
    label2On, gridOn, grid}`` with ``grid`` ``[color, linewidth, alpha,
    linestyle]`` (``test_text_labelsize``,
    ``test_axis_set_tick_params_labelsize_labelcolor``,
    ``test_axes_tick_params_gridlines``, ``test_axes_tick_params_ylabelside``,
    ``test_axes_tick_params_xlabelside``, ``test_tick_param_label_rotation``
    under the canonical names ``labelrotation`` and
    ``labelrotation_mode``, ``test_tick_param_labelfont``,
    ``test_xtickcolor_is_not_markercolor`` with
    ``test_ytickcolor_is_not_markercolor``,
    ``test_xtickcolor_is_not_xticklabelcolor`` with
    ``test_ytickcolor_is_not_yticklabelcolor``,
    ``test_relative_ticklabel_sizes``, ``test_rcparam_grid_minor``).
``Axes.tick_params.rc_context``  {} -> {tick, same}: the first major x tick,
    described as for ``Axes.tick_params``, of axes made inside an
    ``rc_context`` of tick settings and drawn after it, and whether every
    major tick has its size, width, pad, rotation, z-order and direction
    (``test_context_ticks``).
``Axes.tick_params.reset``  {} -> whether the first major x gridline is
    visible after ``tick_params(reset=True, which="major", labelsize=10)``,
    then after ``clear()`` and the same call under ``axes.grid``
    (``test_reset_grid``).
``Axes.grid``  {color} -> ``[alpha, color]`` of the first major gridline of
    each axis after ``grid(True, color=color)`` (``test_grid_color_with_alpha``).
``Axis.get_offset_text``  {axis, data, params?, rc?} -> {color, visible} of
    the offset text of ``axis`` after ``plot(data)`` and
    ``set_tick_params(**params)`` (``test_offset_label_color``,
    ``test_offset_text_visible`` with ``labelleft``/``labelright`` for
    ``label1On``/``label2On``, ``test_xaxis_offsetText_color``,
    ``test_yaxis_offsetText_color``).
``Axis.set_ticklabels``  {ticks, labels, bar?} -> the x tick labels after
    ``set_xticks(ticks)``, ``set_xticklabels(labels)`` and a draw, or the
    error (``test_mismatched_ticklabels``, ``test_empty_ticks_fixed_loc``,
    ``test_warn_too_few_labels``).
``Axis.get_tick_params``  {} -> {before, after}: ``get_tick_params`` of the
    major and minor y ticks before and after ``set_tick_params(labelsize=30,
    labelcolor="red", direction="out", which="both")``
    (``test_axis_get_tick_params``).
``Axis.label``  {rc} -> ``[color, fontsize, fontweight]`` of the x axis
    label (``test_rc_axes_label_formatting``).
``Axis.label.get_ha``  {ha} -> {ha, extent}: the horizontal alignment of
    the y axis label after ``set_ylabel("test", y=1, ha=ha)`` and
    ``yaxis.set_label_position("right")``, and the label's window extent
    after a draw (``test_ylabel_ha_with_position``).
``Axes.title.get_position``  {case, rc?} -> the heights of the titles of a
    scene (see ``_titles``) after a draw (``test_title_location_shared``,
    ``test_titlesetpos``, ``test_title_xticks_top``,
    ``test_title_xticks_top_both``, ``test_title_above_offset``,
    ``test_title_above_hidden_offset``, whose finite title and position
    are recorded, ``test_title_no_move_off_page``, ``test_title_inset_ax``).
``Axes.set_title.twiny``  {} -> the gap between the title and the top x
    label of a ``twiny`` at 72 dpi (``test_titletwiny``).
``Axes.secondary_axis``  {orientation, location} -> ``true`` once
    ``secondary_<orientation>axis(location)`` is made, or the error
    (``test_secondary_fail``).
``Axes.secondary_xaxis.resize``  {} -> the host's position after a
    secondary axis with ``1 / x`` functions, a draw and a resize
    (``test_secondary_resize``).
``Axes.secondary_xaxis.minor_locator``  {} -> the class names of the
    secondary axis' minor locator: new, after ``minorticks_on``, after the
    host turns logarithmic and after it turns linear again
    (``test_secondary_minorloc``).
``Axes.secondary_xaxis.formatter``  {} -> the class name of the major
    formatter of a secondary axis given a ``ScalarFormatter`` on a
    logarithmic host, after a draw (``test_secondary_formatter``).
``Axes.secondary_axis.set_color``  {orientation, how} -> {spines, color,
    labelcolor, label, minor}: after ``secondary_<orientation>axis(1,
    color="red")`` (``how = "keyword"``) or ``set_color("red")`` on the
    secondary axis made without it (``"method"``), the edge colours of its
    four spines (by side), the ``color`` and ``labelcolor`` of its axis'
    ``get_tick_params()``, the colour of the axis label and the colour of
    the minor ticks, which keep theirs (``test_set_secondary_axis_color``).
``Axes.secondary_axis.set_ticks``  {orientation} -> the class name of the
    major locator of a secondary axis at 1 after ``set_<orientation>ticks``
    (``test_secondary_init_xticks``, ticks set by the setter rather than a
    keyword).
``pyplot.subplot``  {nrows, ncols, index} -> ``get_geometry()`` of
    ``plt.subplot(nrows, ncols, index)`` after ``twinx()``
    (``test_subplot_key_hash``).
``Axes.specgram``  {x, scale} -> the extent of the image of
    ``specgram(x, Fs=None, scale=scale)`` (``test_specgram_fs_none``).
``Axes.psd``  {kind} -> whether the y axis is still inverted, and the number
    of points of the line, after ``psd(zeros(5))`` (``csd``) on an inverted
    y axis (``test_psd_csd_edge_cases``).
``Axes.pcolorfast``  {C, x?, y?} -> the class name of the artist
    ``pcolorfast([x, y,] C)`` makes, or the error; ``C`` holds numbers or RGB
    triples (``test_pcolorfast``, ``test_pcolorfast_bad_dims``,
    ``test_pcolorfast_regular_xy_incompatible_size``).
``Axes.clim``  {method} -> the norm's limits of ``scatter``, ``imshow``,
    ``pcolor``, ``pcolormesh`` or ``pcolorfast`` given ``clim=(7, 8)``
    (``test_clim``).
``FigureCanvasBase.inaxes``  {} -> whether the canvas finds axes at ``(200,
    200)`` before and after hiding them (``test_invisible_axes_events``).
``Axes.add_patch``  {} -> the data limits' bounds after adding a wedge
    (``test_patch_bounds``).
``Axes.artist_sublists``  {} -> the lengths of ``collections``, ``images``,
    ``lines``, ``patches``, ``tables`` and ``texts`` after adding six lines,
    a scatter, an image, a rectangle and a text, then after removing each
    in turn (``test_artist_sublists``).
``Axes.autoscale``  {vertices} -> {xlim, ylim} after ``autoscale()`` of a
    path patch through ``vertices`` (``test_small_autoscale``).
``Axes.ecdf``  {x} -> the line's points, or the error (``test_ecdf_invalid``).
``Axes.plot.large_offset``  {} -> {ylim, offset}: the y limits and offset
    text after plotting values near 1e27 that differ by 1e-12 relatively
    (``test_large_offset``).
``Axes.barbs.dates``  {kind} -> {xlim, datalim} after ``barbs`` (``quiver``)
    at datetimes, given as date numbers in plt-rs (``test_barb_units``,
    ``test_quiver_units``).
``Figure.savefig.pdf``  {} -> whether a PDF with a rasterised line under a
    rasterisation z-order starts with ``%PDF``
    (``test_zorder_and_explicit_rasterization``).
``Axes.indicate_inset``  {} -> {connectors, extent}: whether an indicator
    without inset axes has no connectors, and its rectangle's extent after
    a draw (``test_inset``, on an empty square view rather than a mesh).
``Axes.set_box_aspect.constrained``  {} -> the sizes of six square axes
    under the constrained layout (``test_bbox_aspect_axes_init``).
``Axes.get_tightbbox.decorations``  {case} -> {axis, spines, ax, tight}:
    what the upstream ``color_boxes`` measures in a scene at 200 dpi
    outside classic mode: the axes' tight boxes, the spines' extents with
    their ticks, the axes box and its tight box (``test_normal_axes``,
    ``test_nodecorator``, ``test_displaced_spine``, ``test_tickdirs``,
    ``test_minor_accountedfor``).
``Axes.get_tightbbox.axis_off``  {rc} -> the tight box before and after
    ``axis("off")`` (``test_tick_padding_tightbbox``).
``Axes.get_tightbbox.long_label``  {} -> the tight box's width with and
    without a long x label (``test_tightbbox_includes_long_label``).
``Axes.get_tightbbox.nan_text``  {} -> the tight box's width with a text at
    NaN, tick labels and titles removed (``test_gettightbbox_ignore_nan``).
``Axes.add_collection.autolim``  {} -> the data limits and their smallest
    positive values before and after ``relim`` with a collection added with
    ``autolim=False`` (``test_relim_collection_autolim_false``).
``Axes.remove.child_axes``  {} -> the limits set after removing a twinned
    inset axes sharing y (``test_child_axes_removal``).
``Axes.inset_axes.projection``  {projection} -> the projection name of an
    inset axes and whether it is a child axes (``test_inset_projection``).
``Axes.redraw_in_frame``  {} -> ``true`` once ``redraw_in_frame`` follows a
    draw (``test_redraw_in_frame``).
``Axes.set_xlim``  {left, right} -> the x limits after ``set_xlim(left,
    right)``, or the error of a NaN or infinite limit (the value errors of
    ``test_invalid_axis_limits``, whose probes in area ``axes_behaviour``
    give a single limit).
``Figure.add_axes``  {rect} -> the position bounds of ``add_axes(rect)``,
    or the error of a rectangle with a NaN entry or a negative width
    (``test_invalid_figure_add_axes`` in ``test_figure.py``, its value
    errors).

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

Of the second half of the file, area ``axes_behaviour`` covers
``test_vline_limit``, ``test_empty_shared_subplots``,
``test_shared_with_aspect_1``, ``test_shared_with_aspect_2``,
``test_shared_with_aspect_3``, ``test_shared_aspect_error``,
``test_axis_method_errors`` (its value errors), ``test_twin_with_aspect``,
``test_relim_visible_only``, ``test_relim_collection``,
``test_relim_collection_log_scale``, ``test_relim_collection_autoscale_view``,
``test_subsampled_ticklabels``, ``test_tick_label_update``,
``test_margins``, ``test_margin_getters``, ``test_set_margin_updates_limits``,
``test_margins_errors`` (its value errors), ``test_set_xy_bound``,
``test_rc_tick``, ``test_rc_major_minor_tick``, ``test_square_plot``,
``test_shared_scale``, ``test_title_location_roundtrip``,
``test_axes_margins``, ``test_remove_shared_axes``,
``test_remove_shared_axes_relim``, ``test_shared_axes_autoscale``,
``test_adjust_numtick_aspect``, ``test_auto_numticks``,
``test_auto_numticks_log``, ``test_log_margins``,
``test_invalid_axis_limits``, ``test_minorticks_on``,
``test_twinx_knows_limits``, the five ``test_twinning_patch_visibility_*``
tests, ``test_zoom_inset``, ``test_set_position``,
``test_limits_after_scroll_zoom``, ``test_axis_extent_arg``,
``test_axis_extent_arg2``, ``test_set_ticks_inverted``,
``test_aspect_nonlinear_adjustable_box``,
``test_aspect_nonlinear_adjustable_datalim``, ``test_box_aspect``,
``test_box_aspect_custom_position``, ``test_set_aspect_negative``,
``test_unautoscale``, ``test_autoscale_tiny_sticky``,
``test_multiplot_autoscale``, ``test_sharing_does_not_link_positions``,
``test_shared_axes_clear_scale``, ``test_shared_axes_retick``,
``test_centered_bar_label_label_beyond_limits``, ``test_bezier_autoscale``,
``test_get_xticklabel``, ``test_fill_between_axes_limits`` and
``test_xylim_changed_shared``. ``test_indicate_inset_inverted``,
``test_move_offsetlabel`` and ``test_animated_artists_not_drawn_by_default``
are ported as figure cases (group ``mpl_tests_axes``): the indicator's
connectors, the offset text's place and the absence of animated artists
show in the drawing.

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
without positions or with a positional format.

Left out of the second half: Python mechanics: ``test_secondary_repr``
(``repr``), ``test_axes_clear_reference_cycle`` (garbage collection),
``test_twinning_with_axes_class``, ``test_twinning_default_axes_class`` and
``test_inset_subclass`` (``axes_class``), ``test_title_pad`` and
``test_spines_properbbox_after_zoom`` (the private ``titleOffsetTrans`` and
``_set_view_from_bbox``; the ``navigation`` area covers zooming),
``test_cla_clears_children_axes_and_fig`` (the ``axes`` and ``figure`` of
removed artists, whose plt-rs handles are invalid once removed);
keywords and calling conventions typed options rule out:
``test_specgram_origin_kwarg``, ``test_set_ticks_kwargs_raise_error_without_labels``,
``test_color_alias`` (``c=``), ``test_shared_bool``, ``test_axis_errors``,
the argument checks of ``test_axline_args`` and ``test_secondary_fail``
(missing or redundant points, one function, a string transform), the
``set_yticks`` of a secondary x axis in ``test_secondary_init_xticks``,
``fmt=10`` in ``test_bar_label_fmt_error`` and callable formats in
``test_bar_label_fmt``, ``test_grid`` (``grid()`` with ``visible=None``
toggling; plt-rs's ``grid`` takes the visibility) and the
``grid(color=...)`` of ``test_reset_grid`` that turns the grid on, the
list and tuple arithmetic and ``IndexError`` of ``test_artist_sublists``;
``data=`` (``test_automatic_legend``, the data variants of
``test_plot_format_errors``); pandas inputs
(``test_scatter_series_non_zero_index``, the six ``test_pandas_*`` tests);
numpy dtypes and masks (``test_bar_uint8``, the masked array of
``test_ecdf_invalid``); warnings (``test_warn_ignored_scatter_kwargs``, the
first part of ``test_warn_too_few_labels`` and the single RGB colour of
``test_color_length_mismatch``); inputs Rust's types rule out: datetimes
as span limits (``test_axline_minmax``), ``None``, 2-D and 3-D data
(``test_bad_plot_args``, the 2-D parts of ``test_empty_line_plots``,
``test_xticks_bad_args``, ``test_plot_errors``), numbers as labels
(``test_numerical_hist_label``), a scalar as event positions in
``test_eventplot_errors``, and the multi-group calls of
``test_plot_format``. Also left out: ``test_wrong_use_colorizer``
(plt-rs's colour options are one value holding the norm, colormap and
limits, so a colorizer cannot be combined with them; a norm with limits is
rejected as ``test_imshow_norm_vminvmax`` checks).
"""

import datetime
import io

import matplotlib as mpl
import matplotlib.collections as mcollections
import matplotlib.colors as mcolors
import matplotlib.dates as mdates
import matplotlib.font_manager
import matplotlib.patches as mpatches
import matplotlib.path as mpath
import matplotlib.pyplot as plt
import matplotlib.ticker as mticker
import numpy as np
from matplotlib.backend_bases import MouseEvent
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.cbook import boxplot_stats
from matplotlib.figure import Figure
from matplotlib.patches import Polygon
from matplotlib.testing.decorators import remove_ticks_and_titles

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


def _norm_by_name(plot, values, norm, samples, vmin=None, vmax=None):
    ax = _axes()
    kw = {"norm": norm, "vmin": vmin, "vmax": vmax}
    if plot == "hexbin":
        mappable = ax.hexbin([0, 1, 0, 1], [0, 0, 1, 1], C=values, **kw)
    elif plot == "scatter":
        mappable = ax.scatter([0, 1, 2, 3], [0, 1, 2, 3], c=values, **kw)
    else:
        mappable = getattr(ax, plot)(np.reshape(values, (2, 2)), **kw)
    normalized = np.ma.asarray(mappable.norm(samples))
    return {"vmin": mappable.norm.vmin, "vmax": mappable.norm.vmax,
            "normalized": [None if masked else float(value)
                           for value, masked in zip(normalized.filled(0),
                                                    np.ma.getmaskarray(normalized))]}


def test_hexbin_string_norm():
    values = [1.5, 2.5, 4.0, 6.0]
    samples = [-1, 2.5, 3, 4]
    for plot in ["hexbin", "imshow", "pcolormesh", "scatter"]:
        probe("Axes.norm_by_name", _norm_by_name, tol=1e-12, plot=plot, values=values,
              norm="log", vmin=2, vmax=5, samples=samples)
        probe("Axes.norm_by_name", _norm_by_name, tol=1e-12, plot=plot, values=values,
              norm="log", samples=samples)
    for norm in ["linear", "symlog", "asinh"]:
        probe("Axes.norm_by_name", _norm_by_name, tol=1e-12, plot="imshow", values=values,
              norm=norm, vmin=2, samples=samples)
    probe("Axes.norm_by_name", _norm_by_name, tol=1e-12, plot="imshow",
          values=[0.1, 0.3, 0.6, 0.9], norm="logit", vmax=0.8, samples=[0.05, 0.2, 0.5, 1.5])


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


# === the second half of test_axes.py (tests defined from line 5585 on) ================

# --- lines, reference lines and plot formats -----------------------------------------

def _line_props(line):
    return {"color": _rgba(line.get_color()), "marker": _marker(line),
            "linestyle": _linestyle(line)}


def _plot(y, x=None, fmt=None, **kw):
    ax = _axes()
    args = ([] if x is None else [x]) + [y] + ([] if fmt is None else [fmt])
    return [_line_props(line) for line in ax.plot(*args, **kw)]


def test_plot_format_errors():
    for fmt in ["f", "o+", ":-", "rk", ":o-r", "C", ".C"]:
        assert isinstance(probe("Axes.plot", _plot, y=["string"], fmt=fmt), ValueError)


def test_plot_format():
    for fmt in ["1.0", "1", "k3", ".C12:"]:
        probe("Axes.plot", _plot, y=[1, 2, 3], fmt=fmt)


def test_plot_errors():
    assert isinstance(probe("Axes.plot", _plot, x=[1, 2, 3], y=[1]), ValueError)


def test_empty_line_plots():
    probe("Axes.plot", _plot, x=[], y=[])


def test_none_kwargs():
    probe("Axes.plot", _plot, y=list(range(32)), linestyle=None)


def test_color_None():
    probe("Axes.plot", _plot, x=[1, 2], y=[1, 2], color=None)


def _axline(xy1, xy2=None, slope=None, xscale="linear", yscale="linear"):
    ax = _axes()
    ax.set_xscale(xscale)
    ax.set_yscale(yscale)
    ax.axline(tuple(xy1), None if xy2 is None else tuple(xy2), slope=slope)
    ax.figure.canvas.draw()
    return True


def test_axline_args():
    for scales in [dict(xscale="log"), dict(yscale="log")]:
        assert isinstance(probe("Axes.axline", _axline, xy1=[0, 0], slope=1, **scales),
                          TypeError)
    probe("Axes.axline", _axline, xy1=[0, 0], slope=1)
    assert isinstance(probe("Axes.axline", _axline, xy1=[0, 0], xy2=[0, 0]), ValueError)


def _segments(orientation):
    def run():
        ax = _axes()
        lines = getattr(ax, orientation)(0.5, 0, 1)
        return _rgba_list(lines.get_color())

    return run


def test_vlines_default():
    probe("Axes.vlines", _segments("vlines"), rc={"lines.color": "red"})


def test_hlines_default():
    probe("Axes.hlines", _segments("hlines"), rc={"lines.color": "red"})


def _fill_between_cycle():
    ax = _axes()
    colors = []

    def record(cc):
        colors.append([_rgba_list(cc.get_facecolors()), _rgba_list(cc.get_edgecolors())])

    for _ in range(3):
        record(ax.fill_between(range(3), range(3)))
    for _ in range(3):
        record(ax.fill_betweenx(range(3), range(3)))
    for key in ["facecolor", "color", "edgecolor", "edgecolor"]:
        record(ax.fill_between(range(3), range(3), **{key: "k"}))
    return colors


def test_fillbetween_cycle():
    probe("Axes.fill_between.cycle", _fill_between_cycle, style=["default"])


def _stackplot(**kw):
    ax = _axes()
    x = np.linspace(0, 10, 10)
    layers = ax.stackplot(x, 1.0 * x, 2.0 * x + 1, **kw)
    next_line, = ax.plot(x, x)
    return {"faces": [_rgba_list(layer.get_facecolor()) for layer in layers],
            "next": _rgba(next_line.get_color())}


def test_stackplot_facecolor():
    probe("Axes.stackplot", _stackplot, facecolor=["r", "b"], colors=["c", "m"])
    probe("Axes.stackplot", _stackplot, facecolors=["r", "b"], colors=["c", "m"])
    probe("Axes.stackplot", _stackplot, facecolor=["r"])
    probe("Axes.stackplot", _stackplot, colors=["c", "m"])
    probe("Axes.stackplot", _stackplot)


def _bar_color_cycle():
    ax = _axes()
    colors = []
    for _ in range(5):
        ln, = ax.plot(range(3))
        brs = ax.bar(range(3), range(3))
        colors.append([_rgba(ln.get_color()), [_rgba(br.get_facecolor()) for br in brs]])
    return colors


def test_bar_color_cycle():
    probe("Axes.bar.color_cycle", _bar_color_cycle)


# --- event plots (second half) -------------------------------------------------------

def _eventplot_segments(positions, **kw):
    ax = _axes()
    if "colors" in kw:
        kw["colors"] = _colors_arg(kw["colors"])
    collections = ax.eventplot(positions, **kw)
    ax.figure.canvas.draw()
    return [[np.asarray(s, float) for s in c.get_segments()] for c in collections]


def test_empty_eventplot():
    probe("Axes.eventplot.segments", _eventplot_segments, positions=[[]],
          colors=[[0.0, 0.0, 0.0, 0.0]])


def test_eventplot_orientation():
    for data in [[[]], [[], [0, 1]], [[0, 1], []]]:
        for orientation in [None, "vertical", "horizontal"]:
            kw = {} if orientation is None else {"orientation": orientation}
            probe("Axes.eventplot.segments", _eventplot_segments, positions=data, **kw)


def _eventplot_legend(positions, label):
    ax = _axes()
    ax.eventplot(positions, label=label)
    return [t.get_text() for t in ax.legend().get_texts()]


def test_eventplot_legend():
    probe("Axes.eventplot.legend", _eventplot_legend, positions=[1.0], label="Label")


def test_eventplot_errors():
    for kw in [dict(lineoffsets=[]), dict(linelengths=[]), dict(linewidths=[]),
               dict(linestyles=[]), dict(alpha=[]), dict(lineoffsets=[1, 2]),
               dict(linelengths=[1, 2]), dict(linewidths=[1, 2]),
               dict(linestyles=["solid", "dashed"]), dict(alpha=[1, 2]),
               dict(colors=["r", "g"])]:
        assert isinstance(probe("Axes.eventplot", _eventplot, positions=[[1]], **kw),
                          ValueError)


# --- pies ----------------------------------------------------------------------------

def _text_desc(text):
    return {"text": text.get_text(), "ha": text.get_ha(), "va": text.get_va(),
            "rotation": text.get_rotation(),
            "rotation_mode": text.get_rotation_mode() or "default",
            "size": text.get_size(), "color": _rgba(text.get_color())}


def _pie(x, **kw):
    ax = _axes()
    pie = ax.pie(x, **kw)
    texts = list(pie[1:])
    labels = texts[0] if "labels" in kw else []
    return {"wedges": [[w.theta1, w.theta2] for w in pie.wedges],
            "texts": [_text_desc(t) for t in labels],
            "pct": [_text_desc(t) for t in (texts[1] if len(texts) > 1 else [])]}


def test_pie_textprops():
    probe("Axes.pie", _pie, x=[23, 34, 45], labels=["Long name 1", "Long name 2", "Long name 3"],
          autopct="%.2f", textprops=dict(horizontalalignment="center", verticalalignment="top",
                                         rotation=90, rotation_mode="anchor", size=12,
                                         color="red"))


def test_pie_get_negative_values():
    assert isinstance(probe("Axes.pie", _pie, x=[5, 5, -3], explode=[0, .1, .2]), ValueError)


def test_pie_invalid_explode():
    assert isinstance(probe("Axes.pie", _pie, x=[1, 2, 3], explode=[0.1, 0.1]), ValueError)


def test_pie_invalid_labels():
    assert isinstance(probe("Axes.pie", _pie, x=[1, 2, 3], labels=["One", "Two"]), ValueError)


def test_pie_invalid_radius():
    assert isinstance(probe("Axes.pie", _pie, x=[1, 2, 3], radius=-5), ValueError)


def test_normalize_kwarg_pie():
    for normalize in [True, False]:
        probe("Axes.pie", _pie, x=[0.3, 0.3, 0.1], normalize=normalize)


def test_pie_non_finite_values():
    assert isinstance(probe("Axes.pie", _pie, x=[5, np.nan, np.inf], labels=["A", "B", "C"]),
                      ValueError)


def test_pie_all_zeros():
    assert isinstance(probe("Axes.pie", _pie, x=[0, 0], labels=["A", "B"]), ValueError)


def _pie_label(x, labels, startangle=0, **kw):
    ax = _axes()
    pie = ax.pie(x, startangle=startangle)
    return [_text_desc(t) for t in ax.pie_label(pie, labels, **kw)]


def test_pie_label_formatter():
    for labels in ["{absval:03d}", "{frac:.1%}"]:
        probe("Axes.pie_label", _pie_label, x=[2, 3], labels=labels)


def test_pie_label_auto_align():
    for distance in [0.6, 1.1]:
        for rotate in [False, True]:
            probe("Axes.pie_label", _pie_label, x=[1, 1], startangle=45,
                  labels=["spam", "eggs"], distance=distance, rotate=rotate, alignment="auto")


def test_pie_label_fail():
    assert isinstance(probe("Axes.pie_label", _pie_label, x=[15, 30, 45, 10],
                            labels=["Frogs", "Hogs"]), ValueError)


# --- bar labels ----------------------------------------------------------------------

def _bar_label(x, values, orientation="vertical", invert=(), yerr=None, **kw):
    ax = _axes()
    for axis in invert:
        getattr(ax, f"invert_{axis}axis")()
    extra = {} if yerr is None else {"yerr": yerr}
    bars = (ax.bar(x, values, **extra) if orientation == "vertical"
            else ax.barh(x, values, **extra))
    labels = ax.bar_label(bars, **kw)
    return {"labels": [{"text": t.get_text(), "xy": t.xy, "xyann": t.xyann, "ha": t.get_ha(),
                        "va": t.get_va()} for t in labels],
            "ylim": ax.get_ylim()}


BAR_LABEL_X, BAR_LABEL_VALUES = [1, 2], [3, -4]


def test_bar_label_location_vertical():
    probe("Axes.bar_label", _bar_label, x=BAR_LABEL_X, values=BAR_LABEL_VALUES)


def test_bar_label_location_vertical_yinverted():
    probe("Axes.bar_label", _bar_label, x=BAR_LABEL_X, values=BAR_LABEL_VALUES, invert=["y"])


def test_bar_label_location_horizontal():
    probe("Axes.bar_label", _bar_label, x=BAR_LABEL_X, values=BAR_LABEL_VALUES,
          orientation="horizontal")


def test_bar_label_location_horizontal_yinverted():
    probe("Axes.bar_label", _bar_label, x=BAR_LABEL_X, values=BAR_LABEL_VALUES,
          orientation="horizontal", invert=["y"])


def test_bar_label_location_horizontal_xinverted():
    probe("Axes.bar_label", _bar_label, x=BAR_LABEL_X, values=BAR_LABEL_VALUES,
          orientation="horizontal", invert=["x"])


def test_bar_label_location_horizontal_xyinverted():
    probe("Axes.bar_label", _bar_label, x=BAR_LABEL_X, values=BAR_LABEL_VALUES,
          orientation="horizontal", invert=["x", "y"])


def test_bar_label_location_center():
    probe("Axes.bar_label", _bar_label, x=BAR_LABEL_X, values=BAR_LABEL_VALUES,
          orientation="horizontal", label_type="center")


def test_bar_label_location_errorbars():
    probe("Axes.bar_label", _bar_label, x=BAR_LABEL_X, values=BAR_LABEL_VALUES, yerr=1)


def test_bar_label_fmt():
    for fmt in ["%.2f", "{:.2f}"]:
        probe("Axes.bar_label", _bar_label, x=BAR_LABEL_X, values=BAR_LABEL_VALUES, fmt=fmt)


def test_bar_label_labels():
    probe("Axes.bar_label", _bar_label, x=BAR_LABEL_X, values=BAR_LABEL_VALUES,
          labels=["A", "B"])


def test_bar_label_nan_ydata():
    probe("Axes.bar_label", _bar_label, x=[2, 3], values=[np.nan, 1])


def test_bar_label_padding():
    for padding in [5, [2, 8]]:
        probe("Axes.bar_label", _bar_label, x=BAR_LABEL_X, values=[3, 4], padding=padding)
    assert isinstance(probe("Axes.bar_label", _bar_label, x=BAR_LABEL_X, values=[3, 4],
                            padding=[1, 2, 3]), ValueError)


def test_nan_barlabels():
    for values, yerr in [([np.nan, 1, 2], [0.2, 0.4, 0.6]), ([0, 1, 2], [0.2, np.nan, 0.6]),
                         ([np.nan, 1, 2], [np.nan, np.nan, 0.6])]:
        probe("Axes.bar_label", _bar_label, x=[1, 2, 3], values=values, yerr=yerr)


def test_int_fmt_bar_label():
    probe("Axes.bar_label", _bar_label, x=["foo", "bar"], values=[5, 7], fmt="{:d}")


# --- bars, broken bars and histograms (second half) ----------------------------------

def test_bar_color_precedence():
    for kw in [{}, dict(color="red"), dict(facecolor="yellow"),
               dict(color="red", facecolor="green")]:
        probe("Axes.bar", _bar, x=[1, 2, 3], height=[4, 5, 6], **kw)


def test_bar_shape_mismatch():
    assert isinstance(probe("Axes.bar", _bar, x=["foo", "bar"], height=[1, 2, 3]), ValueError)


def _barh(y, width, **kw):
    ax = _axes()
    bars = ax.barh(y, width, **kw)
    return [[r.get_x(), r.get_y(), r.get_width(), r.get_height()] for r in bars]


def test_bar_broadcast_args():
    probe("Axes.bar", _bar, x=[0, 1, 2, 3], height=1)
    probe("Axes.bar", _bar, x=[0, 1], height=[0, 1], edgecolor=[.1, .2, .3, .4])
    probe("Axes.barh", _barh, y=0, width=1, left=[0, 1, 2, 3], height=1)


def test_bar_leading_nan():
    x = [np.nan, 1.0, 2.0]
    probe("Axes.bar", _bar, x=x, height=[0.5, 1.5, 2.0], bottom=[0.77] * 3)
    probe("Axes.barh", _barh, y=x, width=[0.5, 1.5, 2.0], left=[0.77] * 3)


def _bar_errorbar_zorder(x, height, yerr, capsize, zorder):
    ax = _axes()
    bars = ax.bar(x=x, height=height, yerr=yerr, capsize=capsize, zorder=zorder)
    _, caplines, barlinecols = bars.errorbar.lines
    return {"bars": [b.zorder for b in bars.patches], "caps": [c.zorder for c in caplines],
            "barlines": [c.zorder for c in barlinecols]}


def test_bar_errbar_zorder():
    probe("Axes.bar.errorbar", _bar_errorbar_zorder, x=[1, 2, 3], height=[1, 2, 3],
          yerr=[1, 2, 3], capsize=5, zorder=3)


def _broken_barh(xranges, yrange, **kw):
    ax = _axes()
    pc = ax.broken_barh([tuple(r) for r in xranges], tuple(yrange), **kw)
    return {"extents": [p.get_extents().get_points() for p in pc.get_paths()],
            "xlim": ax.get_xlim(), "ylim": ax.get_ylim()}


def test_broken_barh_empty():
    probe("Axes.broken_barh.paths", _broken_barh, xranges=[], yrange=[.1, .5])


def test_broken_barh_timedelta():
    probe("Axes.broken_barh", _broken_barh_dates, start="2018-11-09T00:00:00", hours=1,
          yrange=[1, 2])


def test_broken_barh_align():
    for yrange, align in [([0, 2], None), ([10, 2], "center"), ([20, 2], "top")]:
        kw = {} if align is None else {"align": align}
        probe("Axes.broken_barh.paths", _broken_barh, xranges=[[0, 10]], yrange=yrange, **kw)


def test_broken_barh_categorical():
    for align in [None, "center", "top"]:
        kw = {} if align is None else {"align": align}
        probe("Axes.broken_barh.paths", _broken_barh, xranges=[[0, 10]], yrange=["a", 0.8],
              **kw)


def test_length_one_hist():
    probe("Axes.hist", _hist, x=[1])


def _hist_bins(x, **kw):
    if "range" in kw:
        kw["range"] = tuple(kw["range"])
    n, bins, _ = _axes().hist(x, **kw)
    return {"n": np.asarray(n, float), "bins": bins}


def test_hist_auto_bins():
    probe("Axes.hist.bins", _hist_bins, x=[[1, 2, 3], [3, 4, 5, 6]], bins="auto")


def test_hist_nan_data():
    probe("Axes.hist.bins", _hist_bins, x=[1, 2, 3])
    probe("Axes.hist.bins", _hist_bins, x=[1, 2, 3, np.nan])


def test_hist_range_and_density():
    probe("Axes.hist.bins", _hist_bins, x=rng(8974).uniform(10).tolist(), bins="auto",
          range=[0, 1], density=True)


def test_unicode_hist_label():
    a = (b'\xe5\xbe\x88\xe6\xbc\x82\xe4\xba\xae, ' +
         b'r\xc3\xb6m\xc3\xa4n ch\xc3\xa4r\xc3\xa1ct\xc3\xa8rs').decode("utf-8")
    b = b'\xd7\xa9\xd7\x9c\xd7\x95\xd7\x9d'.decode("utf-8")
    probe("Axes.get_legend_handles_labels", _hist_labels, x=[list(range(15))] * 3,
          label=[a, "hi aardvark", b])


def test_pathological_hexbin():
    probe("Axes.hexbin", _hexbin, x=[10] * 100, y=[10] * 100, gridsize=4)


def test_violin_point_mass():
    probe("Axes.violinplot", _violinplot, data=[[0, 0]])


def test_violinplot_empty_dataset():
    probe("Axes.violinplot", _violinplot, data=[rng(10452).normal(100).tolist(), [],
                                                [np.nan, np.nan]])


def test_boxplot_tick_labels():
    data = rng(19680801).uniform(30).reshape(10, 3).T.tolist()
    probe("Axes.boxplot", _boxplot, data=data, tick_labels=["A", "B", "C"])


# --- error bars (second half) --------------------------------------------------------

def test_errorbar_inputs_shotgun():
    x, y = list(range(5)), [1.0] * 5
    errors = [1, [1] * 5, [[1] * 5, [1] * 5], None]
    for xerr in errors:
        for yerr in errors:
            probe("ErrorbarContainer.remove", _errorbar_remove, x=x, y=y, xerr=xerr, yerr=yerr)
    for xerr in [[], None]:
        for yerr in [[], None]:
            probe("ErrorbarContainer.remove", _errorbar_remove, x=[], y=[], xerr=xerr,
                  yerr=yerr)


def _errorbar_caps(x, y, yerr, **kw):
    ax = _axes()
    line, caps, bars = ax.errorbar(x, y, yerr=yerr, **kw)
    return {"line": line is not None,
            "caps": [[_rgba(c.get_markeredgecolor()), c.get_markersize(),
                      c.get_markeredgewidth()] for c in caps],
            "barlines": [b.get_linewidths() for b in bars]}


ERRORBAR_X = np.linspace(0, 10, 10)


def test_caps_color():
    probe("Axes.errorbar.caps", _errorbar_caps, x=ERRORBAR_X.tolist(),
          y=np.sin(ERRORBAR_X).tolist(), yerr=0.1, ecolor="red",
          rc={"lines.markeredgecolor": "green"})


def test_caps_no_ecolor():
    probe("Axes.errorbar.caps", _errorbar_caps, x=ERRORBAR_X.tolist(),
          y=np.sin(ERRORBAR_X).tolist(), yerr=0.1, rc={"lines.markeredgecolor": "green"})


def test_errorbar_uses_rcparams():
    probe("Axes.errorbar.caps", _errorbar_caps, x=[0, 1, 2], y=[1, 2, 3], yerr=[0.1, 0.2, 0.3],
          fmt="none", rc={"errorbar.capsize": 5.0, "errorbar.capthick": 2.5,
                          "errorbar.elinewidth": 1.75})


def _empty_errorbar_legend():
    ax = _axes()
    ax.errorbar([], [], xerr=[], label="empty y")
    ax.errorbar([], [], yerr=[], label="empty x")
    return [t.get_text() for t in ax.legend().get_texts()]


def test_empty_errorbar_legend():
    probe("Axes.legend.errorbar", _empty_errorbar_legend)


# --- scatter (second half) -----------------------------------------------------------

def test_color_length_mismatch():
    x = list(range(5))
    assert isinstance(probe("Axes.scatter", _scatter, x=x, y=x, c=list(range(6))), ValueError)
    probe("Axes.scatter", _scatter, x=x, y=x, c=[[0.5, 0.5, 0.5]] * 5)


def test_scatter_empty_data():
    probe("Axes.scatter", _scatter, x=[], y=[])
    probe("Axes.scatter", _scatter, x=[], y=[], s=[], c=[])


def test_scatter_color_repr_error():
    assert isinstance(probe("Axes.scatter", _scatter, x=[0, 1], y=[0, 1], c="red\n"),
                      ValueError)


# --- ticks, tick parameters and gridlines --------------------------------------------

def _tick_state(tick):
    """What a tick shows and how: its marks, label and gridline."""
    return {
        "size": tick._size, "width": tick._width, "pad": tick._base_pad,
        "direction": tick._tickdir, "color": _rgba(tick.tick1line.get_color()),
        "markeredgecolor": _rgba(tick.tick1line.get_markeredgecolor()),
        "labelsize": tick.label1.get_size(), "labelcolor": _rgba(tick.label1.get_color()),
        "labelrotation": tick.label1.get_rotation(),
        "rotation_mode": tick.label1.get_rotation_mode() or "default",
        "labelfamily": tick.label1.get_fontfamily()[0],
        "tick1On": tick.tick1line.get_visible(), "tick2On": tick.tick2line.get_visible(),
        "label1On": tick.label1.get_visible(), "label2On": tick.label2.get_visible(),
        "gridOn": tick.gridline.get_visible(),
        "grid": [_rgba(tick.gridline.get_color()), tick.gridline.get_linewidth(),
                 tick.gridline.get_alpha(), _linestyle(tick.gridline)],
    }


def _ticks_state(ax):
    return {f"{name}{level}": _tick_state(getattr(getattr(ax, f"{name}axis"), f"{level}Ticks")[0])
            for name in "xy" for level in ["major", "minor"]}


def _tick_params(calls=(), draw=False):
    ax = _axes()
    for params in calls:
        ax.tick_params(**params)
    if draw:
        ax.figure.canvas.draw()
    return _ticks_state(ax)


def test_text_labelsize():
    probe("Axes.tick_params", _tick_params, calls=[dict(labelsize="large"),
                                                   dict(direction="out")])


def test_axis_set_tick_params_labelsize_labelcolor():
    probe("Axes.tick_params", _tick_params,
          calls=[dict(axis="y", labelsize=30, labelcolor="red", direction="out")])


def test_axes_tick_params_gridlines():
    probe("Axes.tick_params", _tick_params,
          calls=[dict(grid_color="b", grid_linewidth=5, grid_alpha=0.5,
                      grid_linestyle="dashdot")])


def test_axes_tick_params_ylabelside():
    probe("Axes.tick_params", _tick_params,
          calls=[dict(labelleft=False, labelright=True, which=which)
                 for which in ["major", "minor"]])


def test_axes_tick_params_xlabelside():
    probe("Axes.tick_params", _tick_params,
          calls=[dict(labeltop=True, labelbottom=False, which=which)
                 for which in ["major", "minor"]])


def test_tick_param_label_rotation():
    probe("Axes.tick_params", _tick_params,
          calls=[dict(axis="x", which="both", labelrotation=75),
                 dict(axis="y", which="both", labelrotation=90)])
    probe("Axes.tick_params", _tick_params,
          calls=[dict(axis="x", labelrotation=53, labelrotation_mode="xtick"),
                 dict(axis="y", labelrotation=35, labelrotation_mode="ytick")])


def test_tick_param_labelfont():
    probe("Axes.tick_params", _tick_params, calls=[dict(color="r",
                                                        labelfontfamily="monospace")])


def test_xtickcolor_is_not_markercolor():
    probe("Axes.tick_params", _tick_params, rc={"lines.markeredgecolor": "white"})


def test_xtickcolor_is_not_xticklabelcolor():
    probe("Axes.tick_params", _tick_params, rc={"xtick.color": "yellow",
                                                "xtick.labelcolor": "blue",
                                                "ytick.color": "yellow",
                                                "ytick.labelcolor": "blue"})


def test_relative_ticklabel_sizes():
    sizes = [size for size in mpl.font_manager.font_scalings if size is not None]
    for size in sizes + [8, 10, 12]:
        probe("Axes.tick_params", _tick_params, style=["default"], draw=True,
              rc={"xtick.labelsize": size, "ytick.labelsize": size})


def test_grid_rcparams():
    """``test_axis.py::test_grid_rcparams``: ``grid.major.*`` and
    ``grid.minor.*`` take precedence over ``grid.*``."""
    probe("Axes.tick_params", _tick_params, draw=True,
          rc={"axes.grid": True, "axes.grid.which": "both", "ytick.minor.visible": True,
              "xtick.minor.visible": True, "grid.color": "gray", "grid.minor.color": "red",
              "grid.major.linestyle": ":", "grid.major.linewidth": 2, "grid.minor.alpha": 0.6})


def test_rcparam_grid_minor():
    for which in ["both", "major", "minor"]:
        probe("Axes.tick_params", _tick_params, draw=True,
              rc={"axes.grid": True, "axes.grid.which": which})


def test_context_ticks():
    def run():
        with mpl.rc_context({"xtick.direction": "in", "xtick.major.size": 30,
                             "xtick.major.width": 5, "xtick.color": "C0",
                             "xtick.major.pad": 12, "xtick.bottom": True, "xtick.top": True,
                             "xtick.labelsize": 14, "xtick.labelcolor": "C1"}):
            ax = _axes()
        ax.figure.draw_without_rendering()
        ticks = ax.xaxis.majorTicks
        same = all([t._size, t._width, t._base_pad, t._labelrotation, t._zorder, t._tickdir]
                   == [ticks[0]._size, ticks[0]._width, ticks[0]._base_pad,
                       ticks[0]._labelrotation, ticks[0]._zorder, ticks[0]._tickdir]
                   for t in ticks)
        return {"tick": _tick_state(ticks[0]), "same": same}

    probe("Axes.tick_params.rc_context", run, style=["mpl20"])


def _grid_color(color):
    ax = _axes()
    ax.grid(True, color=tuple(color))
    return [[t.gridline.get_alpha(), _rgba(t.gridline.get_color())]
            for t in [ax.xaxis.get_major_ticks()[0], ax.yaxis.get_major_ticks()[0]]]


def test_grid_color_with_alpha():
    probe("Axes.grid", _grid_color, color=[0.5, 0.6, 0.7, 0.3])


def _reset_grid():
    ax = _axes()
    ax.tick_params(reset=True, which="major", labelsize=10)
    visible = [ax.xaxis.majorTicks[0].gridline.get_visible()]
    with mpl.rc_context({"axes.grid": True}):
        ax.clear()
        ax.tick_params(reset=True, which="major", labelsize=10)
        visible.append(ax.xaxis.majorTicks[0].gridline.get_visible())
    return visible


def test_reset_grid():
    probe("Axes.tick_params.reset", _reset_grid)


def _offset_color(axis, data, params=None):
    ax = _axes()
    ax.plot(data)
    target = getattr(ax, f"{axis}axis")
    if params is not None:
        target.set_tick_params(**params)
    text = target.get_offset_text()
    return {"color": _rgba(text.get_color()), "visible": text.get_visible()}


OFFSET_DATA = [1.01e9, 1.02e9, 1.03e9]


def test_offset_label_color():
    probe("Axis.get_offset_text", _offset_color, axis="y", data=OFFSET_DATA,
          params={"labelcolor": "red"})


def test_offset_text_visible():
    probe("Axis.get_offset_text", _offset_color, axis="y", data=OFFSET_DATA,
          params={"labelleft": False, "labelright": True})
    probe("Axis.get_offset_text", _offset_color, axis="y", data=OFFSET_DATA,
          params={"labelright": False})


def test_xaxis_offsetText_color():
    probe("Axis.get_offset_text", _offset_color, axis="x", data=OFFSET_DATA,
          rc={"xtick.labelcolor": "blue"})
    probe("Axis.get_offset_text", _offset_color, axis="x", data=OFFSET_DATA,
          rc={"xtick.color": "yellow", "xtick.labelcolor": "inherit"})


def test_yaxis_offsetText_color():
    probe("Axis.get_offset_text", _offset_color, axis="y", data=OFFSET_DATA,
          rc={"ytick.labelcolor": "green"})
    probe("Axis.get_offset_text", _offset_color, axis="y", data=OFFSET_DATA,
          rc={"ytick.color": "red", "ytick.labelcolor": "inherit"})


def _set_ticklabels(ticks, labels, bar=False):
    ax = _axes()
    if bar:
        ax.bar([1, 2], [1, 2])
    ax.set_xticks(ticks)
    ax.set_xticklabels(labels)
    ax.figure.canvas.draw()
    return [t.get_text() for t in ax.get_xticklabels()]


def test_mismatched_ticklabels():
    assert isinstance(probe("Axis.set_ticklabels", _set_ticklabels, ticks=[1.5, 2.5],
                            labels=["a", "b", "c"]), ValueError)


def test_empty_ticks_fixed_loc():
    probe("Axis.set_ticklabels", _set_ticklabels, ticks=[1, 2], labels=[], bar=True)


def test_warn_too_few_labels():
    assert isinstance(probe("Axis.set_ticklabels", _set_ticklabels, ticks=[0, 0.5, 1],
                            labels=["0", "0.1"]), ValueError)


def _get_tick_params():
    axis = _axes().yaxis

    def params():
        return {which: {key: (_rgba(value) if key == "labelcolor" else value)
                        for key, value in axis.get_tick_params(which=which).items()}
                for which in ["major", "minor"]}

    before = params()
    axis.set_tick_params(labelsize=30, labelcolor="red", direction="out", which="both")
    return {"before": before, "after": params()}


def test_axis_get_tick_params():
    probe("Axis.get_tick_params", _get_tick_params)


def _label_formatting():
    ax = _axes()
    label = ax.xaxis.label
    return [_rgba(label.get_color()), label.get_fontsize(), label.get_fontweight()]


def test_rc_axes_label_formatting():
    probe("Axis.label", _label_formatting, style=["default"],
          rc={"axes.labelcolor": "red", "axes.labelsize": 20, "axes.labelweight": "bold"})


def _ylabel_ha(ha):
    fig = Figure()
    FigureCanvasAgg(fig)
    ax = fig.subplots()
    ax.set_ylabel("test", y=1, ha=ha)
    ax.yaxis.set_label_position("right")
    ha = ax.yaxis.label.get_ha()
    fig.draw_without_rendering()
    return {"ha": ha, "extent": ax.yaxis.label.get_window_extent().extents.tolist()}


def test_ylabel_ha_with_position():
    for ha in ["left", "center", "right"]:
        assert probe("Axis.label.get_ha", _ylabel_ha, ha=ha, tol=1e-9)["ha"] == ha


# --- titles --------------------------------------------------------------------------

def _title_y(title):
    return title.get_position()[1]


def _titles(case):
    if case == "location_shared":
        fig = _figure()
        axs = fig.subplots(2, 1, sharex=True)
        for ax, title in zip(axs, "AB"):
            ax.set_title(title, pad=-40)
        fig.draw_without_rendering()
        return [_title_y(ax.title) for ax in axs]
    if case == "location_unshared":
        fig = _figure()
        axs = fig.subplots(2, 1, sharex=False)
        for ax, title in zip(axs, "AB"):
            ax.set_title(title, pad=-40)
        fig.draw_without_rendering()
        return [_title_y(ax.title) for ax in axs]
    if case == "setpos":
        fig = _figure()
        ax = fig.add_subplot()
        fig.subplots_adjust(top=0.8)
        ax2 = ax.twiny()
        ax.set_xlabel("Xlabel")
        ax2.set_xlabel("Xlabel2")
        ax.set_title("Title")
        ax.title.set_position((0.5, 1.11))
        fig.canvas.draw()
        return list(ax.title.get_position())
    if case == "xticks_top":
        ax = _axes()
        ax.xaxis.set_ticks_position("top")
        ax.set_title("xlabel top")
        ax.figure.canvas.draw()
        return _title_y(ax.title)
    if case == "xticks_top_both":
        ax = _axes()
        ax.tick_params(axis="x", bottom=True, top=True, labelbottom=True, labeltop=True)
        ax.set_title("xlabel top")
        ax.figure.canvas.draw()
        return _title_y(ax.title)
    if case.startswith("above_offset"):
        left, center = {"above_offset_left": ("left", ""),
                        "above_offset_center": ("", "center"),
                        "above_offset_both": ("left", "center")}[case]
        ax = _axes()
        ax.set_ylim(1e11)
        ax.set_title(left, loc="left")
        ax.set_title(center)
        ax.figure.draw_without_rendering()
        return [_title_y(ax._left_title), _title_y(ax.title)]
    if case == "above_hidden_offset":
        fig = _figure()
        axs = fig.subplots(1, 2, sharey=True)
        fig.set_layout_engine("tight")
        for i, ax in enumerate(axs):
            ax.set_title(f"Subplot {i}")
            ax.plot(range(10), [1e53] * 10)
        fig.draw_without_rendering()
        return [bool(np.isfinite(ax.title.get_window_extent().ymin)
                     and np.all(np.isfinite(ax.get_position().bounds))) for ax in axs]
    if case == "no_move_off_page":
        fig = _figure()
        ax = fig.add_axes((0.1, -0.5, 0.8, 0.2))
        ax.tick_params(axis="x", bottom=True, top=True, labelbottom=True, labeltop=True)
        title = ax.set_title("Boo")
        fig.canvas.draw()
        return _title_y(title)
    if case == "inset_ax":
        ax = _axes()
        ax.set_title("Title")
        ax.figure.draw_without_rendering()
        before = _title_y(ax.title)
        ax.inset_axes([0, 1, 1, 0.1])
        ax.figure.draw_without_rendering()
        return [before, _title_y(ax.title)]
    raise ValueError(case)


AUTO_TITLE = {"axes.titley": None}


def test_title_location_shared():
    for case in ["location_shared", "location_unshared"]:
        probe("Axes.title.get_position", _titles, case=case)


def test_titlesetpos():
    probe("Axes.title.get_position", _titles, case="setpos")


def test_title_xticks_top():
    probe("Axes.title.get_position", _titles, case="xticks_top", rc=AUTO_TITLE)


def test_title_xticks_top_both():
    probe("Axes.title.get_position", _titles, case="xticks_top_both", rc=AUTO_TITLE)


def test_title_above_offset():
    for case in ["above_offset_left", "above_offset_center", "above_offset_both"]:
        probe("Axes.title.get_position", _titles, case=case, rc=AUTO_TITLE)


def test_title_above_hidden_offset():
    probe("Axes.title.get_position", _titles, case="above_hidden_offset", rc=AUTO_TITLE)


def test_title_no_move_off_page():
    probe("Axes.title.get_position", _titles, case="no_move_off_page", rc=AUTO_TITLE)


def test_title_inset_ax():
    probe("Axes.title.get_position", _titles, case="inset_ax", rc=AUTO_TITLE)


def _titletwiny():
    fig = Figure(dpi=72)
    FigureCanvasAgg(fig)
    ax = fig.add_subplot()
    ax2 = ax.twiny()
    xlabel2 = ax2.set_xlabel("Xlabel2")
    title = ax.set_title("Title")
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    return title.get_window_extent(renderer).y0 - xlabel2.get_window_extent(renderer).y1


def test_titletwiny():
    probe("Axes.set_title.twiny", _titletwiny, style=["mpl20"], tol=1e-9)


# --- secondary axes ------------------------------------------------------------------

def _invert(x):
    with np.errstate(divide="ignore"):
        return 1 / x


def _secondary(orientation, location):
    ax = _axes()
    ax.plot(np.arange(2, 11), np.arange(2, 11))
    getattr(ax, f"secondary_{orientation}axis")(location)
    return True


def test_secondary_fail():
    assert isinstance(probe("Axes.secondary_axis", _secondary, orientation="x",
                            location="right"), ValueError)
    assert isinstance(probe("Axes.secondary_axis", _secondary, orientation="y",
                            location="bottom"), ValueError)


def _secondary_resize():
    fig = Figure(figsize=(10, 5))
    FigureCanvasAgg(fig)
    ax = fig.add_subplot()
    ax.plot(np.arange(2, 11), np.arange(2, 11))
    ax.secondary_xaxis("top", functions=(_invert, _invert))
    fig.canvas.draw()
    fig.set_size_inches((7, 4))
    return ax.get_position().extents


def test_secondary_resize():
    probe("Axes.secondary_xaxis.resize", _secondary_resize)


def _kind(obj):
    return type(obj).__name__


def _secondary_minorloc():
    fig = Figure(figsize=(10, 5))
    FigureCanvasAgg(fig)
    ax = fig.add_subplot()
    ax.plot(np.arange(2, 11), np.arange(2, 11))
    secax = ax.secondary_xaxis("top", functions=(_invert, _invert))
    kinds = [_kind(secax.xaxis.get_minor_locator())]
    secax.minorticks_on()
    kinds.append(_kind(secax.xaxis.get_minor_locator()))
    ax.set_xscale("log")
    fig.canvas.draw()
    kinds.append(_kind(secax.xaxis.get_minor_locator()))
    ax.set_xscale("linear")
    fig.canvas.draw()
    kinds.append(_kind(secax.xaxis.get_minor_locator()))
    return kinds


def test_secondary_minorloc():
    probe("Axes.secondary_xaxis.minor_locator", _secondary_minorloc)


def _secondary_formatter():
    ax = _axes()
    ax.set_xscale("log")
    secax = ax.secondary_xaxis("top")
    secax.xaxis.set_major_formatter(mticker.ScalarFormatter())
    ax.figure.canvas.draw()
    return _kind(secax.xaxis.get_major_formatter())


def test_secondary_formatter():
    probe("Axes.secondary_xaxis.formatter", _secondary_formatter)


def _secondary_color(orientation, how):
    ax = _axes()
    make = getattr(ax, f"secondary_{orientation}axis")
    if how == "keyword":
        secax = make(1, color="red")
    else:
        secax = make(1)
        secax.set_color("red")
    axis = getattr(secax, f"{orientation}axis")
    params = axis.get_tick_params()
    return {"spines": {side: _rgba(spine.get_edgecolor()) for side, spine in secax.spines.items()},
            "color": _rgba(params["color"]), "labelcolor": _rgba(params["labelcolor"]),
            "label": _rgba(axis.label.get_color()),
            "minor": _rgba(axis.get_minor_ticks(1)[0].tick1line.get_color())}


def test_set_secondary_axis_color():
    for orientation in ["x", "y"]:
        for how in ["keyword", "method"]:
            probe("Axes.secondary_axis.set_color", _secondary_color, orientation=orientation,
                  how=how)


def _secondary_ticks(orientation):
    ax = _axes()
    secax = getattr(ax, f"secondary_{orientation}axis")(1)
    getattr(secax, f"set_{orientation}ticks")([0, 1])
    return _kind(getattr(secax, f"{orientation}axis").get_major_locator())


def test_secondary_init_xticks():
    for orientation in "xy":
        probe("Axes.secondary_axis.set_ticks", _secondary_ticks, orientation=orientation)


# --- axes contents, limits and other calls -------------------------------------------

def _subplot_geometry(nrows, ncols, index):
    plt.close("all")
    try:
        ax = plt.subplot(nrows, ncols, index)
        ax.twinx()
        return ax.get_subplotspec().get_geometry()
    finally:
        plt.close("all")


def test_subplot_key_hash():
    probe("pyplot.subplot", _subplot_geometry, nrows=5, ncols=1, index=1)


def _specgram_extent(x, scale):
    _, _, _, im = _axes().specgram(np.asarray(x, float), Fs=None, scale=scale)
    return im.get_extent()


def test_specgram_fs_none():
    probe("Axes.specgram", _specgram_extent, x=[1.0] * 300, scale="linear")


def _spectral_inverted(kind):
    ax = _axes()
    ax.yaxis.set(inverted=True)
    with np.errstate(divide="ignore"):
        if kind == "psd":
            ax.psd(np.zeros(5))
        else:
            ax.csd(np.zeros(5), np.zeros(5))
    return [ax.yaxis_inverted(), len(ax.lines[0].get_xdata())]


def test_psd_csd_edge_cases():
    for kind in ["psd", "csd"]:
        probe("Axes.psd", _spectral_inverted, kind=kind)


def _pcolorfast_kind(C, x=None, y=None):
    ax = _axes()
    args = [] if x is None else [np.asarray(x, float), np.asarray(y, float)]
    return _kind(ax.pcolorfast(*args, np.asarray(C, float)))


PCOLORFAST_COORDS = [{}, dict(x=[3, 7], y=[2, 6]), dict(x=list(range(5)), y=list(range(4))),
                     dict(x=[1, 2, 4, 8, 16], y=[0, 1, 2, 3])]


def test_pcolorfast():
    gen = rng(7386)
    data = [np.arange(12).reshape((3, 4)).tolist(), gen.uniform(36).reshape(3, 4, 3).tolist()]
    coords = PCOLORFAST_COORDS + [dict(x=gen.uniform(20).reshape(4, 5).tolist(),
                                       y=gen.uniform(20).reshape(4, 5).tolist())]
    for C in data:
        for xy in coords:
            probe("Axes.pcolorfast", _pcolorfast_kind, C=C, **xy)


def test_pcolorfast_bad_dims():
    assert isinstance(probe("Axes.pcolorfast", _pcolorfast_kind, C=np.zeros((8, 8)).tolist(),
                            x=np.zeros(6).tolist(), y=np.zeros((4, 7)).tolist()), TypeError)


def test_pcolorfast_regular_xy_incompatible_size():
    C = rng(7398).uniform(200).reshape(10, 20).tolist()
    for x, y in [(list(range(5)), list(range(11))), (list(range(21)), list(range(5)))]:
        assert isinstance(probe("Axes.pcolorfast", _pcolorfast_kind, C=C, x=x, y=y),
                          ValueError)


def _clim(method):
    ax = _axes()
    data = [[0, 1], [2, 3]]
    if method == "scatter":
        mappable = ax.scatter(range(3), range(3), c=range(3), clim=(7, 8))
    else:
        mappable = getattr(ax, method)(data, clim=(7, 8))
    return [mappable.norm.vmin, mappable.norm.vmax]


def test_clim():
    for method in ["scatter", "imshow", "pcolor", "pcolormesh", "pcolorfast"]:
        probe("Axes.clim", _clim, method=method)


def _invisible_axes_events():
    fig = _figure()
    ax = fig.add_subplot()
    before = fig.canvas.inaxes((200, 200)) is not None
    ax.set_visible(False)
    return [before, fig.canvas.inaxes((200, 200)) is not None]


def test_invisible_axes_events():
    probe("FigureCanvasBase.inaxes", _invisible_axes_events)


def _wedge_bounds():
    ax = _axes()
    ax.add_patch(mpatches.Wedge((0, -1), 1.05, 60, 120, width=0.1))
    return ax.dataLim.bounds


def test_patch_bounds():
    probe("Axes.add_patch", _wedge_bounds)


def _artist_sublists():
    ax = _axes()

    def counts():
        return [len(ax.collections), len(ax.images), len(ax.lines), len(ax.patches),
                len(ax.tables), len(ax.texts)]

    lines = [ax.plot(np.arange(i, i + 5))[0] for i in range(6)]
    col = ax.scatter(np.arange(5), np.arange(5))
    im = ax.imshow(np.zeros((5, 5)))
    patch = ax.add_patch(mpatches.Rectangle((0, 0), 5, 5))
    text = ax.text(0, 0, "foo")
    recorded = [counts()]
    for artist in [col, im, patch, text, *lines]:
        artist.remove()
        recorded.append(counts())
    return recorded


def test_artist_sublists():
    probe("Axes.artist_sublists", _artist_sublists)


SMALL_AUTOSCALE_VERTS = [
    [-5.45, 0.00], [-5.45, 0.00], [-5.29, 0.00], [-5.29, 0.00], [-5.13, 0.00], [-5.13, 0.00],
    [-4.97, 0.00], [-4.97, 0.00], [-4.81, 0.00], [-4.81, 0.00], [-4.65, 0.00], [-4.65, 0.00],
    [-4.49, 0.00], [-4.49, 0.00], [-4.33, 0.00], [-4.33, 0.00], [-4.17, 0.00], [-4.17, 0.00],
    [-4.01, 0.00], [-4.01, 0.00], [-3.85, 0.00], [-3.85, 0.00], [-3.69, 0.00], [-3.69, 0.00],
    [-3.53, 0.00], [-3.53, 0.00], [-3.37, 0.00], [-3.37, 0.00], [-3.21, 0.00], [-3.21, 0.01],
    [-3.05, 0.01], [-3.05, 0.01], [-2.89, 0.01], [-2.89, 0.01], [-2.73, 0.01], [-2.73, 0.02],
    [-2.57, 0.02], [-2.57, 0.04], [-2.41, 0.04], [-2.41, 0.04], [-2.25, 0.04], [-2.25, 0.06],
    [-2.09, 0.06], [-2.09, 0.08], [-1.93, 0.08], [-1.93, 0.10], [-1.77, 0.10], [-1.77, 0.12],
    [-1.61, 0.12], [-1.61, 0.14], [-1.45, 0.14], [-1.45, 0.17], [-1.30, 0.17], [-1.30, 0.19],
    [-1.14, 0.19], [-1.14, 0.22], [-0.98, 0.22], [-0.98, 0.25], [-0.82, 0.25], [-0.82, 0.27],
    [-0.66, 0.27], [-0.66, 0.29], [-0.50, 0.29], [-0.50, 0.30], [-0.34, 0.30], [-0.34, 0.32],
    [-0.18, 0.32], [-0.18, 0.33], [-0.02, 0.33], [-0.02, 0.32], [0.13, 0.32], [0.13, 0.33],
    [0.29, 0.33], [0.29, 0.31], [0.45, 0.31], [0.45, 0.30], [0.61, 0.30], [0.61, 0.28],
    [0.77, 0.28], [0.77, 0.25], [0.93, 0.25], [0.93, 0.22], [1.09, 0.22], [1.09, 0.19],
    [1.25, 0.19], [1.25, 0.17], [1.41, 0.17], [1.41, 0.15], [1.57, 0.15], [1.57, 0.12],
    [1.73, 0.12], [1.73, 0.10], [1.89, 0.10], [1.89, 0.08], [2.05, 0.08], [2.05, 0.07],
    [2.21, 0.07], [2.21, 0.05], [2.37, 0.05], [2.37, 0.04], [2.53, 0.04], [2.53, 0.02],
    [2.69, 0.02], [2.69, 0.02], [2.85, 0.02], [2.85, 0.01], [3.01, 0.01], [3.01, 0.01],
    [3.17, 0.01], [3.17, 0.00], [3.33, 0.00], [3.33, 0.00], [3.49, 0.00], [3.49, 0.00],
    [3.65, 0.00], [3.65, 0.00], [3.81, 0.00], [3.81, 0.00], [3.97, 0.00], [3.97, 0.00],
    [4.13, 0.00], [4.13, 0.00], [4.29, 0.00], [4.29, 0.00], [4.45, 0.00], [4.45, 0.00],
    [4.61, 0.00], [4.61, 0.00], [4.77, 0.00], [4.77, 0.00], [4.93, 0.00], [4.93, 0.00],
]


def _path_autoscale(vertices):
    ax = _axes()
    ax.add_patch(mpatches.PathPatch(mpath.Path(vertices)))
    ax.autoscale()
    return {"xlim": ax.get_xlim(), "ylim": ax.get_ylim()}


def test_small_autoscale():
    probe("Axes.autoscale", _path_autoscale, vertices=SMALL_AUTOSCALE_VERTS)


def _ecdf(x):
    return _axes().ecdf(x).get_xydata()


def test_ecdf_invalid():
    assert isinstance(probe("Axes.ecdf", _ecdf, x=[1, np.nan]), ValueError)


def _large_offset():
    ax = _axes()
    ax.plot((1 + np.array([0, 1.e-12])) * 1.e27)
    ax.figure.canvas.draw()
    return {"ylim": ax.get_ylim(), "offset": ax.yaxis.get_offset_text().get_text()}


def test_large_offset():
    probe("Axes.plot.large_offset", _large_offset)


VECTOR_DATES = [f"2017-07-15T18:{minute:02d}:00" for minute in range(0, 60, 10)]


def _vectors_at_dates(kind):
    ax = _axes()
    dates = [_dt(day) for day in VECTOR_DATES]
    y = np.linspace(0, 5, len(dates))
    u = v = np.linspace(0, 50, len(dates))
    getattr(ax, kind)(dates, y, u, v)
    return {"xlim": ax.get_xlim(), "datalim": ax.dataLim.get_points()}


def test_barb_units():
    probe("Axes.barbs.dates", _vectors_at_dates, kind="barbs")


def test_quiver_units():
    probe("Axes.barbs.dates", _vectors_at_dates, kind="quiver")


def _pdf_rasterized():
    fig = _figure()
    ax = fig.add_subplot()
    ax.set_rasterization_zorder(5)
    ax.plot(range(5), rasterized=True, zorder=1)
    with io.BytesIO() as buffer:
        fig.savefig(buffer, format="pdf")
        return buffer.getvalue().startswith(b"%PDF")


def test_zorder_and_explicit_rasterization():
    probe("Figure.savefig.pdf", _pdf_rasterized)


def _indicate_inset():
    ax = _axes()
    ax.set_xlim(1, 5)
    ax.set_ylim(1, 5)
    ax.set_aspect(1.)
    ax.apply_aspect()
    inset = ax.indicate_inset(bounds=[1.5, 2, 2.15 - 1.5, 2.5 - 2])
    ax.figure.canvas.draw()
    return {"connectors": inset.connectors is None,
            "extent": inset.rectangle.get_window_extent().get_points()}


def test_inset():
    probe("Axes.indicate_inset", _indicate_inset)


def _box_aspect_init():
    fig = _figure()
    axs = fig.subplots(2, 3, subplot_kw=dict(box_aspect=1))
    fig.set_layout_engine("constrained")
    fig.canvas.draw()
    renderer = fig.canvas.get_renderer()
    sizes = []
    for ax in axs.flat:
        bb = ax.get_window_extent(renderer)
        sizes.append([bb.width, bb.height])
    return sizes


def test_bbox_aspect_axes_init():
    probe("Axes.set_box_aspect.constrained", _box_aspect_init, tol=1e-9)


# --- extents of axes decorations -----------------------------------------------------

def _points(bbox):
    return None if bbox is None else bbox.get_points()


def _color_boxes(fig, ax):
    """The extents ``color_boxes`` of the upstream tests measures."""
    fig.canvas.draw()
    return {"axis": [_points(ax.xaxis.get_tightbbox()), _points(ax.yaxis.get_tightbbox())],
            "spines": [_points(ax.spines[side].get_window_extent())
                       for side in ["bottom", "top", "left", "right"]],
            "ax": _points(ax.get_window_extent()), "tight": _points(ax.get_tightbbox())}


def _decorations(case):
    with mpl.rc_context({"_internal.classic_mode": False}):
        fig = Figure(dpi=200, figsize=(6, 6))
        FigureCanvasAgg(fig)
        ax = fig.add_subplot()
        if case in ("nodecorator", "minor_out", "minor_long"):
            fig.canvas.draw()
        if case in ("nodecorator", "displaced_spine"):
            ax.set(xticklabels=[], yticklabels=[])
        if case == "displaced_spine":
            ax.spines.bottom.set_position(("axes", -0.1))
        if case.startswith("tickdirs_"):
            ax.tick_params(direction=case.removeprefix("tickdirs_"))
        if case.startswith("minor_"):
            ax.tick_params(which="both", direction="out")
        if case == "minor_long":
            ax.minorticks_on()
            ax.tick_params(axis="both", which="minor", length=30)
        return _color_boxes(fig, ax)


def test_normal_axes():
    probe("Axes.get_tightbbox.decorations", _decorations, case="normal", tol=1e-9)


def test_nodecorator():
    probe("Axes.get_tightbbox.decorations", _decorations, case="nodecorator", tol=1e-9)


def test_displaced_spine():
    probe("Axes.get_tightbbox.decorations", _decorations, case="displaced_spine", tol=1e-9)


def test_tickdirs():
    for direction in ["in", "out", "inout"]:
        probe("Axes.get_tightbbox.decorations", _decorations, case=f"tickdirs_{direction}",
              tol=1e-9)


def test_minor_accountedfor():
    for case in ["minor_out", "minor_long"]:
        probe("Axes.get_tightbbox.decorations", _decorations, case=case, tol=1e-9)


def _tightbbox_axis_off():
    ax = _axes()
    before = ax.get_tightbbox()
    ax.axis("off")
    return [before.get_points(), ax.get_tightbbox().get_points()]


def test_tick_padding_tightbbox():
    probe("Axes.get_tightbbox.axis_off", _tightbbox_axis_off, tol=1e-9,
          rc={"xtick.direction": "out", "ytick.direction": "out"})


def _tightbbox_long_label():
    ax = _axes()
    before = ax.get_tightbbox(for_layout_only=False).width
    ax.set_xlabel("l" + "o" * 70 + "ng")
    return [before, ax.get_tightbbox(for_layout_only=False).width]


def test_tightbbox_includes_long_label():
    probe("Axes.get_tightbbox.long_label", _tightbbox_long_label, tol=1e-9)


def _tightbbox_nan_text():
    ax = _axes()
    remove_ticks_and_titles(ax.figure)
    ax.text(np.nan, 1, "Boo")
    return ax.get_tightbbox().width


def test_gettightbbox_ignore_nan():
    probe("Axes.get_tightbbox.nan_text", _tightbbox_nan_text, tol=1e-9)


def _relim_autolim_false():
    ax = _axes()
    ax.plot([0, 1], [0, 1])
    ax.relim()
    before = [ax.dataLim.get_points(), ax.dataLim.minpos]
    points = mcollections.PathCollection([])
    points.set_offsets([[100, 200], [300, 400]])
    ax.add_collection(points, autolim=False)
    ax.relim()
    return {"before": before, "after": [ax.dataLim.get_points(), ax.dataLim.minpos]}


def test_relim_collection_autolim_false():
    probe("Axes.add_collection.autolim", _relim_autolim_false)


def _child_axes_removal():
    ax = _axes()
    marginal = ax.inset_axes([1, 0, .1, 1], sharey=ax)
    marginal.twinx()
    marginal.remove()
    ax.set(xlim=(-1, 1), ylim=(10, 20))
    return {"xlim": ax.get_xlim(), "ylim": ax.get_ylim()}


def test_child_axes_removal():
    probe("Axes.remove.child_axes", _child_axes_removal)


def _inset_projection(projection):
    ax = _axes()
    inset = ax.inset_axes([0.2, 0.2, 0.3, 0.3], projection=projection)
    return [inset.name, inset in ax.child_axes]


def test_inset_projection():
    probe("Axes.inset_axes.projection", _inset_projection, projection="hammer")


def _redraw_in_frame():
    ax = _axes()
    ax.plot([1, 2, 3])
    ax.figure.canvas.draw()
    ax.redraw_in_frame()
    return True


def test_redraw_in_frame():
    probe("Axes.redraw_in_frame", _redraw_in_frame)


def _set_xlim(left, right):
    ax = _axes()
    ax.set_xlim(left, right)
    return list(ax.get_xlim())


def test_set_xlim_non_finite():
    for left, right in [(2, 5), (float("nan"), 1), (1, float("inf"))]:
        probe("Axes.set_xlim", _set_xlim, left=left, right=right)


def _add_axes(rect):
    return list(_figure().add_axes(rect).get_position().bounds)


def test_add_axes_invalid_rect():
    for rect in [[0.25, 0.25, 0.5, 0.5], [float("nan"), 0, 1, 1], [0, 0, -1, 1]]:
        probe("Figure.add_axes", _add_axes, rect=rect)
