"""
Statistical plots: `~.Axes.boxplot` and `~.Axes.bxp`,
`~.Axes.violinplot` and `~.Axes.violin`, and `~.Axes.pie`.

The statistics are computed in Python as matplotlib computes them
(`.cbook.boxplot_stats`, `.cbook.violin_stats`); pltrs (Rust) draws them.
"""

from collections.abc import Mapping
import datetime
import math
from numbers import Real

import numpy as np

from ... import _api, _tick_helpers, _values, cbook, collections as mcoll, colors as mcolors
from ... import lines as mlines, patches as mpatches, ticker as mticker
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ...container import PieContainer
from ... import _lazy_imports
from .. import _failed_calls
from ._helpers import check_format, keep_data, takes_properties, wrap, wrap_all

_boxplot_reference = _lazy_imports.module(globals(), "._boxplot_reference")
_pie_reference = _lazy_imports.module(globals(), "._pie_reference")


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from ... import rcParams
    return rcParams


def _native_lineprops(props):
    """
    Line properties of a box plot part in native form. A property given as
    None is `.Line2D`'s default for it, ``lines.<name>``, as matplotlib's
    parts are lines made with what is given over the ``boxplot.*`` ones;
    a colour given as None leaves the part its ``boxplot.*`` one, which
    `_cycled_colors` replaces by the property cycle's.
    """
    rc = _rcParams()
    props = {key: rc[f"lines.{key}"] if value is None else value
             for key, value in cbook.normalize_kwargs(
                 {} if props is None else props, mlines.Line2D).items()
             if value is not None or (key != "color" and f"lines.{key}" in rc)}
    return mlines._native_kwargs(props)


def _cycled_colors(parts):
    """The names of the parts among *parts* (``{name: props}``) whose
    colour is given as None: matplotlib plots their lines with
    ``Axes.plot``, which gives them the colours of the property cycle."""
    cycled = []
    for name, props in parts.items():
        normalized = cbook.normalize_kwargs(props or {}, mlines.Line2D)
        if "color" in normalized and normalized["color"] is None:
            cycled.append(name)
    return cycled


def _check_stats_keys(stats, shownotches, showmeans, showfliers):
    """Raise matplotlib's `KeyError` for the first statistic of *stats*
    that `~.Axes.bxp` reads (in its order, after the label) and that is
    missing."""
    stats.get('label')  # matplotlib's first read: the box's label
    needed = ["q1", "whislo", "q3", "whishi", "med"]
    needed += ["cilo", "cihi"] if shownotches else []
    needed += ["mean"] if showmeans else []
    needed += ["fliers"] if showfliers else []
    for key in needed:
        if key not in stats:
            raise KeyError(key)


def _converted_stats(axes, axis_name, stats, keys):
    """The statistics *stats* (a dict) with the values of *keys* in the
    numbers of the axis *axis_name* of *axes*, which takes their units (as
    matplotlib's `~.Axes.bxp` and `~.Axes.violin` plot them)."""
    converted = dict(stats)
    for key in keys:
        if key in converted:
            converted[key], = axes._process_unit_info([(axis_name, converted[key])])
    return converted


#: The patch properties pltrs (Rust) takes as it makes a patch.
_NATIVE_PATCH_PROPS = frozenset([
    "color", "facecolor", "edgecolor", "linewidth", "linestyle", "fill",
    "hatch", "hatchcolor", "antialiased", "capstyle", "joinstyle", "alpha",
    "zorder", "label", "visible", "clip_on"])


def _keep_at_positions(lines, positions, orientation):
    """Have each of the lines of `~.Axes.bxp`, made at one of *positions*
    (as given), keep its data as matplotlib plots them: the positions
    along the axis of the boxes' positions, the values as made."""
    for line, position in zip(lines, positions):
        if orientation == "vertical":
            ys = line.get_ydata()
            keep_data(line, np.asarray([position] * len(ys)), ys)
        else:
            xs = line.get_xdata()
            keep_data(line, xs, np.asarray([position] * len(xs)))


def _native_patch_props(props, cls):
    """
    The properties *props* of patches of class *cls* (aliases resolved)
    split into those pltrs (Rust) takes as it makes them, in native form,
    and the others, which the patches are set afterwards.
    """
    props = cbook.normalize_kwargs({} if props is None else props, cls)
    native = {k: v for k, v in props.items() if k in _NATIVE_PATCH_PROPS}
    for key in ("facecolor", "edgecolor", "color"):
        # "none" stays itself: no alpha makes it visible.
        if native.get(key) is not None and not cbook._str_lower_equal(native[key], "none"):
            native[key] = mcolors.to_rgba(native[key])
    if isinstance(native.get("linestyle"), tuple):
        offset, seq = native["linestyle"]
        native["linestyle"] = (offset, tuple(seq))
    return native, {k: v for k, v in props.items() if k not in native}


def _colors_or_empty(color):
    """*color* (one or a sequence) as RGBA tuples, empty for None."""
    if color is None:
        return []
    return [tuple(c) for c in mcolors.to_rgba_array(color)]



def _wrap_pie_shadows(axes, wedges, ox, oy):
    """Make the shadows pltrs (Rust) added after each of the pie's *wedges*
    `.Shadow` objects of them, offset by (*ox*, *oy*) points, as
    matplotlib's ``pie`` adds them."""
    from ..._native import artist as _artist
    natives = _plotting.get_patches(axes._native)
    order = {_artist.id(native): index for index, native in enumerate(natives)}
    for wedge in wedges:
        shadow = wrap(axes, natives[order[_artist.id(wedge._native)] + 1],
                      mpatches.Shadow)
        shadow._shadowing(wedge, ox, oy)


def _pie_geometry(wedges, fracs, explode, radius, startangle, counterclock, center):
    """Keep the geometry of the pie's *wedges* as matplotlib computes and
    types it (its loop over the fractions, in numpy and Python numbers)."""
    theta1 = startangle / 360
    for wedge, frac, expl in zip(wedges, fracs, explode):
        x_pos, y_pos = center
        theta2 = (theta1 + frac) if counterclock else (theta1 - frac)
        thetam = 2 * np.pi * 0.5 * (theta1 + theta2)
        x_pos += expl * math.cos(thetam)
        y_pos += expl * math.sin(thetam)
        _values.remember_all(wedge, {
            "center": (x_pos, y_pos), "r": radius,
            "theta1": 360. * min(theta1, theta2),
            "theta2": 360. * max(theta1, theta2)}, mpatches.Wedge._FIELDS)
        theta1 = theta2


def _pie_plain(colors, pctdistance, labeldistance, startangle, radius, wedgeprops,
               textprops, center, hatch, explode):
    """Whether pltrs (Rust) makes a pie of these arguments as matplotlib's
    code does: colours to cycle (some), distances, angles and the radius
    that are numbers, properties that are mappings, a centre of two
    numbers, hatches that are text and explosions that are numbers."""
    def numbers(values):
        return np.iterable(values) and all(isinstance(v, Real) for v in values)
    return ((colors is None or (np.iterable(colors) and not isinstance(colors, str)
                                and len(colors) > 0))
            and isinstance(pctdistance, Real)
            and (labeldistance is None or isinstance(labeldistance, Real))
            and isinstance(startangle, Real) and math.isfinite(startangle)
            and isinstance(radius, Real) and math.isfinite(radius)
            and all(props is None or isinstance(props, Mapping)
                    for props in (wedgeprops, textprops))
            and numbers(center) and len(center) == 2
            and (hatch is None or all(isinstance(h, str) for h in np.atleast_1d(hatch)))
            and (explode is None or numbers(explode)))

def _pie_props_taken(wedgeprops, textprops):
    """Whether a wedge and a text take *wedgeprops* and *textprops* (see
    `.takes_properties`)."""
    from ...text import Text
    return ((not wedgeprops or takes_properties(mpatches.Wedge((0, 0), 1, 0, 90), wedgeprops))
            and (not textprops or takes_properties(Text(0, 0, ''), textprops)))


class StatsMixin:
    """`boxplot`, `bxp`, `violinplot`, `violin` and `pie`."""

    @_api.make_keyword_only("3.10", "notch")
    @_preprocess_data()
    def boxplot(self, x, notch=None, sym=None, vert=None,
                orientation='vertical', whis=None, positions=None,
                widths=None, patch_artist=None, bootstrap=None,
                usermedians=None, conf_intervals=None,
                meanline=None, showmeans=None, showcaps=None,
                showbox=None, showfliers=None, boxprops=None,
                tick_labels=None, flierprops=None, medianprops=None,
                meanprops=None, capprops=None, whiskerprops=None,
                manage_ticks=True, autorange=False, zorder=None,
                capwidths=None, label=None):
        """
        Draw a box and whisker plot.

        The box extends from the first quartile (Q1) to the third
        quartile (Q3) of the data, with a line at the median.
        The whiskers extend from the box to the farthest data point
        lying within 1.5x the inter-quartile range (IQR) from the box.
        Flier points are those past the end of the whiskers.
        See https://en.wikipedia.org/wiki/Box_plot for reference.

        .. code-block:: none

                  Q1-1.5IQR   Q1   median  Q3   Q3+1.5IQR
                               |-----:-----|
               o      |--------|     :     |--------|    o  o
                               |-----:-----|
             flier             <----------->            fliers
                                    IQR


        Parameters
        ----------
        x : 1D array or sequence of 1D arrays or 2D array
            The input data. Possible values:

            - 1D array: A single box is drawn.
            - sequence of 1D arrays: A box is drawn for each array in the sequence.
            - 2D array: A box is drawn for each column in the array.

        notch : bool, default: :rc:`boxplot.notch`
            Whether to draw a notched boxplot (`True`), or a rectangular
            boxplot (`False`).  The notches represent the confidence interval
            (CI) around the median.  The documentation for *bootstrap*
            describes how the locations of the notches are computed by
            default, but their locations may also be overridden by setting the
            *conf_intervals* parameter.

            .. note::

                In cases where the values of the CI are less than the
                lower quartile or greater than the upper quartile, the
                notches will extend beyond the box, giving it a
                distinctive "flipped" appearance. This is expected
                behavior and consistent with other statistical
                visualization packages.

        sym : str, optional
            The default symbol for flier points.  An empty string ('') hides
            the fliers.  If `None`, then the fliers default to 'b+'.  More
            control is provided by the *flierprops* parameter.

        vert : bool, optional
            .. deprecated:: 3.11
                Use *orientation* instead.

                If this is given during the deprecation period, it overrides
                the *orientation* parameter.

            If True, plots the boxes vertically.
            If False, plots the boxes horizontally.

        orientation : {'vertical', 'horizontal'}, default: 'vertical'
            If 'horizontal', plots the boxes horizontally.
            Otherwise, plots the boxes vertically.

            .. versionadded:: 3.10

        whis : float or (float, float), default: 1.5
            The position of the whiskers.

            If a float, the lower whisker is at the lowest datum above
            ``Q1 - whis*(Q3-Q1)``, and the upper whisker at the highest datum
            below ``Q3 + whis*(Q3-Q1)``, where Q1 and Q3 are the first and
            third quartiles.  The default value of ``whis = 1.5`` corresponds
            to Tukey's original definition of boxplots.

            If a pair of floats, they indicate the percentiles at which to
            draw the whiskers (e.g., (5, 95)).  In particular, setting this to
            (0, 100) results in whiskers covering the whole range of the data.

            In the edge case where ``Q1 == Q3``, *whis* is automatically set
            to (0, 100) (cover the whole range of the data) if *autorange* is
            True.

            Beyond the whiskers, data are considered outliers and are plotted
            as individual points.

        bootstrap : int, optional
            Specifies whether to bootstrap the confidence intervals
            around the median for notched boxplots. If *bootstrap* is
            None, no bootstrapping is performed, and notches are
            calculated using a Gaussian-based asymptotic approximation
            (see McGill, R., Tukey, J.W., and Larsen, W.A., 1978, and
            Kendall and Stuart, 1967). Otherwise, bootstrap specifies
            the number of times to bootstrap the median to determine its
            95% confidence intervals. Values between 1000 and 10000 are
            recommended.

        usermedians : 1D array-like, optional
            A 1D array-like of length ``len(x)``.  Each entry that is not
            `None` forces the value of the median for the corresponding
            dataset.  For entries that are `None`, the medians are computed
            by Matplotlib as normal.

        conf_intervals : array-like, optional
            A 2D array-like of shape ``(len(x), 2)``.  Each entry that is not
            None forces the location of the corresponding notch (which is
            only drawn if *notch* is `True`).  For entries that are `None`,
            the notches are computed by the method specified by the other
            parameters (e.g., *bootstrap*).

        positions : array-like, optional
            The positions of the boxes. The ticks and limits are
            automatically set to match the positions. Defaults to
            ``range(1, N+1)`` where N is the number of boxes to be drawn.

        widths : float or array-like
            The widths of the boxes.  The default is 0.5, or ``0.15*(distance
            between extreme positions)``, if that is smaller.

        patch_artist : bool, default: :rc:`boxplot.patchartist`
            If `False` produces boxes with the Line2D artist. Otherwise,
            boxes are drawn with Patch artists.

        tick_labels : list of str, optional
            The tick labels of each boxplot.
            Ticks are always placed at the box *positions*. If *tick_labels* is given,
            the ticks are labelled accordingly. Otherwise, they keep their numeric
            values.

            .. versionchanged:: 3.9
                Renamed from *labels*, which is also removed in 3.11.

        manage_ticks : bool, default: True
            If True, the tick locations and labels will be adjusted to match
            the boxplot positions.

        autorange : bool, default: False
            When `True` and the data are distributed such that the 25th and
            75th percentiles are equal, *whis* is set to (0, 100) such
            that the whisker ends are at the minimum and maximum of the data.

        meanline : bool, default: :rc:`boxplot.meanline`
            If `True` (and *showmeans* is `True`), will try to render the
            mean as a line spanning the full width of the box according to
            *meanprops* (see below).  Not recommended if *shownotches* is also
            True.  Otherwise, means will be shown as points.

        zorder : float, default: ``Line2D.zorder = 2``
            The zorder of the boxplot.

        Returns
        -------
        dict
          A dictionary mapping each component of the boxplot to a list
          of the `.Line2D` instances created. That dictionary has the
          following keys (assuming vertical boxplots):

          - ``boxes``: the main body of the boxplot showing the
            quartiles and the median's confidence intervals if
            enabled.

          - ``medians``: horizontal lines at the median of each box.

          - ``whiskers``: the vertical lines extending to the most
            extreme, non-outlier data points.

          - ``caps``: the horizontal lines at the ends of the
            whiskers.

          - ``fliers``: points representing data that extend beyond
            the whiskers (fliers).

          - ``means``: points or lines representing the means.

        Other Parameters
        ----------------
        showcaps : bool, default: :rc:`boxplot.showcaps`
            Show the caps on the ends of whiskers.
        showbox : bool, default: :rc:`boxplot.showbox`
            Show the central box.
        showfliers : bool, default: :rc:`boxplot.showfliers`
            Show the outliers beyond the caps.
        showmeans : bool, default: :rc:`boxplot.showmeans`
            Show the arithmetic means.
        capprops : dict, default: None
            The style of the caps.
        capwidths : float or array, default: None
            The widths of the caps.
        boxprops : dict, default: None
            The style of the box.
        whiskerprops : dict, default: None
            The style of the whiskers.
        flierprops : dict, default: None
            The style of the fliers.
        medianprops : dict, default: None
            The style of the median.
        meanprops : dict, default: None
            The style of the mean.
        label : str or list of str, optional
            Legend labels. Use a single string when all boxes have the same style and
            you only want a single legend entry for them. Use a list of strings to
            label all boxes individually. To be distinguishable, the boxes should be
            styled individually, which is currently only possible by modifying the
            returned artists, see e.g. :doc:`/gallery/statistics/boxplot_demo`.

            In the case of a single string, the legend entry will technically be
            associated with the first box only. By default, the legend will show the
            median line (``result["medians"]``); if *patch_artist* is True, the legend
            will show the box `.Patch` artists (``result["boxes"]``) instead.

            .. versionadded:: 3.9

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        See Also
        --------
        .Axes.bxp : Draw a boxplot from pre-computed statistics.
        violinplot : Draw an estimate of the probability density function.
        """
        rc = _rcParams()
        whis = rc['boxplot.whiskers'] if whis is None else whis
        bootstrap = rc['boxplot.bootstrap'] if bootstrap is None else bootstrap
        if not _boxplot_reference.is_plain(x, whis, bootstrap):
            _boxplot_reference.boxplot_stats(x, whis=whis, bootstrap=bootstrap,
                                             labels=tick_labels, autorange=autorange)
        bxpstats = cbook.boxplot_stats(x, whis=whis, bootstrap=bootstrap,
                                       labels=tick_labels, autorange=autorange)
        notch = rc['boxplot.notch'] if notch is None else notch
        patch_artist = (rc['boxplot.patchartist'] if patch_artist is None
                        else patch_artist)
        meanline = rc['boxplot.meanline'] if meanline is None else meanline
        showmeans = rc['boxplot.showmeans'] if showmeans is None else showmeans
        showcaps = rc['boxplot.showcaps'] if showcaps is None else showcaps
        showbox = rc['boxplot.showbox'] if showbox is None else showbox
        showfliers = rc['boxplot.showfliers'] if showfliers is None else showfliers
        boxprops = {} if boxprops is None else boxprops
        whiskerprops = {} if whiskerprops is None else whiskerprops
        capprops = {} if capprops is None else capprops
        medianprops = {} if medianprops is None else medianprops
        meanprops = {} if meanprops is None else meanprops
        flierprops = {} if flierprops is None else flierprops
        if patch_artist:
            boxprops['linestyle'] = 'solid'  # Not consistent with bxp.
            if 'color' in boxprops:
                boxprops['edgecolor'] = boxprops.pop('color')
        if sym is not None:
            if sym == '':
                flierprops = dict(linestyle='none', marker='', color='none')
                showfliers = False
            else:
                check_format(sym)
                _, marker, color = _plotting.format_parts(sym)
                if marker is not None and marker != 'None':
                    flierprops['marker'] = marker
                if color is not None:
                    flierprops['color'] = color
                    flierprops['markerfacecolor'] = color
                    flierprops['markeredgecolor'] = color
        if usermedians is not None:
            if (len(np.ravel(usermedians)) != len(bxpstats) or
                    np.shape(usermedians)[0] != len(bxpstats)):
                raise ValueError(
                    "'usermedians' and 'x' have different lengths")
            for stats, med in zip(bxpstats, usermedians):
                if med is not None:
                    stats['med'] = med
        if conf_intervals is not None:
            if len(conf_intervals) != len(bxpstats):
                raise ValueError(
                    "'conf_intervals' and 'x' have different lengths")
            for stats, ci in zip(bxpstats, conf_intervals):
                if ci is not None:
                    if len(ci) != 2:
                        raise ValueError('each confidence interval must '
                                         'have two values')
                    if ci[0] is not None:
                        stats['cilo'] = ci[0]
                    if ci[1] is not None:
                        stats['cihi'] = ci[1]
        return self.bxp(bxpstats, positions=positions, widths=widths,
                        vert=vert, patch_artist=patch_artist,
                        shownotches=notch, showmeans=showmeans,
                        showcaps=showcaps, showbox=showbox,
                        boxprops=boxprops, flierprops=flierprops,
                        medianprops=medianprops, meanprops=meanprops,
                        meanline=meanline, showfliers=showfliers,
                        capprops=capprops, whiskerprops=whiskerprops,
                        manage_ticks=manage_ticks, zorder=zorder,
                        capwidths=capwidths, label=label,
                        orientation=orientation)

    @_api.make_keyword_only("3.10", "widths")
    def bxp(self, bxpstats, positions=None, widths=None, vert=None,
            orientation='vertical', patch_artist=False, shownotches=False,
            showmeans=False, showcaps=True, showbox=True, showfliers=True,
            boxprops=None, whiskerprops=None, flierprops=None,
            medianprops=None, capprops=None, meanprops=None,
            meanline=False, manage_ticks=True, zorder=None,
            capwidths=None, label=None):
        """
        Draw a box and whisker plot from pre-computed statistics.

        The box extends from the first quartile *q1* to the third
        quartile *q3* of the data, with a line at the median (*med*).
        The whiskers extend from *whislow* to *whishi*.
        Flier points are markers past the end of the whiskers.
        See https://en.wikipedia.org/wiki/Box_plot for reference.

        .. code-block:: none

                   whislow    q1    med    q3    whishi
                               |-----:-----|
               o      |--------|     :     |--------|    o  o
                               |-----:-----|
             flier                                      fliers

        .. note::
            This is a low-level drawing function for when you already
            have the statistical parameters. If you want a boxplot based
            on a dataset, use `~.Axes.boxplot` instead.

        Parameters
        ----------
        bxpstats : list of dicts
            A list of dictionaries containing stats for each boxplot.
            Required keys are:

            - ``med``: Median (float).
            - ``q1``, ``q3``: First & third quartiles (float).
            - ``whislo``, ``whishi``: Lower & upper whisker positions (float).

            Optional keys are:

            - ``mean``: Mean (float).  Needed if ``showmeans=True``.
            - ``fliers``: Data beyond the whiskers (array-like).
              Needed if ``showfliers=True``.
            - ``cilo``, ``cihi``: Lower & upper confidence intervals
              about the median. Needed if ``shownotches=True``.
            - ``label``: Name of the dataset (str).  If available,
              this will be used a tick label for the boxplot

        positions : array-like, default: [1, 2, ..., n]
            The positions of the boxes. The ticks and limits
            are automatically set to match the positions.

        widths : float or array-like, default: None
            The widths of the boxes.  The default is
            ``clip(0.15*(distance between extreme positions), 0.15, 0.5)``.

        capwidths : float or array-like, default: None
            Either a scalar or a vector and sets the width of each cap.
            The default is ``0.5*(width of the box)``, see *widths*.

        vert : bool, optional
            .. deprecated:: 3.11
                Use *orientation* instead.

                If this is given during the deprecation period, it overrides
                the *orientation* parameter.

            If True, plots the boxes vertically.
            If False, plots the boxes horizontally.

        orientation : {'vertical', 'horizontal'}, default: 'vertical'
            If 'horizontal', plots the boxes horizontally.
            Otherwise, plots the boxes vertically.

            .. versionadded:: 3.10

        patch_artist : bool, default: False
            If `False` produces boxes with the `.Line2D` artist.
            If `True` produces boxes with the `~matplotlib.patches.Patch` artist.

        shownotches, showmeans, showcaps, showbox, showfliers : bool
            Whether to draw the CI notches, the mean value (both default to
            False), the caps, the box, and the fliers (all three default to
            True).

        boxprops, whiskerprops, capprops, flierprops, medianprops, meanprops :\
 dict, optional
            Artist properties for the boxes, whiskers, caps, fliers, medians, and
            means.

        meanline : bool, default: False
            If `True` (and *showmeans* is `True`), will try to render the mean
            as a line spanning the full width of the box according to
            *meanprops*. Not recommended if *shownotches* is also True.
            Otherwise, means will be shown as points.

        manage_ticks : bool, default: True
            If True, the tick locations and labels will be adjusted to match the
            boxplot positions.

        label : str or list of str, optional
            Legend labels. Use a single string when all boxes have the same style and
            you only want a single legend entry for them. Use a list of strings to
            label all boxes individually. To be distinguishable, the boxes should be
            styled individually, which is currently only possible by modifying the
            returned artists, see e.g. :doc:`/gallery/statistics/boxplot_demo`.

            In the case of a single string, the legend entry will technically be
            associated with the first box only. By default, the legend will show the
            median line (``result["medians"]``); if *patch_artist* is True, the legend
            will show the box `.Patch` artists (``result["boxes"]``) instead.

            .. versionadded:: 3.9

        zorder : float, default: ``Line2D.zorder = 2``
            The zorder of the resulting boxplot.

        Returns
        -------
        dict
            A dictionary mapping each component of the boxplot to a list
            of the `.Line2D` instances created. That dictionary has the
            following keys (assuming vertical boxplots):

            - ``boxes``: main bodies of the boxplot showing the quartiles, and
              the median's confidence intervals if enabled.
            - ``medians``: horizontal lines at the median of each box.
            - ``whiskers``: vertical lines up to the last non-outlier data.
            - ``caps``: horizontal lines at the ends of the whiskers.
            - ``fliers``: points representing data beyond the whiskers (fliers).
            - ``means``: points or lines representing the means.

        See Also
        --------
        boxplot : Draw a boxplot from data instead of pre-computed statistics.
        """
        if vert is None:
            vert = _rcParams()['boxplot.vertical']
        else:
            _api.warn_deprecated(
                "3.11",
                name="vert: bool",
                alternative="orientation: {'vertical', 'horizontal'}",
            )
        if vert is False:
            orientation = 'horizontal'
        _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)
        if not _rcParams()['boxplot.vertical']:
            _api.warn_deprecated(
                "3.10",
                name='boxplot.vertical', obj_type="rcparam"
            )
        N = len(bxpstats)
        datashape_message = ("List of boxplot statistics and `{0}` "
                             "values must have same the length")
        if positions is None:
            positions = list(range(1, N + 1))
        elif len(positions) != N:
            raise ValueError(datashape_message.format("positions"))
        positions = np.array(positions)
        if len(positions) > 0 and not all(isinstance(p, Real) for p in positions):
            raise TypeError("positions should be an iterable of numbers")
        if widths is None:
            widths = [np.clip(0.15 * np.ptp(positions), 0.15, 0.5)] * N
        elif np.isscalar(widths):
            widths = [widths] * N
        elif len(widths) != N:
            raise ValueError(datashape_message.format("widths"))
        if capwidths is None:
            capwidths = 0.5 * np.array(widths)
        elif np.isscalar(capwidths):
            capwidths = [capwidths] * N
        elif len(capwidths) != N:
            raise ValueError(datashape_message.format("capwidths"))
        # matplotlib labels the first box with a single label, which it
        # makes a string, and each box with a sequence of them.
        if not label:
            label = None
        elif cbook.is_scalar_or_string(label):
            label = [str(label)]
        else:
            label = [str(lbl) for lbl in label]
            if len(label) != N:
                raise ValueError(datashape_message.format("label"))
        for stats in bxpstats:
            _check_stats_keys(stats, shownotches, showmeans, showfliers)
        value_axis = "y" if orientation == "vertical" else "x"
        bxpstats = [_converted_stats(self, value_axis, stats, (
            "med", "q1", "q3", "whislo", "whishi", "mean", "cilo", "cihi",
            "fliers")) for stats in bxpstats]
        # matplotlib unpacks the medians' and the means' properties first
        # (none for false ones), then merges those of the parts in this
        # order over the rcParams and the z-order, failing on the first
        # that is no mapping.
        medianprops, meanprops = ({**(props or {})} for props in (medianprops, meanprops))
        if zorder is not None:
            zorder + 0
        for props, cls in ((boxprops, mpatches.PathPatch if patch_artist else mlines.Line2D),
                           (whiskerprops, mlines.Line2D), (capprops, mlines.Line2D),
                           (flierprops, mlines.Line2D)):
            cbook.normalize_kwargs(props, cls)
        # matplotlib's caps reach half their width to each side.
        for capwidth in capwidths:
            capwidth * 0.5
        # The means' properties only matter where the means are drawn.
        meanprops_given = meanprops if showmeans else {}
        box_rest = {}
        if patch_artist:
            boxkw, box_rest = _native_patch_props(boxprops, mpatches.PathPatch)
        else:
            boxkw = _native_lineprops(boxprops)
        mean_kw = _native_lineprops(meanprops_given)
        tick_axis = self._axis_map["x" if orientation == 'vertical' else "y"]
        previous = tick_axis.get_major_locator() if manage_ticks else None
        whiskers, caps, box_lines, box_patches, medians, fliers, means = _plotting.bxp(
            self._native, list(bxpstats),
            (list(map(float, positions)), list(map(float, widths)),
             list(map(float, capwidths)), positions.dtype.kind in "iu"),
            orientation,
            (bool(shownotches), bool(showmeans), bool(showcaps), bool(showbox),
             bool(showfliers), bool(meanline)),
            (bool(patch_artist), boxkw),
            (_native_lineprops(whiskerprops), _native_lineprops(flierprops),
             _native_lineprops(medianprops), _native_lineprops(capprops), mean_kw),
            (bool(manage_ticks), None if zorder is None else float(zorder), label,
             _cycled_colors({"boxes": None if patch_artist else boxprops,
                             "whiskers": whiskerprops, "fliers": flierprops,
                             "medians": medianprops, "caps": capprops,
                             "means": meanprops_given})))
        if manage_ticks:
            # matplotlib adds the positions to the locations of its fixed
            # locator, as an array of them.
            locator = (previous if isinstance(previous, mticker.FixedLocator)
                       else mticker.FixedLocator([]))
            locator.locs = np.array([*locator.locs, *positions])
            _tick_helpers.adopt(tick_axis, locator, "locator", False)
        boxes = (wrap_all(self, box_patches, mpatches.PathPatch) if patch_artist
                 else wrap_all(self, box_lines, mlines.Line2D))
        for box in boxes if box_rest else ():
            box.set(**box_rest)
        if patch_artist:
            # The face as matplotlib's boxes are made with it, which reads
            # back as given (the colour of the cycle :rc:`patch.facecolor`
            # names, say).
            face = cbook.normalize_kwargs(boxprops, mpatches.PathPatch).get(
                "facecolor", _rcParams()["patch.facecolor"])
            for box in boxes:
                _values.remember(box, "facecolor", face)
        whiskers = wrap_all(self, whiskers, mlines.Line2D)
        means = wrap_all(self, means, mlines.Line2D)
        if positions.dtype.kind in "iu":
            # matplotlib plots the whiskers, and the means drawn as points,
            # at the positions as given: integers stay integers.
            _keep_at_positions(whiskers, np.repeat(positions, 2), orientation)
            if not meanline:
                _keep_at_positions(means, positions, orientation)
        return dict(whiskers=whiskers,
                    caps=wrap_all(self, caps, mlines.Line2D), boxes=boxes,
                    medians=wrap_all(self, medians, mlines.Line2D),
                    fliers=wrap_all(self, fliers, mlines.Line2D),
                    means=means)

    @_api.make_keyword_only("3.10", "vert")
    @_preprocess_data(replace_names=["dataset"])
    def violinplot(self, dataset, positions=None, vert=None,
                   orientation='vertical', widths=0.5, showmeans=False,
                   showextrema=True, showmedians=False, quantiles=None,
                   points=100, bw_method=None, side='both',
                   facecolor=None, linecolor=None):
        """
        Make a violin plot.

        Make a violin plot for each column of *dataset* or each vector in
        sequence *dataset*.  Each filled area extends to represent the
        entire data range, with optional lines at the mean, the median,
        the minimum, the maximum, and user-specified quantiles.

        Parameters
        ----------
        dataset : 1D array or sequence of 1D arrays or 2D array
            The input data. Possible values:

            - 1D array: A single violin is drawn.
            - sequence of 1D arrays: A violin is drawn for each array in the sequence.
            - 2D array: A violin is drawn for each column in the array.

            Non-finite and masked values are ignored.

        positions : array-like, default: [1, 2, ..., n]
            The positions of the violins; i.e. coordinates on the x-axis for
            vertical violins (or y-axis for horizontal violins).

        vert : bool, optional
            .. deprecated:: 3.10
                Use *orientation* instead.

                If this is given during the deprecation period, it overrides
                the *orientation* parameter.

            If True, plots the violins vertically.
            If False, plots the violins horizontally.

        orientation : {'vertical', 'horizontal'}, default: 'vertical'
            If 'horizontal', plots the violins horizontally.
            Otherwise, plots the violins vertically.

            .. versionadded:: 3.10

        widths : float or array-like, default: 0.5
            The maximum width of each violin in units of the *positions* axis.
            The default is 0.5, which is half the available space when using default
            *positions*.

        showmeans : bool, default: False
            Whether to show the mean with a line.

        showextrema : bool, default: True
            Whether to show extrema with a line.

        showmedians : bool, default: False
            Whether to show the median with a line.

        quantiles : array-like, default: None
            If not None, set a list of floats in interval [0, 1] for each violin,
            which stands for the quantiles that will be rendered for that
            violin.

        points : int, default: 100
            The number of points to evaluate each of the gaussian kernel density
            estimations at.

        bw_method : {'scott', 'silverman'} or float or callable, default: 'scott'
            The method used to calculate the estimator bandwidth.  If a
            float, this will be used directly as `!kde.factor`.  If a
            callable, it should take a `matplotlib.mlab.GaussianKDE` instance as
            its only parameter and return a float.

        side : {'both', 'low', 'high'}, default: 'both'
            'both' plots standard violins. 'low'/'high' only
            plots the side below/above the positions value.

        facecolor : :mpltype:`color` or list of :mpltype:`color`, optional
            If provided, will set the face color(s) of the violins.

            .. versionadded:: 3.11

        linecolor : :mpltype:`color` or list of :mpltype:`color`, optional
            If provided, will set the line color(s) of the violins (the
            horizontal and vertical spines and body edges).

            .. versionadded:: 3.11

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        Returns
        -------
        dict
            A dictionary mapping each component of the violinplot to a
            list of the corresponding collection instances created. The
            dictionary has the following keys:

            - ``bodies``: A list of the `~.collections.PolyCollection`
              instances containing the filled area of each violin.

            - ``cmeans``: A `~.collections.LineCollection` instance that marks
              the mean values of each of the violin's distribution.

            - ``cmins``: A `~.collections.LineCollection` instance that marks
              the bottom of each violin's distribution.

            - ``cmaxes``: A `~.collections.LineCollection` instance that marks
              the top of each violin's distribution.

            - ``cbars``: A `~.collections.LineCollection` instance that marks
              the centers of each violin's distribution.

            - ``cmedians``: A `~.collections.LineCollection` instance that
              marks the median values of each of the violin's distribution.

            - ``cquantiles``: A `~.collections.LineCollection` instance created
              to identify the quantile values of each of the violin's
              distribution.

        See Also
        --------
        .Axes.violin : Draw a violin from pre-computed statistics.
        boxplot : Draw a box and whisker plot.
        """
        vpstats = cbook.violin_stats(dataset, ("GaussianKDE", bw_method),
                                     points=points, quantiles=quantiles)
        return self.violin(vpstats, positions=positions, vert=vert,
                           orientation=orientation, widths=widths,
                           showmeans=showmeans, showextrema=showextrema,
                           showmedians=showmedians, side=side,
                           facecolor=facecolor, linecolor=linecolor)

    @_api.make_keyword_only("3.10", "vert")
    def violin(self, vpstats, positions=None, vert=None,
               orientation='vertical', widths=0.5, showmeans=False,
               showextrema=True, showmedians=False, side='both',
               facecolor=None, linecolor=None):
        """
        Draw a violin plot from pre-computed statistics.

        Draw a violin plot for each column of *vpstats*. Each filled area
        extends to represent the entire data range, with optional lines at the
        mean, the median, the minimum, the maximum, and the quantiles values.

        Parameters
        ----------
        vpstats : list of dicts
            A list of dictionaries containing stats for each violin plot.
            Required keys are:

            - ``coords``: A list of scalars containing the coordinates that
              the violin's kernel density estimate were evaluated at.

            - ``vals``: A list of scalars containing the values of the
              kernel density estimate at each of the coordinates given
              in *coords*.

            - ``mean``: The mean value for this violin's dataset.

            - ``median``: The median value for this violin's dataset.

            - ``min``: The minimum value for this violin's dataset.

            - ``max``: The maximum value for this violin's dataset.

            Optional keys are:

            - ``quantiles``: A list of scalars containing the quantile values
              for this violin's dataset.

        positions : array-like, default: [1, 2, ..., n]
            The positions of the violins; i.e. coordinates on the x-axis for
            vertical violins (or y-axis for horizontal violins).

        vert : bool, optional
            .. deprecated:: 3.10
                Use *orientation* instead.

                If this is given during the deprecation period, it overrides
                the *orientation* parameter.

            If True, plots the violins vertically.
            If False, plots the violins horizontally.

        orientation : {'vertical', 'horizontal'}, default: 'vertical'
            If 'horizontal', plots the violins horizontally.
            Otherwise, plots the violins vertically.

            .. versionadded:: 3.10

        widths : float or array-like, default: 0.5
            The maximum width of each violin in units of the *positions* axis.
            The default is 0.5, which is half available space when using default
            *positions*.

        showmeans : bool, default: False
            Whether to show the mean with a line.

        showextrema : bool, default: True
            Whether to show extrema with a line.

        showmedians : bool, default: False
            Whether to show the median with a line.

        side : {'both', 'low', 'high'}, default: 'both'
            'both' plots standard violins. 'low'/'high' only
            plots the side below/above the positions value.

        facecolor : :mpltype:`color` or list of :mpltype:`color`, optional
            If provided, will set the face color(s) of the violins.

            .. versionadded:: 3.11

            For backward compatibility, if *facecolor* is not given, the body
            will get an Artist-level transparency `alpha <.Artist.set_alpha>`
            of 0.3, which will persist if you afterwards change the facecolor,
            e.g. via ``result['bodies'][0].set_facecolor('red')``.
            If *facecolor* is given, there is no Artist-level transparency.
            To set transparency for *facecolor* or *edgecolor* use
            ``(color, alpha)`` tuples.

        linecolor : :mpltype:`color` or list of :mpltype:`color`, optional
            If provided, will set the line color(s) of the violins (the
            horizontal and vertical spines and body edges).

            .. versionadded:: 3.11

        Returns
        -------
        dict
            A dictionary mapping each component of the violinplot to a
            list of the corresponding collection instances created. The
            dictionary has the following keys:

            - ``bodies``: A list of the `~.collections.PolyCollection`
              instances containing the filled area of each violin.

            - ``cmeans``: A `~.collections.LineCollection` instance that marks
              the mean values of each of the violin's distribution.

            - ``cmins``: A `~.collections.LineCollection` instance that marks
              the bottom of each violin's distribution.

            - ``cmaxes``: A `~.collections.LineCollection` instance that marks
              the top of each violin's distribution.

            - ``cbars``: A `~.collections.LineCollection` instance that marks
              the centers of each violin's distribution.

            - ``cmedians``: A `~.collections.LineCollection` instance that
              marks the median values of each of the violin's distribution.

            - ``cquantiles``: A `~.collections.LineCollection` instance created
              to identify the quantiles values of each of the violin's
              distribution.

        See Also
        --------
        violinplot :
            Draw a violin plot from data instead of pre-computed statistics.
        .cbook.violin_stats:
            Calculate a *vpstats* dictionary from data, suitable for passing to violin.
        """
        N = len(vpstats)
        datashape_message = ("List of violinplot statistics and `{0}` "
                             "values must have the same length")
        if vert is not None:
            _api.warn_deprecated(
                "3.11",
                name="vert: bool",
                alternative="orientation: {'vertical', 'horizontal'}",
            )
            orientation = 'vertical' if vert else 'horizontal'
        _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)
        if positions is None:
            positions = range(1, N + 1)
        elif len(positions) != N:
            raise ValueError(datashape_message.format("positions"))
        if np.isscalar(widths):
            widths = [widths] * N
        elif len(widths) != N:
            raise ValueError(datashape_message.format("widths"))
        if N > 0:
            pos0 = positions[0]
            width0 = widths[0]
            if (isinstance(pos0, (datetime.datetime, datetime.date))
                    and not isinstance(width0, datetime.timedelta)):
                raise TypeError(
                    "datetime/date 'position' values require timedelta 'widths'. "
                    "For example, use positions=[datetime.date(2024, 1, 1)] "
                    "and widths=[datetime.timedelta(days=1)].")
            elif (isinstance(pos0, np.datetime64)
                    and not isinstance(width0, np.timedelta64)):
                raise TypeError(
                    "np.datetime64 'position' values require np.timedelta64 'widths'")
        _api.check_in_list(["both", "low", "high"], side=side)
        # matplotlib computes the ends of the statistics lines, then reads
        # each violin's values as it fills its body.
        ([[-0.25 if side in ['both', 'low'] else 0],
          [0.25 if side in ['both', 'high'] else 0]] * np.array(widths) + positions)
        for stats in vpstats:
            stats['vals']  # noqa: B018
        value_axis = "y" if orientation == "vertical" else "x"
        vpstats = [_converted_stats(self, value_axis, stats, (
            "coords", "mean", "median", "min", "max", "quantiles"))
            for stats in vpstats]
        parts = _plotting.violin(
            self._native, list(vpstats),
            (list(map(float, positions)), list(map(float, widths))),
            orientation, (bool(showmeans), bool(showextrema), bool(showmedians)),
            side, (_colors_or_empty(facecolor), _colors_or_empty(linecolor)))
        bodies, *lines = parts
        # matplotlib fills each body between two curves along the values.
        artists = {'bodies': wrap_all(self, bodies, mcoll.FillBetweenPolyCollection)}
        for body in artists['bodies']:
            body.t_direction = value_axis
        for name, native in zip(('cmeans', 'cmaxes', 'cmins', 'cbars',
                                 'cmedians', 'cquantiles'), lines):
            if native is not None:
                artists[name] = wrap(self, native, mcoll.LineCollection)
        return artists

    @_api.make_keyword_only("3.10", "explode")
    @_preprocess_data(replace_names=["x", "explode", "labels", "colors"])
    def pie(self, x, explode=None, labels=None, colors=None,
            autopct=None, pctdistance=0.6, shadow=False, labeldistance=1.1,
            startangle=0, radius=1, counterclock=True,
            wedgeprops=None, textprops=None, center=(0, 0),
            frame=False, rotatelabels=False, *, normalize=True, hatch=None):
        """
        Plot a pie chart.

        Make a pie chart of array *x*.  The fractional area of each wedge is
        given by ``x/sum(x)``.

        The wedges are plotted counterclockwise, by default starting from the
        x-axis.

        Parameters
        ----------
        x : 1D array-like
            The wedge sizes.

        explode : array-like, default: None
            If not *None*, is a ``len(x)`` array which specifies the fraction
            of the radius with which to offset each wedge.

        labels : list, default: None
            A sequence of strings providing the labels for each wedge

        colors : :mpltype:`color` or list of :mpltype:`color`, default: None
            A sequence of colors through which the pie chart will cycle.  If
            *None*, will use the colors in the currently active cycle.

        hatch : str or list, default: None
            Hatching pattern applied to all pie wedges or sequence of patterns
            through which the chart will cycle. For a list of valid patterns,
            see :doc:`/gallery/shapes_and_collections/hatch_style_reference`.

            .. versionadded:: 3.7

        autopct : None or str or callable, default: None
            If not *None*, *autopct* is a string or function used to label the
            wedges with their numeric value. The label will be placed inside
            the wedge. If *autopct* is a format string, the label will be
            ``fmt % pct``. If *autopct* is a function, then it will be called.

        pctdistance : float, default: 0.6
            The relative distance along the radius at which the text
            generated by *autopct* is drawn. To draw the text outside the pie,
            set *pctdistance* > 1. This parameter is ignored if *autopct* is
            ``None``.

        labeldistance : float or None, default: 1.1
            The relative distance along the radius at which the labels are
            drawn. To draw the labels inside the pie, set  *labeldistance* < 1.
            If set to ``None``, labels are not drawn but are still stored for
            use in `.legend`.

        shadow : bool or dict, default: False
            If bool, whether to draw a shadow beneath the pie. If dict, draw a shadow
            passing the properties in the dict to `.Shadow`.

            .. versionadded:: 3.8
                *shadow* can be a dict.

        startangle : float, default: 0 degrees
            The angle by which the start of the pie is rotated,
            counterclockwise from the x-axis.

        radius : float, default: 1
            The radius of the pie.

        counterclock : bool, default: True
            Specify fractions direction, clockwise or counterclockwise.

        wedgeprops : dict, default: None
            Dict of arguments passed to each `.patches.Wedge` of the pie.
            For example, ``wedgeprops = {'linewidth': 3}`` sets the width of
            the wedge border lines equal to 3. By default, ``clip_on=False``.
            When there is a conflict between these properties and other
            keywords, properties passed to *wedgeprops* take precedence.

        textprops : dict, default: None
            Dict of arguments to pass to the `.Text` objects.

        center : (float, float), default: (0, 0)
            The coordinates of the center of the chart.

        frame : bool, default: False
            Plot Axes frame with the chart if true.

        rotatelabels : bool, default: False
            Rotate each label to the angle of the corresponding slice if true.

        normalize : bool, default: True
            When *True*, always make a full pie by normalizing x so that
            ``sum(x) == 1``. *False* makes a partial pie if ``sum(x) <= 1``
            and raises a `ValueError` for ``sum(x) > 1``.

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        Returns
        -------
        `.PieContainer`
            Container with all the wedge patches and any associated text objects.

        .. versionchanged:: 3.11
           Previously the wedges and texts were returned in a tuple.

        Notes
        -----
        The pie chart will probably look best if the figure and Axes are
        square, or the Axes aspect is equal.
        This method sets the aspect ratio of the axis to "equal".
        The Axes aspect ratio can be controlled with `.Axes.set_aspect`.
        """
        arguments = (x, explode, labels, colors, autopct, pctdistance, shadow,
                     labeldistance, startangle, radius, counterclock, wedgeprops,
                     textprops, center, frame, rotatelabels, normalize, hatch)
        if not (_pie_plain(colors, pctdistance, labeldistance, startangle, radius,
                           wedgeprops, textprops, center, hatch, explode)
                and _pie_props_taken(wedgeprops, textprops)):
            return _pie_reference.pie(self, *arguments)
        try:
            with _failed_calls.leaving_nothing(self):
                return self._native_pie(*arguments)
        except (TypeError, ValueError):
            # Refused by pltrs (Rust): matplotlib's code raises its own
            # error, or makes what pltrs (Rust) does not.
            return _pie_reference.pie(self, *arguments)

    def _native_pie(self, x, explode, labels, colors, autopct, pctdistance, shadow,
                    labeldistance, startangle, radius, counterclock, wedgeprops,
                    textprops, center, frame, rotatelabels, normalize, hatch):
        """`pie` made by pltrs (Rust)."""
        x = np.asarray(x)
        if x.ndim > 1:
            raise ValueError("x must be 1D")
        if np.any(x < 0):
            raise ValueError("Wedge sizes 'x' must be non negative values")
        if not np.all(np.isfinite(x)):
            raise ValueError('Wedge sizes must be finite numbers')
        sx = x.sum()
        if sx == 0:
            raise ValueError('All wedge sizes are zero')
        if not normalize and sx > 1:
            raise ValueError('Cannot plot an unnormalized pie with sum(x) > 1')
        if labels is None:
            labels = [''] * len(x)
        if explode is None:
            explode = [0] * len(x)
        if len(x) != len(labels):
            raise ValueError(f"'labels' must be of length 'x', not {len(labels)}")
        if len(x) != len(explode):
            raise ValueError(f"'explode' must be of length 'x', not {len(explode)}")
        _api.check_isinstance(Real, radius=radius, startangle=startangle)
        if radius <= 0:
            raise ValueError(f"'radius' must be a positive number, not {radius}")
        if autopct is not None and not (isinstance(autopct, str) or callable(autopct)):
            raise TypeError('autopct must be callable or a format string')
        if isinstance(autopct, str) and '%' not in autopct.replace('%%', ''):
            # matplotlib formats each percentage, a numpy float, which `%`
            # takes as a mapping: a template of no conversion is its text.
            text = autopct.replace('%%', '%')
            autopct = lambda pct: text  # noqa: E731
        textprops = dict(textprops or {})
        shadow_spec = None
        if shadow:
            spec = {'ox': -0.02, 'oy': -0.02, 'shade': 0.7}
            if isinstance(shadow, dict):
                spec.update(shadow)
            ox, oy, shade = (float(spec.pop(key)) for key in ('ox', 'oy', 'shade'))
            shadow_native, _ = _native_patch_props(spec, mpatches.Shadow)
            shadow_spec = (ox, oy, shade, shadow_native)
        hatches = [] if hatch is None else [str(h) for h in np.atleast_1d(hatch)]
        # The wedges take their properties before the shadows copy them.
        wedge_native, wedge_rest = _native_patch_props(wedgeprops, mpatches.Wedge)
        wedges, label_texts, pct_texts = _plotting.pie(
            self._native, [float(v) for v in x],
            ([float(e) for e in explode], float(radius), float(startangle),
             bool(counterclock), None, tuple(map(float, center))),
            ([str(lbl) for lbl in labels], autopct, float(pctdistance),
             None if labeldistance is None else float(labeldistance),
             bool(rotatelabels), (None, None, textprops.get('usetex'))),
            (_colors_or_empty(colors), hatches, wedge_native, shadow_spec,
             bool(frame), bool(normalize)))
        from ...text import Text
        wedges = wrap_all(self, wedges, mpatches.Wedge)
        if shadow_spec is not None:
            _wrap_pie_shadows(self, wedges, *shadow_spec[:2])
        _pie_geometry(wedges, x / sx if normalize else x, explode, radius,
                      startangle, counterclock, center)
        if colors is None:
            for wedge in wedges:
                wedge._take_cycle_color("facecolor")
        for wedge in wedges if wedge_rest else ():
            wedge.set(**wedge_rest)
        container = PieContainer(wedges, x, normalize)
        label_texts = wrap_all(self, label_texts, Text)
        pct_texts = wrap_all(self, pct_texts, Text)
        for text in [*label_texts, *pct_texts]:
            text._internal_update(dict(textprops))
        container.add_texts(label_texts)
        if autopct is not None:
            container.add_texts(pct_texts)
        return container
