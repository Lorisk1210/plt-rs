"""
Classes for the ticks and x- and y-axis (matplotlib's ``axis``).

pltrs (Rust) keeps every axis of an axes, ticks and tick helpers
included, and draws it. An `Axis` reaches its pltrs (Rust) axis through
its axes and its name: its locators and formatters are matplotlib's
objects kept by `._tick_helpers`, and its ticks, label and offset text are
artists mirroring what pltrs (Rust) draws (`._axis_artists`).
"""

import functools

import numpy as np

from . import _api, _artist_clipping, _tick_helpers, cbook, units as munits
from ._axis_artists import AxisLabel, OffsetText, Tick, XTick, YTick
from ._native import axes as _axes, axis as _axis, axes_base as _axes_base
from ._native import scale as _native_scale
from .artist import Artist

__all__ = ["Tick", "XTick", "YTick", "Ticker", "Axis", "XAxis", "YAxis"]

#: The number of segments of a gridline curved by its projection (polar
#: axes' circles), as matplotlib interpolates them.
GRIDLINE_INTERPOLATION_STEPS = 180

#: The most tick objects an axis makes of a level: asked for more, it
#: refuses, where matplotlib makes ticks until memory runs out.
_MAX_TICK_OBJECTS = 100_000

# The Line2D properties gridlines take as ``grid_<name>`` tick parameters.
_gridline_param_names = [
    'grid_' + name for name in (
        "agg_filter", "alpha", "animated", "antialiased", "clip_box",
        "clip_on", "clip_path", "color", "dash_capstyle", "dash_joinstyle",
        "dashes", "data", "drawstyle", "figure", "fillstyle", "gapcolor",
        "gid", "in_layout", "label", "linestyle", "linewidth", "marker",
        "markeredgecolor", "markeredgewidth", "markerfacecolor",
        "markerfacecoloralt", "markersize", "markevery", "mouseover",
        "path_effects", "picker", "pickradius", "rasterized", "sketch_params",
        "snap", "solid_capstyle", "solid_joinstyle", "transform", "url",
        "visible", "xdata", "ydata", "zorder",
        "aa", "c", "ds", "ls", "lw", "mec", "mew", "mfc", "mfcalt", "ms")]


def _typed_locs(locator, locs, minor):
    """The tick locations *locs* pltrs (Rust) computed with *locator*, as
    matplotlib's locator gives them: a `.FixedLocator` placing all its
    locations gives its array (the minor ones on major ticks left out, in
    its dtype), a categorical one the list of the category numbers, a
    radial one of a full circle the list of its ticks."""
    from . import ticker as mticker
    from .category import StrCategoryLocator
    from .projections.polar import RadialLocator
    if isinstance(locator, RadialLocator) and locator._keeps_off_origin():
        return list(np.asarray(locs))
    if isinstance(locator, StrCategoryLocator) and not minor:
        values = list(locator._units.values())
        return values if np.array_equal(np.asarray(values, float), locs) else locs
    if (isinstance(locator, mticker.FixedLocator) and locator.nbins is None
            and np.issubdtype(locator.locs.dtype, np.number)):
        given = locator.locs
        if not minor and np.array_equal(given.astype(float), locs):
            return given
        if np.isin(locs, given.astype(float)).all():
            return np.asarray(locs).astype(given.dtype)
    return locs


def _native_scale_of(scale):
    """The name and options of pltrs (Rust)'s scale matching the scale
    object *scale*, None for a scale of the program's."""
    from . import scale as mscale
    kind = type(scale)
    if kind is mscale.LinearScale:
        return "linear", {}
    if kind is mscale.LogScale:
        nonpositive = "clip" if scale._transform._clip else "mask"
        return "log", {"base": scale.base, "subs": _floats(scale.subs),
                       "nonpositive": nonpositive}
    if kind is mscale.SymmetricalLogScale:
        transform = scale._transform
        return "symlog", {"base": transform.base,
                          "linthresh": transform.linthresh,
                          "linscale": transform.linscale,
                          "subs": _floats(scale.subs)}
    if kind is mscale.LogitScale:
        return "logit", {"nonpositive": scale._transform._nonpositive,
                         "one_half": scale._one_half,
                         "use_overline": scale._use_overline}
    if kind is mscale.AsinhScale:
        return "asinh", {"linear_width": scale.linear_width,
                         "base": scale._base, "subs": _floats(scale._subs)}
    if kind is mscale.FuncScale:
        transform = scale._transform
        return "function", {"functions": _array_functions(transform)}
    if kind is mscale.FuncScaleLog:
        return "functionlog", {"functions": _array_functions(scale._transform._a),
                               "base": scale.base}
    return None


def _native_converter_kind(converter):
    """The kind of pltrs (Rust)'s converter *converter* is: dates with
    ``"auto"`` or ``"concise"`` ticks, ``"category"`` names, or None for a
    converter of the program's."""
    from . import category, dates
    if isinstance(converter, dates.ConciseDateConverter):
        return "concise"
    if isinstance(converter, dates.DateConverter):
        return "auto"
    if isinstance(converter, category.StrCategoryConverter):
        return "category"
    return None


def _is_native_converter(converter):
    """Whether pltrs (Rust) converts what *converter* converts (dates, as
    :rc:`date.converter` says, and category names), which None, no
    converter, stands for too. Another date converter (a
    `.ConciseDateConverter` a program registered) converts as any
    converter of the program's."""
    from . import category, dates
    if type(converter) is dates.DateConverter:
        return converter._interval_multiples
    return converter is None or isinstance(
        converter, (dates._SwitchableDateConverter, category.StrCategoryConverter))


def _keeps_limit_types(locator):
    """Whether the limits an axis sets keep the types they were given in
    under *locator*: matplotlib's date, log and logit locators make them
    nonsingular without making them floats, as the others do."""
    from . import dates, ticker
    return isinstance(locator, (dates.DateLocator, ticker.LogLocator,
                                ticker.LogitLocator))


def _converted_limit(axis, value):
    """The limit *value* of *axis* as matplotlib keeps it: a plain number
    as it is, a value in units converted (a numpy scalar)."""
    if munits._is_natively_supported(value):
        return value
    converted = axis.convert_units(value)
    return np.asarray(converted)[()] if np.ndim(converted) == 0 else converted


def _first_value(data):
    """The first finite value of *data* as a list of one, or *data* itself
    when it is an empty array (whose type says what it holds)."""
    data = cbook._unpack_to_numpy(data)
    if not np.iterable(data):
        return [data]
    if isinstance(data, np.ndarray):
        if not data.size:
            return data
        data = data.ravel()
    return [cbook._safe_first_finite(data)]


def _floats(values):
    """*values* as a list of floats, None staying None."""
    return None if values is None else [float(v) for v in np.ravel(values)]


def _array_functions(transform):
    """The forward and inverse functions of a `.scale.FuncTransform`, which
    pltrs (Rust) calls with arrays, as matplotlib's transform does."""
    return (transform._forward, transform._inverse)


def _interval_method(kind):
    """
    Name the decorated interval method as matplotlib's
    ``_make_getset_interval`` names the *kind* (``'getter'`` or
    ``'setter'``) it makes, which Python's errors for a wrong call give
    (``... setter() missing 1 required positional argument: 'vmax'``).
    """
    def name(method):
        method.__qualname__ = f"_make_getset_interval.<locals>.{kind}"
        return method
    return name


class Ticker:
    """
    A container for the objects defining tick position and format.

    The tickers of an `Axis` stand for the locator and the formatter of one
    of its tick levels, which pltrs (Rust) keeps; a ticker made alone
    (``Ticker()``, as seaborn's scales make them) holds its own.

    Attributes
    ----------
    locator : `~matplotlib.ticker.Locator` subclass
        Determines the positions of the ticks.
    formatter : `~matplotlib.ticker.Formatter` subclass
        Determines the format of the tick labels.
    """

    def __init__(self):
        self._axis = None
        self._minor = False
        self._own = {"locator": None, "formatter": None}

    @classmethod
    def _of_axis(cls, axis, minor):
        """The ticker of the major (or *minor*) ticks of *axis*."""
        ticker = cls()
        ticker._axis = axis
        ticker._minor = minor
        return ticker

    def _replace(self, kind, helper):
        """Put *helper* in place of the current helper of its kind."""
        if self._axis is None:
            self._own[kind] = helper
            return
        _, token = _axis.helper(self._axis.axes._native, self._axis.axis_name,
                                kind, self._minor)
        _tick_helpers._install(self._axis, kind, self._minor, helper, token)

    def _current(self, kind):
        """The helper of *kind* in use."""
        if self._axis is None:
            return self._own[kind]
        return _tick_helpers.get(self._axis, kind, self._minor)

    @property
    def locator(self):
        return self._current("locator")

    @locator.setter
    def locator(self, locator):
        from .ticker import Locator
        if not isinstance(locator, Locator):
            raise TypeError('locator must be a subclass of '
                            'matplotlib.ticker.Locator')
        self._replace("locator", locator)

    @property
    def formatter(self):
        return self._current("formatter")

    @formatter.setter
    def formatter(self, formatter):
        from .ticker import Formatter
        if not isinstance(formatter, Formatter):
            raise TypeError('formatter must be a subclass of '
                            'matplotlib.ticker.Formatter')
        self._replace("formatter", formatter)


class Axis(Artist):
    """
    Base class for `.XAxis` and `.YAxis`.

    Attributes
    ----------
    axes : `~matplotlib.axes.Axes`
        The `~.axes.Axes` to which the Axis belongs.
    major : `~matplotlib.axis.Ticker`
        Determines the major tick positions and their label format.
    minor : `~matplotlib.axis.Ticker`
        Determines the minor tick positions and their label format.
    callbacks : `~matplotlib.cbook.CallbackRegistry`
    label : `~matplotlib.text.Text`
        The axis label.
    labelpad : float
        The distance between the axis label and the tick labels.
    offsetText : `~matplotlib.text.Text`
        A `.Text` object containing the data offset of the ticks (if any).
    majorTicks : list of `.Tick`
        The major ticks.
    minorTicks : list of `.Tick`
        The minor ticks.
    """

    OFFSETTEXTPAD = 3
    axis_name = None
    _tick_class = None
    #: Whether pltrs (Rust) converts the values in units plotted along the
    #: axis itself, so that they pass as they are (the axes of 3-D axes).
    _native_units = False

    converter = _api.deprecate_privatize_attribute(
                    "3.10",
                    alternative="get_converter and set_converter methods"
                )

    def __str__(self):
        return "{}({},{})".format(
            type(self).__name__, *self.axes.transAxes.transform((0, 0)))

    def __init__(self, axes, *, pickradius=15, clear=True):
        """
        Parameters
        ----------
        axes : `~matplotlib.axes.Axes`
            The `~.axes.Axes` to which the created Axis belongs.
        pickradius : float
            The acceptance radius for containment tests.
        clear : bool, default: True
            Whether matplotlib clears the Axis on creation; the axis of a
            pltrs (Rust) axes is in its default state when the axes is made,
            either way.
        """
        super().__init__()
        self._axes = axes
        self.major = Ticker._of_axis(self, minor=False)
        self.minor = Ticker._of_axis(self, minor=True)
        self.callbacks = cbook.CallbackRegistry(signals=["units"])
        self.pickradius = pickradius
        #: The description pltrs (Rust) gave of the axis while its tick
        #: helpers tick it (see `._tick_helpers.drawing`).
        self._drawn_info = None
        self._ticks = {False: [], True: []}
        self._tick_kw = {False: {}, True: {}}
        self._label = AxisLabel(self)
        self._offset_text = OffsetText(self)
        self._units = None
        self._category_units = None
        #: The scale object the axis was last given (see `_scale`).
        self._given_scale = None
        self._converter = None
        self._converter_is_explicit = False

    @property
    def axes(self):
        """The `~.axes.Axes` the axis belongs to."""
        return self._axes

    #: The zorder of an axis by the ``axisbelow`` of its Axes (matplotlib's
    #: ``set_axisbelow`` sets it on the axes of the Axes).
    _AXISBELOW_ZORDER = {True: 0.5, "line": 1.5, False: 2.5}

    def get_zorder(self):
        """Return the zorder of the axis: the one set, else the one the
        ``axisbelow`` of its Axes gives it."""
        set_ = self.__dict__.get("_zorder_set")
        if set_ is not None:
            return set_
        return self._AXISBELOW_ZORDER[self.axes.get_axisbelow()]

    def set_zorder(self, level):
        # docstring inherited
        if level is None:
            level = type(self).zorder
        if level != self.get_zorder():
            self._zorder_set = level
            self.pchanged()
            self.stale = True

    zorder = property(get_zorder, set_zorder)

    @axes.setter
    def axes(self, axes):
        self._axes = axes

    @property
    def figure(self):
        """The root `.Figure` of the axis' axes."""
        return self.axes.figure

    def get_figure(self, root=False):
        """Return the `.Figure` or `.SubFigure` the axis is on."""
        return self.axes.get_figure(root=root)

    def get_clip_box(self):
        """Return the clipbox: that of the Axes' patch for its x and y axes
        unless set (matplotlib's ``Axes.clear`` clips those to it)."""
        if "_clipbox" in self.__dict__ or self.axis_name not in ("x", "y"):
            return super().get_clip_box()
        return _artist_clipping.patch_clip_box(self, self.axes)

    def _args(self):
        """The native axes and the axis name."""
        return self.axes._native, self.axis_name

    # --- Tick helpers --------------------------------------------------------

    def _default(self, minor, kind):
        return _axis.is_default_helper(*self._args(), kind, bool(minor))

    @property
    def isDefault_majloc(self):
        return self._default(False, "locator")

    @property
    def isDefault_majfmt(self):
        return self._default(False, "formatter")

    @property
    def isDefault_minloc(self):
        return self._default(True, "locator")

    @property
    def isDefault_minfmt(self):
        return self._default(True, "formatter")

    @property
    def isDefault_label(self):
        """Whether the label text was never set, so that the unit
        converter's label replaces it."""
        return _axis.is_default_label(*self._args())

    @property
    def label_position(self):
        """The side the label is on (see `get_label_position`)."""
        return self.get_label_position()

    @label_position.setter
    def label_position(self, position):
        self.set_label_position(position)

    def get_major_locator(self):
        """Get the locator of the major ticker."""
        return self.major.locator

    def get_minor_locator(self):
        """Get the locator of the minor ticker."""
        return self.minor.locator

    def get_major_formatter(self):
        """Get the formatter of the major ticker."""
        return self.major.formatter

    def get_minor_formatter(self):
        """Get the formatter of the minor ticker."""
        return self.minor.formatter

    def set_major_formatter(self, formatter):
        """
        Set the formatter of the major ticker.

        In addition to a `~matplotlib.ticker.Formatter` instance,
        this also accepts a ``str`` or function.

        For a ``str`` a `~matplotlib.ticker.StrMethodFormatter` is used.
        The field used for the value must be labeled ``'x'`` and the field used
        for the position must be labeled ``'pos'``.

        For a function, a `~matplotlib.ticker.FuncFormatter` is used.
        The function must take two inputs (a tick value ``x`` and a
        position ``pos``), and return a string containing the corresponding
        tick label.

        Parameters
        ----------
        formatter : `~matplotlib.ticker.Formatter`, ``str``, or function
        """
        self._set_formatter(formatter, minor=False)

    def set_minor_formatter(self, formatter):
        """
        Set the formatter of the minor ticker.

        In addition to a `~matplotlib.ticker.Formatter` instance,
        this also accepts a ``str`` or function.
        See `.Axis.set_major_formatter` for more information.

        Parameters
        ----------
        formatter : `~matplotlib.ticker.Formatter`, ``str``, or function
        """
        self._set_formatter(formatter, minor=True)

    def _set_formatter(self, formatter, minor):
        from . import ticker as mticker
        if isinstance(formatter, str):
            formatter = mticker.StrMethodFormatter(formatter)
        # Don't allow any other TickHelper to avoid easy-to-make errors,
        # like using a Locator instead of a Formatter.
        elif (callable(formatter) and
              not isinstance(formatter, mticker.TickHelper)):
            formatter = mticker.FuncFormatter(formatter)
        else:
            _api.check_isinstance(mticker.Formatter, formatter=formatter)
        formatter.set_axis(self)
        _tick_helpers.install(self, "formatter", minor, formatter)

    def set_major_locator(self, locator):
        """
        Set the locator of the major ticker.

        Parameters
        ----------
        locator : `~matplotlib.ticker.Locator`
        """
        self._set_locator(locator, minor=False)

    def set_minor_locator(self, locator):
        """
        Set the locator of the minor ticker.

        Parameters
        ----------
        locator : `~matplotlib.ticker.Locator`
        """
        self._set_locator(locator, minor=True)

    def _set_locator(self, locator, minor):
        from . import ticker as mticker
        _api.check_isinstance(mticker.Locator, locator=locator)
        locator.set_axis(self)
        _tick_helpers.install(self, "locator", minor, locator)
        formatter = self.minor.formatter if minor else self.major.formatter
        formatter._set_locator(locator)

    def minorticks_on(self):
        """
        Display default minor ticks on the Axis, depending on the scale
        (`~.axis.Axis.get_scale`).

        Scales use specific minor locators:

        - log: `~.LogLocator`
        - symlog: `~.SymmetricalLogLocator`
        - asinh: `~.AsinhLocator`
        - logit: `~.LogitLocator`
        - default: `~.AutoMinorLocator`

        Displaying minor ticks may reduce performance; you may turn them off
        using `minorticks_off()` if drawing speed is a problem.
        """
        _axis.minorticks(*self._args(), True)

    def minorticks_off(self):
        """Remove minor ticks from the Axis."""
        _axis.minorticks(*self._args(), False)

    def get_remove_overlapping_locs(self):
        return _axis.get_remove_overlapping_locs(*self._args())

    def set_remove_overlapping_locs(self, val):
        _axis.set_remove_overlapping_locs(*self._args(), bool(val))

    remove_overlapping_locs = property(
        get_remove_overlapping_locs, set_remove_overlapping_locs,
        doc=('If minor ticker locations that overlap with major '
             'ticker locations should be trimmed.'))

    # --- Intervals -----------------------------------------------------------

    def _drawn_fields(self):
        """The fields of the description pltrs (Rust) is drawing the axis
        with, or None outside a draw."""
        if self._drawn_info is None:
            return None
        return _axis.info_fields(self._drawn_info)

    def _native_info(self):
        """What the axis' tick helpers know of it (an `AxisInfo`)."""
        if self._drawn_info is not None:
            return self._drawn_info
        return _axis.axis_info(*self._args())

    @_interval_method("getter")
    def get_view_interval(self):
        """Return the ``(min, max)`` view limits of this axis."""
        fields = self._drawn_fields()
        if fields is not None:
            return np.array(fields[0])
        return np.array(_axis.get_view_interval(*self._args()))

    @_interval_method("setter")
    def set_view_interval(self, vmin, vmax, ignore=False):
        """
        Set the axis view limits.  This method is for internal use; Matplotlib
        users should typically use e.g. `~.Axes.set_xlim` or `~.Axes.set_ylim`.

        If *ignore* is False (the default), this method will never reduce the
        preexisting view limits, only expand them if *vmin* or *vmax* are not
        within them.  Moreover, the order of *vmin* and *vmax* does not matter;
        the orientation of the axis will not change.

        If *ignore* is True, the view limits will be set exactly to ``(vmin,
        vmax)`` in that order.
        """
        _axis.set_view_interval(*self._args(), float(vmin), float(vmax),
                                bool(ignore))

    @_interval_method("getter")
    def get_data_interval(self):
        """Return the ``(min, max)`` data limits of this axis."""
        fields = self._drawn_fields()
        if fields is not None:
            return np.array(fields[1])
        return np.array(_axis.get_data_interval(*self._args()))

    @_interval_method("setter")
    def set_data_interval(self, vmin, vmax, ignore=False):
        """
        Set the axis data limits.  This method is for internal use.

        If *ignore* is False (the default), this method will never reduce the
        preexisting data limits, only expand them if *vmin* or *vmax* are not
        within them.  Moreover, the order of *vmin* and *vmax* does not matter;
        the orientation of the axis will not change.

        If *ignore* is True, the data limits will be set exactly to ``(vmin,
        vmax)`` in that order.
        """
        _axis.set_data_interval(*self._args(), float(vmin), float(vmax),
                                bool(ignore))

    def get_minpos(self):
        """Return the smallest positive datum along the axis."""
        fields = self._drawn_fields()
        if fields is not None:
            return fields[2]
        # matplotlib reads it from the data limits, a numpy box.
        return np.float64(_axis.get_minpos(*self._args()))

    def get_tick_space(self):
        """Return the estimated number of ticks that can fit on the axis."""
        fields = self._drawn_fields()
        if fields is not None:
            return fields[3]
        return _axis.get_tick_space(*self._args())

    def get_inverted(self):
        """
        Return whether this Axis is oriented in the "inverse" direction.

        The "normal" direction is increasing to the right for the x-axis and to
        the top for the y-axis; the "inverse" direction is increasing to the
        left for the x-axis and to the bottom for the y-axis.
        """
        low, high = self.get_view_interval()
        return high < low

    def set_inverted(self, inverted):
        """
        Set whether this Axis is oriented in the "inverse" direction.

        The "normal" direction is increasing to the right for the x-axis and to
        the top for the y-axis; the "inverse" direction is increasing to the
        left for the x-axis and to the bottom for the y-axis.
        """
        _axis.set_inverted(*self._args(), bool(inverted))

    def _checked_limits(self, v0, v1):
        """
        The view limits *v0*, *v1* as numbers of the axis, warning as
        matplotlib's ``Axis._set_lim`` does about the ones it cannot take
        (non-positive limits on a log axis keep the current ones).
        """
        v0, v1 = (float(np.asarray(self.convert_units(v), dtype=float))
                  for v in (v0, v1))
        name = self.axis_name
        if self.get_scale() == 'log' and (v0 <= 0 or v1 <= 0):
            old0, old1 = self.get_view_interval()
            if v0 <= 0:
                _api.warn_external(f"Attempt to set non-positive {name}lim on "
                                   f"a log-scaled axis will be ignored.")
                v0 = old0
            if v1 <= 0:
                _api.warn_external(f"Attempt to set non-positive {name}lim on "
                                   f"a log-scaled axis will be ignored.")
                v1 = old1
        if v0 == v1:
            _api.warn_external(
                f"Attempting to set identical low and high {name}lims "
                f"makes transformation singular; automatically expanding.")
        return v0, v1

    def _set_lim(self, v0, v1, *, emit=True, auto):
        """
        Set view limits.

        This method is a helper for the Axes ``set_xlim``, ``set_ylim``, and
        ``set_zlim`` methods.

        Parameters
        ----------
        v0, v1 : float
            The view limits.  (Passing *v0* as a (low, high) pair is not
            supported; normalization must occur in the Axes setters.)
        emit : bool, default: True
            Whether to notify observers of limit change.
        auto : bool or None, default: False
            Whether to turn on autoscaling of the x-axis. True turns on, False
            turns off, None leaves unchanged.
        """
        # Limits given in units set the axis' units first.
        self.axes._process_unit_info([(self.axis_name, (v0, v1))], convert=False)
        # matplotlib sets the limits of 2-D axes through ``viewLim``, which
        # applies the autoscaling pending (of the other axis too) with the
        # rcParams of now; those of 3-D axes have boxes of their own.
        if self.axes.name != "3d":
            self.axes._unstale_viewLim()
        if v0 is None or v1 is None:
            # The view as it is now, autoscaled first.
            old0, old1 = getattr(self.axes, f"get_{self.axis_name}lim")()
            v0 = old0 if v0 is None else v0
            v1 = old1 if v1 is None else v1
        given = [_converted_limit(self, v) for v in (v0, v1)]
        v0, v1 = self._checked_limits(v0, v1)
        limits = _axes.set_lim(self.axes._native, self.axis_name, (v0, v1),
                               None if auto is None else bool(auto), bool(emit))
        if not _keeps_limit_types(self.get_major_locator()):
            return limits
        # The limits as matplotlib returns them through these locators: the
        # numbers given (after their units), numpy floats where the axis
        # changed them.
        return tuple(value if np.ndim(value) == 0 and value == limit
                     else np.float64(limit)
                     for value, limit in zip(given, limits))

    def get_scale(self):
        """Return this Axis' scale (as a str)."""
        return _axes_base.get_scale(*self._args())

    @property
    def _scale(self):
        """The axis' scale as an object of `.scale`: the one it was set
        with, or one with the default options of the scale in use."""
        name = self.get_scale()
        given = self._given_scale
        if given is not None and given.name == name:
            return given
        return self.axes._default_axis_scale(self, name)

    def _set_axes_scale(self, value, **kwargs):
        """
        Set this Axis' scale.

        Parameters
        ----------
        value : str or `.ScaleBase`
            The axis scale type to apply.  Valid string values are the names of
            scale classes ("linear", "log", "function",...).  These may be the
            names of any of the :ref:`built-in scales<builtin_scales>` or of any
            custom scales registered using `matplotlib.scale.register_scale`.

        **kwargs
            If *value* is a string, keywords are passed to the instantiation
            method of the respective class.
        """
        from . import scale as mscale
        if isinstance(value, mscale.ScaleBase):
            scale = value
        else:
            scale = mscale.scale_factory(value, self, **kwargs)
        native = _native_scale_of(scale)
        if native is None:
            transform = scale.get_transform()
            inverse = transform.inverted()
            # pltrs (Rust) hands the transforms arrays, as matplotlib does.
            _native_scale.set_python_scale(
                *self._args(), scale.name,
                (transform.transform, inverse.transform),
                lambda vmin, vmax, minpos: tuple(
                    map(float, scale.limit_range_for_scale(vmin, vmax, minpos))))
            scale.set_default_locators_and_formatters(self)
            _axis.mark_default_helpers(*self._args())
        else:
            _native_scale.set_scale(*self._args(), *native)
        for axis in self._get_shared_axis():
            axis._given_scale = scale
            axis.axes._update_transScale()

    def get_transform(self):
        """Return the transform used in the Axis' scale."""
        return self._scale.get_transform()

    def limit_range_for_scale(self, vmin, vmax):
        """
        Return the range *vmin*, *vmax*, restricted to the domain supported by
        the current scale.
        """
        return self._scale.limit_range_for_scale(vmin, vmax, self.get_minpos())

    # --- Ticks ---------------------------------------------------------------

    def _tick_objects(self, minor, count):
        """The first *count* (at least one) ticks of a level, synced."""
        if count > _MAX_TICK_OBJECTS:
            raise ValueError(f"{count} ticks are more than an axis can make "
                             f"(at most {_MAX_TICK_OBJECTS})")
        ticks = self._ticks[minor]
        tick_class = self._tick_class or Tick
        while len(ticks) < max(count, 1):
            ticks.append(tick_class._of_axis(self, not minor, len(ticks)))
        for tick in ticks:
            tick._sync()
        return ticks

    def _get_tick(self, major):
        """Return the default tick instance (a new tick of the axis' class
        at 0, outside the ticks the axis draws)."""
        if self._tick_class is None:
            raise NotImplementedError(
                f"The Axis subclass {self.__class__.__name__} must define "
                "_tick_class or reimplement _get_tick()")
        tick_kw = self._major_tick_kw if major else self._minor_tick_kw
        return self._tick_class(self.axes, 0, major=major, **tick_kw)

    @property
    def majorTicks(self):
        """The major ticks (see `get_major_ticks`)."""
        return self._tick_objects(False, len(self.get_majorticklocs()))

    @property
    def minorTicks(self):
        """The minor ticks (see `get_minor_ticks`)."""
        return self._tick_objects(True, len(self.get_minorticklocs()))

    def get_major_ticks(self, numticks=None):
        r"""Return the list of major `.Tick`\s."""
        if numticks is None:
            numticks = len(self.get_majorticklocs())
        return self._tick_objects(False, numticks)[:numticks]

    def get_minor_ticks(self, numticks=None):
        r"""Return the list of minor `.Tick`\s."""
        if numticks is None:
            numticks = len(self.get_minorticklocs())
        return self._tick_objects(True, numticks)[:numticks]

    def clear(self):
        """
        Clear the axis.

        This resets axis properties to their default values:

        - the label
        - the scale
        - locators, formatters and ticks
        - major and minor grid
        - units
        - registered callbacks
        """
        _axis.clear(*self._args())
        # Clear the callback registry for this axis, or it may "leak"
        self.callbacks = cbook.CallbackRegistry(signals=["units"])
        self.reset_ticks()
        self._given_scale = None
        self._converter = None
        self._converter_is_explicit = False
        self._units = None
        self._category_units = None
        self.__dict__.pop("_category_keys", None)
        self.stale = True

    def set_default_intervals(self):
        """
        Set the default limits for the axis data and view interval if they
        have not been not mutated yet.

        pltrs (Rust) shows the default limits of the axis info of the
        axis' converter whenever the axis has neither data nor a view of
        its own; this gives it those of the converter and units in force.
        """
        converter = self._converter
        if converter is not None and not _is_native_converter(converter):
            info = converter.axisinfo(self.units, self)
            if info is not None and info.default_limits is not None:
                _tick_helpers.use_axis_info(
                    self, munits.AxisInfo(), self._default_limits(info))
        self.stale = True

    def reset_ticks(self):
        """
        Re-initialize the major and minor Tick lists.

        Each list starts with a single fresh Tick.
        """
        self._ticks = {False: [], True: []}

    def _update_ticks(self):
        """
        Update ticks (position and labels) using the current data interval of
        the axes.  Return the list of ticks that will be drawn.
        """
        _axis.place_ticks(*self._args())
        ticks = []
        for minor in (False, True):
            locs = self.get_ticklocs(minor=minor)
            if not len(locs):
                continue
            ticker = self.minor if minor else self.major
            ticker.formatter.format_ticks(locs)
            ticks += self._tick_objects(minor, len(locs))[:len(locs)]
        view_low, view_high = sorted(self.get_view_interval())
        return [tick for tick in ticks if view_low <= tick.get_loc() <= view_high
                or np.isclose(tick.get_loc(), (view_low, view_high)).any()]

    def get_ticklocs(self, *, minor=False):
        """
        Return this Axis' tick locations in data coordinates.

        The locations are not clipped to the current axis limits and hence
        may contain locations that are not visible in the output.
        """
        if self._drawn_info is not None and not minor:
            return self._drawn_fields()[4]
        locs = _axis.get_ticklocs(*self._args(), bool(minor))
        locator = self.get_minor_locator() if minor else self.get_major_locator()
        return _typed_locs(locator, locs, minor)

    def get_majorticklocs(self):
        """Return this Axis' major tick locations in data coordinates."""
        return self.get_ticklocs(minor=False)

    def get_minorticklocs(self):
        """Return this Axis' minor tick locations in data coordinates."""
        return self.get_ticklocs(minor=True)

    def get_majorticklabels(self):
        """Return this Axis' major tick labels, as a list of `~.text.Text`."""
        return self._ticklabels(minor=False)

    def get_minorticklabels(self):
        """Return this Axis' minor tick labels, as a list of `~.text.Text`."""
        return self._ticklabels(minor=True)

    def _ticklabels(self, minor):
        # matplotlib places the ticks (`_update_ticks`) to read their labels.
        _axis.place_ticks(*self._args())
        ticks = self.get_minor_ticks() if minor else self.get_major_ticks()
        labels1 = [tick.label1 for tick in ticks if tick.label1.get_visible()]
        labels2 = [tick.label2 for tick in ticks if tick.label2.get_visible()]
        return labels1 + labels2

    def get_ticklabels(self, minor=False, which=None):
        """
        Get this Axis' tick labels.

        Parameters
        ----------
        minor : bool
           Whether to return the minor or the major ticklabels.

        which : None, ('minor', 'major', 'both')
           Overrides *minor*.

           Selects which ticklabels to return

        Returns
        -------
        list of `~matplotlib.text.Text`
        """
        if which is not None:
            if which == 'minor':
                return self.get_minorticklabels()
            elif which == 'major':
                return self.get_majorticklabels()
            elif which == 'both':
                return self.get_majorticklabels() + self.get_minorticklabels()
            else:
                _api.check_in_list(['major', 'minor', 'both'], which=which)
        if minor:
            return self.get_minorticklabels()
        return self.get_majorticklabels()

    def get_majorticklines(self):
        """Return this Axis' major tick lines as a list of `.Line2D`."""
        return self._ticklines(self.get_major_ticks())

    def get_minorticklines(self):
        """Return this Axis' minor tick lines as a list of `.Line2D`."""
        return self._ticklines(self.get_minor_ticks())

    @staticmethod
    def _ticklines(ticks):
        lines = []
        for tick in ticks:
            lines.append(tick.tick1line)
            lines.append(tick.tick2line)
        return cbook.silent_list('Line2D ticklines', lines)

    def get_ticklines(self, minor=False):
        """Return this Axis' tick lines as a list of `.Line2D`."""
        if minor:
            return self.get_minorticklines()
        return self.get_majorticklines()

    def get_gridlines(self):
        """Return this Axis' grid lines as a list of `.Line2D`."""
        ticks = self.get_major_ticks()
        return cbook.silent_list('Line2D gridline',
                                 [tick.gridline for tick in ticks])

    def get_ticks_direction(self, minor=False):
        """
        Return an array of this Axis' tick directions.

        Parameters
        ----------
        minor : bool, default: False
            True to return the minor tick directions,
            False to return the major tick directions.

        Returns
        -------
        array of tick directions
        """
        ticks = self.get_minor_ticks() if minor else self.get_major_ticks()
        return np.array([tick._tickdir for tick in ticks])

    def get_tick_padding(self):
        """The tick marks' reach out of the axes, in points."""
        return _axis.get_tick_padding(*self._args())

    def _get_tick_label_size(self, axis_name):
        """Return the text size of tick labels for this Axis."""
        from .font_manager import FontProperties
        size = self._tick_kw[False].get(
            'labelsize', self._rc(f'{axis_name}tick.labelsize'))
        return FontProperties(size=size).get_size_in_points()

    @staticmethod
    def _rc(key):
        from . import rcParams
        return rcParams[key]

    def set_ticks(self, ticks, labels=None, *, minor=False, **kwargs):
        """
        Set this Axis' tick locations and optionally tick labels.

        If necessary, the view limits of the Axis are expanded so that all
        given ticks are visible.

        Parameters
        ----------
        ticks : 1D array-like
            Array of tick locations (either floats or in axis units). The axis
            `.Locator` is replaced by a `~.ticker.FixedLocator`.

            Pass an empty list (``set_ticks([])``) to remove all ticks.

            Some tick formatters will not label arbitrary tick positions;
            e.g. log formatters only label decade ticks by default. In
            such a case you can set a formatter explicitly on the axis
            using `.Axis.set_major_formatter` or provide formatted
            *labels* yourself.

        labels : list of str, optional
            Tick labels for each location in *ticks*; must have the same length
            as *ticks*. If set, the labels are used as is, via a
            `.FixedFormatter`. If not set, the labels are generated using the
            axis tick `.Formatter`.

        minor : bool, default: False
            If ``False``, set only the major ticks; if ``True``, only the minor
            ticks.

        **kwargs
            `.Text` properties for the labels. Using these is only allowed if
            you pass *labels*. In other cases, please use `~.Axes.tick_params`.
        """
        if labels is None and kwargs:
            first_key = next(iter(kwargs))
            raise ValueError(
                f"Incorrect use of keyword argument {first_key!r}. Keyword arguments "
                "other than 'minor' modify the text labels and can only be used if "
                "'labels' are passed as well.")
        if not np.iterable(ticks) or isinstance(ticks, str):
            raise TypeError(
                f"ticks must be a sequence, not {type(ticks).__name__}")
        values = np.asarray(ticks)
        # matplotlib's FixedLocator takes the locations.
        _api.check_shape((None,), locs=values)
        if values.dtype.kind in "fiub" or not values.size:
            native = values.astype(float)
        else:
            from .axes._plotting._helpers import axis_values
            native = axis_values(ticks)
        _axis.set_ticks(*self._args(), native, bool(minor))
        if values.dtype.kind in "iub" and values.size:
            # matplotlib's locator keeps the locations as given (ints).
            from .ticker import FixedLocator
            _tick_helpers.adopt(self, FixedLocator(values), "locator", minor)
        if labels is not None:
            self.set_ticklabels(labels, minor=minor, **kwargs)
        get_ticks = self.get_minor_ticks if minor else self.get_major_ticks
        return get_ticks(len(values))

    @staticmethod
    def _format_with_dict(tickd, x, pos):
        return tickd.get(x, "")

    def set_ticklabels(self, labels, *, minor=False, fontdict=None, **kwargs):
        r"""
        [*Discouraged*] Set this Axis' tick labels with list of string labels.

        Parameters
        ----------
        labels : sequence of str or of `.Text`\s
            Texts for labeling each tick location in the sequence set by
            `.Axis.set_ticks`; the number of labels must match the number of
            locations.

        minor : bool
            If True, set minor ticks instead of major ticks.

        fontdict : dict, optional
            A dictionary controlling the appearance of the ticklabels.

        **kwargs
            Text properties.

        Returns
        -------
        list of `.Text`\s
            For each tick, includes ``tick.label1`` if it is visible, then
            ``tick.label2`` if it is visible, in that order.
        """
        from . import ticker as mticker
        try:
            labels = [t.get_text() if hasattr(t, 'get_text') else t
                      for t in labels]
        except TypeError:
            raise TypeError(f"{labels=!r} must be a sequence") from None
        locator = (self.get_minor_locator() if minor
                   else self.get_major_locator())
        if not labels:
            formatter = mticker.NullFormatter()
        elif isinstance(getattr(locator, "base", locator), mticker.FixedLocator):
            fixed = getattr(locator, "base", locator)
            # Passing [] as a list of labels is often used as a way to
            # remove all tick labels, so only error for > 0 labels
            if len(fixed.locs) != len(labels) and len(labels) != 0:
                raise ValueError(
                    "The number of FixedLocator locations"
                    f" ({len(fixed.locs)}), usually from a call to"
                    " set_ticks, does not match"
                    f" the number of labels ({len(labels)}).")
            tickd = {loc: lab for loc, lab in zip(fixed.locs, labels)}
            func = functools.partial(self._format_with_dict, tickd)
            formatter = mticker.FuncFormatter(func)
        else:
            _api.warn_external(
                "set_ticklabels() should only be used with a fixed number of "
                "ticks, i.e. after set_ticks() or using a FixedLocator. "
                "Otherwise, ticks may be mislabeled.")
            formatter = mticker.FixedFormatter(labels)
        self._set_formatter(formatter, minor=minor)
        # matplotlib labels the ticks of the level as it returns them.
        _axis.place_ticks(*self._args())
        ticks = self.get_minor_ticks() if minor else self.get_major_ticks()
        kwargs = {**(fontdict or {}), **kwargs}
        ret = []
        for tick in ticks:
            if kwargs:
                tick.label1._internal_update(kwargs)
                tick.label2._internal_update(kwargs)
            if tick.label1.get_visible():
                ret.append(tick.label1)
            if tick.label2.get_visible():
                ret.append(tick.label2)
        return ret

    def set_tick_params(self, which='major', reset=False, **kwargs):
        """
        Set appearance parameters for ticks, ticklabels, and gridlines.

        For documentation of keyword arguments, see
        :meth:`matplotlib.axes.Axes.tick_params`.
        """
        _api.check_in_list(['major', 'minor', 'both'], which=which)
        kwtrans = self._translate_tick_params(kwargs)
        for minor in (False, True):
            if which in ('minor' if minor else 'major', 'both'):
                if reset:
                    self._tick_kw[minor] = {}
                self._tick_kw[minor].update(kwtrans)
        # The ticks are drawn with their axis, as a unit, so that their
        # zorder only orders them among themselves: it is kept, not drawn.
        native = {k: v for k, v in kwtrans.items() if k != 'zorder'}
        if 'grid_path_effects' in native:
            native['grid_path_effects'] = [
                effect._native_spec() for effect in native['grid_path_effects']]
        if 'grid_clip_box' in native:
            native['grid_clip_box'] = _artist_clipping.clip_box_spec(
                native['grid_clip_box'])
        if 'grid_clip_path' in native:
            native['grid_clip_path'] = _artist_clipping.clip_path_spec(
                native['grid_clip_path'])
        _axis.set_tick_params(*self._args(), which, bool(reset), native)
        if not reset:
            # labelOn and labelcolor also apply to the offset text.
            if 'label1On' in kwtrans or 'label2On' in kwtrans:
                major = self._major_tick_kw
                self.offsetText.set_visible(major.get('label1On', False)
                                            or major.get('label2On', False))

    def get_tick_params(self, which='major'):
        """
        Get appearance parameters for ticks, ticklabels, and gridlines.

        Parameters
        ----------
        which : {'major', 'minor'}, default: 'major'
            The group of ticks for which the parameters are retrieved.

        Returns
        -------
        dict
            Properties for styling tick elements added to the axis.
        """
        _api.check_in_list(['major', 'minor'], which=which)
        kw = self._major_tick_kw if which == 'major' else self._minor_tick_kw
        return self._translate_tick_params(kw, reverse=True)

    @property
    def _major_tick_kw(self):
        """The keyword state of the major ticks: the sides with marks and
        labels and the grid, and the tick parameters set."""
        return self._tick_kw_of(minor=False)

    @property
    def _minor_tick_kw(self):
        """The keyword state of the minor ticks (see `_major_tick_kw`)."""
        return self._tick_kw_of(minor=True)

    def _tick_kw_of(self, minor):
        keys = ("tick1On", "tick2On", "label1On", "label2On", "gridOn")
        switches = dict(zip(keys, _axis.tick_switches(*self._args(), minor)))
        return {**self._tick_kw[minor], **switches}

    @classmethod
    def _translate_tick_params(cls, kw, reverse=False):
        """
        Translate the kwargs supported by `.Axis.set_tick_params` to kwargs
        supported by `.Tick._apply_params`.

        In particular, this maps axis specific names like 'top', 'left'
        to the generic tick1, tick2 logic of the axis. Additionally, there
        are some other name translations.

        Returns a new dict of translated kwargs.

        Note: Use reverse=True to translate from those supported by
        `.Tick._apply_params` back to those supported by
        `.Axis.set_tick_params`.
        """
        kw_ = {**kw}

        allowed_keys = [
            'size', 'width', 'color', 'tickdir', 'pad',
            'labelsize', 'labelcolor', 'labelfontfamily', 'zorder', 'gridOn',
            'tick1On', 'tick2On', 'label1On', 'label2On',
            'length', 'direction', 'left', 'bottom', 'right', 'top',
            'labelleft', 'labelbottom', 'labelright', 'labeltop',
            'labelrotation', 'labelrotation_mode',
            *_gridline_param_names]

        keymap = {
            # tick_params key -> axis key
            'length': 'size',
            'direction': 'tickdir',
            'rotation': 'labelrotation',
            'rotation_mode': 'labelrotation_mode',
            'left': 'tick1On',
            'bottom': 'tick1On',
            'right': 'tick2On',
            'top': 'tick2On',
            'labelleft': 'label1On',
            'labelbottom': 'label1On',
            'labelright': 'label2On',
            'labeltop': 'label2On',
        }
        if reverse:
            kwtrans = {}
            is_x_axis = cls.axis_name == 'x'
            y_axis_keys = ['left', 'right', 'labelleft', 'labelright']
            for oldkey, newkey in keymap.items():
                if newkey in kw_:
                    if is_x_axis and oldkey in y_axis_keys:
                        continue
                    else:
                        kwtrans[oldkey] = kw_.pop(newkey)
        else:
            kwtrans = {
                newkey: kw_.pop(oldkey)
                for oldkey, newkey in keymap.items() if oldkey in kw_
            }
        if 'colors' in kw_:
            c = kw_.pop('colors')
            kwtrans['color'] = c
            kwtrans['labelcolor'] = c
        # Maybe move the checking up to the caller of this method.
        for key in kw_:
            if key not in allowed_keys:
                raise ValueError(
                    "keyword %s is not recognized; valid keywords are %s"
                    % (key, allowed_keys))
        kwtrans.update(kw_)
        return kwtrans

    def grid(self, visible=None, which='major', **kwargs):
        """
        Configure the grid lines.

        Parameters
        ----------
        visible : bool or None
            Whether to show the grid lines.  If any *kwargs* are supplied, it
            is assumed you want the grid on and *visible* will be set to True.

            If *visible* is *None* and there are no *kwargs*, this toggles the
            visibility of the lines.

        which : {'major', 'minor', 'both'}
            The grid lines to apply the changes on.

        **kwargs : `~matplotlib.lines.Line2D` properties
            Define the line properties of the grid, e.g.::

                grid(color='r', linestyle='-', linewidth=2)
        """
        if kwargs:
            if visible is None:
                visible = True
            elif not visible:  # something false-like but not None
                _api.warn_external('First parameter to grid() is false, '
                                   'but line properties are supplied. The '
                                   'grid will be enabled.')
                visible = True
        which = which.lower()
        _api.check_in_list(['major', 'minor', 'both'], which=which)
        gridkw = {f'grid_{name}': value for name, value in kwargs.items()}
        if which in ['minor', 'both']:
            gridkw['gridOn'] = (not self._minor_tick_kw['gridOn']
                                if visible is None else visible)
            self.set_tick_params(which='minor', **gridkw)
        if which in ['major', 'both']:
            gridkw['gridOn'] = (not self._major_tick_kw['gridOn']
                                if visible is None else visible)
            self.set_tick_params(which='major', **gridkw)

    def get_ticks_position(self):
        """
        Return the ticks position ("top", "bottom", "left", "right",
        "default" or "unknown").
        """
        return _axis.get_ticks_position(*self._args())

    def _set_ticks_position(self, position):
        """Put the ticks where *position* says (natively checked)."""
        _axis.set_ticks_position(*self._args(), position)

    # --- Label and offset text -----------------------------------------------

    @property
    def label(self):
        """The `.Text` object of the axis label."""
        return self._label._synced()

    @property
    def offsetText(self):
        """The `.Text` object of the offset (or scale) of the ticks."""
        return self._offset_text._synced()

    def get_label(self):
        """
        [*Discouraged*] Return the axis label as a Text instance.

        .. admonition:: Discouraged

           This overrides `.Artist.get_label`, which is for legend labels, with
           a new semantic. It is recommended to use the attribute
           ``Axis.label`` instead.
        """
        return self.label

    def set_label(self, s):
        """Assigning legend labels is not supported. Raises RuntimeError."""
        raise RuntimeError(
            "A legend label cannot be assigned to an Axis. Did you mean to "
            "set the axis label via set_label_text()?")

    def get_offset_text(self):
        """Return the axis offsetText as a Text instance."""
        return self.offsetText

    def get_label_text(self):
        """Get the text of the label."""
        return self.label.get_text()

    def set_label_text(self, label, fontdict=None, **kwargs):
        """
        Set the text value of the axis label.

        Parameters
        ----------
        label : str
            Text string.
        fontdict : dict
            Text properties.

            .. admonition:: Discouraged

               The use of *fontdict* is discouraged. Parameters should be passed
               as individual keyword arguments or using dictionary-unpacking
               ``set_label_text(..., **fontdict)``.

        **kwargs
            Merged into fontdict.
        """
        text = self.label
        # The label counts as set (`isDefault_label`), even to its text.
        _axis.set_label_text(*self._args(), '' if label is None else str(label))
        text.set_text(label)
        if fontdict is not None:
            text.update(fontdict)
        text.update(kwargs)
        return text

    def _set_axes_label(self, label, fontdict, labelpad, loc, kwargs):
        """
        Label the axis as `.Axes.set_xlabel` and `.Axes.set_ylabel` do:
        *loc* places the label along the axis unless the keywords place it.
        """
        if labelpad is not None:
            self.labelpad = labelpad
        along = self.axis_name
        protected_kw = [along, 'horizontalalignment', 'ha']
        if {*kwargs} & {*protected_kw}:
            if loc is not None:
                raise TypeError(f"Specifying 'loc' is disallowed when any of "
                                f"its corresponding low level keyword "
                                f"arguments ({protected_kw}) are also "
                                f"supplied")
        else:
            names = (('left', 'center', 'right') if along == 'x'
                     else ('bottom', 'center', 'top'))
            loc = loc if loc is not None else self._rc(f'{along}axis.labellocation')
            _api.check_in_list(names, loc=loc)
            fraction = names.index(loc) / 2
            ha = ('left', 'center', 'right')[names.index(loc)]
            kwargs = {**kwargs, along: fraction, 'horizontalalignment': ha}
        return self.set_label_text(label, fontdict, **kwargs)

    @property
    def labelpad(self):
        """The distance between the axis label and the tick labels, in
        points."""
        return _axis.get_labelpad(*self._args())

    @labelpad.setter
    def labelpad(self, pad):
        _axis.set_labelpad(*self._args(), float(pad))

    def set_label_coords(self, x, y, transform=None):
        """
        Set the coordinates of the label.

        By default, the x coordinate of the y label and the y coordinate of the
        x label are determined by the tick label bounding boxes, but this can
        lead to poor alignment of multiple labels if there are multiple Axes.

        You can also specify the coordinate system of the label with the
        transform.  If None, the default coordinate system will be the axes
        coordinate system: (0, 0) is bottom left, (0.5, 0.5) is center, etc.
        """
        if transform is not None:
            x, y = (transform - self.axes.transAxes).transform((x, y))
        _axis.set_label_coords(*self._args(), float(x), float(y))
        # The label's transform, as matplotlib gives it (`_axis_artists`).
        self._label_transform = (self.axes.transAxes if transform is None
                                 else transform)

    def get_label_position(self):
        """
        Return the label position (top, bottom, left or right).
        """
        return _axis.get_label_position(*self._args())

    def set_label_position(self, position):
        """Set the label position (a side of the axes)."""
        _axis.set_label_position(*self._args(), position)

    # --- Visibility and layout -----------------------------------------------

    def get_visible(self):
        """Return whether the axis is drawn."""
        return _axis.get_visible(*self._args())

    def set_visible(self, b):
        """Set whether the axis is drawn: its ticks, labels and offset text."""
        _axis.set_visible(*self._args(), bool(b))

    def get_in_layout(self):
        """Return whether layouts make room for the axis' texts."""
        return _axis.get_in_layout(*self._args())

    def set_in_layout(self, in_layout):
        """Set whether layouts make room for the axis' texts."""
        _axis.set_in_layout(*self._args(), bool(in_layout))

    def get_children(self):
        return [self.label, self.offsetText,
                *self.get_major_ticks(), *self.get_minor_ticks()]

    def get_gid(self):
        """Return the group id of the axis."""
        return _axis.get_gid(*self._args())

    def set_gid(self, gid):
        """
        Set the (group) id of the axis: SVG output names the group holding
        the axis' ticks and texts after it.

        Parameters
        ----------
        gid : str
        """
        _axis.set_gid(*self._args(), None if gid is None else str(gid))

    def get_tightbbox(self, renderer=None, *, for_layout_only=False):
        """
        Return a bounding box that encloses the axis. It only accounts
        tick labels, axis label, and offsetText.

        If *for_layout_only* is True, then the width of the label (if this
        is an x-axis) or the height of the label (if this is a y-axis) is
        collapsed to near zero.  This allows tight/constrained_layout to ignore
        too-long labels when doing their layout.
        """
        from .transforms import Bbox
        extent = _axis.get_tightbbox(*self._args(), bool(for_layout_only))
        if extent is None:
            return None
        x0, y0, x1, y1 = extent
        return Bbox([[x0, y0], [x1, y1]])

    def get_pickradius(self):
        """Return the depth of the axis used by the picker."""
        return self._pickradius

    def set_pickradius(self, pickradius):
        """
        Set the depth of the axis used by the picker.

        Parameters
        ----------
        pickradius : float
            The acceptance radius for containment tests.
            See also `.Axis.contains`.
        """
        from numbers import Real
        if not isinstance(pickradius, Real) or pickradius < 0:
            raise ValueError("pick radius should be a distance")
        self._pickradius = pickradius

    pickradius = property(
        get_pickradius, set_pickradius, doc="The acceptance radius for "
        "containment tests. See also `.Axis.contains`.")

    # --- Units ---------------------------------------------------------------

    @property
    def units(self):
        """The units of the axis: the category data of a categorical axis,
        else what `set_units` set."""
        if not _axis.has_categories(*self._args()):
            return self._units
        units = self._shared_category_units()
        keys = self._shared_category_keys()
        names = _axis.category_names(*self._args())
        units._mapping.clear()
        units._mapping.update((keys.get(name, name), index)
                              for index, name in enumerate(names))
        return units

    @units.setter
    def units(self, units):
        self._units = units

    def _shared_category_units(self):
        """The category data of the axis, which the axes sharing it share,
        as in matplotlib."""
        if self._category_units is None:
            from .category import UnitData
            shared = self._get_shared_axis()
            units = next((axis._category_units for axis in shared
                          if axis._category_units is not None), UnitData())
            for axis in shared:
                axis._category_units = units
        return self._category_units

    def _shared_category_keys(self):
        """The category names given as bytes, by the name pltrs (Rust) keeps
        for them, so that they stay bytes; the axes sharing the axis share
        them."""
        keys = vars(self).get("_category_keys")
        if keys is None:
            shared = self._get_shared_axis()
            keys = next((vars(axis)["_category_keys"] for axis in shared
                         if "_category_keys" in vars(axis)), {})
            for axis in shared:
                axis._category_keys = keys
        return keys

    def have_units(self):
        """Return whether the axis converts units."""
        return (self._converter is not None or self._units is not None
                or _axis.have_units(*self._args()))

    def get_units(self):
        """Return the units for axis."""
        return self.units

    def set_units(self, u):
        """
        Set the units for axis.

        Parameters
        ----------
        u : units tag

        Notes
        -----
        The units of any shared axis will also be updated.
        """
        if u == self.units:
            return
        from .category import UnitData
        if isinstance(u, UnitData):
            # pltrs (Rust) keeps the names of a categorical axis.
            self._update_native_units(list(u._mapping))
        for axis in self._get_shared_axis():
            axis._units = u
            axis._update_axisinfo()
            axis.axes._unit_change_handler(axis.axis_name)
            axis.callbacks.process('units')

    def _get_shared_axis(self):
        """Return list of shared axis for current axis."""
        name = self.axis_name
        return [getattr(ax, f"{name}axis")
                for ax in self.axes._shared_axes[name].get_siblings(self.axes)]

    def update_units(self, data):
        """
        Introspect *data* for units converter and update the
        ``axis.get_converter`` instance if necessary. Return *True*
        if *data* is registered for unit conversion.
        """
        if self._converter_is_explicit:
            converter = self._converter
        else:
            converter = munits.registry.get_converter(data)
        if converter is None:
            return False
        if _is_native_converter(converter):
            if not _is_native_converter(self._converter):
                self._set_converter(converter)
            from .category import StrCategoryConverter
            if not isinstance(converter, StrCategoryConverter):
                # The first date tells pltrs (Rust) all it needs: that the
                # axis shows dates (the others may be no dates at all, as
                # the timedeltas of a bar's (start, width) pair).
                data = _first_value(data)
            return self._update_native_units(data)
        neednew = self._converter != converter
        self._set_converter(converter)
        default = self._converter.default_units(data, self)
        if default is not None and self.units is None:
            self.set_units(default)
        elif neednew:
            self._update_axisinfo()
        return True

    def _update_native_units(self, data):
        """Have pltrs (Rust), which converts dates and category names,
        take the units of *data*; whether *data* has units."""
        from .axes._plotting._helpers import axis_values
        values = axis_values(data)
        if isinstance(values, list):
            # Category names keep the type they were given in.
            keys = self._shared_category_keys()
            for value in np.atleast_1d(np.array(data, dtype=object)).ravel():
                if isinstance(value, bytes):
                    keys.setdefault(value.decode(), value)
        return _axis.update_units(*self._args(), values)

    def _update_axisinfo(self):
        """Give the axis the tick helpers, label and default limits its
        converter gives for its units, when the converter is one of the
        program's (pltrs (Rust) gives dates and categories theirs)."""
        if _is_native_converter(self._converter):
            return
        info = self._converter.axisinfo(self.units, self)
        if info is not None:
            _tick_helpers.use_axis_info(self, info, self._default_limits(info))

    def _default_limits(self, info):
        """The default limits of the converter's axis info *info*, in axis
        numbers, or None."""
        if info.default_limits is None:
            return None
        low, high = self.convert_units(info.default_limits)
        return float(low), float(high)

    def convert_units(self, x):
        """Convert *x* to the numbers of the axis, which takes the units
        it brings."""
        if munits._is_natively_supported(x):
            return x
        converter = self._converter
        if converter is None and _axis.converter_kind(*self._args()) is None:
            converter = munits.registry.get_converter(x)
            if converter is None:
                return x
            if not _is_native_converter(converter):
                self._set_converter(converter)
        if _is_native_converter(converter):
            from .axes._plotting._helpers import convert_units
            return convert_units(self.axes, self.axis_name, x)
        try:
            return converter.convert(x, self.units, self)
        except Exception as e:
            raise munits.ConversionError('Failed to convert value(s) to axis '
                                         f'units: {x!r}') from e

    def get_converter(self):
        """Get the unit converter for axis."""
        if self._converter is not None:
            return self._converter
        kind = _axis.converter_kind(*self._args())
        if kind is None:
            return None
        return munits.registry.get_converter(
            ["a"] if kind == "category" else [np.datetime64("2000-01-01")])

    def set_converter(self, converter):
        """
        Set the unit converter for axis.

        Parameters
        ----------
        converter : `~matplotlib.units.ConversionInterface`
        """
        self._set_converter(converter)
        self._converter_is_explicit = True

    def _set_converter(self, converter):
        current = self.get_converter()
        if current is converter or current == converter:
            return
        if self._converter_is_explicit:
            raise RuntimeError("Axis already has an explicit converter set")
        elif (current is not None and
              not isinstance(converter, type(current)) and
              not isinstance(current, type(converter))):
            _api.warn_external(
                "This axis already has a converter set and "
                "is updating to a potentially incompatible converter"
            )
        kind = _native_converter_kind(converter)
        if kind is not None:
            _axis.set_converter(*self._args(), kind)
        self._converter = converter

    def axis_date(self, tz=None):
        """
        Set up axis ticks and labels to treat data along this Axis as dates.

        Parameters
        ----------
        tz : str or `datetime.tzinfo`, default: :rc:`timezone`
            The timezone used to create date labels.
        """
        from ._date_locators import _native_tz
        from .dates import _get_tzinfo
        _axis.axis_date(*self._args(), _native_tz(_get_tzinfo(tz)))


class XAxis(Axis):
    """The x-axis of an Axes."""

    __name__ = 'xaxis'
    axis_name = 'x'  #: Read-only name identifying the axis.
    _tick_class = XTick
    #: matplotlib's record of the offset text's side, which its draws no
    #: longer read (the side of the ticks places the text).
    offset_text_position = 'bottom'

    def set_label_position(self, position):
        """
        Set the label position (top or bottom)

        Parameters
        ----------
        position : {'top', 'bottom'}
        """
        _api.check_in_list(['top', 'bottom'], position=position)
        super().set_label_position(position)

    def set_ticks_position(self, position):
        """
        Set the ticks position.

        Parameters
        ----------
        position : {'top', 'bottom', 'both', 'default', 'none'}
            'both' sets the ticks to appear on both positions, but does not
            change the tick labels.  'default' resets the tick positions to
            the default: ticks on both positions, labels at bottom.  'none'
            can be used if you don't want any ticks. 'none' and 'both'
            affect only the ticks, not the labels.
        """
        _api.check_in_list(['top', 'bottom', 'both', 'default', 'none'],
                           position=position)
        self._set_ticks_position(position)

    def tick_top(self):
        """
        Move ticks and ticklabels (if present) to the top of the Axes.
        """
        _axis.tick_to(*self._args(), "top")

    def tick_bottom(self):
        """
        Move ticks and ticklabels (if present) to the bottom of the Axes.
        """
        _axis.tick_to(*self._args(), "bottom")


class YAxis(Axis):
    """The y-axis of an Axes."""

    __name__ = 'yaxis'
    axis_name = 'y'  #: Read-only name identifying the axis.
    _tick_class = YTick
    #: matplotlib's record of the offset text's side, which its draws no
    #: longer read (`set_offset_position` places the text).
    offset_text_position = 'left'

    def set_label_position(self, position):
        """
        Set the label position (left or right)

        Parameters
        ----------
        position : {'left', 'right'}
        """
        _api.check_in_list(['left', 'right'], position=position)
        super().set_label_position(position)

    def set_offset_position(self, position):
        """
        Parameters
        ----------
        position : {'left', 'right'}
        """
        _api.check_in_list(['left', 'right'], position=position)
        _axis.set_offset_position(*self._args(), position)

    def set_ticks_position(self, position):
        """
        Set the ticks position.

        Parameters
        ----------
        position : {'left', 'right', 'both', 'default', 'none'}
            'both' sets the ticks to appear on both positions, but does not
            change the tick labels.  'default' resets the tick positions to
            the default: ticks on both positions, labels at left.  'none'
            can be used if you don't want any ticks. 'none' and 'both'
            affect only the ticks, not the labels.
        """
        _api.check_in_list(['left', 'right', 'both', 'default', 'none'],
                           position=position)
        self._set_ticks_position(position)

    def tick_right(self):
        """
        Move ticks and ticklabels (if present) to the right of the Axes.
        """
        _axis.tick_to(*self._args(), "right")

    def tick_left(self):
        """
        Move ticks and ticklabels (if present) to the left of the Axes.
        """
        _axis.tick_to(*self._args(), "left")


cbook._set_module(__name__, Tick, XTick, YTick)
