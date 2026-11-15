"""
matplotlib's plotting methods that make what they plot through other
plotting methods of the Axes (`~.Axes.bxp` through `~.Axes.plot`,
`~.Axes.hist` through `~.Axes.bar` and `~.Axes.fill`, the spectra and
`~.Axes.ecdf` through `~.Axes.plot`, `~.Axes.bar_label` through
`~.Axes.annotate`), for an Axes class of the program's that overrides
one of those, which pltrs (Rust) does without otherwise (matplotlib
3.11's code, nearly as it is, under the Matplotlib License).
"""

import datetime
import itertools
import math
from numbers import Number, Real

import numpy as np

from ... import (_api, _style_helpers, _val_or_rc, cbook, colors as mcolors, lines as mlines, mlab,
                 patches as mpatches, path as mpath, rcParams, ticker as mticker,
                 transforms as mtransforms)


def bar_label(axes, container, labels=None, *, fmt="%g", label_type="edge",
              padding=0, **kwargs):
    """`~.Axes.bar_label` of *axes* as matplotlib makes it, through its
    `~.Axes.annotate`."""
    for key in ['horizontalalignment', 'ha', 'verticalalignment', 'va']:
        if key in kwargs:
            raise ValueError(
                f"Passing {key!r} to bar_label() is not supported.")

    a, b = axes.yaxis.get_view_interval()
    y_inverted = a > b
    c, d = axes.xaxis.get_view_interval()
    x_inverted = c > d

    # want to know whether to put label on positive or negative direction
    # cannot use np.sign here because it will return 0 if x == 0
    def sign(x):
        return 1 if x >= 0 else -1

    _api.check_in_list(['edge', 'center'], label_type=label_type)

    bars = container.patches
    errorbar = container.errorbar
    datavalues = container.datavalues
    orientation = container.orientation

    if errorbar:
        # check "ErrorbarContainer" for the definition of these elements
        lines = errorbar.lines  # attribute of "ErrorbarContainer" (tuple)
        barlinecols = lines[2]  # 0: data_line, 1: caplines, 2: barlinecols
        barlinecol = barlinecols[0]  # the "LineCollection" of error bars
        errs = barlinecol.get_segments()
    else:
        errs = []

    if labels is None:
        labels = []

    annotations = []

    if np.iterable(padding):
        # if padding iterable, check length
        padding = np.asarray(padding)
        if len(padding) != len(bars):
            raise ValueError(
                f"padding must be of length {len(bars)} when passed as a sequence")
    else:
        # single value, apply to all labels
        padding = [padding] * len(bars)

    for bar, err, dat, lbl, pad in itertools.zip_longest(
            bars, errs, datavalues, labels, padding
    ):
        (x0, y0), (x1, y1) = bar.get_bbox().get_points()
        xc, yc = (x0 + x1) / 2, (y0 + y1) / 2

        if orientation == "vertical":
            extrema = max(y0, y1) if dat >= 0 else min(y0, y1)
        else:  # horizontal
            extrema = max(x0, x1) if dat >= 0 else min(x0, x1)

        if err is None or np.size(err) == 0:
            endpt = extrema
        elif orientation == "vertical":
            endpt = err[:, 1].max() if dat >= 0 else err[:, 1].min()
        else:  # horizontal
            endpt = err[:, 0].max() if dat >= 0 else err[:, 0].min()

        if label_type == "center":
            xy = (0.5, 0.5)
            kwargs["xycoords"] = (
                lambda r, b=bar:
                    mtransforms.Bbox.intersection(
                        b.get_window_extent(r), b.get_clip_box()
                    ) or mtransforms.Bbox.null()
            )
        else:  # edge
            if orientation == "vertical":
                xy = xc, endpt
            else:  # horizontal
                xy = endpt, yc

        if orientation == "vertical":
            y_direction = -1 if y_inverted else 1
            xytext = 0, y_direction * sign(dat) * pad
        else:  # horizontal
            x_direction = -1 if x_inverted else 1
            xytext = x_direction * sign(dat) * pad, 0

        if label_type == "center":
            ha, va = "center", "center"
        else:  # edge
            if orientation == "vertical":
                ha = 'center'
                if y_inverted:
                    va = 'top' if dat > 0 else 'bottom'  # also handles NaN
                else:
                    va = 'top' if dat < 0 else 'bottom'  # also handles NaN
            else:  # horizontal
                if x_inverted:
                    ha = 'right' if dat > 0 else 'left'  # also handles NaN
                else:
                    ha = 'right' if dat < 0 else 'left'  # also handles NaN
                va = 'center'

        if np.isnan(dat):
            lbl = ''

        if lbl is None:
            if isinstance(fmt, str):
                lbl = cbook._auto_format_str(fmt, dat)
            elif callable(fmt):
                lbl = fmt(dat)
            else:
                raise TypeError("fmt must be a str or callable")
        annotation = axes.annotate(lbl,
                                   xy, xytext, textcoords="offset points",
                                   ha=ha, va=va, **kwargs)
        annotations.append(annotation)

    return annotations


def bxp(axes, bxpstats, positions=None, widths=None, vert=None,
        orientation='vertical', patch_artist=False, shownotches=False,
        showmeans=False, showcaps=True, showbox=True, showfliers=True,
        boxprops=None, whiskerprops=None, flierprops=None,
        medianprops=None, capprops=None, meanprops=None,
        meanline=False, manage_ticks=True, zorder=None,
        capwidths=None, label=None):
    """`~.Axes.bxp` of *axes* as matplotlib makes it, through its
    `~.Axes.plot`."""
    medianprops = {
        "solid_capstyle": "butt",
        "dash_capstyle": "butt",
        **(medianprops or {}),
    }
    meanprops = {
        "solid_capstyle": "butt",
        "dash_capstyle": "butt",
        **(meanprops or {}),
    }

    # lists of artists to be output
    whiskers = []
    caps = []
    boxes = []
    medians = []
    means = []
    fliers = []

    # empty list of xticklabels
    datalabels = []

    # Use default zorder if none specified
    if zorder is None:
        zorder = mlines.Line2D.zorder

    zdelta = 0.1

    def merge_kw_rc(subkey, explicit, zdelta=0, usemarker=True):
        d = {k.split('.')[-1]: v for k, v in rcParams.items()
             if k.startswith(f'boxplot.{subkey}props')}
        d['zorder'] = zorder + zdelta
        if not usemarker:
            d['marker'] = ''
        d.update(cbook.normalize_kwargs(explicit, mlines.Line2D))
        return d

    box_kw = {
        'linestyle': rcParams['boxplot.boxprops.linestyle'],
        'linewidth': rcParams['boxplot.boxprops.linewidth'],
        'edgecolor': rcParams['boxplot.boxprops.color'],
        'facecolor': ('white' if rcParams['_internal.classic_mode']
                      else rcParams['patch.facecolor']),
        'zorder': zorder,
        **cbook.normalize_kwargs(boxprops, mpatches.PathPatch)
    } if patch_artist else merge_kw_rc('box', boxprops, usemarker=False)
    whisker_kw = merge_kw_rc('whisker', whiskerprops, usemarker=False)
    cap_kw = merge_kw_rc('cap', capprops, usemarker=False)
    flier_kw = merge_kw_rc('flier', flierprops)
    median_kw = merge_kw_rc('median', medianprops, zdelta, usemarker=False)
    mean_kw = merge_kw_rc('mean', meanprops, zdelta)
    removed_prop = 'marker' if meanline else 'linestyle'
    # Only remove the property if it's not set explicitly as a parameter.
    if meanprops is None or removed_prop not in meanprops:
        mean_kw[removed_prop] = ''

    # vert and orientation parameters are linked until vert's
    # deprecation period expires. vert only takes precedence
    # if set to False.
    if vert is None:
        vert = rcParams['boxplot.vertical']
    else:
        _api.warn_deprecated(
            "3.11",
            name="vert: bool",
            alternative="orientation: {'vertical', 'horizontal'}",
        )
    if vert is False:
        orientation = 'horizontal'
    _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)

    if not rcParams['boxplot.vertical']:
        _api.warn_deprecated(
            "3.10",
            name='boxplot.vertical', obj_type="rcparam"
        )

    # vertical or horizontal plot?
    maybe_swap = slice(None) if orientation == 'vertical' else slice(None, None, -1)

    def do_plot(xs, ys, **kwargs):
        return axes.plot(*[xs, ys][maybe_swap], **kwargs)[0]

    def do_patch(xs, ys, **kwargs):
        path = mpath.Path._create_closed(
            np.column_stack([xs, ys][maybe_swap]))
        patch = mpatches.PathPatch(path, **kwargs)
        axes.add_artist(patch)
        return patch

    # input validation
    N = len(bxpstats)
    datashape_message = ("List of boxplot statistics and `{0}` "
                         "values must have same the length")
    # check position
    if positions is None:
        positions = list(range(1, N + 1))
    elif len(positions) != N:
        raise ValueError(datashape_message.format("positions"))

    positions = np.array(positions)
    if len(positions) > 0 and not all(isinstance(p, Real) for p in positions):
        raise TypeError("positions should be an iterable of numbers")

    # width
    if widths is None:
        widths = [np.clip(0.15 * np.ptp(positions), 0.15, 0.5)] * N
    elif np.isscalar(widths):
        widths = [widths] * N
    elif len(widths) != N:
        raise ValueError(datashape_message.format("widths"))

    # capwidth
    if capwidths is None:
        capwidths = 0.5 * np.array(widths)
    elif np.isscalar(capwidths):
        capwidths = [capwidths] * N
    elif len(capwidths) != N:
        raise ValueError(datashape_message.format("capwidths"))

    for pos, width, stats, capwidth in zip(positions, widths, bxpstats,
                                           capwidths):
        # try to find a new label
        datalabels.append(stats.get('label', pos))

        # whisker coords
        whis_x = [pos, pos]
        whislo_y = [stats['q1'], stats['whislo']]
        whishi_y = [stats['q3'], stats['whishi']]
        # cap coords
        cap_left = pos - capwidth * 0.5
        cap_right = pos + capwidth * 0.5
        cap_x = [cap_left, cap_right]
        cap_lo = np.full(2, stats['whislo'])
        cap_hi = np.full(2, stats['whishi'])
        # box and median coords
        box_left = pos - width * 0.5
        box_right = pos + width * 0.5
        med_y = [stats['med'], stats['med']]
        # notched boxes
        if shownotches:
            notch_left = pos - width * 0.25
            notch_right = pos + width * 0.25
            box_x = [box_left, box_right, box_right, notch_right,
                     box_right, box_right, box_left, box_left, notch_left,
                     box_left, box_left]
            box_y = [stats['q1'], stats['q1'], stats['cilo'],
                     stats['med'], stats['cihi'], stats['q3'],
                     stats['q3'], stats['cihi'], stats['med'],
                     stats['cilo'], stats['q1']]
            med_x = [notch_left, notch_right]
        # plain boxes
        else:
            box_x = [box_left, box_right, box_right, box_left, box_left]
            box_y = [stats['q1'], stats['q1'], stats['q3'], stats['q3'],
                     stats['q1']]
            med_x = [box_left, box_right]

        # maybe draw the box
        if showbox:
            do_box = do_patch if patch_artist else do_plot
            boxes.append(do_box(box_x, box_y, **box_kw))
            median_kw.setdefault('label', '_nolegend_')
        # draw the whiskers
        whisker_kw.setdefault('label', '_nolegend_')
        whiskers.append(do_plot(whis_x, whislo_y, **whisker_kw))
        whiskers.append(do_plot(whis_x, whishi_y, **whisker_kw))
        # maybe draw the caps
        if showcaps:
            cap_kw.setdefault('label', '_nolegend_')
            caps.append(do_plot(cap_x, cap_lo, **cap_kw))
            caps.append(do_plot(cap_x, cap_hi, **cap_kw))
        # draw the medians
        medians.append(do_plot(med_x, med_y, **median_kw))
        # maybe draw the means
        if showmeans:
            if meanline:
                means.append(do_plot(
                    [box_left, box_right], [stats['mean'], stats['mean']],
                    **mean_kw
                ))
            else:
                means.append(do_plot([pos], [stats['mean']], **mean_kw))
        # maybe draw the fliers
        if showfliers:
            flier_kw.setdefault('label', '_nolegend_')
            flier_x = np.full(len(stats['fliers']), pos, dtype=np.float64)
            flier_y = stats['fliers']
            fliers.append(do_plot(flier_x, flier_y, **flier_kw))

    # Set legend labels
    if label:
        box_or_med = boxes if showbox and patch_artist else medians
        if cbook.is_scalar_or_string(label):
            # assign the label only to the first box
            box_or_med[0].set_label(label)
        else:  # label is a sequence
            if len(box_or_med) != len(label):
                raise ValueError(datashape_message.format("label"))
            for artist, lbl in zip(box_or_med, label):
                artist.set_label(lbl)

    if manage_ticks:
        axis_name = "x" if orientation == 'vertical' else "y"
        interval = getattr(axes.dataLim, f"interval{axis_name}")
        axis = axes._axis_map[axis_name]
        positions = axis.convert_units(positions)
        # The 0.5 additional padding ensures reasonable-looking boxes
        # even when drawing a single box.  We set the sticky edge to
        # prevent margins expansion, in order to match old behavior (back
        # when separate calls to boxplot() would completely reset the axis
        # limits regardless of what was drawn before).  The sticky edges
        # are attached to the median lines, as they are always present.
        # (Set through the box's setter: pltrs' data limits are not a view
        # of the array matplotlib's writes in place.)
        setattr(axes.dataLim, f"interval{axis_name}",
                (min(interval[0], min(positions) - .5),
                 max(interval[1], max(positions) + .5)))
        for median, position in zip(medians, positions):
            getattr(median.sticky_edges, axis_name).extend(
                [position - .5, position + .5])
        # Modified from Axis.set_ticks and Axis.set_ticklabels.
        locator = axis.get_major_locator()
        if not isinstance(axis.get_major_locator(),
                          mticker.FixedLocator):
            locator = mticker.FixedLocator([])
            axis.set_major_locator(locator)
        locator.locs = np.array([*locator.locs, *positions])
        formatter = axis.get_major_formatter()
        if not isinstance(axis.get_major_formatter(),
                          mticker.FixedFormatter):
            formatter = mticker.FixedFormatter([])
            axis.set_major_formatter(formatter)
        formatter.seq = [*formatter.seq, *datalabels]

        axes._request_autoscale_view()

    return dict(whiskers=whiskers, caps=caps, boxes=boxes,
                medians=medians, fliers=fliers, means=means)


def hist(axes, x, bins=None, range=None, density=False, weights=None,
         cumulative=False, bottom=None, histtype='bar', align='mid',
         orientation='vertical', rwidth=None, log=False,
         color=None, label=None, stacked=False, **kwargs):
    """`~.Axes.hist` of *axes* as matplotlib makes it, through its
    `~.Axes.bar`, `~.Axes.barh`, `~.Axes.fill`."""
    bin_range = range
    from builtins import range

    kwargs = cbook.normalize_kwargs(kwargs, mpatches.Patch)

    if np.isscalar(x):
        x = [x]

    bins = _val_or_rc(bins, 'hist.bins')

    # Validate string inputs here to avoid cluttering subsequent code.
    _api.check_in_list(['bar', 'barstacked', 'step', 'stepfilled'],
                       histtype=histtype)
    _api.check_in_list(['left', 'mid', 'right'], align=align)
    _api.check_in_list(['horizontal', 'vertical'], orientation=orientation)

    if histtype == 'barstacked' and not stacked:
        stacked = True

    # Massage 'x' for processing.
    x = cbook._reshape_2D(x, 'x')
    nx = len(x)  # number of datasets

    for arr in x:
        if len(arr) > 0 and isinstance(
            arr[0], (datetime.timedelta, np.timedelta64)
        ):
            raise TypeError(
                "Axes.hist does not currently support timedelta inputs. "
                "Convert to numeric values  (e.g., .total_seconds()) first."
            )

    # Process unit information.  _process_unit_info sets the unit and
    # converts the first dataset; then we convert each following dataset
    # one at a time.
    if orientation == "vertical":
        convert_units = axes.convert_xunits
        x = [*axes._process_unit_info([("x", x[0])], kwargs),
             *map(convert_units, x[1:])]
    else:  # horizontal
        convert_units = axes.convert_yunits
        x = [*axes._process_unit_info([("y", x[0])], kwargs),
             *map(convert_units, x[1:])]

    if bin_range is not None:
        bin_range = convert_units(bin_range)

    if not cbook.is_scalar_or_string(bins):
        bins = convert_units(bins)

    # We need to do to 'weights' what was done to 'x'
    if weights is not None:
        w = cbook._reshape_2D(weights, 'weights')
    else:
        w = [None] * nx

    if len(w) != nx:
        raise ValueError('weights should have the same shape as x')

    input_empty = True
    for xi, wi in zip(x, w):
        len_xi = len(xi)
        if wi is not None and len(wi) != len_xi:
            raise ValueError('weights should have the same shape as x')
        if len_xi:
            input_empty = False

    if color is None:
        colors = [axes._get_lines.get_next_color() for i in range(nx)]
    else:
        if mcolors.is_color_like(color):
            color = [color]*nx
        colors = mcolors.to_rgba_array(color)
        if len(colors) != nx:
            raise ValueError(f"The 'color' keyword argument must have one "
                             f"color per dataset, but {nx} datasets and "
                             f"{len(colors)} colors were provided")

    hist_kwargs = dict()

    # if the bin_range is not given, compute without nan numpy
    # does not do this for us when guessing the range (but will
    # happily ignore nans when computing the histogram).
    if bin_range is None:
        xmin = np.inf
        xmax = -np.inf
        for xi in x:
            if len(xi):
                # python's min/max ignore nan,
                # np.minnan returns nan for all nan input
                xmin = min(xmin, np.nanmin(xi))
                xmax = max(xmax, np.nanmax(xi))
        if xmin <= xmax:  # Only happens if we have seen a finite value.
            bin_range = (xmin, xmax)

    # If bins are not specified either explicitly or via range,
    # we need to figure out the range required for all datasets,
    # and supply that to np.histogram.
    if not input_empty and len(x) > 1:
        if weights is not None:
            _w = np.concatenate(w)
        else:
            _w = None
        bins = np.histogram_bin_edges(
            np.concatenate(x), bins, bin_range, _w)
    else:
        hist_kwargs['range'] = bin_range

    density = bool(density)
    if density and not stacked:
        hist_kwargs['density'] = density

    # List to store all the top coordinates of the histograms
    tops = []  # Will have shape (n_datasets, n_bins).
    # Loop through datasets
    for i in range(nx):
        # this will automatically overwrite bins,
        # so that each histogram uses the same bins
        m, bins = np.histogram(x[i], bins, weights=w[i], **hist_kwargs)
        tops.append(m)
    tops = np.array(tops, float)  # causes problems later if it's an int
    bins = np.array(bins, float)  # causes problems if float16
    if stacked:
        tops = tops.cumsum(axis=0)
        # If a stacked density plot, normalize so the area of all the
        # stacked histograms together is 1
        if density:
            tops = (tops / np.diff(bins)) / tops[-1].sum()
    if cumulative:
        slc = slice(None)
        if isinstance(cumulative, Number) and cumulative < 0:
            slc = slice(None, None, -1)
        if density:
            tops = (tops * np.diff(bins))[:, slc].cumsum(axis=1)[:, slc]
        else:
            tops = tops[:, slc].cumsum(axis=1)[:, slc]

    patches = []

    if histtype.startswith('bar'):

        totwidth = np.diff(bins)

        if rwidth is not None:
            dr = np.clip(rwidth, 0, 1)
        elif (len(tops) > 1 and
              ((not stacked) or rcParams['_internal.classic_mode'])):
            dr = 0.8
        else:
            dr = 1.0

        if histtype == 'bar' and not stacked:
            width = dr * totwidth / nx
            dw = width
            boffset = -0.5 * dr * totwidth * (1 - 1 / nx)
        elif histtype == 'barstacked' or stacked:
            width = dr * totwidth
            boffset, dw = 0.0, 0.0

        if align == 'mid':
            boffset += 0.5 * totwidth
        elif align == 'right':
            boffset += totwidth

        if orientation == 'horizontal':
            _barfunc = axes.barh
            bottom_kwarg = 'left'
        else:  # orientation == 'vertical'
            _barfunc = axes.bar
            bottom_kwarg = 'bottom'

        for top, color in zip(tops, colors):
            if bottom is None:
                bottom = np.zeros(len(top))
            if stacked:
                height = top - bottom
            else:
                height = top
            bars = _barfunc(bins[:-1]+boffset, height, width,
                            align='center', log=log,
                            color=color, **{bottom_kwarg: bottom})
            patches.append(bars)
            if stacked:
                bottom = top
            boffset += dw
        # Remove stickies from all bars but the lowest ones, as otherwise
        # margin expansion would be unable to cross the stickies in the
        # middle of the bars.
        for bars in patches[1:]:
            for patch in bars:
                patch.sticky_edges.x[:] = patch.sticky_edges.y[:] = []

    elif histtype.startswith('step'):
        # these define the perimeter of the polygon
        x = np.zeros(4 * len(bins) - 3)
        y = np.zeros(4 * len(bins) - 3)

        x[0:2*len(bins)-1:2], x[1:2*len(bins)-1:2] = bins, bins[:-1]
        x[2*len(bins)-1:] = x[1:2*len(bins)-1][::-1]

        if bottom is None:
            bottom = 0

        y[1:2*len(bins)-1:2] = y[2:2*len(bins):2] = bottom
        y[2*len(bins)-1:] = y[1:2*len(bins)-1][::-1]

        if log:
            if orientation == 'horizontal':
                axes.set_xscale('log', nonpositive='clip')
            else:  # orientation == 'vertical'
                axes.set_yscale('log', nonpositive='clip')

        if align == 'left':
            x -= 0.5*(bins[1]-bins[0])
        elif align == 'right':
            x += 0.5*(bins[1]-bins[0])

        # If fill kwarg is set, it will be passed to the patch collection,
        # overriding this
        fill = (histtype == 'stepfilled')

        xvals, yvals = [], []
        for top in tops:
            if stacked:
                # top of the previous polygon becomes the bottom
                y[2*len(bins)-1:] = y[1:2*len(bins)-1][::-1]
            # set the top of this polygon
            y[1:2*len(bins)-1:2] = y[2:2*len(bins):2] = top + bottom

            # The starting point of the polygon has not yet been
            # updated. So far only the endpoint was adjusted. This
            # assignment closes the polygon. The redundant endpoint is
            # later discarded (for step and stepfilled).
            y[0] = y[-1]

            if orientation == 'horizontal':
                xvals.append(y.copy())
                yvals.append(x.copy())
            else:
                xvals.append(x.copy())
                yvals.append(y.copy())

        # stepfill is closed, step is not
        split = -1 if fill else 2 * len(bins)
        # add patches in reverse order so that when stacking,
        # items lower in the stack are plotted on top of
        # items higher in the stack
        for x, y, color in reversed(list(zip(xvals, yvals, colors))):
            patches.append(axes.fill(
                x[:split], y[:split],
                closed=True if fill else None,
                facecolor=color,
                edgecolor=None if fill else color,
                fill=fill if fill else None,
                zorder=None if fill else mlines.Line2D.zorder))
        for patch_list in patches:
            for patch in patch_list:
                if orientation == 'vertical':
                    patch.sticky_edges.y.append(0)
                elif orientation == 'horizontal':
                    patch.sticky_edges.x.append(0)

        # we return patches, so put it back in the expected order
        patches.reverse()

    # If None, make all labels None (via zip_longest below); otherwise,
    # cast each element to str, but keep a single str as it.
    labels = [] if label is None else np.atleast_1d(np.asarray(label, str))

    if histtype == "step":
        kwargs.setdefault('edgecolor', colors)

    kwargs, style_gen = _style_helpers.style_generator(kwargs)

    for patch, lbl in itertools.zip_longest(patches, labels):
        if not patch:
            continue
        p = patch[0]
        kwargs.update(next(style_gen))
        p._internal_update(kwargs)
        if lbl is not None:
            p.set_label(lbl)
        for p in patch[1:]:
            p._internal_update(kwargs)
            p.set_label('_nolegend_')

    if nx == 1:
        return tops[0], bins, patches[0]
    else:
        patch_type = ("BarContainer" if histtype.startswith("bar")
                      else "list[Polygon]")
        return tops, bins, cbook.silent_list(patch_type, patches)


def ecdf(axes, x, weights=None, *, complementary=False,
         orientation="vertical", compress=False, **kwargs):
    """`~.Axes.ecdf` of *axes* as matplotlib makes it, through its
    `~.Axes.plot`."""
    _api.check_in_list(["horizontal", "vertical"], orientation=orientation)
    if "drawstyle" in kwargs or "ds" in kwargs:
        raise TypeError("Cannot pass 'drawstyle' or 'ds' to ecdf()")
    if np.ma.getmask(x).any():
        raise ValueError("ecdf() does not support masked entries")
    x = np.asarray(x)
    if np.isnan(x).any():
        raise ValueError("ecdf() does not support NaNs")
    argsort = np.argsort(x)
    x = x[argsort]
    if weights is None:
        # Ensure that we end at exactly 1, avoiding floating point errors.
        cum_weights = (1 + np.arange(len(x))) / len(x)
    else:
        weights = np.take(weights, argsort)   # Reorder weights like we reordered x.
        cum_weights = np.cumsum(weights / np.sum(weights))
    if compress:
        # Get indices of unique x values.
        compress_idxs = [0, *(x[:-1] != x[1:]).nonzero()[0] + 1]
        x = x[compress_idxs]
        cum_weights = cum_weights[compress_idxs]
    if orientation == "vertical":
        if not complementary:
            line, = axes.plot([x[0], *x], [0, *cum_weights],
                              drawstyle="steps-post", **kwargs)
        else:
            line, = axes.plot([*x, x[-1]], [1, *1 - cum_weights],
                              drawstyle="steps-pre", **kwargs)
        line.sticky_edges.y[:] = [0, 1]
    else:  # orientation == "horizontal":
        if not complementary:
            line, = axes.plot([0, *cum_weights], [x[0], *x],
                              drawstyle="steps-pre", **kwargs)
        else:
            line, = axes.plot([1, *1 - cum_weights], [*x, x[-1]],
                              drawstyle="steps-post", **kwargs)
        line.sticky_edges.x[:] = [0, 1]
    return line


def psd(axes, x, NFFT=None, Fs=None, Fc=None, detrend=None,
        window=None, noverlap=None, pad_to=None,
        sides=None, scale_by_freq=None, return_line=None, **kwargs):
    """`~.Axes.psd` of *axes* as matplotlib makes it, through its
    `~.Axes.plot`."""
    if Fc is None:
        Fc = 0

    pxx, freqs = mlab.psd(x=x, NFFT=NFFT, Fs=Fs, detrend=detrend,
                          window=window, noverlap=noverlap, pad_to=pad_to,
                          sides=sides, scale_by_freq=scale_by_freq)
    freqs += Fc

    if scale_by_freq in (None, True):
        psd_units = 'dB/Hz'
    else:
        psd_units = 'dB'

    line = axes.plot(freqs, 10 * np.log10(pxx), **kwargs)
    axes.set_xlabel('Frequency')
    axes.set_ylabel('Power Spectral Density (%s)' % psd_units)
    axes.grid(True)

    vmin, vmax = axes.get_ybound()
    step = max(10 * int(np.log10(vmax - vmin)), 1)
    ticks = np.arange(math.floor(vmin), math.ceil(vmax) + 1, step)
    axes.set_yticks(ticks)

    if return_line is None or not return_line:
        return pxx, freqs
    else:
        return pxx, freqs, line


def csd(axes, x, y, NFFT=None, Fs=None, Fc=None, detrend=None,
        window=None, noverlap=None, pad_to=None,
        sides=None, scale_by_freq=None, return_line=None, **kwargs):
    """`~.Axes.csd` of *axes* as matplotlib makes it, through its
    `~.Axes.plot`."""
    if Fc is None:
        Fc = 0

    pxy, freqs = mlab.csd(x=x, y=y, NFFT=NFFT, Fs=Fs, detrend=detrend,
                          window=window, noverlap=noverlap, pad_to=pad_to,
                          sides=sides, scale_by_freq=scale_by_freq)
    # pxy is complex
    freqs += Fc

    line = axes.plot(freqs, 10 * np.log10(np.abs(pxy)), **kwargs)
    axes.set_xlabel('Frequency')
    axes.set_ylabel('Cross Spectrum Magnitude (dB)')
    axes.grid(True)

    vmin, vmax = axes.get_ybound()
    step = max(10 * int(np.log10(vmax - vmin)), 1)
    ticks = np.arange(math.floor(vmin), math.ceil(vmax) + 1, step)
    axes.set_yticks(ticks)

    if return_line is None or not return_line:
        return pxy, freqs
    else:
        return pxy, freqs, line


def magnitude_spectrum(axes, x, Fs=None, Fc=None, window=None,
                       pad_to=None, sides=None, scale=None,
                       **kwargs):
    """`~.Axes.magnitude_spectrum` of *axes* as matplotlib makes it, through its
    `~.Axes.plot`."""
    if Fc is None:
        Fc = 0

    spec, freqs = mlab.magnitude_spectrum(x=x, Fs=Fs, window=window,
                                          pad_to=pad_to, sides=sides)
    freqs += Fc

    yunits = _api.getitem_checked(
        {None: 'energy', 'default': 'energy', 'linear': 'energy',
         'dB': 'dB'},
        scale=scale)
    if yunits == 'energy':
        Z = spec
    else:  # yunits == 'dB'
        Z = 20. * np.log10(spec)

    line, = axes.plot(freqs, Z, **kwargs)
    axes.set_xlabel('Frequency')
    axes.set_ylabel('Magnitude (%s)' % yunits)

    return spec, freqs, line


def angle_spectrum(axes, x, Fs=None, Fc=None, window=None,
                   pad_to=None, sides=None, **kwargs):
    """`~.Axes.angle_spectrum` of *axes* as matplotlib makes it, through its
    `~.Axes.plot`."""
    if Fc is None:
        Fc = 0

    spec, freqs = mlab.angle_spectrum(x=x, Fs=Fs, window=window,
                                      pad_to=pad_to, sides=sides)
    freqs += Fc

    lines = axes.plot(freqs, spec, **kwargs)
    axes.set_xlabel('Frequency')
    axes.set_ylabel('Angle (radians)')

    return spec, freqs, lines[0]


def phase_spectrum(axes, x, Fs=None, Fc=None, window=None,
                   pad_to=None, sides=None, **kwargs):
    """`~.Axes.phase_spectrum` of *axes* as matplotlib makes it, through its
    `~.Axes.plot`."""
    if Fc is None:
        Fc = 0

    spec, freqs = mlab.phase_spectrum(x=x, Fs=Fs, window=window,
                                      pad_to=pad_to, sides=sides)
    freqs += Fc

    lines = axes.plot(freqs, spec, **kwargs)
    axes.set_xlabel('Frequency')
    axes.set_ylabel('Phase (radians)')

    return spec, freqs, lines[0]


def cohere(axes, x, y, NFFT=256, Fs=2, Fc=0, detrend=mlab.detrend_none,
           window=mlab.window_hanning, noverlap=0, pad_to=None,
           sides='default', scale_by_freq=None, **kwargs):
    """`~.Axes.cohere` of *axes* as matplotlib makes it, through its
    `~.Axes.plot`."""
    cxy, freqs = mlab.cohere(x=x, y=y, NFFT=NFFT, Fs=Fs, detrend=detrend,
                             window=window, noverlap=noverlap,
                             scale_by_freq=scale_by_freq, sides=sides,
                             pad_to=pad_to)
    freqs += Fc

    axes.plot(freqs, cxy, **kwargs)
    axes.set_xlabel('Frequency')
    axes.set_ylabel('Coherence')
    axes.grid(True)

    return cxy, freqs
