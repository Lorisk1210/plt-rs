"""
Bars: `~.Axes.bar`, `~.Axes.barh`, `~.Axes.broken_barh` and
`~.Axes.grouped_bar`.
"""

import itertools

import numpy as np

from ... import _docstring, _transform_trees
from ... import _api, _values, cbook, collections as mcoll, colors as mcolors, lines as mlines
from ... import patches as mpatches, units as munits
from ..._program_classes import takes_any_from_program
from ..._native import patches as _patches, plotting as _plotting
from ..._preprocess import _preprocess_data
from ...container import BarContainer, ErrorbarContainer
from ... import _lazy_imports
from ... import _unusable
from ._helpers import floats, record_container, wrap, wrap_all
from .errorbar import _error_bounds

_bars_reference = _lazy_imports.module(globals(), "._bars_reference")


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


def _aligned_span(y0, dy, align):
    """The bottom and top of `~.Axes.broken_barh`'s bars at *y0*, *dy*
    high, aligned there as *align* tells, as matplotlib computes them (a
    height in a list sums to an array, or fails to)."""
    if align == "bottom":
        return y0, y0 + dy
    if align == "center":
        return y0 - dy / 2, y0 + dy / 2
    return y0 - dy, y0


def _broadcast(*values):
    """``np.broadcast_arrays(*values)``, made at once for values of one
    element and at most one dimension (a ``bar`` call per bar), where it is
    each as a one-element array: numpy's broadcasting is Python code whose
    calls differ from release to release."""
    arrays = [np.asarray(value) for value in values]
    if all(array.size == 1 and array.ndim <= 1 for array in arrays):
        return [array.reshape(1) for array in arrays]
    return np.broadcast_arrays(*arrays)


def _bar_geometry(patches, vertical, align, x, y, width, height):
    """
    Keep the geometry of the bars *patches* as matplotlib types it (the
    numpy scalars of the arguments broadcast, then of the left or bottom
    edges centred), and return the values the bars show, as
    `.BarContainer` keeps them (*height*, or *width* for horizontal bars).
    """
    try:
        x, height, width, y = _broadcast(np.atleast_1d(x), height, width, y)
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
            [hatch] if isinstance(hatch, str)
            else ["" if h is None else str(h) for h in hatch])


def _numpy_number(value):
    """Whether *value* is a number numpy makes a number array of, or such
    an array (the dtype kinds `_check_bar_geometry` passes), told by its
    type and dtype alone."""
    kind = type(value)
    if kind is int:
        return -2**63 <= value < 2**63
    if kind is np.ndarray:
        # (`bar` converted the positions to arrays; their shapes broadcast,
        # as `bar` checked before.)
        return value.dtype.kind in "biuf"
    return kind is float or kind is bool or kind is np.float64 or kind is np.int64


def _check_bar_geometry(x, height, width, y, align, orientation):
    """
    Fail as matplotlib's ``bar`` fails with geometry that is no numbers:
    aligning the bars, then making the first bar's rectangle, which adds
    its extent to its corner.
    """
    if all(map(_numpy_number, (x, height, width, y))):
        return  # (numbers numpy keeps as such: nothing fails)
    x, height, width, y = np.broadcast_arrays(np.atleast_1d(x), height, width, y)
    if not x.size or all(a.dtype.kind in "biuf" for a in (x, height, width, y)):
        return
    left, bottom = x, y
    if align == 'center':
        if orientation == 'vertical':
            try:
                left = x - width / 2
            except TypeError as e:
                raise TypeError(f'the dtypes of parameters x ({x.dtype}) '
                                f'and width ({width.dtype}) '
                                f'are incompatible') from e
        else:  # horizontal
            try:
                bottom = y - height / 2
            except TypeError as e:
                raise TypeError(f'the dtypes of parameters y ({y.dtype}) '
                                f'and height ({height.dtype}) '
                                f'are incompatible') from e
    _ = left[0] + width[0], bottom[0] + height[0]


def _plain_groups(heights, positions, colors):
    """Whether pltrs (Rust) makes `~.Axes.grouped_bar`'s bars as
    matplotlib's calls of `~.Axes.bar` make them (else `._bars_reference`
    does): *positions* none or one per group, *colors* none or a sequence
    of single colours, one per dataset, cycled."""
    try:
        groups = len(next(iter(heights)))
    except Exception:
        return False
    return ((positions is None or np.shape(positions) == (groups,))
            and (colors is None or (not isinstance(colors, str) and np.iterable(colors)
                                    and all(mcolors.is_color_like(c) for c in colors))))


def _grouped_bar_checks(heights, positions, spacing, orientation, labels):
    """
    Fail as matplotlib's ``grouped_bar`` fails before it makes its bars:
    datasets of different sizes, *positions* not equidistant, an unknown
    *orientation*, spacings it cannot compute the bars' width and spacing
    with, and *labels* not one per dataset (an assertion), in that order.
    """
    group_spacing, bar_spacing = spacing
    num_datasets = len(heights)
    num_groups = len(next(iter(heights)))  # inferred from first dataset
    if not hasattr(heights, 'shape'):
        for i, dataset in enumerate(heights):
            if len(dataset) != num_groups:
                raise ValueError(
                    "'heights' contains datasets with different number of "
                    f"elements. dataset 0 has {num_groups} elements but "
                    f"dataset {i} has {len(dataset)} elements."
                )
    group_distance = 1
    if positions is not None:
        group_centers = np.asanyarray(positions)
        if len(group_centers) > 1:
            d = np.diff(group_centers)
            if not np.allclose(d, d.mean()):
                raise ValueError("'positions' must be equidistant")
            group_distance = d[0]
    _api.check_in_list(["vertical", "horizontal"], orientation=orientation)
    bar_width = (group_distance /
                 (num_datasets + (num_datasets - 1) * bar_spacing + group_spacing))
    # The spacing between bars and the margins of the groups.
    _ = bar_spacing * bar_width, 0.5 * group_spacing * bar_width
    if labels is not None:
        assert len(labels) == num_datasets


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
    @_docstring.interpd
    def bar(self, x, height, width=0.8, bottom=None, *, align="center",
            **kwargs):
        r"""
        Make a bar plot.

        The bars are positioned at *x* with the given *align*\ment. Their
        dimensions are given by *height* and *width*. The vertical baseline
        is *bottom* (default 0).

        Many parameters can take either a single value applying to all bars
        or a sequence of values, one for each bar.

        Parameters
        ----------
        x : float or array-like
            The x coordinates of the bars. See also *align* for the
            alignment of the bars to the coordinates.

            Bars are often used for categorical data, i.e. string labels below
            the bars. You can provide a list of strings directly to *x*.
            ``bar(['A', 'B', 'C'], [1, 2, 3])`` is often a shorter and more
            convenient notation compared to
            ``bar(range(3), [1, 2, 3], tick_label=['A', 'B', 'C'])``. They are
            equivalent as long as the names are unique. The explicit *tick_label*
            notation draws the names in the sequence given. However, when having
            duplicate values in categorical *x* data, these values map to the same
            numerical x coordinate, and hence the corresponding bars are drawn on
            top of each other.

        height : float or array-like
            The height(s) of the bars.

            Note that if *bottom* has units (e.g. datetime), *height* should be in
            units that are a difference from the value of *bottom* (e.g. timedelta).

        width : float or array-like, default: 0.8
            The width(s) of the bars.

            Note that if *x* has units (e.g. datetime), then *width* should be in
            units that are a difference (e.g. timedelta) around the *x* values.

        bottom : float or array-like, default: 0
            The y coordinate(s) of the bottom side(s) of the bars.

            Note that if *bottom* has units, then the y-axis will get a Locator and
            Formatter appropriate for the units (e.g. dates, or categorical).

        align : {'center', 'edge'}, default: 'center'
            Alignment of the bars to the *x* coordinates:

            - 'center': Center the base on the *x* positions.
            - 'edge': Align the left edges of the bars with the *x* positions.

            To align the bars on the right edge pass a negative *width* and
            ``align='edge'``.

        Returns
        -------
        `.BarContainer`
            Container with all the bars and optionally errorbars.

        Other Parameters
        ----------------
        color : :mpltype:`color` or list of :mpltype:`color`, optional
            The colors of the bar faces. This is an alias for *facecolor*.
            If both are given, *facecolor* takes precedence.

        facecolor : :mpltype:`color` or list of :mpltype:`color`, optional
            The colors of the bar faces.
            If both *color* and *facecolor are given, *facecolor* takes precedence.

        edgecolor : :mpltype:`color` or list of :mpltype:`color`, optional
            The colors of the bar edges.

        linewidth : float or array-like, optional
            Width of the bar edge(s). If 0, don't draw edges.

        tick_label : str or list of str, optional
            The tick labels of the bars.
            Default: None (Use default numeric labels.)

        label : str or list of str, optional
            A single label is attached to the resulting `.BarContainer` as a
            legend label for the whole dataset.
            If a list is provided, it must be the same length as *x* and
            labels the individual bars. Repeated labels are not de-duplicated
            and will cause repeated label entries, so this is best used when
            bars also differ in style (e.g., by passing a list to *color*).

            Tip: Use `.bar_label` to place labels on the bars.

        xerr, yerr : float or array-like of shape(N,) or shape(2, N), optional
            If not *None*, add horizontal / vertical errorbars to the bar tips.
            The values are +/- sizes relative to the data:

            - scalar: symmetric +/- values for all bars
            - shape(N,): symmetric +/- values for each bar
            - shape(2, N): Separate - and + values for each bar. First row
              contains the lower errors, the second row contains the upper
              errors.
            - *None*: No errorbar. (Default)

            This is a convenience shortcut for an extra `~.axes.Axes.errorbar`
            call. See its documentation and
            :doc:`/gallery/statistics/errorbar_features` for an example on
            the usage of *xerr* and *yerr*.

        ecolor : :mpltype:`color` or list of :mpltype:`color`, default: 'black'
            The line color of the errorbars.
            Multiple colors are only supported if the errorbars do not have
            caps. If you need individually colored errorbars with caps, instead
            use explicit `~.axes.Axes.errorbar` calls for each data point.

        capsize : float, default: :rc:`errorbar.capsize`
           The length of the error bar caps in points.

        error_kw : dict, optional
            Dictionary of keyword arguments to be passed to the
            `~.Axes.errorbar` method. Values of *ecolor* or *capsize* defined
            here take precedence over the independent keyword arguments.

        log : bool, default: False
            If *True*, set the y-axis to be log scale.

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs : `.Rectangle` properties

        %(Rectangle:kwdoc)s

        See Also
        --------
        barh : Plot a horizontal bar plot.
        grouped_bar : Plot multiple datasets as grouped bar plot.
        bar_label : Add labels to bars.

        Notes
        -----
        Stacked bars can be achieved by passing individual *bottom* values per
        bar. See :doc:`/gallery/lines_bars_and_markers/bar_stacked`.
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
            # In matplotlib's order, whatever the orientation (`barh` is a
            # `bar` with its left as *x* and its bottom as *y*).
            n = np.broadcast_shapes(np.shape(np.atleast_1d(cbook._unpack_to_numpy(x))),
                                    np.shape(height), np.shape(width), np.shape(y),
                                    np.shape(linewidth), np.shape(hatch))
        except ValueError as e:
            message = str(e)
            for i, name in enumerate(["'x'", "'height'", "'width'", "'y'",
                                      "'linewidth'", "'hatch'"]):
                message = message.replace(f"arg {i}", name)
            raise ValueError(message) from e
        _check_bar_geometry(x, height, width, y, align, orientation)
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
        # A bar of no hatch among hatched ones keeps its place.
        hatches = ([] if hatch is None
                   else ["" if h is None else str(h) for h in np.atleast_1d(hatch)])

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

        native, rectangles, errorbar, values = _plotting.bar(
            self._native, orientation == 'horizontal',
            (floats(position), floats(value)),
            (floats(thickness), floats(base) if has_base else None),
            align, (facecolors, edgecolors, linewidths, hatches),
            # The container's label as text, as `.Container` keeps it.
            ('' if bar_container_label is None else str(bar_container_label),
             patch_labels, [], bool(log)),
            (errors.get("x"), errors.get("y"), error_style,
             mlines._native_kwargs(cbook.normalize_kwargs(error_kw, mlines.Line2D))))
        patches = wrap_all(self, rectangles, mpatches.Rectangle)
        # matplotlib cycles the hatches as numpy strings.
        hatch_cycle = itertools.cycle(np.atleast_1d(hatch))
        # (The kinds read without a Python call: most bars are of floats.)
        kinds = (np.asarray(position).dtype.kind, np.asarray(value).dtype.kind,
                 np.asarray(thickness).dtype.kind, np.asarray(base).dtype.kind)
        complex_extents = _complex_extents(kinds, align) if "c" in kinds else 0
        for patch in patches:
            if hatch is not None:
                _values.remember(patch, "hatch", next(hatch_cycle))
            patch._made_with(kwargs)
            if complex_extents:
                _unusable.keep(patch, "complex extents", complex_extents,
                               _cast_complex_extents, measured=False)
        if patches:
            # matplotlib's ``r.get_path()._interpolation_steps = 100``,
            # through the path all rectangles share.
            _patches.set_shared_rectangle_steps(100)
        datavalues = _bar_geometry(patches, orientation == 'vertical', align,
                                   x, y, width, height)
        _transform_trees.patches_added(self, patches)
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
        record_container(self, container, native)
        if tick_labels is not None:
            # matplotlib's, with the positions as given (integers stay).
            axis = self.xaxis if orientation == 'vertical' else self.yaxis
            axis.set_ticks(np.broadcast_to(position, n))
            axis.set_ticklabels(np.broadcast_to(tick_labels, n))
        return container

    @_docstring.interpd
    def barh(self, y, width, height=0.8, left=None, *, align="center",
             data=None, **kwargs):
        r"""
        Make a horizontal bar plot.

        The bars are positioned at *y* with the given *align*\ment. Their
        dimensions are given by *width* and *height*. The horizontal baseline
        is *left* (default 0).

        Many parameters can take either a single value applying to all bars
        or a sequence of values, one for each bar.

        Parameters
        ----------
        y : float or array-like
            The y coordinates of the bars. See also *align* for the
            alignment of the bars to the coordinates.

            Bars are often used for categorical data, i.e. string labels below
            the bars. You can provide a list of strings directly to *y*.
            ``barh(['A', 'B', 'C'], [1, 2, 3])`` is often a shorter and more
            convenient notation compared to
            ``barh(range(3), [1, 2, 3], tick_label=['A', 'B', 'C'])``. They are
            equivalent as long as the names are unique. The explicit *tick_label*
            notation draws the names in the sequence given. However, when having
            duplicate values in categorical *y* data, these values map to the same
            numerical y coordinate, and hence the corresponding bars are drawn on
            top of each other.

        width : float or array-like
            The width(s) of the bars.

            Note that if *left* has units (e.g. datetime), *width* should be in
            units that are a difference from the value of *left* (e.g. timedelta).

        height : float or array-like, default: 0.8
            The heights of the bars.

            Note that if *y* has units (e.g. datetime), then *height* should be in
            units that are a difference (e.g. timedelta) around the *y* values.

        left : float or array-like, default: 0
            The x coordinates of the left side(s) of the bars.

            Note that if *left* has units, then the x-axis will get a Locator and
            Formatter appropriate for the units (e.g. dates, or categorical).

        align : {'center', 'edge'}, default: 'center'
            Alignment of the base to the *y* coordinates*:

            - 'center': Center the bars on the *y* positions.
            - 'edge': Align the bottom edges of the bars with the *y*
              positions.

            To align the bars on the top edge pass a negative *height* and
            ``align='edge'``.

        Returns
        -------
        `.BarContainer`
            Container with all the bars and optionally errorbars.

        Other Parameters
        ----------------
        color : :mpltype:`color` or list of :mpltype:`color`, optional
            The colors of the bar faces.

        edgecolor : :mpltype:`color` or list of :mpltype:`color`, optional
            The colors of the bar edges.

        linewidth : float or array-like, optional
            Width of the bar edge(s). If 0, don't draw edges.

        tick_label : str or list of str, optional
            The tick labels of the bars.
            Default: None (Use default numeric labels.)

        label : str or list of str, optional
            A single label is attached to the resulting `.BarContainer` as a
            legend label for the whole dataset.
            If a list is provided, it must be the same length as *x* and
            labels the individual bars. Repeated labels are not de-duplicated
            and will cause repeated label entries, so this is best used when
            bars also differ in style (e.g., by passing a list to *color*).

            Tip: Use `.bar_label` to place labels on the bars.

        xerr, yerr : float or array-like of shape(N,) or shape(2, N), optional
            If not *None*, add horizontal / vertical errorbars to the bar tips.
            The values are +/- sizes relative to the data:

            - scalar: symmetric +/- values for all bars
            - shape(N,): symmetric +/- values for each bar
            - shape(2, N): Separate - and + values for each bar. First row
              contains the lower errors, the second row contains the upper
              errors.
            - *None*: No errorbar. (default)

            See :doc:`/gallery/statistics/errorbar_features` for an example on
            the usage of *xerr* and *yerr*.

        ecolor : :mpltype:`color` or list of :mpltype:`color`, default: 'black'
            The line color of the errorbars.

        capsize : float, default: :rc:`errorbar.capsize`
           The length of the error bar caps in points.

        error_kw : dict, optional
            Dictionary of keyword arguments to be passed to the
            `~.Axes.errorbar` method. Values of *ecolor* or *capsize* defined
            here take precedence over the independent keyword arguments.

        log : bool, default: False
            If ``True``, set the x-axis to be log scale.

        data : indexable object, optional
            If given, all parameters also accept a string ``s``, which is
            interpreted as ``data[s]`` if  ``s`` is a key in ``data``.

        **kwargs : `.Rectangle` properties

        %(Rectangle:kwdoc)s

        See Also
        --------
        bar : Plot a vertical bar plot.
        bar_label : Add labels to bars.

        Notes
        -----
        Stacked bars can be achieved by passing individual *left* values per
        bar. See
        :doc:`/gallery/lines_bars_and_markers/horizontal_barchart_distribution`.
        """
        kwargs.setdefault('orientation', 'horizontal')
        patches = self.bar(x=left, height=height, width=width, bottom=y,
                           align=align, data=data, **kwargs)
        return patches

    @_preprocess_data()
    @_docstring.interpd
    def broken_barh(self, xranges, yrange, align="bottom", **kwargs):
        """
        Plot a horizontal sequence of rectangles.

        A rectangle is drawn for each element of *xranges*. All rectangles
        have the same vertical position and size defined by *yrange*.

        Parameters
        ----------
        xranges : sequence of tuples (*xmin*, *xwidth*)
            The x-positions and extents of the rectangles. For each tuple
            (*xmin*, *xwidth*) a rectangle is drawn from *xmin* to *xmin* +
            *xwidth*.
        yrange : (*ypos*, *yheight*)
            The y-position and extent for all the rectangles.
        align : {"bottom", "center", "top"}, default: 'bottom'
            The alignment of the yrange with respect to the y-position. One of:

            - "bottom": Resulting y-range [ypos, ypos + yheight]
            - "center": Resulting y-range [ypos - yheight/2, ypos + yheight/2]
            - "top": Resulting y-range [ypos - yheight, ypos]

            .. versionadded:: 3.11

        Returns
        -------
        `~.collections.PolyCollection`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        **kwargs : `.PolyCollection` properties

            Each *kwarg* can be either a single argument applying to all
            rectangles, e.g.::

                facecolors='black'

            or a sequence of arguments over which is cycled, e.g.::

                facecolors=('black', 'blue')

            would create interleaving black and blue rectangles.

            Supported keywords:

            %(PolyCollection:kwdoc)s
        """
        # process the unit information
        xdata = cbook._safe_first_finite(xranges) if len(xranges) else None
        ydata = cbook._safe_first_finite(yrange) if len(yrange) else None
        self._process_unit_info(
            [("x", xdata), ("y", ydata)], kwargs, convert=False)
        ypos, height = yrange
        # The height converts as bar widths do (`_convert_dx`, whose
        # fallback hands a scalar back in a list).
        y0 = self.convert_yunits(ypos)
        dy = self._convert_dx(height, ypos, np.array(y0), self.convert_yunits)
        _api.check_in_list(['bottom', 'center', 'top'], align=align)
        low, high = _aligned_span(y0, dy, align)
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
        if np.ndim(low) or np.ndim(high):
            # matplotlib's vertices, of a height in a list, are ragged.
            np.array([[(x0, low), (x0, high), (x0 + dx, high), (x0 + dx, low)]
                      for x0, dx in zip(starts, widths)])
        native = _plotting.broken_barh(
            self._native, (floats(starts), floats(widths)),
            (float(y0), float(np.ravel(dy)[0])), align)
        collection = wrap(self, native, mcoll.PolyCollection)
        collection._internal_update(cbook.normalize_kwargs(kwargs, mcoll.PolyCollection))
        return collection

    @_docstring.interpd
    def grouped_bar(self, heights, *, positions=None, group_spacing=1.5, bar_spacing=0,
                    tick_labels=None, labels=None, orientation="vertical", colors=None,
                    **kwargs):
        """
        Make a grouped bar plot.

        .. versionadded:: 3.11

            The API is still provisional. We may still fine-tune some aspects based on
            user-feedback.

        Grouped bar charts visualize a collection of categorical datasets. Each value
        in a dataset belongs to a distinct category and these categories are the same
        across all datasets. The categories typically have string names, but could
        also be dates or index keys. The values in each dataset are represented by a
        sequence of bars of the same color. The bars of all datasets are grouped
        together by their shared categories. The category names are drawn as the tick
        labels for each bar group. Each dataset has a distinct bar color, and can
        optionally get a label that is used for the legend.

        Example:

        .. code-block:: python

           grouped_bar([dataset_0, dataset_1, dataset_2],
                       tick_labels=['A', 'B'],
                       labels=['dataset 0', 'dataset 1', 'dataset 2'])

        .. plot:: _embedded_plots/grouped_bar.py

        Parameters
        ----------
        heights : list of array-like or dict of array-like or 2D array \
or pandas.DataFrame
            The heights for all x and groups. One of:

            - list of array-like: A list of datasets, each dataset must have
              the same number of elements.

              .. code-block:: none

                  #           category_A, category_B
                  dataset_0 = [value_0_A, value_0_B]
                  dataset_1 = [value_1_A, value_1_B]
                  dataset_2 = [value_2_A, value_2_B]

              Example call::

                  grouped_bar([dataset_0, dataset_1, dataset_2])

            - dict of array-like: A mapping from names to datasets. Each dataset
              (dict value) must have the same number of elements.

              Example call:

              .. code-block:: python

                data_dict = {'ds0': dataset_0, 'ds1': dataset_1, 'ds2': dataset_2}
                grouped_bar(data_dict)

              The names are used as *labels*, i.e. this is equivalent to

              .. code-block:: python

                grouped_bar(data_dict.values(), labels=data_dict.keys())

              When using a dict input, you must not pass *labels* explicitly.

            - a 2D array: The rows are the categories, the columns are the different
              datasets.

              .. code-block:: none

                             dataset_0 dataset_1 dataset_2
                 category_A    ds0_a     ds1_a     ds2_a
                 category_B    ds0_b     ds1_b     ds2_b

              Example call:

              .. code-block:: python

                  categories = ["A", "B"]
                  dataset_labels = ["dataset_0", "dataset_1", "dataset_2"]
                  array = np.random.random((2, 3))
                  grouped_bar(array, tick_labels=categories, labels=dataset_labels)

            - a `pandas.DataFrame`.

              The index is used for the categories, the columns are used for the
              datasets.

              .. code-block:: python

                  df = pd.DataFrame(
                      np.random.random((2, 3)),
                      index=["A", "B"],
                      columns=["dataset_0", "dataset_1", "dataset_2"]
                  )
                  grouped_bar(df)

              i.e. this is equivalent to

              .. code-block::

                  grouped_bar(df.to_numpy(), tick_labels=df.index, labels=df.columns)

              Note that ``grouped_bar(df)`` produces a structurally equivalent plot like
              ``df.plot.bar()``.

        positions : array-like, optional
            The center positions of the bar groups. The values have to be equidistant.
            If not given, a sequence of integer positions 0, 1, 2, ... is used.

        tick_labels : list of str, optional
            The category labels, which are placed on ticks at the center *positions*
            of the bar groups. If not set, the axis ticks (positions and labels) are
            left unchanged.

        labels : list of str, optional
            The labels of the datasets, i.e. the bars within one group.
            These will show up in the legend.

        group_spacing : float, default: 1.5
            The space between two bar groups as a multiple of bar width.

            The default value of 1.5 thus means that there's a gap of
            1.5 bar widths between bar groups.

        bar_spacing : float, default: 0
            The space between bars as a multiple of bar width.

        orientation : {"vertical", "horizontal"}, default: "vertical"
            The direction of the bars.

        colors : list of :mpltype:`color`, optional
            A sequence of colors to be cycled through and used to color bars
            of the different datasets. The sequence need not be exactly the
            same length as the number of provided y, in which case the colors
            will repeat from the beginning.

            If not specified, the colors from the Axes property cycle will be used.

        **kwargs : `.Rectangle` properties

            Properties applied to all bars. The following properties additionally
            accept a sequence of values corresponding to the datasets in
            *heights*:

            - *edgecolor*
            - *facecolor*
            - *linewidth*
            - *linestyle*
            - *hatch*

            %(Rectangle:kwdoc)s

        Returns
        -------
        _GroupedBarReturn

            A provisional result object. This will be refined in the future.
            For now, the guaranteed API on the returned object is limited to

            - the attribute ``bar_containers``, which is a list of
              `.BarContainer`, i.e. the results of the individual `~.Axes.bar`
              calls for each dataset.

            - a ``remove()`` method, that remove all bars from the Axes.
              See also `.Artist.remove()`.

        See Also
        --------
        bar : A lower-level API for bar plots, with more degrees of freedom like
              individual bar sizes and colors.

        Notes
        -----
        For a better understanding, we compare the `~.Axes.grouped_bar` API with
        those of `~.Axes.bar` and `~.Axes.boxplot`.

        **Comparison to bar()**

        `~.Axes.grouped_bar` intentionally deviates from the `~.Axes.bar` API in some
        aspects. ``bar(x, y)`` is a lower-level API and places bars with height *y*
        at explicit positions *x*. It also allows to specify individual bar widths
        and colors. This kind of detailed control and flexibility is difficult to
        manage and often not needed when plotting multiple datasets as a grouped bar
        plot. Therefore, ``grouped_bar`` focusses on the abstraction of bar plots
        as visualization of categorical data.

        The following examples may help to transfer from ``bar`` to
        ``grouped_bar``.

        Positions are de-emphasized due to categories, and default to integer values.
        If you have used ``range(N)`` as positions, you can leave that value out::

           bar(range(N), heights)
           grouped_bar([heights])

        If needed, positions can be passed as keyword arguments::

           bar(x, heights)
           grouped_bar([heights], positions=x)

        To place category labels in `~.Axes.bar` you could use the argument
        *tick_label* or use a list of category names as *x*.
        `~.Axes.grouped_bar` expects them in the argument *tick_labels*::

           bar(range(N), heights, tick_label=["A", "B"])
           bar(["A", "B"], heights)
           grouped_bar([heights], tick_labels=["A", "B"])

        Dataset labels, which are shown in the legend, are still passed via the
        *label* parameter::

           bar(..., label="dataset")
           grouped_bar(..., label=["dataset"])

        **Comparison to boxplot()**

        Both, `~.Axes.grouped_bar` and `~.Axes.boxplot` visualize categorical data
        from multiple datasets. The basic API on *tick_labels* and *positions*
        is the same, so that you can easily switch between plotting all
        individual values as `~.Axes.grouped_bar` or the statistical distribution
        per category as `~.Axes.boxplot`::

           grouped_bar(values, positions=..., tick_labels=...)
           boxplot(values, positions=..., tick_labels=...)

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
        if (not _plain_groups(heights, positions, colors)
                or takes_any_from_program(type(self), ("bar", "barh"))):
            # (matplotlib's makes the bars through these.)
            return _GroupedBarReturn(_bars_reference.grouped_bar(
                self, heights, positions, (group_spacing, bar_spacing), tick_labels,
                labels, orientation, colors, kwargs))
        _grouped_bar_checks(heights, positions, (group_spacing, bar_spacing),
                            orientation, labels)
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
                patch._made_with(kwargs)
            if patches:
                # As `bar`, which matplotlib's grouped bars are made with.
                _patches.set_shared_rectangle_steps(100)
            _transform_trees.patches_added(self, patches)
            container = BarContainer(patches, None, datavalues=np.asarray(values),
                                     orientation=orientation,
                                     label='_nolegend_' if label is None else str(label))
            containers.append(record_container(self, container, native))
        return _GroupedBarReturn(containers)


def _complex_extents(kinds, align):
    """
    How many of each bar's four extents are complex numbers, of the arrays
    `bar` made its bars from (the dtype kinds *kinds* of their positions,
    values, widths and bases): matplotlib's `.Rectangle` keeps them, and
    casts them to floats each time it computes its box (``get_bbox``, as
    its draw does), numpy warning of each (`_cast_complex_extents`). The
    near edge is complex with a complex position (or a complex width
    centred on it), the far one with the near one or a complex width; the
    base with a complex base, the top with it or a complex value.
    """
    position, value, thickness, base = (kind == "c" for kind in kinds)
    near = position or (align == "center" and thickness)
    return near + (near or thickness) + base + (base or value)


def _cast_complex_extents(count):
    """numpy's casts of the *count* complex extents of a bar to floats,
    which matplotlib's `.Rectangle` makes each time it is drawn
    (``Bbox.from_extents``), each warning that the imaginary part goes."""
    np.asarray([np.complex128(0)] * count, dtype=float)
