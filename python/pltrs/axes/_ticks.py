"""
The tick and scale methods of `~.axes.Axes` (matplotlib's ``axes._base``):
tick labels, lines and gridlines, tick parameters, grids, minor ticks,
locator parameters and tick label formats, each forwarding to the `.Axis`
objects as matplotlib's ``_axis_method_wrapper`` does, and the plots on
log scales (``loglog``, ``semilogx``, ``semilogy``).
"""

import inspect
from operator import attrgetter

from .. import _docstring
from .. import _api, rcParams
from .._native import axis as _axis
from ..axis import Axis
from ..rcsetup import validate_axisbelow
from ..transforms import ScaledTranslation


class _axis_method_wrapper:
    """
    Helper to generate Axes methods wrapping Axis methods.

    After ::

        get_foo = _axis_method_wrapper("xaxis", "get_bar")

    (in the body of a class) ``get_foo`` is a method that forwards it arguments
    to the ``get_bar`` method of the ``xaxis`` attribute, and gets its
    signature and docstring from ``Axis.get_bar``.

    Kept from matplotlib 3.11.2 (Matplotlib License).
    """

    def __init__(self, attr_name, method_name, *, doc_sub=None):
        self.attr_name = attr_name
        self.method_name = method_name
        doc = inspect.getdoc(getattr(Axis, method_name))
        if doc:
            doc_sub = {"this Axis": f"the {self.attr_name}", **(doc_sub or {})}
            for k, v in doc_sub.items():
                doc = doc.replace(k, v)
        self.__doc__ = doc

    def __set_name__(self, owner, name):
        get_method = attrgetter(f"{self.attr_name}.{self.method_name}")

        def wrapper(self, *args, **kwargs):
            return get_method(self)(*args, **kwargs)

        wrapper.__module__ = owner.__module__
        wrapper.__name__ = name
        wrapper.__qualname__ = f"{owner.__qualname__}.{name}"
        wrapper.__doc__ = self.__doc__
        wrapper.__signature__ = inspect.signature(
            getattr(Axis, self.method_name))
        setattr(owner, name, wrapper)


class TicksMixin:
    """The tick and scale methods of `~.axes.Axes`."""

    get_xgridlines = _axis_method_wrapper("xaxis", "get_gridlines")
    get_xticklines = _axis_method_wrapper("xaxis", "get_ticklines")
    get_ygridlines = _axis_method_wrapper("yaxis", "get_gridlines")
    get_yticklines = _axis_method_wrapper("yaxis", "get_ticklines")
    get_xmajorticklabels = _axis_method_wrapper("xaxis", "get_majorticklabels")
    get_xminorticklabels = _axis_method_wrapper("xaxis", "get_minorticklabels")
    get_xticklabels = _axis_method_wrapper("xaxis", "get_ticklabels")
    get_ymajorticklabels = _axis_method_wrapper("yaxis", "get_majorticklabels")
    get_yminorticklabels = _axis_method_wrapper("yaxis", "get_minorticklabels")
    get_yticklabels = _axis_method_wrapper("yaxis", "get_ticklabels")
    xaxis_date = _axis_method_wrapper("xaxis", "axis_date")
    yaxis_date = _axis_method_wrapper("yaxis", "axis_date")

    def _padded(self, transform, dx, dy):
        """*transform* followed by an offset of (*dx*, *dy*) points."""
        return transform + ScaledTranslation(
            dx / 72, dy / 72, self.get_figure(root=False).dpi_scale_trans)

    def get_xaxis_text1_transform(self, pad_points):
        """
        Returns
        -------
        transform : Transform
            The transform used for drawing x-axis labels, which will add
            *pad_points* of padding (in points) between the axis and the label.
            The x-direction is in data coordinates and the y-direction is in
            axis coordinates
        valign : {'center', 'top', 'bottom', 'baseline', 'center_baseline'}
            The text vertical alignment.
        halign : {'center', 'left', 'right'}
            The text horizontal alignment.
        """
        return (self._padded(self.get_xaxis_transform(which='tick1'),
                             0, -pad_points),
                "top", rcParams["xtick.alignment"])

    def get_xaxis_text2_transform(self, pad_points):
        """
        Returns
        -------
        transform : Transform
            The transform used for drawing secondary x-axis labels, which will
            add *pad_points* of padding (in points) between the axis and the
            label.  The x-direction is in data coordinates and the y-direction
            is in axis coordinates
        valign : {'center', 'top', 'bottom', 'baseline', 'center_baseline'}
            The text vertical alignment.
        halign : {'center', 'left', 'right'}
            The text horizontal alignment.
        """
        return (self._padded(self.get_xaxis_transform(which='tick2'),
                             0, pad_points),
                "bottom", rcParams["xtick.alignment"])

    def get_yaxis_text1_transform(self, pad_points):
        """
        Returns
        -------
        transform : Transform
            The transform used for drawing y-axis labels, which will add
            *pad_points* of padding (in points) between the axis and the label.
            The x-direction is in axis coordinates and the y-direction is in
            data coordinates
        valign : {'center', 'top', 'bottom', 'baseline', 'center_baseline'}
            The text vertical alignment.
        halign : {'center', 'left', 'right'}
            The text horizontal alignment.
        """
        return (self._padded(self.get_yaxis_transform(which='tick1'),
                             -pad_points, 0),
                rcParams["ytick.alignment"], "right")

    def get_yaxis_text2_transform(self, pad_points):
        """
        Returns
        -------
        transform : Transform
            The transform used for drawing secondary y-axis labels, which will
            add *pad_points* of padding (in points) between the axis and the
            label.  The x-direction is in axis coordinates and the y-direction
            is in data coordinates
        valign : {'center', 'top', 'bottom', 'baseline', 'center_baseline'}
            The text vertical alignment.
        halign : {'center', 'left', 'right'}
            The text horizontal alignment.
        """
        return (self._padded(self.get_yaxis_transform(which='tick2'),
                             pad_points, 0),
                rcParams["ytick.alignment"], "left")

    def get_axisbelow(self):
        """
        Get whether axis ticks and gridlines are above or below most artists.

        Returns
        -------
        bool or 'line'

        See Also
        --------
        set_axisbelow
        """
        return _axis.get_axisbelow(self._native)

    def set_axisbelow(self, b):
        """
        Set whether axis ticks and gridlines are above or below most artists.

        This controls the zorder of the ticks and gridlines. For more
        information on the zorder see :doc:`/gallery/misc/zorder_demo`.

        Parameters
        ----------
        b : bool or 'line'
            Possible values:

            - *True* (zorder = 0.5): Ticks and gridlines are below patches and
              lines, though still above images.
            - 'line' (zorder = 1.5): Ticks and gridlines are above patches
              (e.g. rectangles, with default zorder = 1) but still below lines
              and markers (with their default zorder = 2).
            - *False* (zorder = 2.5): Ticks and gridlines are above patches
              and lines / markers.

        Notes
        -----
        For more control, call the `~.Artist.set_zorder` method of each axis.

        See Also
        --------
        get_axisbelow
        """
        axisbelow = validate_axisbelow(b)
        _axis.set_axisbelow(self._native, {True: "below", "line": "line",
                                           False: "above"}[axisbelow])
        # The axes' axes take the zorder of the new setting.
        for name in self._axis_names:
            getattr(self, f"{name}axis").__dict__.pop("_zorder_set", None)

    # matplotlib's attribute, which subclasses read in their ``draw``
    # (astropy's ``WCSAxes`` orders its own ticks by it).
    _axisbelow = property(get_axisbelow, set_axisbelow)

    @_docstring.interpd
    def grid(self, visible=None, which='major', axis='both', **kwargs):
        """
        Configure the grid lines.

        Parameters
        ----------
        visible : bool or None, optional
            Whether to show the grid lines.  If any *kwargs* are supplied, it
            is assumed you want the grid on and *visible* will be set to True.

            If *visible* is *None* and there are no *kwargs*, this toggles the
            visibility of the lines.

        which : {'major', 'minor', 'both'}, optional
            The grid lines to apply the changes on.

        axis : {'both', 'x', 'y'}, optional
            The axis to apply the changes on.

        **kwargs : `~matplotlib.lines.Line2D` properties
            Define the line properties of the grid, e.g.::

                grid(color='r', linestyle='-', linewidth=2)

            Valid keyword arguments are:

            %(Line2D:kwdoc)s

        Notes
        -----
        The axis is drawn as a unit, so the effective zorder for drawing the
        grid is determined by the zorder of each axis, not by the zorder of the
        `.Line2D` objects comprising the grid.  Therefore, to set grid zorder,
        use `.set_axisbelow` or, for more control, call the
        `~.Artist.set_zorder` method of each axis.
        """
        _api.check_in_list(['x', 'y', 'both'], axis=axis)
        if axis in ['x', 'both']:
            self.xaxis.grid(visible, which=which, **kwargs)
        if axis in ['y', 'both']:
            self.yaxis.grid(visible, which=which, **kwargs)

    def ticklabel_format(self, *, axis='both', style=None, scilimits=None,
                         useOffset=None, useLocale=None, useMathText=None):
        r"""
        Configure the `.ScalarFormatter` used by default for linear Axes.

        If a parameter is not set, the corresponding property of the formatter
        is left unchanged.

        Parameters
        ----------
        axis : {'x', 'y', 'both'}, default: 'both'
            The axis to configure.  Only major ticks are affected.

        style : {'sci', 'scientific', 'plain'}
            Whether to use scientific notation.
            The formatter default is to use scientific notation.
            'sci' is equivalent to 'scientific'.

        scilimits : pair of ints (m, n)
            Scientific notation is used only for numbers outside the range
            10\ :sup:`m` to 10\ :sup:`n` (and only if the formatter is
            configured to use scientific notation at all).  Use (0, 0) to
            include all numbers.  Use (m, m) where m != 0 to fix the order of
            magnitude to 10\ :sup:`m`.
            The formatter default is :rc:`axes.formatter.limits`.

        useOffset : bool or float
            If True, the offset is calculated as needed.
            If False, no offset is used.
            If a numeric value, it sets the offset.
            The formatter default is :rc:`axes.formatter.useoffset`.

        useLocale : bool
            Whether to format the number using the current locale or using the
            C (English) locale.  This affects e.g. the decimal separator.  The
            formatter default is :rc:`axes.formatter.use_locale`.

        useMathText : bool
            Render the offset and scientific notation in mathtext.
            The formatter default is :rc:`axes.formatter.use_mathtext`.

        Raises
        ------
        AttributeError
            If the current formatter is not a `.ScalarFormatter`.
        """
        if isinstance(style, str):
            style = style.lower()
        axis = axis.lower()
        if scilimits is not None:
            try:
                m, n = scilimits
                m + n + 1  # check that both are numbers
            except (ValueError, TypeError) as err:
                raise ValueError("scilimits must be a sequence of 2 integers"
                                 ) from err
        STYLES = {'sci': True, 'scientific': True, 'plain': False, '': None,
                  None: None}
        is_sci_style = _api.getitem_checked(STYLES, style=style)
        axises_of = {name: getattr(self, f"{name}axis")
                     for name in self._axis_names}
        axis_map = {**{k: [v] for k, v in axises_of.items()},
                    'both': list(axises_of.values())}
        axises = _api.getitem_checked(axis_map, axis=axis)
        try:
            for axis in axises:
                if is_sci_style is not None:
                    axis.major.formatter.set_scientific(is_sci_style)
                if scilimits is not None:
                    axis.major.formatter.set_powerlimits(scilimits)
                if useOffset is not None:
                    axis.major.formatter.set_useOffset(useOffset)
                if useLocale is not None:
                    axis.major.formatter.set_useLocale(useLocale)
                if useMathText is not None:
                    axis.major.formatter.set_useMathText(useMathText)
        except AttributeError as err:
            raise AttributeError(
                "This method only works with the ScalarFormatter") from err

    def locator_params(self, axis='both', tight=None, **kwargs):
        """
        Control behavior of major tick locators.

        Because the locator is involved in autoscaling, `~.Axes.autoscale_view`
        is called automatically after the parameters are changed.

        Parameters
        ----------
        axis : {'both', 'x', 'y'}, default: 'both'
            The axis on which to operate.  (For 3D Axes, *axis* can also be
            set to 'z', and 'both' refers to all three axes.)
        tight : bool or None, optional
            Parameter passed to `~.Axes.autoscale_view`.
            Default is None, for no change.

        Other Parameters
        ----------------
        **kwargs
            Remaining keyword arguments are passed to directly to the
            ``set_params()`` method of the locator. Supported keywords depend
            on the type of the locator. See for example
            `~.ticker.MaxNLocator.set_params` for the `.ticker.MaxNLocator`
            used by default for linear.

        Examples
        --------
        When plotting small subplots, one might want to reduce the maximum
        number of ticks and use tight bounds, for example::

            ax.locator_params(tight=True, nbins=4)

        """
        _api.check_in_list([*self._axis_names, "both"], axis=axis)
        for name in self._axis_names:
            if axis in [name, "both"]:
                loc = getattr(self, f"{name}axis").get_major_locator()
                loc.set_params(**kwargs)
                self._request_autoscale_view(name, tight=tight)

    def _request_autoscale_view(self, axis="all", tight=None):
        """
        Mark a single axis, or all of them, as stale wrt. autoscaling.

        No computation is performed until the next autoscaling; thus, separate
        calls to control individual `Axis`s incur negligible performance cost.

        Parameters
        ----------
        axis : str, default: "all"
            Either an element of ``self._axis_names``, or "all".
        tight : bool or None, default: None
        """
        axis_names = _api.getitem_checked(
            {**{k: [k] for k in self._axis_names}, "all": self._axis_names},
            axis=axis)
        for name in axis_names:
            _axis.request_autoscale(self._native, name, tight)

    def tick_params(self, axis='both', **kwargs):
        """
        Change the appearance of ticks, tick labels, and gridlines.

        Tick properties that are not explicitly set using the keyword
        arguments remain unchanged unless *reset* is True. For the current
        style settings, see `.Axis.get_tick_params`.

        Parameters
        ----------
        axis : {'x', 'y', 'both'}, default: 'both'
            The axis to which the parameters are applied.
        which : {'major', 'minor', 'both'}, default: 'major'
            The group of ticks to which the parameters are applied.
        reset : bool, default: False
            Whether to reset the ticks to defaults before updating them.

        Other Parameters
        ----------------
        direction : {'in', 'out', 'inout'}
            Puts ticks inside the Axes, outside the Axes, or both.
        length : float
            Tick length in points.
        width : float
            Tick width in points.
        color : :mpltype:`color`
            Tick color.
        pad : float
            Distance in points between tick and label.
        labelsize : float or str
            Tick label font size in points or as a string (e.g., 'large').
        labelcolor : :mpltype:`color`
            Tick label color.
        labelfontfamily : str
            Tick label font.
        colors : :mpltype:`color`
            Tick color and label color.
        zorder : float
            Tick and label zorder.
        bottom, top, left, right : bool
            Whether to draw the respective ticks.
        labelbottom, labeltop, labelleft, labelright : bool
            Whether to draw the respective tick labels.
        labelrotation : float
            Tick label rotation angle in degrees. See `.Text.set_rotation`.
        labelrotation_mode : {'default', 'anchor', 'xtick', 'ytick'}
            Tick label rotation mode. See `.Text.set_rotation_mode`.
        grid_color : :mpltype:`color`
            Gridline color.
        grid_alpha : float
            Transparency of gridlines: 0 (transparent) to 1 (opaque).
        grid_linewidth : float
            Width of gridlines in points.
        grid_linestyle : str
            Any valid `.Line2D` line style spec.

        Examples
        --------
        ::

            ax.tick_params(direction='out', length=6, width=2, colors='r',
                           grid_color='r', grid_alpha=0.5)

        This will make all major ticks be red, pointing out of the box,
        and with dimensions 6 points by 2 points.  Tick labels will
        also be red.  Gridlines will be red and translucent.

        """
        _api.check_in_list(['x', 'y', 'both'], axis=axis)
        if axis in ['x', 'both']:
            xkw = dict(kwargs)
            xkw.pop('left', None)
            xkw.pop('right', None)
            xkw.pop('labelleft', None)
            xkw.pop('labelright', None)
            self.xaxis.set_tick_params(**xkw)
        if axis in ['y', 'both']:
            ykw = dict(kwargs)
            ykw.pop('top', None)
            ykw.pop('bottom', None)
            ykw.pop('labeltop', None)
            ykw.pop('labelbottom', None)
            self.yaxis.set_tick_params(**ykw)

    def minorticks_on(self):
        """
        Display minor ticks on the Axes.

        Displaying minor ticks may reduce performance; you may turn them off
        using `minorticks_off()` if drawing speed is a problem.
        """
        self.xaxis.minorticks_on()
        self.yaxis.minorticks_on()

    def minorticks_off(self):
        """Remove minor ticks from the Axes."""
        self.xaxis.minorticks_off()
        self.yaxis.minorticks_off()


class LogScalePlotsMixin:
    """The plots of `~.axes.Axes` on log scales."""

    def _log_kwargs(self, kwargs, name):
        """The scale keywords of *kwargs* for the *name* axis of a log
        plot (matplotlib's ``loglog``, ``semilogx`` and ``semilogy``)."""
        return {k: v for k, v in kwargs.items()
                if k in ['base', 'subs', 'nonpositive',
                         f'base{name}', f'subs{name}', f'nonpos{name}']}

    @_docstring.interpd
    def loglog(self, *args, **kwargs):
        """
        Make a plot with log scaling on both the x- and y-axis.

        Call signatures::

            loglog([x], y, [fmt], data=None, **kwargs)
            loglog([x], y, [fmt], [x2], y2, [fmt2], ..., **kwargs)

        This is just a thin wrapper around `.plot` which additionally changes
        both the x-axis and the y-axis to log scaling. All the concepts and
        parameters of plot can be used here as well.

        The additional parameters *base*, *subs* and *nonpositive* control the
        x/y-axis properties. They are just forwarded to `.Axes.set_xscale` and
        `.Axes.set_yscale`. To use different properties on the x-axis and the
        y-axis, use e.g.
        ``ax.set_xscale("log", base=10); ax.set_yscale("log", base=2)``.

        Parameters
        ----------
        base : float, default: 10
            Base of the logarithm.

        subs : sequence, optional
            The location of the minor ticks. If *None*, reasonable locations
            are automatically chosen depending on the number of decades in the
            plot. See `.Axes.set_xscale`/`.Axes.set_yscale` for details.

        nonpositive : {'mask', 'clip'}, default: 'clip'
            Non-positive values can be masked as invalid, or clipped to a very
            small positive number.

        **kwargs
            All parameters supported by `.plot`.

        Returns
        -------
        list of `.Line2D`
            Objects representing the plotted data.
        """
        dx = self._log_kwargs(kwargs, 'x')
        self.set_xscale('log', **dx)
        dy = self._log_kwargs(kwargs, 'y')
        self.set_yscale('log', **dy)
        return self.plot(
            *args, **{k: v for k, v in kwargs.items() if k not in {*dx, *dy}})

    @_docstring.interpd
    def semilogx(self, *args, **kwargs):
        """
        Make a plot with log scaling on the x-axis.

        Call signatures::

            semilogx([x], y, [fmt], data=None, **kwargs)
            semilogx([x], y, [fmt], [x2], y2, [fmt2], ..., **kwargs)

        This is just a thin wrapper around `.plot` which additionally changes
        the x-axis to log scaling. All the concepts and parameters of plot can
        be used here as well.

        The additional parameters *base*, *subs*, and *nonpositive* control the
        x-axis properties. They are just forwarded to `.Axes.set_xscale`.

        Parameters
        ----------
        base : float, default: 10
            Base of the x logarithm.

        subs : array-like, optional
            The location of the minor xticks. If *None*, reasonable locations
            are automatically chosen depending on the number of decades in the
            plot. See `.Axes.set_xscale` for details.

        nonpositive : {'mask', 'clip'}, default: 'clip'
            Non-positive values in x can be masked as invalid, or clipped to a
            very small positive number.

        **kwargs
            All parameters supported by `.plot`.

        Returns
        -------
        list of `.Line2D`
            Objects representing the plotted data.
        """
        d = self._log_kwargs(kwargs, 'x')
        self.set_xscale('log', **d)
        return self.plot(
            *args, **{k: v for k, v in kwargs.items() if k not in d})

    @_docstring.interpd
    def semilogy(self, *args, **kwargs):
        """
        Make a plot with log scaling on the y-axis.

        Call signatures::

            semilogy([x], y, [fmt], data=None, **kwargs)
            semilogy([x], y, [fmt], [x2], y2, [fmt2], ..., **kwargs)

        This is just a thin wrapper around `.plot` which additionally changes
        the y-axis to log scaling. All the concepts and parameters of plot can
        be used here as well.

        The additional parameters *base*, *subs*, and *nonpositive* control the
        y-axis properties. They are just forwarded to `.Axes.set_yscale`.

        Parameters
        ----------
        base : float, default: 10
            Base of the y logarithm.

        subs : array-like, optional
            The location of the minor yticks. If *None*, reasonable locations
            are automatically chosen depending on the number of decades in the
            plot. See `.Axes.set_yscale` for details.

        nonpositive : {'mask', 'clip'}, default: 'clip'
            Non-positive values in y can be masked as invalid, or clipped to a
            very small positive number.

        **kwargs
            All parameters supported by `.plot`.

        Returns
        -------
        list of `.Line2D`
            Objects representing the plotted data.
        """
        d = self._log_kwargs(kwargs, 'y')
        self.set_yscale('log', **d)
        return self.plot(
            *args, **{k: v for k, v in kwargs.items() if k not in d})
