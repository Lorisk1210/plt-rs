"""
Bars: `~.Axes.bar`, `~.Axes.barh`, `~.Axes.broken_barh` and
`~.Axes.grouped_bar`.
"""

import numpy as np

from ... import _api, _values, cbook, collections as mcoll, colors as mcolors, lines as mlines
from ... import patches as mpatches, units as munits
from ..._native import patches as _patches, plotting as _plotting
from ..._preprocess import _preprocess_data
from ...container import BarContainer, ErrorbarContainer
from ._helpers import floats, record_container, wrap, wrap_all
from .errorbar import _error_bounds


def _rcParams():
    """The global rcParams (imported when first needed)."""
    from ... import rcParams
    return rcParams


def _converted_span(convert, start, extent):
    """
    *start* and the *extent* from it as axis numbers, through *convert*:
    numbers stay as they are, values in units convert through the end
    they reach (matplotlib's ``convert((x0, x0 + dx))``).
    """
    if all(munits._is_natively_supported(v) for v in (start, extent)):
        return start, extent
    low, high = convert((start, start + extent))
    return low, high - low


def _bar_geometry(patches, vertical, align, x, y, width, height):
    """
    Keep the geometry of the bars *patches* as matplotlib types it (the
    numpy scalars of the arguments broadcast, then of the left or bottom
    edges centred), and return the values the bars show, as
    `.BarContainer` keeps them (*height*, or *width* for horizontal bars).
    """
    try:
        x, height, width, y = np.broadcast_arrays(np.atleast_1d(x), height, width, y)
        if align == 'center' and vertical:
            x = x - width / 2
        elif align == 'center':
            y = y - height / 2
    except (TypeError, ValueError):
        return None
    for patch, *fields in zip(patches, x, y, width, height):
        _values.remember_all(patch, dict(zip(("x", "y", "width", "height"), fields)),
                             mpatches.Rectangle._FIELDS)
    return height if vertical else width


def _errorbar_style(error_kw):
    """The bar style tuple of the native error bars, popped from *error_kw*."""
    elinestyle = error_kw.pop("elinestyle", None)
    if isinstance(elinestyle, tuple):
        offset, seq = elinestyle
        elinestyle = (offset, None if seq is None else tuple(seq))
    return (error_kw.pop("ecolor", None), error_kw.pop("elinewidth", None),
            elinestyle, error_kw.pop("capsize", None),
            error_kw.pop("capthick", None), bool(error_kw.pop("barsabove", False)))


class _GroupedBarReturn:
    """
    A provisional result object for `.Axes.grouped_bar`.

    This is a placeholder for a future better return type. We try to build in
    backward compatibility / migration possibilities.

    The only public interfaces are the ``bar_containers`` attribute and the
    ``remove()`` method.
    """
    def __init__(self, bar_containers):
        self.bar_containers = bar_containers

    def remove(self):
        [b.remove() for b in self.bar_containers]


# matplotlib defines the class in ``axes._axes``, which imports it from here.
_GroupedBarReturn.__module__ = __name__.rpartition("._plotting")[0] + "._axes"


def _dataset_styles(kwargs):
    """
    The per-dataset styles of `~.Axes.grouped_bar` popped from *kwargs*
    (whose aliases are resolved), in native form: ``(facecolor, edgecolor,
    linewidth, linestyle, hatch)``, each a list cycled over the datasets.
    """
    def colors(key):
        value = kwargs.pop(key, None)
        return [] if value is None else [tuple(c) for c in mcolors.to_rgba_array(value)]

    linewidth = kwargs.pop('linewidth', None)
    linestyle = kwargs.pop('linestyle', None)
    hatch = kwargs.pop('hatch', None)
    return (colors('facecolor'), colors('edgecolor'),
            [] if linewidth is None else [float(w) for w in np.atleast_1d(linewidth)],
            [] if linestyle is None else mcoll._linestyle_specs(linestyle),
            [] if hatch is None else
            [hatch] if isinstance(hatch, str) else [str(h) for h in hatch])


class BarsMixin:
    """`bar`, `barh` and `broken_barh`."""

    def _parse_bar_color_args(self, kwargs):
        """
        Helper function to process color-related arguments of `.Axes.bar`.

        Returns the facecolor as an (N, 4) RGBA array, and the edgecolor
        as given.
        """
        color = kwargs.pop('color', None)

        facecolor = kwargs.pop('facecolor', color)
        edgecolor = kwargs.pop('edgecolor', None)

        facecolor = (facecolor if facecolor is not None
                     else _plotting.next_patch_color(self._native))

        try:
            facecolor = mcolors.to_rgba_array(facecolor)
        except ValueError as err:
            raise ValueError(
                "'facecolor' or 'color' argument must be a valid color or "
                "sequence of colors."
            ) from err

        return facecolor, edgecolor

    @_preprocess_data()
    def bar(self, x, height, width=0.8, bottom=None, *, align="center",
            **kwargs):
        r"""
        Make a bar plot.

        Returns
        -------
        `.BarContainer`
            Container with all the bars and optionally errorbars.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        kwargs = cbook.normalize_kwargs(kwargs, mpatches.Patch)
        facecolor, edgecolor = self._parse_bar_color_args(kwargs)
        linewidth = kwargs.pop('linewidth', None)
        hatch = kwargs.pop('hatch', None)
        xerr = kwargs.pop('xerr', None)
        yerr = kwargs.pop('yerr', None)
        error_kw = kwargs.pop('error_kw', None)
        error_kw = {} if error_kw is None else error_kw.copy()
        ezorder = error_kw.pop('zorder', None)
        if ezorder is None:
            ezorder = kwargs.get('zorder', None)
            if ezorder is not None:
                ezorder += 0.01
        error_kw.setdefault('zorder', ezorder)
        ecolor = kwargs.pop('ecolor', 'k')
        capsize = kwargs.pop('capsize', _rcParams()["errorbar.capsize"])
        error_kw.setdefault('ecolor', ecolor)
        error_kw.setdefault('capsize', capsize)
        orientation = kwargs.pop('orientation', 'vertical')
        _api.check_in_list(['vertical', 'horizontal'], orientation=orientation)
        log = kwargs.pop('log', False)
        label = kwargs.pop('label', '')
        tick_labels = kwargs.pop('tick_label', None)
        _api.check_in_list(['center', 'edge'], align=align)

        y = bottom  # Matches barh call signature.
        if orientation == 'vertical':
            has_base = y is not None
            if y is None:
                y = 0
            # It is possible for y (bottom) to contain unit information.
            # However, it is also possible for y=0 for the default and height
            # to contain unit information.  This will prioritize the units of y.
            self._process_unit_info(
                [("x", x), ("y", y), ("y", height)], kwargs, convert=False)
        else:  # horizontal
            has_base = x is not None
            if x is None:
                x = 0
            self._process_unit_info(
                [("x", x), ("x", width), ("y", y)], kwargs, convert=False)
        # Convert now, since some types cannot be subtracted uniformly.
        x0, y0 = x, y
        x = np.asarray(self.convert_xunits(x))
        width = self._convert_dx(width, x0, x, self.convert_xunits)
        if xerr is not None:
            xerr = self._convert_dx(xerr, x0, x, self.convert_xunits)
        y = np.asarray(self.convert_yunits(y))
        height = self._convert_dx(height, y0, y, self.convert_yunits)
        if yerr is not None:
            yerr = self._convert_dx(yerr, y0, y, self.convert_yunits)

        position, value = (x, height) if orientation == 'vertical' else (y, width)
        thickness, base = (width, y) if orientation == 'vertical' else (height, x)
        position = np.atleast_1d(cbook._unpack_to_numpy(position))
        try:
            n = np.broadcast_shapes(position.shape, np.shape(value),
                                    np.shape(thickness), np.shape(base),
                                    np.shape(linewidth), np.shape(hatch))
        except ValueError as e:
            names = (["'x'", "'height'", "'width'", "'y'"] if orientation == 'vertical'
                     else ["'y'", "'width'", "'height'", "'x'"])
            message = str(e)
            for i, name in enumerate(names + ["'linewidth'", "'hatch'"]):
                message = message.replace(f"arg {i}", name)
            raise ValueError(message) from e
        n = int(np.prod(n)) if n else 1

        if not isinstance(label, str) and np.iterable(label):
            bar_container_label = '_nolegend_'
            patch_labels = [str(lbl) for lbl in label]
        else:
            bar_container_label = label
            patch_labels = []
        if patch_labels and len(patch_labels) != n:
            raise ValueError(f'number of labels ({len(patch_labels)}) '
                             f'does not match number of bars ({n}).')

        edgecolors = ([] if edgecolor is None
                      else [tuple(c) for c in mcolors.to_rgba_array(edgecolor)])
        # 'none' converts to no colours; the bars then stay transparent
        # whatever their alpha, as matplotlib's fall back to 'none'.
        if edgecolor is not None and not edgecolors:
            edgecolors = ['none']
        facecolors = [tuple(c) for c in facecolor] or ['none']
        linewidths = ([] if linewidth is None
                      else [float(lw) for lw in np.atleast_1d(linewidth)])
        hatches = ([] if hatch is None
                   else [str(h) for h in np.atleast_1d(hatch) if h is not None])

        errors = {}
        if xerr is not None or yerr is not None:
            ex, ey = [np.zeros(n)] * 2
            for name, err, dep in (("x", xerr, ex), ("y", yerr, ey)):
                if err is not None:
                    if not isinstance(err, np.ndarray):
                        err = np.asarray(err, dtype=object)
                    errors[name] = _error_bounds(name, dep, err)
        error_label = error_kw.pop("label", '_nolegend_')
        error_style = _errorbar_style(error_kw)
        error_kw = {k: v for k, v in error_kw.items() if v is not None}
        error_kw["label"] = error_label
        if tick_labels is not None:
            tick_labels = [str(t) for t in np.broadcast_to(tick_labels, n)]

        native, rectangles, errorbar, values = _plotting.bar(
            self._native, orientation == 'horizontal',
            (floats(position), floats(value)),
            (floats(thickness), floats(base) if has_base else None),
            align, (facecolors, edgecolors, linewidths, hatches),
            # The container's label as text, as `.Container` keeps it.
            ('' if bar_container_label is None else str(bar_container_label),
             patch_labels, tick_labels or [], bool(log)),
            (errors.get("x"), errors.get("y"), error_style,
             mlines._native_kwargs(cbook.normalize_kwargs(error_kw, mlines.Line2D))))
        patches = wrap_all(self, rectangles, mpatches.Rectangle)
        for patch in patches:
            patch._internal_update(kwargs)
        if patches:
            # matplotlib's ``r.get_path()._interpolation_steps = 100``,
            # through the path all rectangles share.
            _patches.set_shared_rectangle_steps(100)
        datavalues = _bar_geometry(patches, orientation == 'vertical', align,
                                   x, y, width, height)
        errorbar_container = None
        if errorbar is not None:
            enative, data_line, caplines, barcols = errorbar
            errorbar_container = record_container(self, ErrorbarContainer(
                (wrap(self, data_line, mlines.Line2D),
                 tuple(wrap_all(self, caplines, mlines.Line2D)),
                 tuple(wrap_all(self, barcols, mcoll.LineCollection))),
                has_xerr=xerr is not None, has_yerr=yerr is not None,
                label=error_label), enative)
        container = BarContainer(patches, errorbar_container,
                                 datavalues=(np.asarray(values) if datavalues is None
                                             else datavalues),
                                 orientation=orientation,
                                 label=bar_container_label)
        return record_container(self, container, native)

    def barh(self, y, width, height=0.8, left=None, *, align="center",
             data=None, **kwargs):
        r"""
        Make a horizontal bar plot.

        Returns
        -------
        `.BarContainer`
            Container with all the bars and optionally errorbars.
        """
        kwargs.setdefault('orientation', 'horizontal')
        patches = self.bar(x=left, height=height, width=width, bottom=y,
                           align=align, data=data, **kwargs)
        return patches

    @_preprocess_data()
    def broken_barh(self, xranges, yrange, align="bottom", **kwargs):
        """
        Plot a horizontal sequence of rectangles.

        Returns
        -------
        `~.collections.PolyCollection`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        # process the unit information
        xdata = cbook._safe_first_finite(xranges) if len(xranges) else None
        ydata = cbook._safe_first_finite(yrange) if len(yrange) else None
        self._process_unit_info(
            [("x", xdata), ("y", ydata)], kwargs, convert=False)
        _api.check_in_list(['bottom', 'center', 'top'], align=align)
        starts, widths = [], []
        for xr in xranges:  # convert the absolute values, not the x and dx
            try:
                x0, dx = xr
            except Exception:
                raise ValueError(
                    "each range in xrange must be a sequence with two "
                    "elements (i.e. xrange must be an (N, 2) array)") from None
            x0, dx = _converted_span(self.convert_xunits, x0, dx)
            starts.append(x0)
            widths.append(dx)
        ypos, height = yrange
        # The height converts as bar widths do (`_convert_dx`, whose
        # fallback hands a scalar back in a list).
        y0 = self.convert_yunits(ypos)
        dy = self._convert_dx(height, ypos, np.array(y0), self.convert_yunits)
        native = _plotting.broken_barh(
            self._native, (floats(starts), floats(widths)),
            (float(y0), float(np.ravel(dy)[0])), align)
        collection = wrap(self, native, mcoll.PolyCollection)
        collection._internal_update(cbook.normalize_kwargs(kwargs, mcoll.PolyCollection))
        return collection

    def grouped_bar(self, heights, *, positions=None, group_spacing=1.5, bar_spacing=0,
                    tick_labels=None, labels=None, orientation="vertical", colors=None,
                    **kwargs):
        """
        Make a grouped bar plot.

        Returns
        -------
        _GroupedBarReturn
            A provisional result object, whose ``bar_containers`` attribute
            is a list of `.BarContainer` (one per dataset) and whose
            ``remove()`` method removes all bars from the Axes.
        """
        if cbook._is_pandas_dataframe(heights):
            if labels is None:
                labels = heights.columns.tolist()
            if tick_labels is None:
                tick_labels = heights.index.tolist()
            heights = heights.to_numpy().T
        elif hasattr(heights, 'keys'):  # dict
            if labels is not None:
                raise ValueError("'labels' cannot be used if 'heights' is a mapping")
            labels = heights.keys()
            heights = list(heights.values())
        elif hasattr(heights, 'shape'):  # numpy array
            heights = heights.T
        _api.check_in_list(["vertical", "horizontal"], orientation=orientation)
        # Where the bars start, `bar`'s *bottom* (`barh`'s *left*), which
        # matplotlib's grouped_bar hands its bars.
        base = kwargs.pop("bottom" if orientation == "vertical" else "left", None)
        kwargs = cbook.normalize_kwargs(kwargs, mpatches.Rectangle)
        facecolor, edgecolor, linewidth, linestyle, hatch = _dataset_styles(kwargs)
        datasets = [[float(v) for v in dataset] for dataset in heights]
        bars = _plotting.grouped_bar(
            self._native, datasets,
            (None if positions is None else [float(p) for p in np.asanyarray(positions)],
             None if base is None else [float(v) for v in np.atleast_1d(base)]),
            (float(group_spacing), float(bar_spacing)),
            ([] if tick_labels is None else [str(t) for t in tick_labels],
             [] if labels is None else [str(label) for label in labels]),
            orientation == "horizontal",
            ([] if colors is None else [tuple(c) for c in mcolors.to_rgba_array(colors)],
             facecolor, edgecolor, linewidth, linestyle, hatch))
        containers = []
        for (native, rectangles, values), label in zip(
                bars, [None] * len(bars) if labels is None else list(labels)):
            patches = wrap_all(self, rectangles, mpatches.Rectangle)
            for patch in patches:
                patch._internal_update(kwargs)
            if patches:
                # As `bar`, which matplotlib's grouped bars are made with.
                _patches.set_shared_rectangle_steps(100)
            container = BarContainer(patches, None, datavalues=np.asarray(values),
                                     orientation=orientation,
                                     label='_nolegend_' if label is None else str(label))
            containers.append(record_container(self, container, native))
        return _GroupedBarReturn(containers)
