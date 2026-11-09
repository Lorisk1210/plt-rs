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

from numbers import Real

import numpy as np

from pltrs import _api, _mirrors, _values, artist, axis as maxis, colors as mcolors
from pltrs import collections as mcoll, lines as mlines, patches as mpatches
from pltrs._native import artist as _artist, mplot3d as _mplot3d

from . import art3d
from .art3d import _no_projection_error



def _same_truth(value, native):
    """Whether the flag *value*, as given, is the bool *native*."""
    return bool(value) == native

class _AxisLinePart(_mirrors.Mirroring):
    """
    The bolded line of a 3-D axis, which pltrs (Rust) draws along the edge
    its tick position names: ``axis.line`` (matplotlib's `.Line2D`)
    mirrors it, its visibility, width and colour read back into the axis.
    """

    def __init__(self, axis):
        self.axis = axis

    def push(self, artist):
        _mplot3d.push_line(*self.axis._native_args(), (
            bool(artist.get_visible()), float(artist.get_linewidth()),
            mcolors.to_rgba(artist.get_color(), artist.get_alpha())))

    def figure(self, artist, root=False):
        return self.axis.get_figure(root=root)

    def transform(self, artist):
        return self.axis.axes.transData

    def transform_set(self, artist):
        return True


class Axis(maxis.XAxis):
    """An Axis class for the 3D plots."""

    # pltrs (Rust) converts the values in units of 3-D axes.
    _native_units = True

    def _drawn_without_axes(self):
        # matplotlib's draw of a 3-D axis, visible or not, starts from the
        # scaled limits of its Axes.
        getattr(None, "_get_scaled_limits")

    #: What `.XAxis` records of its label's side, which a 3-D axis does
    #: not read: its label goes where `get_label_position` says.
    label_position = 'bottom'

    pane = property(lambda self: self._pane_artist(),
                    doc="The polygon the pane of the axis is drawn as.")
    gridlines = property(lambda self: self._gridline_artist(),
                         doc="The collection the gridlines are drawn as.")

    def __init__(self, axes, *, rotate_label=None, **kwargs):
        super().__init__(axes, **kwargs)
        if rotate_label is not None:
            self.set_rotate_label(rotate_label)

    @property
    def line(self):
        """
        The line of the axis (matplotlib's `.Line2D`), between the ends
        of the edge the last draw put it on (``(0, 0)`` before the first).
        """
        line = self.__dict__.get("_line")
        if line is None:
            visible, width, color = _mplot3d.line_state(*self._native_args())
            line = self._line = mlines.Line2D(
                xdata=(0, 0), ydata=(0, 0), linewidth=width, color=color,
                antialiased=True, visible=visible)
            _mirrors.attach(line, _AxisLinePart(self))
        data = (None if self.axes.M is None
                else _mplot3d.axis_line_data(*self._native_args()))
        if data is not None:
            with _mirrors.detached(line):
                line.set_data(np.array(data[0]), np.array(data[1]))
        return line

    adir = _api.deprecated("3.6", pending=True)(
        property(lambda self: self.axis_name))

    @_api.deprecated("3.6", pending=True)
    def init3d(self):
        self.line.set_visible(True)

    def _native_args(self):
        """The native 3-D axes and the axis name."""
        return self.axes._native, self.axis_name

    # The values matplotlib's draw of the axes computes with, kept
    # unchecked: the axes' draw fails on them as matplotlib's does
    # (`.Axes3D._record_draw_problems`).

    def set_zorder(self, level):
        # docstring inherited
        super().set_zorder(level)
        self._record_draw_problems()

    def set_agg_filter(self, filter_func):
        # docstring inherited
        super().set_agg_filter(filter_func)
        self._record_draw_problems()

    def _record_draw_problems(self):
        """Have the 3-D axes record the problems its draws meet, once it
        has its three axes (`.Axes3D._record_draw_problems`)."""
        if all(name in vars(self.axes) for name in ("xaxis", "yaxis", "zaxis")):
            self.axes._record_draw_problems()

    @property
    def labelpad(self):
        """The distance between the axis label and the tick labels, in
        points (as given, as matplotlib keeps it)."""
        return maxis.Axis.labelpad.fget(self)

    @labelpad.setter
    def labelpad(self, pad):
        if isinstance(pad, Real):
            maxis.Axis.labelpad.fset(self, pad)
        else:
            _values.keep(self, "labelpad", pad)
        self._record_draw_problems()

    def _get_autoscale_on(self):
        # docstring inherited; as given, as matplotlib keeps it, while
        # pltrs (Rust) holds its truth.
        return _values.given(self, "autoscale_on",
                             _mplot3d.get_autoscale_on(*self._native_args()),
                             same=_same_truth)

    def _set_autoscale_on(self, b):
        # docstring inherited
        if b is not None:
            _mplot3d.set_autoscale_on(*self._native_args(), bool(b))
            _values.remember(self, "autoscale_on", b)

    def _drawn_by_drawer(self):
        # A 3-D axes draws its axes whenever it shows them (matplotlib's
        # `Axes3D.draw`), hidden and animated ones too.
        return self.axes._axis3don

    def _ticklabels(self, minor):
        # matplotlib's `_update_ticks` moves the labels of the ticks it
        # places back to their location along the axis (``update_position``)
        # after the draw put them where the projection shows them.
        self._ticks_placed = _artist.draw_count(self.axes._native)
        return super()._ticklabels(minor)

    def _tick_objects(self, minor, count):
        # docstring inherited; the labels at their location along the axis
        # when matplotlib's `_update_ticks` placed them after the last draw.
        ticks = super()._tick_objects(minor, count)
        drawn = _artist.draw_count(self.axes._native)
        if drawn and self.__dict__.get("_ticks_placed") == drawn:
            # Typed as the locator's array, which is not asked again.
            locator = self.get_minor_locator() if minor else self.get_major_locator()
            locs = maxis._typed_locs(locator, np.array([tick._loc for tick in ticks]),
                                     minor, self.get_view_interval())
            for tick, loc in zip(ticks, locs):
                for label in (tick.label1, tick.label2):
                    with _mirrors.detached(label):
                        label.set_x(loc)
                    _values.remember(label, "x", loc)
        return ticks

    @maxis._interval_method("getter")
    def get_data_interval(self):
        # docstring inherited.
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

    @maxis._interval_method("setter")
    def set_data_interval(self, vmin, vmax, ignore=False):
        # docstring inherited.
        self._set_interval(
            self.get_data_interval,
            lambda lo, hi: _mplot3d.set_data_interval(
                *self._native_args(), (lo, hi)),
            vmin, vmax, ignore)

    @maxis._interval_method("setter")
    def set_view_interval(self, vmin, vmax, ignore=False):
        # docstring inherited.
        self._set_interval(
            self.get_view_interval,
            lambda lo, hi: _mplot3d.set_view_interval(
                *self._native_args(), (lo, hi)),
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
        # matplotlib answers with the value as given.
        self._rotate_label = val
        self.stale = True

    def get_rotate_label(self, text):
        given = self.__dict__.get("_rotate_label")
        if given is not None:
            return given
        return _mplot3d.get_rotate_label(*self._native_args(), str(text))

    def active_pane(self):
        pane = _mplot3d.active_pane(*self._native_args())
        if pane is None:
            raise _no_projection_error()
        corners, loc = pane
        return np.array(corners), np.float64(loc)

    def _pane_artist(self):
        """The polygon the pane is drawn as (matplotlib's ``axis.pane``),
        in the axes' figure, coloured as the pane is now."""
        pane = self.__dict__.get("_pane")
        if pane is None:
            pane = self._pane = mpatches.Polygon([[0, 0], [0, 1]], closed=False)
            pane.set_transform(self.axes.transData)
            pane.set_figure(self.axes.get_figure(root=True))
        # The colour as `set_pane_color` gave it, typed as `to_rgba`
        # types it, while the axis still has it.
        color = _values.given(self, "pane_color",
                              _mplot3d.get_pane_color(*self._native_args()))
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

    def draw(self, renderer):
        # docstring inherited; matplotlib's projects the axes' box through
        # the view of the last draw first.
        if self.axes.M is None:
            raise _no_projection_error()
        super().draw(renderer)

    def _gridline_artist(self):
        """The collection the gridlines are drawn as (matplotlib's
        ``axis.gridlines``), in the axes' figure."""
        gridlines = self.__dict__.get("_gridlines")
        if gridlines is None:
            gridlines = self._gridlines = art3d.Line3DCollection([])
            gridlines.set_transform(self.axes.transData)
            gridlines.set_figure(self.axes.get_figure(root=True))
        return gridlines

    @artist.allow_rasterization
    def draw_grid(self, renderer):
        if not _mplot3d.get_grid(self.axes._native):
            return
        renderer.open_group("grid3d", gid=self.get_gid())
        lines = _mplot3d.gridlines(*self._native_args())
        if lines:
            gridlines = self._gridline_artist()
            # Projected already: the 2-D segments of the collection.
            mcoll.LineCollection.set_segments(
                gridlines, [points for points, *_ in lines])
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
        # matplotlib picks it out of a numpy array.
        return np.int64(_mplot3d.tick_direction(self.axes._native, self.axis_name,
                                                position))

    @maxis._interval_method("getter")
    def get_view_interval(self):
        # docstring inherited.
        return np.array(_mplot3d.get_lim(self.axes._native, self.axis_name))

    def get_scale(self):
        """Return this Axis' scale (as a str)."""
        return _mplot3d.get_scale(self.axes._native, self.axis_name)

    def get_inverted(self):
        """Return whether this Axis is oriented in the "inverse" direction."""
        # A numpy bool, as matplotlib compares the ends of the view interval.
        return np.bool_(_mplot3d.get_inverted(self.axes._native, self.axis_name))

    def set_inverted(self, inverted):
        """Set whether this Axis is oriented in the "inverse" direction."""
        # matplotlib sets the view interval again, in the order asked for:
        # its limits are checked again.
        if not np.isfinite(self.get_view_interval()).all():
            raise ValueError("Axis limits cannot be NaN or Inf")
        _mplot3d.set_inverted(self.axes._native, self.axis_name, bool(inverted))

    def get_tick_space(self):
        """Return the estimated number of ticks that can fit on the axis
        (matplotlib's ``XAxis.get_tick_space``, which a 3-D axis
        inherits: the width of the axes over three label heights)."""
        (x0, _), (x1, _) = self.axes.transAxes.transform([[0, 0], [1, 1]])
        length = (x1 - x0) / self.get_figure(root=True).dpi * 72
        size = self._get_tick_label_size('x') * 3
        return int(np.floor(length / size)) if size > 0 else 2**31 - 1

    def get_minpos(self):
        """Return the smallest positive x datum of the axes: the axes of a
        3-D axes are `.XAxis` objects in matplotlib, reading the 2-D data
        limits of the axes."""
        return self.axes.dataLim.minposx

    def set_label_coords(self, x, y, transform=None):
        """
        Set the coordinates of the label, by default in the axes
        coordinates (matplotlib's `.Axis.set_label_coords`; a 3-D axes
        places its labels itself when it draws).
        """
        self._autolabelpos = False
        if transform is None:
            transform = self.axes.transAxes
        self.label.set_transform(transform)
        self.label.set_position((x, y))
        self.stale = True

    def get_tick_padding(self):
        """Return the space the outward part of the first ticks of each
        level takes, in points."""
        return max((ticks[0].get_tick_padding()
                    for ticks in (self.majorTicks, self.minorTicks) if len(ticks)),
                   default=0)

    def get_in_layout(self):
        """Return whether the axis counts in layouts (a 3-D axes counts its
        axes in its tight box whatever they say, as matplotlib's does)."""
        return self.__dict__.get("_in_layout", True)

    def set_in_layout(self, in_layout):
        # Set whether the axis counts in layouts (see `get_in_layout`).
        self._in_layout = in_layout

    set_in_layout.__doc__ = maxis.Axis.set_in_layout.__doc__

    def set_ticks(self, ticks, labels=None, *, minor=False, **kwargs):
        # Set this Axis' tick locations and optionally tick labels. *kwargs* are
        # `.Text` properties for the labels, allowed only with *labels* (see
        # `set_ticklabels`).
        if labels is None and kwargs:
            first_key = next(iter(kwargs))
            raise ValueError(
                f"Incorrect use of keyword argument {first_key!r}. Keyword "
                "arguments other than 'minor' modify the text labels and can "
                "only be used if 'labels' are passed as well.")
        # matplotlib converts the locations, then its FixedLocator checks
        # them.
        ticks = self.convert_units(ticks)
        values = np.asarray(ticks)
        _api.check_shape((None,), locs=values)
        _mplot3d.set_ticks(self.axes._native, self.axis_name,
                           [float(t) for t in ticks], bool(minor))
        if values.dtype.kind in "iub" and values.size:
            # matplotlib's locator keeps the locations as given (ints).
            from pltrs import _tick_helpers, ticker as mticker
            _tick_helpers.adopt(self, mticker.FixedLocator(values), "locator", minor)
        result = (self.get_minor_ticks(len(ticks)) if minor
                  else self.get_major_ticks(len(ticks)))
        if labels is not None:
            self.set_ticklabels(labels, minor=minor, **kwargs)
        return result

    set_ticks.__doc__ = maxis.Axis.set_ticks.__doc__

    def get_major_ticks(self, numticks=None):
        # docstring inherited
        return super().get_major_ticks(numticks)

    def get_minor_ticks(self, numticks=None):
        # docstring inherited
        return super().get_minor_ticks(numticks)

    def get_tightbbox(self, renderer=None, *, for_layout_only=False):
        # docstring inherited
        return super().get_tightbbox(renderer, for_layout_only=for_layout_only)

    def get_ticklocs(self, *, minor=False):
        """Return this Axis' tick locations in data coordinates."""
        locs = np.array(_mplot3d.get_ticklocs(self.axes._native,
                                              self.axis_name, bool(minor)))
        locator = self.get_minor_locator() if minor else self.get_major_locator()
        return maxis._typed_locs(locator, locs, minor, self.get_view_interval())

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

    def tick_top(self):
        """
        Move ticks and ticklabels to the top, as `.XAxis.tick_top` does
        through `set_ticks_position`, which takes no 'top' on a 3-D axis.
        """
        self.set_ticks_position('top')

    def tick_bottom(self):
        """
        Move ticks and ticklabels to the bottom, as `.XAxis.tick_bottom`
        does through `set_ticks_position`, which takes no 'bottom' on a
        3-D axis.
        """
        self.set_ticks_position('bottom')

    def get_ticks_position(self):
        """
        Get the ticks position.

        Returns
        -------
        str : {'lower', 'upper', 'both', 'default', 'none'}
            The position of the bolded axis lines, ticks, and tick labels.
        """
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
        """
        Get the label position.

        Returns
        -------
        str : {'lower', 'upper', 'both', 'default', 'none'}
            The position of the axis label.
        """
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
        color = mcolors.to_rgba(color, alpha)
        _mplot3d.set_pane_color(self.axes._native, self.axis_name, color)
        _values.remember(self, "pane_color", color)


class XAxis(Axis):
    axis_name = "x"


class YAxis(Axis):
    axis_name = "y"


class ZAxis(Axis):
    axis_name = "z"


# What matplotlib's module also has from its imports, which programs
# import from it.
import inspect  # noqa: E402, F401
import pltrs as mpl  # noqa: E402, F401
from dataclasses import dataclass  # noqa: E402, F401
from pltrs import text as mtext, transforms as mtransforms  # noqa: E402, F401
from pltrs.mpl_toolkits.mplot3d import proj3d  # noqa: E402, F401
