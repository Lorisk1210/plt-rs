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

import inspect
from numbers import Real
import weakref

import numpy as np

import pltrs as mpl
from pltrs import (
    _api, _mirrors, _unusable, _values, artist, axis as maxis, cbook,
    colors as mcolors)
from pltrs import collections as mcoll, lines as mlines, patches as mpatches
from pltrs._native import artist as _artist, mplot3d as _mplot3d

from . import art3d
from .art3d import _no_projection_error



class _AxisInfo(dict):
    """
    matplotlib's ``_axinfo`` dict of a 3-D axis, whose ``juggled`` entry
    (the order in which the axis' line is taken from the box's near
    corner) pltrs (Rust) keeps for the axis: read from it and written to
    it, never held here.
    """

    def __init__(self, axis, entries):
        super().__init__(entries)
        dict.__setitem__(self, "juggled", None)
        self._axis = weakref.ref(axis)

    def _juggled(self):
        axis = self._axis()
        return None if axis is None else _mplot3d.get_juggled(*axis._native_args())

    def __getitem__(self, key):
        return self._juggled() if key == "juggled" else super().__getitem__(key)

    def __setitem__(self, key, value):
        if key == "juggled":
            axis = self._axis()
            if axis is not None:
                first, second, last = value
                _mplot3d.set_juggled(*axis._native_args(),
                                     (int(first), int(second), int(last)))
            return
        super().__setitem__(key, value)

    def get(self, key, default=None):
        return self[key] if key in self else default

    def update(self, *args, **kwargs):
        for key, value in dict(*args, **kwargs).items():
            self[key] = value

    def items(self):
        return [(key, self[key]) for key in self]

    def values(self):
        return [self[key] for key in self]

    def copy(self):
        return dict(self.items())

    def __eq__(self, other):
        return dict(self.items()) == other

    __hash__ = None

    def __repr__(self):
        return repr(dict(self.items()))


def _matplotlib_axinfo(name):
    """matplotlib 3.11.2's ``_axinfo`` of a new 3-D axis *name* under the
    rcParams of now (under the Matplotlib License), but ``juggled``, which
    the axis keeps natively (`_AxisInfo`)."""
    rc = mpl.rcParams
    info = {key: value for key, value in Axis._AXINFO[name].items() if key != 'juggled'}
    info.update({
        'label': {'va': 'center', 'ha': 'center', 'rotation_mode': 'anchor'},
        'color': rc[f'axes3d.{name}axis.panecolor'],
        'tick': {'inward_factor': 0.2, 'outward_factor': 0.1},
    })
    if rc['_internal.classic_mode']:
        info.update({
            'axisline': {'linewidth': 0.75, 'color': (0, 0, 0, 1)},
            'grid': {'color': (0.9, 0.9, 0.9, 1), 'linewidth': 1.0, 'linestyle': '-'},
        })
        info['tick'].update({'linewidth': {True: rc['lines.linewidth'],
                                           False: rc['lines.linewidth']}})
    else:
        info.update({
            'axisline': {'linewidth': rc['axes.linewidth'], 'color': rc['axes.edgecolor']},
            'grid': {'color': rc['grid.color'], 'linewidth': rc['grid.linewidth'],
                     'linestyle': rc['grid.linestyle']},
        })
        info['tick'].update({'linewidth': {
            True: rc['xtick.major.width'] if name in 'xz' else rc['ytick.major.width'],
            False: rc['xtick.minor.width'] if name in 'xz' else rc['ytick.minor.width'],
        }})
    return info


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


class _Pane(mpatches.Polygon):
    """The polygon an axis' pane is drawn as (matplotlib's ``axis.pane``):
    once the program restyles it (its colours, fill, alpha, line), pltrs
    (Rust) draws the pane as it looks (``_axis``, the axis, set once it
    is coloured as the pane)."""

    def _restyled(self):
        """Have pltrs (Rust) draw the pane as the polygon looks now."""
        axis = self.__dict__.get("_axis")
        if axis is not None:
            _mplot3d.set_pane_patch(axis.axes._native, axis.axis_name, self._native)

    def _set_native(self, **kwargs):
        super()._set_native(**kwargs)
        self._restyled()

    def set_alpha(self, alpha):
        super().set_alpha(alpha)
        self._restyled()

    def set_visible(self, b):
        super().set_visible(b)
        self._restyled()


# Named as the class matplotlib's pane is.
_Pane.__name__ = _Pane.__qualname__ = "Polygon"
_Pane.__module__ = mpatches.Polygon.__module__


class Axis(maxis.XAxis):
    """An Axis class for the 3D plots."""

    # pltrs (Rust) converts the values in units of 3-D axes.
    _native_units = True

    def _ask_tick_transforms(self):
        # docstring inherited: pltrs (Rust) places the ticks of a 3-D axis
        # itself, through no transform of the 2-D spines of its Axes.
        pass

    def _drawn_without_axes(self):
        # matplotlib's draw of a 3-D axis, visible or not, starts from the
        # scaled limits of its Axes.
        getattr(None, "_get_scaled_limits")

    def _decouple(self, removed):
        # docstring inherited; matplotlib's draw of the Axes fails where
        # it draws the pane of the axis, once the panes of the axes before
        # it are drawn: pltrs (Rust)'s draw raises it there.
        axes = self._axes
        super()._decouple(removed)
        _unusable.drop(axes, f"{self.axis_name}axis without its Axes")
        try:
            self._drawn_without_axes()
        except AttributeError as exc:
            _mplot3d.set_pane_failure(
                axes._native, self.axis_name, (type(exc).__name__, str(exc)))

    def _recouple(self):
        # docstring inherited
        super()._recouple()
        _mplot3d.set_pane_failure(self._axes._native, self.axis_name, None)

    #: What `.XAxis` records of its label's side, which a 3-D axis does
    #: not read: its label goes where `get_label_position` says.
    label_position = 'bottom'

    pane = property(lambda self: self._pane_artist(),
                    doc="The polygon the pane of the axis is drawn as.")
    gridlines = property(lambda self: self._gridline_artist(),
                         doc="The collection the gridlines are drawn as.")

    def _old_init(self, adir, v_intervalx, d_intervalx, axes, *args,
                  rotate_label=None, **kwargs):
        return locals()

    def _new_init(self, axes, *, rotate_label=None, **kwargs):
        return locals()

    def __init__(self, *args, **kwargs):
        params = _api.select_matching_signature(
            [self._old_init, self._new_init], *args, **kwargs)
        if "adir" in params:
            _api.warn_deprecated(
                "3.6", message=f"The signature of 3D Axis constructors has "
                f"changed in %(since)s; the new signature is "
                f"{inspect.signature(type(self).__init__)}", pending=True)
            if params["adir"] != self.axis_name:
                raise ValueError(f"Cannot instantiate {type(self).__name__} "
                                 f"with adir={params['adir']!r}")
        super().__init__(params["axes"], *params.get("args", ()), **params["kwargs"])
        if "d_intervalx" in params:
            self.set_data_interval(*params["d_intervalx"])
        if "v_intervalx" in params:
            self.set_view_interval(*params["v_intervalx"])
        if params["rotate_label"] is not None:
            self.set_rotate_label(params["rotate_label"])

    __init__.__signature__ = inspect.signature(_new_init)

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

    #: The vertices of the six planes of the axes' box, in the order of
    #: `.Axes3D._transformed_cube`'s corners: the yz, xz and xy planes.
    _PLANES = (
        (0, 3, 7, 4), (1, 2, 6, 5),  # yz planes
        (0, 1, 5, 4), (3, 2, 6, 7),  # xz planes
        (0, 1, 2, 3), (4, 5, 6, 7),  # xy planes
    )

    def _get_coord_info(self):
        """The minima and maxima of the scaled limits, the box projected and
        which of each pair of parallel planes is higher up (matplotlib
        3.11.2's, under the Matplotlib License): what programs read of the
        box the axis is drawn on (qutip's histograms)."""
        xmin, xmax, ymin, ymax, zmin, zmax = self.axes._get_scaled_limits()
        mins = np.array([min(xmin, xmax), min(ymin, ymax), min(zmin, zmax)])
        maxs = np.array([max(xmin, xmax), max(ymin, ymax), max(zmin, zmax)])
        bounds = (*self.axes.get_xbound(),
                  *self.axes.get_ybound(),
                  *self.axes.get_zbound())
        bounds_proj = self.axes._transformed_cube(bounds)
        means_z0 = np.zeros(3)
        means_z1 = np.zeros(3)
        for i in range(3):
            means_z0[i] = np.mean(bounds_proj[self._PLANES[2 * i], 2])
            means_z1[i] = np.mean(bounds_proj[self._PLANES[2 * i + 1], 2])
        highs = means_z0 < means_z1
        # Special handling for edge-on views
        equals = np.abs(means_z0 - means_z1) <= np.finfo(float).eps
        if np.sum(equals) == 2:
            vertical = np.where(~equals)[0][0]
            if vertical == 2:  # looking at XY plane
                highs = np.array([True, True, highs[2]])
            elif vertical == 1:  # looking at XZ plane
                highs = np.array([True, highs[1], False])
            elif vertical == 0:  # looking at YZ plane
                highs = np.array([highs[0], False, False])
        return mins, maxs, bounds_proj, highs

    def _calc_centers_deltas(self, maxs, mins):
        """The centres of the box and the offsets of its decorations
        (matplotlib 3.11.2's, under the Matplotlib License)."""
        centers = 0.5 * (maxs + mins)
        scale = 0.08
        deltas = (maxs - mins) * scale
        return centers, deltas

    #: The methods matplotlib's draw asks for the box the axis is drawn on,
    #: which a program may replace on the axis (qutip's ``_get_coord_info``
    #: shrinks the box of its histograms).
    _BOX_METHODS = ("_get_coord_info", "_calc_centers_deltas")

    def _follow_given_box(self):
        """Hand pltrs (Rust) the box the program's methods give the axis
        at every draw and before the layouts measuring it
        (`_push_given_box`), from now on."""
        figure = self.axes.get_figure(root=True)
        if figure is not None:
            # Imported where used: the import of pyplot leaves them out.
            from pltrs import _computed_transforms
            from pltrs.axes import _program_locators
            _computed_transforms.follow(figure).add(self)
            _program_locators.follow_box(self, figure)

    def _push_given_box(self, draws=1, measures=0):
        """Hand pltrs (Rust) the box the axis' ``_get_coord_info`` and
        ``_calc_centers_deltas`` give, asking them as often as *draws*
        draws and *measures* tight boxes of matplotlib's do; none once the
        program's methods are gone."""
        if not any(name in self.__dict__ for name in self._BOX_METHODS):
            _mplot3d.set_axis_box(*self._native_args(), None)
            return
        # A draw of the axis asks the box for its pane, ticks and line (and
        # its grid, when drawn), and the centres for its ticks and line; a
        # tight box asks each once for its ticks and once for the Axes'.
        axes = self.axes
        if not (axes.get_visible() and axes._axis3don):
            # Neither drawn nor measured.
            return
        if not self.get_visible():
            # Drawn (asking the box), not measured.
            measures = 0
        if not draws + measures:
            return
        if axes.M is None:
            # As matplotlib's tight box of the axis before the first draw.
            axes.M = axes.get_proj()
            axes.invM = np.linalg.inv(axes.M)
        grid = bool(_mplot3d.get_grid(axes._native))
        for _ in range(draws * (3 + grid) + 2 * measures):
            mins, maxs, bounds_proj, highs = self._get_coord_info()
        for _ in range(2 * (draws + measures)):
            centers, deltas = self._calc_centers_deltas(maxs, mins)

        def floats(values):
            return [float(value) for value in values]

        _mplot3d.set_axis_box(*self._native_args(), (
            floats(mins), floats(maxs), [floats(corner[:3]) for corner in bounds_proj],
            [bool(high) for high in highs], floats(centers), floats(deltas)))

    #: matplotlib's fixed entries of ``_axinfo``, by axis name (``juggled``
    #: as matplotlib makes it; pltrs (Rust) keeps the axis' own).
    _AXINFO = {
        'x': {'i': 0, 'tickdir': 1, 'juggled': (1, 0, 2)},
        'y': {'i': 1, 'tickdir': 0, 'juggled': (0, 1, 2)},
        'z': {'i': 2, 'tickdir': 0, 'juggled': (0, 2, 1)},
    }

    @property
    def _axinfo(self):
        """
        matplotlib's dict of how the axis is drawn, made when first read
        with the values matplotlib's axis is made with. The entries its
        draws read every time (the ``grid``'s colour, width and style, the
        ``tick``'s widths and inward and outward factors, the ``label``'s
        alignment and rotation mode) reach pltrs (Rust) before every draw
        of the figure from then on; ``juggled`` is read from and written
        to pltrs (Rust), which keeps it for the axis (`_AxisInfo`);
        ``axisline`` and ``color``, which matplotlib reads when it makes
        the axis, and ``i`` and ``tickdir`` are kept as they are.
        """
        if "_axinfo_made" not in self.__dict__:
            self._axinfo = _matplotlib_axinfo(self.axis_name)
        return self.__dict__["_axinfo_made"]

    @_axinfo.setter
    def _axinfo(self, info):
        if "_axinfo_made" not in self.__dict__:
            figure = self.axes.get_figure(root=True)
            if figure is not None:
                from pltrs.axes import _program_locators
                _program_locators.follow_axinfo(self, figure)
        made = _AxisInfo(self, {key: value for key, value in info.items()
                                if key != "juggled"})
        if "juggled" in info:
            made["juggled"] = info["juggled"]
        self.__dict__["_axinfo_made"] = made

    def _push_axinfo(self):
        """Hand pltrs (Rust) what the program left in ``_axinfo``, as
        matplotlib's draw reads it."""
        info = self._axinfo
        grid, tick, label = info['grid'], info['tick'], info['label']
        _mplot3d.set_axis_info(
            *self._native_args(),
            (mcolors.to_rgba(grid['color']), float(grid['linewidth']), grid['linestyle']),
            (float(tick['linewidth'][True]), float(tick['linewidth'][False]),
             float(tick['inward_factor']), float(tick['outward_factor'])),
            (label['ha'], label['va'], label['rotation_mode']))

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
            # Coloured as the pane is drawn so far: as `set_pane_color`
            # gave it (the axis' rcParam, which matplotlib's axis is made
            # with, for none), typed as `to_rgba` types it, while the axis
            # still has it; restyled by the program from then on.
            made_with = mcolors.to_rgba(mpl.rcParams[f"axes3d.{self.axis_name}axis.panecolor"])
            color = _values.given(self, "pane_color",
                                  _mplot3d.get_pane_color(*self._native_args()),
                                  default=made_with)
            pane = mpatches.Polygon([[0, 0], [0, 1]], closed=False)
            pane.set_transform(self.axes.transData)
            pane.set_figure(self.axes.get_figure(root=True))
            pane.set_edgecolor(color)
            pane.set_facecolor(color)
            pane.set_alpha(color[-1])
            pane.__class__ = _Pane
            pane._axis = self
            self._pane = pane
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
        # docstring inherited; matplotlib's measures the tick labels with
        # the renderer given, else with the one the first of them was last
        # given: by a draw of the axes, or by a measure (one that failed on
        # it included).
        draws = _artist.draw_count(self.axes._native)
        if renderer is None:
            given, at = self.__dict__.get("_labels_renderer", (None, None))
            if at == draws:
                renderer = given
        elif self.get_visible():
            self._labels_renderer = (renderer, draws)
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

    @property
    def _tick_position(self):
        # matplotlib's field behind the ticks position.
        return self.get_ticks_position()

    @_tick_position.setter
    def _tick_position(self, position):
        self.set_ticks_position(position)

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
        pane = self.__dict__.get("_pane")
        if pane is not None:
            # matplotlib's sets the colours of its polygon.
            pane.set_edgecolor(color)
            pane.set_facecolor(color)
            pane.set_alpha(color[-1])


class XAxis(Axis):
    axis_name = "x"


class YAxis(Axis):
    axis_name = "y"


class ZAxis(Axis):
    axis_name = "z"


# What matplotlib's 3-D `Axis` inherits, which pltrs' class defines to
# reach the 3-D axis pltrs (Rust) draws.
cbook._inherited_methods(Axis, artist.Artist, ["set_agg_filter", "set_zorder"])
cbook._inherited_methods(Axis, maxis.XAxis, ["set_data_interval", "set_view_interval"])

# What matplotlib's module also has from its imports, which programs
# import from it.
import pltrs as mpl  # noqa: E402, F401
from dataclasses import dataclass  # noqa: E402, F401
from pltrs import text as mtext, transforms as mtransforms  # noqa: E402, F401
from pltrs.mpl_toolkits.mplot3d import proj3d  # noqa: E402, F401
