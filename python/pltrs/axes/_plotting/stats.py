"""
Statistical plots: `~.Axes.boxplot` and `~.Axes.bxp`,
`~.Axes.violinplot` and `~.Axes.violin`, and `~.Axes.pie`.

The statistics are computed in Python as matplotlib computes them
(`.cbook.boxplot_stats`, `.cbook.violin_stats`); pltrs (Rust) draws them.
"""

import datetime
import math
from numbers import Real

import numpy as np

from ... import _api, _tick_helpers, _values, cbook, collections as mcoll, colors as mcolors
from ... import lines as mlines, patches as mpatches, ticker as mticker
from ..._native import plotting as _plotting
from ..._preprocess import _preprocess_data
from ...container import PieContainer
from ._helpers import wrap, wrap_all


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from ... import rcParams
    return rcParams


def _native_lineprops(props):
    """Line properties of a box plot part in native form."""
    return mlines._native_kwargs(cbook.normalize_kwargs(props or {}, mlines.Line2D))


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


def _native_patch_props(props, cls):
    """
    The properties *props* of patches of class *cls* (aliases resolved)
    split into those pltrs (Rust) takes as it makes them, in native form,
    and the others, which the patches are set afterwards.
    """
    props = cbook.normalize_kwargs(props or {}, cls)
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
            "theta2": 360. * max(theta1, theta2)}, mpatches.Wedge._GIVEN)
        theta1 = theta2

class StatsMixin:
    """`boxplot`, `bxp`, `violinplot`, `violin` and `pie`."""

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

        Returns
        -------
        dict
          A dictionary mapping each component of the boxplot to a list
          of the `.Line2D` instances created.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        rc = _rcParams()
        whis = rc['boxplot.whiskers'] if whis is None else whis
        if isinstance(whis, str):  # the rc text of a float or of percentiles
            whis = ([float(v) for v in whis.split(',')] if ',' in whis
                    else float(whis))
        bootstrap = rc['boxplot.bootstrap'] if bootstrap is None else bootstrap
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

    def bxp(self, bxpstats, positions=None, widths=None, vert=None,
            orientation='vertical', patch_artist=False, shownotches=False,
            showmeans=False, showcaps=True, showbox=True, showfliers=True,
            boxprops=None, whiskerprops=None, flierprops=None,
            medianprops=None, capprops=None, meanprops=None,
            meanline=False, manage_ticks=True, zorder=None,
            capwidths=None, label=None):
        """
        Draw a box and whisker plot from pre-computed statistics.

        Returns
        -------
        dict
          A dictionary mapping each component of the boxplot to a list
          of the `.Line2D` instances created.
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
        if label is not None and not isinstance(label, str):
            label = [str(lbl) for lbl in label]
            if len(label) != N:
                raise ValueError(datashape_message.format("label"))
        elif label is not None:
            label = [label]
        value_axis = "y" if orientation == "vertical" else "x"
        bxpstats = [_converted_stats(self, value_axis, stats, (
            "med", "q1", "q3", "whislo", "whishi", "mean", "cilo", "cihi",
            "fliers")) for stats in bxpstats]
        meanprops_given = meanprops or {}
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
            (bool(manage_ticks), None if zorder is None else float(zorder), label))
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
        return dict(whiskers=wrap_all(self, whiskers, mlines.Line2D),
                    caps=wrap_all(self, caps, mlines.Line2D), boxes=boxes,
                    medians=wrap_all(self, medians, mlines.Line2D),
                    fliers=wrap_all(self, fliers, mlines.Line2D),
                    means=wrap_all(self, means, mlines.Line2D))

    @_preprocess_data(replace_names=["dataset"])
    def violinplot(self, dataset, positions=None, vert=None,
                   orientation='vertical', widths=0.5, showmeans=False,
                   showextrema=True, showmedians=False, quantiles=None,
                   points=100, bw_method=None, side='both',
                   facecolor=None, linecolor=None):
        """
        Make a violin plot.

        Returns
        -------
        dict
            A dictionary mapping each component of the violinplot to a
            list of the corresponding collection instances created.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        vpstats = cbook.violin_stats(dataset, ("GaussianKDE", bw_method),
                                     points=points, quantiles=quantiles)
        return self.violin(vpstats, positions=positions, vert=vert,
                           orientation=orientation, widths=widths,
                           showmeans=showmeans, showextrema=showextrema,
                           showmedians=showmedians, side=side,
                           facecolor=facecolor, linecolor=linecolor)

    def violin(self, vpstats, positions=None, vert=None,
               orientation='vertical', widths=0.5, showmeans=False,
               showextrema=True, showmedians=False, side='both',
               facecolor=None, linecolor=None):
        """
        Draw a violin plot from pre-computed statistics.

        Returns
        -------
        dict
            A dictionary mapping each component of the violinplot to a
            list of the corresponding collection instances created.
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

    @_preprocess_data(replace_names=["x", "explode", "labels", "colors"])
    def pie(self, x, explode=None, labels=None, colors=None,
            autopct=None, pctdistance=0.6, shadow=False, labeldistance=1.1,
            startangle=0, radius=1, counterclock=True,
            wedgeprops=None, textprops=None, center=(0, 0),
            frame=False, rotatelabels=False, *, normalize=True, hatch=None):
        """
        Plot a pie chart.

        Returns
        -------
        `.PieContainer`
            Container with all the wedge patches and any associated text objects.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
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
