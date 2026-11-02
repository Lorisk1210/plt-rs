r"""
The axes of a 3-D axes (matplotlib's ``mpl_toolkits.mplot3d.axis3d``):
`XAxis`, `YAxis` and `ZAxis`, each reaching pltrs (Rust)'s axis through its
`.Axes3D` and its name.

As in matplotlib, a 3-D axis is an `.axis.XAxis`: its tick helpers, its
`.Tick`\s, ``label``, ``offsetText`` and ``labelpad``, its tick parameters
and its tight box are those of a 2-D axis, which ``_native.axis`` routes to
the 3-D axis its axes and name designate. What only 3-D axes have (edges,
panes, the axis line) is ``_native.mplot3d``'s.
"""

import numpy as np

from pltrs import _api, axis as maxis, cbook, colors as mcolors
from pltrs import collections as mcoll, patches as mpatches
from pltrs._native import mplot3d as _mplot3d


#: The `.Text` properties of tick labels a 3-D axis draws, as the tick
#: parameters setting them for a whole tick level.
_LABEL_PARAMS = {"color": "labelcolor", "fontsize": "labelsize",
                 "rotation": "labelrotation", "fontfamily": "labelfontfamily"}


class _AxisLine:
    """
    The bolded line of a 3-D axis (matplotlib's ``axis.line``): pltrs
    (Rust) draws it along the edge the tick position names; this reads its
    projected end points and shows or hides it.
    """

    def __init__(self, axis):
        self._axis = axis

    def _args(self):
        return self._axis.axes._native, self._axis.axis_name

    def get_data(self, orig=True):
        """Return the x and y data of the line's end points, in projected
        coordinates."""
        data = _mplot3d.axis_line_data(*self._args())
        if data is None:
            return np.array([]), np.array([])
        return np.array(data[0]), np.array(data[1])

    def get_xdata(self, orig=True):
        """Return the x data of the line's end points."""
        return self.get_data()[0]

    def get_ydata(self, orig=True):
        """Return the y data of the line's end points."""
        return self.get_data()[1]

    def get_visible(self):
        """Return whether the line is drawn."""
        return _mplot3d.get_line_visible(*self._args())

    def set_visible(self, b):
        """Set whether the line is drawn."""
        _mplot3d.set_line_visible(*self._args(), bool(b))


class Axis(maxis.XAxis):
    """An Axis class for the 3D plots."""

    # pltrs (Rust) converts the values in units of 3-D axes.
    _native_units = True

    def __init__(self, axes, *, rotate_label=None):
        super().__init__(axes)
        self.line = _AxisLine(self)
        if rotate_label is not None:
            self.set_rotate_label(rotate_label)

    adir = _api.deprecated("3.6", pending=True)(
        property(lambda self: self.axis_name))

    @_api.deprecated("3.6", pending=True)
    def init3d(self):
        """Make the axis line again, as a new axis has it: shown."""
        self.line.set_visible(True)

    def _native_args(self):
        """The native 3-D axes and the axis name."""
        return self.axes._native, self.axis_name

    def get_data_interval(self):
        """Return the ``(min, max)`` data limits of this axis."""
        return np.array(_mplot3d.get_data_interval(*self._native_args()))

    def _set_interval(self, getter, assign, vmin, vmax, ignore):
        """Set an interval as matplotlib's ``_make_getset_interval`` setters
        do: to (*vmin*, *vmax*) with *ignore*, else widened to hold the
        current one, keeping its orientation."""
        if not ignore:
            oldmin, oldmax = getter()
            if oldmin < oldmax:
                vmin, vmax = min(vmin, vmax, oldmin), max(vmin, vmax, oldmax)
            else:
                vmin, vmax = max(vmin, vmax, oldmin), min(vmin, vmax, oldmax)
        assign(float(vmin), float(vmax))
        self.stale = True

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
        self._set_interval(
            self.get_data_interval,
            lambda lo, hi: _mplot3d.set_data_interval(
                *self._native_args(), (lo, hi)),
            vmin, vmax, ignore)

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
        self._set_interval(
            self.get_view_interval,
            lambda lo, hi: _mplot3d.set_lim(
                *self._native_args(), (lo, hi), 0.0, None),
            vmin, vmax, ignore)

    d_interval = _api.deprecated(
        "3.6", alternative="get_data_interval", pending=True)(
            property(lambda self: self.get_data_interval(),
                     lambda self, minmax: self.set_data_interval(*minmax)))
    v_interval = _api.deprecated(
        "3.6", alternative="get_view_interval", pending=True)(
            property(lambda self: self.get_view_interval(),
                     lambda self, minmax: self.set_view_interval(*minmax)))

    def set_rotate_label(self, val):
        """
        Whether to rotate the axis label: True, False or None.
        If set to None the label will be rotated if longer than 4 chars.
        """
        _mplot3d.set_rotate_label(*self._native_args(),
                                  None if val is None else bool(val))
        self.stale = True

    def get_rotate_label(self, text):
        """Return whether a label *text* of this axis is drawn along the
        axis: as `set_rotate_label` says, else when longer than 4 chars."""
        return _mplot3d.get_rotate_label(*self._native_args(), str(text))

    def active_pane(self):
        """
        The pane of this axis the view shows, the side of the box farthest
        from the viewer along the axis: its four corners projected (x and y
        on the projection plane, then depth), and where it lies along the
        axis.
        """
        corners, loc = _mplot3d.active_pane(*self._native_args())
        return np.array(corners), loc

    def _pane_artist(self):
        """The polygon the pane is drawn as (matplotlib's ``axis.pane``),
        in the axes' figure, coloured as the pane is now."""
        pane = self.__dict__.get("_pane")
        if pane is None:
            pane = self._pane = mpatches.Polygon([[0, 0], [0, 1]], closed=False)
            pane.set_transform(self.axes.transData)
            pane.set_figure(self.axes.get_figure(root=True))
        color = _mplot3d.get_pane_color(*self._native_args())
        pane.set_edgecolor(color)
        pane.set_facecolor(color)
        pane.set_alpha(color[-1])
        return pane

    def draw_pane(self, renderer):
        """
        Draw pane.

        Parameters
        ----------
        renderer : `~matplotlib.backend_bases.RendererBase` subclass
        """
        renderer.open_group('pane3d', gid=self.get_gid())
        xys, loc = self.active_pane()
        pane = self._pane_artist()
        pane.xy = xys[:, :2]
        pane.draw(renderer)
        renderer.close_group('pane3d')

    def _gridline_artist(self):
        """The collection the gridlines are drawn as (matplotlib's
        ``axis.gridlines``), in the axes' figure."""
        gridlines = self.__dict__.get("_gridlines")
        if gridlines is None:
            gridlines = self._gridlines = mcoll.LineCollection([])
            gridlines.set_transform(self.axes.transData)
            gridlines.set_figure(self.axes.get_figure(root=True))
        return gridlines

    def draw_grid(self, renderer):
        """Draw the gridlines of this axis across the panes, as the axes
        draws them, if its grid is on."""
        if not _mplot3d.get_grid(self.axes._native):
            return
        renderer.open_group("grid3d", gid=self.get_gid())
        lines = _mplot3d.gridlines(*self._native_args())
        if lines:
            gridlines = self._gridline_artist()
            gridlines.set_segments([points for points, *_ in lines])
            gridlines.set_color([color for _, color, _, _ in lines])
            gridlines.set_linewidth([width for _, _, width, _ in lines])
            gridlines.set_linestyle([style for *_, style in lines])
            gridlines.draw(renderer)
        renderer.close_group('grid3d')

    def _get_tickdir(self, position):
        """
        Get the direction of the tick.

        Parameters
        ----------
        position : str, optional : {'upper', 'lower', 'default'}
            The position of the axis.

        Returns
        -------
        tickdir : int
            Index which indicates which coordinate the tick line will
            align with.
        """
        _api.check_in_list(('upper', 'lower', 'default'), position=position)
        return _mplot3d.tick_direction(self.axes._native, self.axis_name, position)

    def get_view_interval(self):
        """Return the view limits ``(min, max)`` of this axis."""
        return np.array(_mplot3d.get_lim(self.axes._native, self.axis_name))

    def get_scale(self):
        """Return this Axis' scale (as a str)."""
        return _mplot3d.get_scale(self.axes._native, self.axis_name)

    def get_inverted(self):
        """Return whether this Axis is oriented in the "inverse" direction."""
        return _mplot3d.get_inverted(self.axes._native, self.axis_name)

    def set_inverted(self, inverted):
        """Set whether this Axis is oriented in the "inverse" direction."""
        _mplot3d.set_inverted(self.axes._native, self.axis_name, bool(inverted))

    def set_ticks(self, ticks, labels=None, *, minor=False, **kwargs):
        """
        Set this Axis' tick locations and optionally tick labels.

        *kwargs* are `.Text` properties for the labels, allowed only with
        *labels* (see `set_ticklabels`).
        """
        if labels is None and kwargs:
            first_key = next(iter(kwargs))
            raise ValueError(
                f"Incorrect use of keyword argument {first_key!r}. Keyword "
                "arguments other than 'minor' modify the text labels and can "
                "only be used if 'labels' are passed as well.")
        _mplot3d.set_ticks(self.axes._native, self.axis_name,
                           [float(t) for t in ticks], bool(minor))
        result = (self.get_minor_ticks(len(ticks)) if minor
                  else self.get_major_ticks(len(ticks)))
        if labels is not None:
            self.set_ticklabels(labels, minor=minor, **kwargs)
        return result

    def set_ticklabels(self, labels, *, minor=False, fontdict=None, **kwargs):
        """
        Set this Axis' tick labels with list of string labels.

        *fontdict* and *kwargs* are `.Text` properties for the labels. A
        3-D axis draws the labels of a tick level in one style, so those
        `~.Axis.set_tick_params` sets for its labels (``color``,
        ``fontsize``, ``rotation``, ``fontfamily``) apply to the level.

        Returns the visible labels of the level's ticks, ``label1`` then
        ``label2`` of each tick.
        """
        from pltrs.text import Text
        if fontdict is not None:
            kwargs.update(fontdict)
        params = {}
        for key, value in cbook.normalize_kwargs(kwargs, Text).items():
            if key not in _LABEL_PARAMS:
                raise NotImplementedError(
                    f"pltrs: set_ticklabels({key}=...) on 3-D axes: a 3-D "
                    f"axis draws the labels of a tick level in one style, "
                    f"of the properties tick_params sets")
            params[_LABEL_PARAMS[key]] = value
        _mplot3d.set_ticklabels(self.axes._native, self.axis_name,
                                [str(label) for label in labels], bool(minor))
        if params:
            self.set_tick_params(which="minor" if minor else "major", **params)
        ticks = self.get_minor_ticks() if minor else self.get_major_ticks()
        return [label for tick in ticks for label in (tick.label1, tick.label2)
                if label.get_visible()]

    def get_ticklocs(self, *, minor=False):
        """Return this Axis' tick locations in data coordinates."""
        return np.array(_mplot3d.get_ticklocs(self.axes._native,
                                              self.axis_name, bool(minor)))

    def get_majorticklocs(self):
        """Return this Axis' major tick locations in data coordinates."""
        return self.get_ticklocs()

    def get_minorticklocs(self):
        """Return this Axis' minor tick locations in data coordinates."""
        return self.get_ticklocs(minor=True)

    def set_ticks_position(self, position):
        """
        Set the ticks position.

        Parameters
        ----------
        position : {'lower', 'upper', 'both', 'default', 'none'}
            The position of the bolded axis lines, ticks, and tick labels.
        """
        _api.check_in_list(['lower', 'upper', 'both', 'default', 'none'],
                           position=position)
        _mplot3d.set_ticks_position(self.axes._native, self.axis_name, position)

    def get_ticks_position(self):
        """Get the ticks position."""
        return _mplot3d.get_ticks_position(self.axes._native, self.axis_name)

    def set_label_position(self, position):
        """
        Set the label position.

        Parameters
        ----------
        position : {'lower', 'upper', 'both', 'default', 'none'}
            The position of the axis label.
        """
        _api.check_in_list(['lower', 'upper', 'both', 'default', 'none'],
                           position=position)
        _mplot3d.set_label_position(self.axes._native, self.axis_name, position)

    def get_label_position(self):
        """Get the label position."""
        return _mplot3d.get_label_position(self.axes._native, self.axis_name)

    def set_pane_color(self, color, alpha=None):
        """
        Set pane color.

        Parameters
        ----------
        color : :mpltype:`color`
            Color for axis pane.
        alpha : float, optional
            Alpha value for axis pane. If None, base it on *color*.
        """
        _mplot3d.set_pane_color(self.axes._native, self.axis_name,
                                mcolors.to_rgba(color, alpha))


class XAxis(Axis):
    """The x axis of a 3-D axes."""
    axis_name = "x"


class YAxis(Axis):
    """The y axis of a 3-D axes."""
    axis_name = "y"


class ZAxis(Axis):
    """The z axis of a 3-D axes."""
    axis_name = "z"
