"""
`Axes3D`, the 3-D axes (matplotlib's ``mpl_toolkits.mplot3d.axes3d``):
pltrs (Rust)'s 3-D axes, with matplotlib's signatures.

An `Axes3D` is an `~.axes.Axes` whose 2-D side is its projection plane:
the 2-D methods it inherits (titles, 2-D texts, legends, the position)
act on the plane, whose data coordinates are the projected coordinates,
and the 3-D methods here on the 3-D axes (``_native.mplot3d``).

`_Quaternion` is matplotlib's (from matplotlib 3.11.2, under the
Matplotlib License, kept as it is).
"""

import textwrap
import warnings

import numpy as np

from pltrs import _api, _handles, _values, cbook, rcParams
from pltrs import collections as mcoll, colors as mcolors, lines as mlines
from pltrs._native import artist as _artist, axes as _axes, mplot3d as _mplot3d
from pltrs.axes import Axes
from pltrs.axes._ticks import _axis_method_wrapper
from pltrs._artist_kwargs import keep_program_picker
from pltrs._preprocess import _preprocess_data
from pltrs.axes._plotting._helpers import wrap
from pltrs.axes._plotting.lines import _plot_datasets
from pltrs.axes._plotting.scatter import _mapping
from pltrs.markers import MarkerStyle
from pltrs.transforms import Bbox

from . import art3d, axis3d
from ._charts import ChartsMixin
from ._contours import ContoursMixin
from ._limit_boxes import LimitBoxesMixin
from ._voxels import VoxelsMixin


def _check_margin(m):
    """matplotlib's check of a margin."""
    if m <= -0.5:
        raise ValueError("margin must be greater than -0.5")
    return float(m)


def _floats(*arrays):
    """The arrays broadcast together and flattened to floats, masked values
    as NaN."""
    arrays = np.broadcast_arrays(*[np.ma.filled(np.ma.asarray(a, dtype=float),
                                                np.nan) for a in arrays])
    return tuple(np.ascontiguousarray(a, dtype=float).ravel() for a in arrays)


def _shading(shade, lightsource):
    """The native shading: whether to shade (None: the method's default)
    and the light of a `~matplotlib.colors.LightSource` (None: mplot3d's)."""
    return (None if shade is None else bool(shade),
            None if lightsource is None
            else (float(lightsource.azdeg), float(lightsource.altdeg)))


def _surface_mapping(cmap, norm, vmin, vmax):
    """The colour mapping of a surface, ``(colorizer, native)`` as
    `._mapping` gives it, or ``(None, None)`` for a surface coloured
    without one (no *cmap* nor *norm*)."""
    if cmap is None and norm is None:
        return None, None
    return _mapping(cmap, norm, vmin, vmax)


def _sampling(rstride, cstride, rcount, ccount):
    """The native sampling of a surface or wireframe (strides or counts,
    which matplotlib refuses together)."""
    has_stride = rstride is not None or cstride is not None
    has_count = rcount is not None or ccount is not None
    if has_stride and has_count:
        raise ValueError("Cannot specify both stride and count arguments")
    if has_stride:
        return ("stride", 10 if rstride is None else int(rstride),
                10 if cstride is None else int(cstride))
    if has_count:
        return ("count", 50 if rcount is None else int(rcount),
                50 if ccount is None else int(ccount))
    return None


def _face_colors(fcolors, shape):
    """
    The colours *fcolors* of the faces of a surface over a grid of *shape*
    as an RGBA array of that shape: a face takes the colour at its first
    vertex, as matplotlib's ``plot_surface`` indexes them, so one colour
    per face (a row and a column fewer than vertices) is enough.
    """
    rows, cols = np.shape(fcolors)[:2]
    rgba = mcolors.to_rgba_array(
        np.reshape(fcolors, (-1,) + np.shape(fcolors)[2:])).reshape(rows, cols, 4)
    if rows < shape[0] - 1 or cols < shape[1] - 1:
        raise ValueError(f"facecolors of shape {np.shape(fcolors)} do not "
                         f"cover the faces of a surface of shape {shape}")
    padded = np.zeros(shape + (4,))
    rows, cols = min(rows, shape[0]), min(cols, shape[1])
    padded[:rows, :cols] = rgba[:rows, :cols]
    return padded

#: The names of the view angles, in the order of a view.
_VIEW_ANGLES = ("elev", "azim", "roll")


class Axes3D(ChartsMixin, ContoursMixin, VoxelsMixin, LimitBoxesMixin, Axes):
    """
    3D Axes object.

    .. note::

        As a user, you do not instantiate Axes directly, but use Axes creation
        methods instead; e.g. from `.pyplot` or `.Figure`:
        `~.pyplot.subplots`, `~.pyplot.subplot_mosaic` or `.Figure.add_axes`.
    """
    name = '3d'

    _axis_names = ("x", "y", "z")

    def __init__(self, fig, rect=None, *args, elev=30, azim=-60, roll=0,
                 shareview=None, sharez=None, proj_type='persp',
                 focal_length=None, box_aspect=None, computed_zorder=True,
                 **kwargs):
        """
        Build a 3D Axes in a figure, outside it until ``fig.add_axes(ax)``
        adds it, as an `~.axes.Axes` made by its constructor is.

        Parameters
        ----------
        fig : Figure
            The parent figure.
        rect : tuple (left, bottom, width, height), default: None.
            The ``(left, bottom, width, height)`` Axes position, or the
            ``(nrows, ncols, index)`` of a subplot (a `.SubplotSpec`, or
            numbers with *args*).
        elev, azim, roll : float, default: 30, -60, 0
            The initial view angles, in degrees.
        shareview, sharez : Axes3D, optional
            Other Axes to share the view angles or the z limits with.
        proj_type : {'persp', 'ortho'}
            The projection type, default 'persp'.
        focal_length : float, default: None
            For a projection type of 'persp', the focal length of the
            virtual camera.
        box_aspect : 3-tuple of floats, default: None
            Changes the physical dimensions of the Axes3D.
        computed_zorder : bool, default: True
            If True, the draw order is computed based on the average
            position of the `.Artist`\\s along the view direction.
        **kwargs
            Other optional keyword arguments are Axes properties
            (*sharex* and *sharey* included).
        """
        if rect is None:
            rect = [0.0, 0.0, 1.0, 1.0]
        options, kwargs = self._native_options({
            **kwargs, 'elev': elev, 'azim': azim, 'roll': roll,
            'shareview': shareview, 'sharez': sharez, 'proj_type': proj_type,
            'focal_length': focal_length, 'box_aspect': box_aspect,
            'computed_zorder': computed_zorder})
        args = (rect, *args)
        if len(args) == 1 and (isinstance(rect, Bbox) or np.iterable(rect)):
            bounds = (rect.bounds if isinstance(rect, Bbox)
                      else Bbox.from_bounds(*rect).bounds)
            native = _mplot3d.removed_axes(fig._native, tuple(map(float, bounds)),
                                           options)
        else:
            from pltrs.gridspec import SubplotSpec
            spec = SubplotSpec._from_subplot_args(fig, args)
            native = _mplot3d.removed_subplot(fig._native, spec._native_spec(),
                                              options)
        _handles.adopt(self, native, fig.figure)
        self._init_3d_state(fig.figure)
        self._internal_update(kwargs)

    @classmethod
    def _native_options(cls, kwargs):
        """
        The native options of a new 3-D axes from matplotlib's `Axes3D`
        keyword arguments in *kwargs*, and the other keyword arguments.
        """
        kwargs = dict(kwargs)
        initial = (float(kwargs.pop('elev', 30)), float(kwargs.pop('azim', -60)),
                   float(kwargs.pop('roll', 0)))
        proj_type = kwargs.pop('proj_type', 'persp')
        focal_length = cls._checked_focal_length(
            proj_type, kwargs.pop('focal_length', None))
        box_aspect = kwargs.pop('box_aspect', None)
        computed_zorder = bool(kwargs.pop('computed_zorder', True))
        shares = tuple(
            None if other is None else other._native
            for other in (kwargs.pop('sharex', None), kwargs.pop('sharey', None),
                          kwargs.pop('sharez', None), kwargs.pop('shareview', None)))
        options = ((initial, 'z', proj_type, focal_length),
                   None if box_aspect is None
                   else tuple(map(float, box_aspect)),
                   computed_zorder, shares)
        return options, kwargs

    @classmethod
    def _make_native(cls, figure, kwargs, *, spec=None, rect=None):
        """
        The native 3-D axes ``fig.add_subplot`` (on the grid block *spec*)
        or ``fig.add_axes`` (at *rect*) makes, with matplotlib's `Axes3D`
        keyword arguments taken from *kwargs*; return it and the other
        keyword arguments.
        """
        options, kwargs = cls._native_options(kwargs)
        if spec is not None:
            native = _mplot3d.add_subplot(figure._native, spec, options)
        else:
            native = _mplot3d.add_axes(figure._native, rect, options)
        return native, kwargs

    @classmethod
    def _from_native(cls, native, figure):
        ax = super()._from_native(native, figure)
        ax._init_3d_state(figure)
        return ax

    def _init_3d_state(self, figure):
        """Set up the Python state of the axes around its native handle in
        the root *figure*."""
        self.xaxis = axis3d.XAxis(self)
        self.yaxis = axis3d.YAxis(self)
        self.zaxis = axis3d.ZAxis(self)
        # A new axes looks from its initial view; matplotlib's default
        # angles are int literals (``elev=30``), which it keeps as given.
        self.initial_elev, self.initial_azim, self.initial_roll = (
            default if angle == default else angle
            for angle, default in zip(_mplot3d.get_view(self._native), (30, -60, 0)))
        _values.remember_all(self, dict(zip(_VIEW_ANGLES, (
            self.initial_elev, self.initial_azim, self.initial_roll))), _VIEW_ANGLES)
        self.button_pressed = None
        self.mouse_init()
        canvas = figure.canvas
        canvas.mpl_connect('motion_notify_event', self._on_move)
        canvas.mpl_connect('button_press_event', self._button_press)
        canvas.mpl_connect('button_release_event', self._button_release)
        # The projection plane stands for the axes where an artist's parent
        # or the current axes is asked for.
        figure._artists[_artist.id(_mplot3d.plane(self._native))] = self

    @staticmethod
    def _checked_focal_length(proj_type, focal_length):
        """matplotlib's checks of the focal length of *proj_type*."""
        _api.check_in_list(['persp', 'ortho'], proj_type=proj_type)
        if proj_type == 'persp':
            if focal_length is not None and focal_length <= 0:
                raise ValueError(f"focal_length = {focal_length} must be "
                                 "greater than 0")
            return None if focal_length is None else float(focal_length)
        if focal_length not in (None, np.inf):
            raise ValueError(f"focal_length = {focal_length} must be "
                             f"None for proj_type = {proj_type}")
        return None

    def __repr__(self):
        fields = []
        if self.get_label():
            fields += [f"label={self.get_label()!r}"]
        titles = {}
        for loc in ["left", "center", "right"]:
            title = self.get_title(loc=loc)
            if title:
                titles[loc] = title
        if titles:
            fields += [f"title={titles}"]
        for name in self._axis_names:
            label = _mplot3d.get_label(self._native, name)
            if label:
                fields += [f"{name}label={label!r}"]
        return f"<{self.__class__.__name__}: " + ", ".join(fields) + ">"

    # -- axes --------------------------------------------------------------

    def get_zaxis(self):
        """Return the ``ZAxis`` (`~.axis3d.Axis`) instance."""
        return self.zaxis

    def get_subplotspec(self):
        """Return the `.SubplotSpec` associated with the subplot, or None."""
        from pltrs.gridspec import SubplotSpec
        native = _mplot3d.subplot_spec(self._native)
        return None if native is None else SubplotSpec._from_native(
            native, self.get_figure(root=False))

    def clear(self):
        # docstring inherited.
        _mplot3d.clear(self._native)
        for axis in (self.xaxis, self.yaxis, self.zaxis):
            axis._given_scale = None

    get_zgridlines = _axis_method_wrapper("zaxis", "get_gridlines")
    get_zticklines = _axis_method_wrapper("zaxis", "get_ticklines")
    get_zmajorticklabels = _axis_method_wrapper("zaxis", "get_majorticklabels")
    get_zminorticklabels = _axis_method_wrapper("zaxis", "get_minorticklabels")
    get_zticklabels = _axis_method_wrapper("zaxis", "get_ticklabels")
    set_zinverted = _axis_method_wrapper("zaxis", "set_inverted")
    get_zinverted = _axis_method_wrapper("zaxis", "get_inverted")

    zaxis_date = _axis_method_wrapper("zaxis", "axis_date")
    if zaxis_date.__doc__:
        zaxis_date.__doc__ += textwrap.dedent("""

        Notes
        -----
        This function is merely provided for completeness, but 3D Axes do not
        support dates for ticks, and so this may not work as expected.
        """)

    _msg = "Use ax.set_xscale/set_yscale/set_zscale and ax.plot(...) instead."
    semilogz = _api.unsupported_method(append_message=_msg)

    def convert_zunits(self, z):
        """
        For artists in an Axes, if the zaxis has units support,
        convert *z* using zaxis unit type
        """
        return self.zaxis.convert_units(z)

    def set_top_view(self):
        """Show the window of projected coordinates the axes draws in, as
        a new 3-D axes does."""
        _mplot3d.set_top_view(self._native)
        self.stale = True

    def update_datalim(self, xys, **kwargs):
        """
        Not implemented in `~mpl_toolkits.mplot3d.axes3d.Axes3D`.
        """
        pass

    def get_zticks(self, *, minor=False):
        """Return the zaxis' tick locations in data coordinates."""
        return self.zaxis.get_ticklocs(minor=minor)

    def set_zticks(self, ticks, labels=None, *, minor=False, **kwargs):
        """Set the zaxis' tick locations (`.Axis.set_ticks`)."""
        return self.zaxis.set_ticks(ticks, labels, minor=minor, **kwargs)

    def set_zticklabels(self, labels, *, minor=False, fontdict=None, **kwargs):
        """Set the zaxis' tick labels (`.Axis.set_ticklabels`)."""
        return self.zaxis.set_ticklabels(labels, minor=minor,
                                         fontdict=fontdict, **kwargs)

    def autoscale_view(self, tight=None,
                       scalex=True, scaley=True, scalez=True):
        """
        Autoscale the view limits using the data limits.

        See `.Axes.autoscale_view` for full documentation.  Because this
        function applies to 3D Axes, it also takes a *scalez* argument.
        """
        _mplot3d.autoscale_view(
            self._native, None if tight is None else bool(tight),
            (bool(scalex), bool(scaley), bool(scalez)))

    def has_data(self):
        """Return whether any artists have been added to the Axes."""
        return _mplot3d.has_data(self._native)

    def auto_scale_xyz(self, X, Y, Z=None, had_data=None):
        """
        Include the data *X*, *Y* (and *Z*) in the data limits, replacing
        them unless *had_data*, and autoscale the view.
        """
        def values(data):
            return np.ma.filled(np.ma.asarray(data, dtype=float), np.nan).ravel()
        _mplot3d.auto_scale_xyz(
            self._native,
            (values(X), values(Y), None if Z is None else values(Z)),
            bool(had_data))

    # -- limits ------------------------------------------------------------

    def _set_lim3d(self, name, lower=None, upper=None, *, emit=True,
                   auto=False, view_margin=None, axmin=None, axmax=None):
        """Set the view limits of axis *name* (matplotlib's ``_set_lim3d``)."""
        current = _mplot3d.get_lim(self._native, name)
        if upper is None:
            if np.iterable(lower):
                lower, upper = lower
            elif axmax is None:
                upper = current[1]
        if lower is None and axmin is None:
            lower = current[0]
        if axmin is not None:
            if lower is not None:
                raise TypeError("Cannot pass both 'lower' and 'min'")
            lower = axmin
        if axmax is not None:
            if upper is not None:
                raise TypeError("Cannot pass both 'upper' and 'max'")
            upper = axmax
        lower, upper = float(lower), float(upper)
        if np.isinf(lower) or np.isinf(upper):
            raise ValueError(f"Axis limits {lower}, {upper} cannot be infinite")
        if np.isnan(lower) or np.isnan(upper):
            raise ValueError("Axis limits cannot be NaN or Inf")
        if _mplot3d.get_scale(self._native, name) == 'log':
            if lower <= 0:
                _api.warn_external(
                    f"Attempt to set non-positive {name}lim on a log-scaled "
                    "axis will be ignored.")
                lower = current[0]
            if upper <= 0:
                _api.warn_external(
                    f"Attempt to set non-positive {name}lim on a log-scaled "
                    "axis will be ignored.")
                upper = current[1]
        _mplot3d.set_lim(self._native, name, (lower, upper),
                         None if view_margin is None else float(view_margin),
                         None if auto is None else bool(auto))
        return _mplot3d.get_lim(self._native, name)

    def set_xlim(self, left=None, right=None, *, emit=True, auto=False,
                 view_margin=None, xmin=None, xmax=None):
        """Set the 3D x-axis view limits."""
        return self._set_lim3d('x', left, right, emit=emit, auto=auto,
                               view_margin=view_margin, axmin=xmin, axmax=xmax)

    def set_ylim(self, bottom=None, top=None, *, emit=True, auto=False,
                 view_margin=None, ymin=None, ymax=None):
        """Set the 3D y-axis view limits."""
        return self._set_lim3d('y', bottom, top, emit=emit, auto=auto,
                               view_margin=view_margin, axmin=ymin, axmax=ymax)

    def set_zlim(self, bottom=None, top=None, *, emit=True, auto=False,
                 view_margin=None, zmin=None, zmax=None):
        """Set the 3D z-axis view limits."""
        return self._set_lim3d('z', bottom, top, emit=emit, auto=auto,
                               view_margin=view_margin, axmin=zmin, axmax=zmax)

    set_xlim3d = set_xlim
    set_ylim3d = set_ylim
    set_zlim3d = set_zlim

    def get_xlim(self):
        # docstring inherited
        return _values.float64s(_mplot3d.get_lim(self._native, 'x'))

    def get_ylim(self):
        # docstring inherited
        return _values.float64s(_mplot3d.get_lim(self._native, 'y'))

    def get_zlim(self):
        """Return the 3D z-axis view limits."""
        return _values.float64s(_mplot3d.get_lim(self._native, 'z'))

    get_xlim3d = get_xlim
    get_ylim3d = get_ylim
    get_zlim3d = get_zlim

    def get_w_lims(self):
        """Get 3D world limits."""
        minx, maxx = self.get_xlim3d()
        miny, maxy = self.get_ylim3d()
        minz, maxz = self.get_zlim3d()
        return minx, maxx, miny, maxy, minz, maxz

    def _set_bound3d(self, name, lower, upper, view_margin):
        if upper is None and np.iterable(lower):
            lower, upper = lower
        _mplot3d.set_bound(self._native, name,
                           (None if lower is None else float(lower),
                            None if upper is None else float(upper)),
                           None if view_margin is None else float(view_margin))

    def set_xbound(self, lower=None, upper=None, view_margin=None):
        """Set the lower and upper numerical bounds of the x-axis."""
        self._set_bound3d('x', lower, upper, view_margin)

    def set_ybound(self, lower=None, upper=None, view_margin=None):
        """Set the lower and upper numerical bounds of the y-axis."""
        self._set_bound3d('y', lower, upper, view_margin)

    def set_zbound(self, lower=None, upper=None, view_margin=None):
        """Set the lower and upper numerical bounds of the z-axis."""
        self._set_bound3d('z', lower, upper, view_margin)

    def get_xbound(self):
        """Return the lower and upper x-axis bounds, in increasing order."""
        return _values.float64s(_mplot3d.get_bound(self._native, 'x'))

    def get_ybound(self):
        """Return the lower and upper y-axis bounds, in increasing order."""
        return _values.float64s(_mplot3d.get_bound(self._native, 'y'))

    def get_zbound(self):
        """Return the lower and upper z-axis bounds, in increasing order."""
        return _values.float64s(_mplot3d.get_bound(self._native, 'z'))

    def invert_xaxis(self):
        """[*Discouraged*] Invert the x-axis."""
        _mplot3d.set_inverted(self._native, 'x', not self.xaxis_inverted())

    def invert_yaxis(self):
        """[*Discouraged*] Invert the y-axis."""
        _mplot3d.set_inverted(self._native, 'y', not self.yaxis_inverted())

    def invert_zaxis(self):
        """[*Discouraged*] Invert the z-axis."""
        _mplot3d.set_inverted(self._native, 'z', not self.zaxis_inverted())

    def xaxis_inverted(self):
        """Return whether the xaxis is oriented in the "inverse" direction."""
        return _mplot3d.get_inverted(self._native, 'x')

    def yaxis_inverted(self):
        """Return whether the yaxis is oriented in the "inverse" direction."""
        return _mplot3d.get_inverted(self._native, 'y')

    def zaxis_inverted(self):
        """Return whether the zaxis is oriented in the "inverse" direction."""
        return _mplot3d.get_inverted(self._native, 'z')

    # -- autoscaling -------------------------------------------------------

    @property
    def _view_margin(self):
        """The padding of autoscaled limits, and of limits set under
        :rc:`axes3d.automargin`, as a fraction of their span."""
        return _mplot3d.get_view_margin(self._native)

    @_view_margin.setter
    def _view_margin(self, margin):
        _mplot3d.set_view_margin(self._native, float(margin))

    def get_autoscale_on(self):
        """Return True if each axis is autoscaled, False otherwise."""
        return all(_mplot3d.get_autoscale_on(self._native, name)
                   for name in self._axis_names)

    def set_autoscale_on(self, b):
        """Set whether autoscaling is applied to each axis on the next draw."""
        for name in self._axis_names:
            _mplot3d.set_autoscale_on(self._native, name, bool(b))

    def get_autoscalex_on(self):
        """Return whether the xaxis is autoscaled."""
        return _mplot3d.get_autoscale_on(self._native, 'x')

    def get_autoscaley_on(self):
        """Return whether the yaxis is autoscaled."""
        return _mplot3d.get_autoscale_on(self._native, 'y')

    def get_autoscalez_on(self):
        """Return whether the zaxis is autoscaled."""
        return _mplot3d.get_autoscale_on(self._native, 'z')

    def set_autoscalex_on(self, b):
        """Set whether the xaxis is autoscaled."""
        _mplot3d.set_autoscale_on(self._native, 'x', bool(b))

    def set_autoscaley_on(self, b):
        """Set whether the yaxis is autoscaled."""
        _mplot3d.set_autoscale_on(self._native, 'y', bool(b))

    def set_autoscalez_on(self, b):
        """Set whether the zaxis is autoscaled."""
        _mplot3d.set_autoscale_on(self._native, 'z', bool(b))

    def autoscale(self, enable=True, axis='both', tight=None):
        """
        Convenience method for simple axis view autoscaling.

        See `.Axes.autoscale` for full documentation.  Because this function
        applies to 3D Axes, *axis* can also be set to 'z', and setting *axis*
        to 'both' autoscales all three axes.
        """
        _api.check_in_list(['both', 'x', 'y', 'z'], axis=axis)
        _mplot3d.autoscale(self._native,
                           None if enable is None else bool(enable),
                           None if axis == 'both' else axis,
                           None if tight is None else bool(tight))

    def _request_autoscale_view(self, axis="all", tight=None):
        """
        Autoscale a single axis, or all of them (`.Axes._request_autoscale_view`):
        pltrs (Rust) autoscales 3-D axes at once, where matplotlib marks
        them stale until the limits are next read.
        """
        axis_names = _api.getitem_checked(
            {**{k: [k] for k in self._axis_names}, "all": self._axis_names},
            axis=axis)
        self.autoscale_view(tight=tight, scalex="x" in axis_names,
                            scaley="y" in axis_names, scalez="z" in axis_names)

    def _get_margin(self, name):
        """The margin of axis *name*, as given (matplotlib keeps it)."""
        return _values.given(self, f"{name}margin", _mplot3d.get_margin(self._native, name))

    def _set_margins(self, margins, tight):
        """Set the margins (x, y, z) given, None keeping one."""
        _mplot3d.set_margins(
            self._native,
            tuple(None if m is None else _check_margin(m) for m in margins),
            None if tight is None else bool(tight))
        for name, m in zip(self._axis_names, margins):
            if m is not None:
                _values.remember(self, f"{name}margin", m)

    def get_xmargin(self):
        """Retrieve autoscaling margin of the x-axis."""
        return self._get_margin('x')

    def get_ymargin(self):
        """Retrieve autoscaling margin of the y-axis."""
        return self._get_margin('y')

    def get_zmargin(self):
        """Retrieve autoscaling margin of the z-axis."""
        return self._get_margin('z')

    def set_xmargin(self, m):
        """Set padding of X data limits prior to autoscaling."""
        self._set_margins((m, None, None), None)

    def set_ymargin(self, m):
        """Set padding of Y data limits prior to autoscaling."""
        self._set_margins((None, m, None), None)

    def set_zmargin(self, m):
        """Set padding of Z data limits prior to autoscaling."""
        self._set_margins((None, None, m), None)

    def margins(self, *margins, x=None, y=None, z=None, tight=True):
        """
        Set or retrieve autoscaling margins.

        See `.Axes.margins` for full documentation.  Because this function
        applies to 3D Axes, it also takes a *z* argument, and returns
        ``(xmargin, ymargin, zmargin)``.
        """
        if margins and (x is not None or y is not None or z is not None):
            raise TypeError('Cannot pass both positional and keyword '
                            'arguments for x, y, and/or z.')
        elif len(margins) == 1:
            x = y = z = margins[0]
        elif len(margins) == 3:
            x, y, z = margins
        elif margins:
            raise TypeError('Must pass a single positional argument for all '
                            'margins, or one for each margin (x, y, z).')

        if x is None and y is None and z is None:
            if tight is not True:
                _api.warn_external(f'ignoring tight={tight!r} in get mode')
            return self.get_xmargin(), self.get_ymargin(), self.get_zmargin()

        self._set_margins((x, y, z), tight)

    # -- scales ------------------------------------------------------------

    def _set_axis_scale(self, name, value, **kwargs):
        """
        Set the scale of axis *name*: a scale object, or the name of a
        scale class with its keyword arguments, checked by the class as
        matplotlib's ``scale_factory`` does. pltrs (Rust) maps through a
        scale of the program's, calling its transform and range limits.
        """
        from pltrs import scale as mscale
        from pltrs._native import axis as _axis
        from pltrs.axis import _native_scale_of
        axis = getattr(self, f'{name}axis')
        if isinstance(value, mscale.ScaleBase):
            scale = value
            native = _native_scale_of(scale)
        else:
            scale_cls = _api.getitem_checked(mscale._scale_mapping, scale=value)
            scale = scale_cls(axis, **kwargs)
            native = (None if _native_scale_of(scale) is None
                      else (value, kwargs))
        if native is None:
            transform = scale.get_transform()
            inverse = transform.inverted()
            _mplot3d.set_python_scale(
                self._native, name, scale.name,
                (lambda x: float(transform.transform(x)),
                 lambda x: float(inverse.transform(x))),
                lambda vmin, vmax, minpos: tuple(
                    map(float, scale.limit_range_for_scale(vmin, vmax, minpos))))
            scale.set_default_locators_and_formatters(axis)
            _axis.mark_default_helpers(self._native, name)
        else:
            _mplot3d.set_scale(self._native, name, *native)
        axis._given_scale = scale

    def set_xscale(self, value, **kwargs):
        """Set the x-axis scale."""
        self._set_axis_scale('x', value, **kwargs)

    def set_yscale(self, value, **kwargs):
        """Set the y-axis scale."""
        self._set_axis_scale('y', value, **kwargs)

    def set_zscale(self, value, **kwargs):
        """Set the z-axis scale."""
        self._set_axis_scale('z', value, **kwargs)

    def get_xscale(self):
        """Return the xaxis' scale (as a str)."""
        return _mplot3d.get_scale(self._native, 'x')

    def get_yscale(self):
        """Return the yaxis' scale (as a str)."""
        return _mplot3d.get_scale(self._native, 'y')

    def get_zscale(self):
        """Return the zaxis' scale (as a str)."""
        return _mplot3d.get_scale(self._native, 'z')

    # -- the camera and the box --------------------------------------------

    def view_init(self, elev=None, azim=None, roll=None, vertical_axis="z",
                  share=False):
        """
        Set the elevation and azimuth of the Axes in degrees (not radians).

        Parameters
        ----------
        elev, azim, roll : float, default: None
            The elevation, azimuth and roll angles in degrees; None takes the
            initial value given to the `Axes3D` constructor.
        vertical_axis : {"z", "x", "y"}, default: "z"
            The axis to align vertically. *azim* rotates about this axis.
        share : bool, default: False
            If ``True``, apply the settings to all Axes with shared views.
        """
        _api.check_in_list(self._axis_names, vertical_axis=vertical_axis)
        self.__dict__['_vertical_axis_name'] = vertical_axis
        view = (self.initial_elev if elev is None else elev,
                self.initial_azim if azim is None else azim,
                self.initial_roll if roll is None else roll)
        _mplot3d.view_init(self._native, tuple(map(float, view)), vertical_axis,
                           bool(share))
        _values.remember_all(self, dict(zip(_VIEW_ANGLES, view)), _VIEW_ANGLES)

    def _view_angle(index):
        """The property of one of the view angles (as given, as matplotlib
        keeps them, until the view turns)."""
        name = _VIEW_ANGLES[index]

        def get(self):
            return _values.given(self, name, _mplot3d.get_view(self._native)[index])

        def set(self, value):
            view = list(_mplot3d.get_view(self._native))
            view[index] = float(value)
            _mplot3d.view_init(self._native, tuple(view),
                               self._axis_names[self._vertical_axis], False)
            _values.remember(self, name, value)
        return property(get, set)

    elev = _view_angle(0)
    azim = _view_angle(1)
    roll = _view_angle(2)
    del _view_angle

    @property
    def _vertical_axis(self):
        """The index of the vertical axis in ``_axis_names``."""
        return self._axis_names.index(
            self.__dict__.get('_vertical_axis_name', 'z'))

    def set_proj_type(self, proj_type, focal_length=None):
        """
        Set the projection type.

        Parameters
        ----------
        proj_type : {'persp', 'ortho'}
            The projection type.
        focal_length : float, default: None
            For a projection type of 'persp', the focal length of the virtual
            camera. Must be > 0. If None, defaults to 1.
        """
        focal_length = self._checked_focal_length(proj_type, focal_length)
        _mplot3d.set_proj_type(self._native, proj_type, focal_length)

    def set_box_aspect(self, aspect, *, zoom=1):
        """
        Set the Axes box aspect.

        Parameters
        ----------
        aspect : 3-tuple of floats or None
            Changes the physical dimensions of the Axes3D, such that the ratio
            of the axis lengths in display units is x:y:z.
            If None, defaults to (4, 4, 3).
        zoom : float, default: 1
            Control overall size of the Axes3D in the figure. Must be > 0.
        """
        if zoom <= 0:
            raise ValueError(f'Argument zoom = {zoom} must be > 0')
        if aspect is not None:
            aspect = np.asarray(aspect, dtype=float)
            _api.check_shape((3,), aspect=aspect)
        _mplot3d.set_box_aspect(
            self._native,
            None if aspect is None else tuple(map(float, aspect)),
            float(zoom))

    def get_box_aspect(self):
        """The x:y:z ratio of the box edges."""
        return np.array(_mplot3d.get_box_aspect(self._native))

    @property
    def _box_aspect(self):
        """The x:y:z ratio of the box edges (matplotlib's attribute)."""
        return self.get_box_aspect()

    @_api.delete_parameter("3.11", "share")
    @_api.delete_parameter("3.11", "anchor")
    def set_aspect(self, aspect, adjustable=None, anchor=None, share=False):
        """
        Set the aspect ratios.

        Parameters
        ----------
        aspect : {'auto', 'equal', 'equalxy', 'equalxz', 'equalyz'}
        adjustable : {'box', 'datalim'}, default: 'box'
        anchor : None or str or 2-tuple of float, optional
            .. deprecated:: 3.11
                This parameter has no effect.
        share : bool, default: False
            .. deprecated:: 3.11
                This parameter has no effect.
        """
        if adjustable is None:
            adjustable = 'box'
        _api.check_in_list(['box', 'datalim'], adjustable=adjustable)
        _api.check_in_list(('auto', 'equal', 'equalxy', 'equalyz', 'equalxz'),
                           aspect=aspect)
        _mplot3d.set_aspect(self._native, aspect, adjustable)

    def get_aspect(self):
        """Return the aspect: 'auto', 'equal', 'equalxy', 'equalxz' or
        'equalyz'."""
        return _mplot3d.get_aspect(self._native)[0]

    def get_adjustable(self):
        """Return whether the box ('box') or the data limits ('datalim')
        keep the aspect."""
        return _mplot3d.get_aspect(self._native)[1]

    def get_proj(self):
        """Create the projection matrix from the current viewing position."""
        return _mplot3d.get_proj(self._native)

    @property
    def M(self):
        """The projection matrix of the current view."""
        return self.get_proj()

    @property
    def invM(self):
        """The inverse of the projection matrix `M`."""
        return np.linalg.inv(self.M)

    @property
    def computed_zorder(self):
        """Whether collections and patches are drawn in depth order."""
        return self.__dict__.get('_computed_zorder', True)

    @computed_zorder.setter
    def computed_zorder(self, computed):
        self._computed_zorder = bool(computed)
        _mplot3d.set_computed_zorder(self._native, bool(computed))

    def sharez(self, other):
        """
        Share the z-axis with *other*.

        This is equivalent to passing ``sharez=other`` when constructing the
        Axes, and cannot be used if the z-axis is already being shared with
        another Axes.  Note that it is not possible to unshare axes.
        """
        _api.check_isinstance(Axes3D, other=other)
        _mplot3d.share(self._native, 'z', other._native)

    def shareview(self, other):
        """
        Share the view angles with *other*.

        This is equivalent to passing ``shareview=other`` when constructing the
        Axes, and cannot be used if the view angles are already being shared
        with another Axes.  Note that it is not possible to unshare axes.
        """
        _api.check_isinstance(Axes3D, other=other)
        _mplot3d.share(self._native, 'view', other._native)
        self.__dict__['_vertical_axis_name'] = other._axis_names[
            other._vertical_axis]

    def set_anchor(self, anchor, share=False):
        # docstring inherited; the square box of the 3-D axes is anchored
        # there too.
        super().set_anchor(anchor, share)
        named = isinstance(anchor, str)
        for ax in self._aspect_group(share):
            _mplot3d.set_anchor(ax._native, anchor if named else None,
                                (0.0, 0.0) if named else tuple(map(float, anchor)))

    def get_position(self, original=False):
        """
        Return the position of the Axes within the figure as a `.Bbox`:
        its box made square in it (``apply_aspect``), or the original
        position.
        """
        return Bbox.from_extents(*_mplot3d.position(self._native, bool(original)))

    def get_tightbbox(self, renderer=None, *, call_axes_locator=True,
                      bbox_extra_artists=None, for_layout_only=False):
        # docstring inherited
        extent = _mplot3d.tightbbox(self._native, bool(for_layout_only))
        if extent is None:
            return None
        x0, y0, x1, y1 = extent
        boxes = [Bbox([[x0, y0], [x1, y1]])]
        for artist in bbox_extra_artists or ():
            bbox = artist.get_tightbbox(renderer)
            if (bbox is not None and 0 < bbox.width < np.inf
                    and 0 < bbox.height < np.inf):
                boxes.append(bbox)
        return Bbox.union(boxes)

    def get_axis_position(self):
        """Return whether the x, y and z axes are drawn on their high side."""
        return _mplot3d.get_axis_position(self._native)

    def _calc_coord(self, xv, yv, renderer=None):
        """
        Given the 2D view coordinates, find the point on the nearest axis pane
        that lies directly below those coordinates. Returns a 3D point in data
        coordinates.
        """
        found = _mplot3d.calc_coord(self._native, float(xv), float(yv))
        if found is None:
            raise np.linalg.LinAlgError("Singular matrix")
        point, pane_idx = found
        return np.array(point), pane_idx

    def disable_mouse_rotation(self):
        """Disable mouse buttons for 3D rotation, panning, and zooming."""
        _mplot3d.disable_mouse_rotation(self._native)

    def format_coord(self, xv, yv, renderer=None):
        """
        Return a string giving the current view rotation angles, or the x, y, z
        coordinates of the point on the nearest axis pane underneath the mouse
        cursor, depending on the mouse button pressed.
        """
        return _mplot3d.format_coord(self._native, float(xv), float(yv))

    def _rotation_coords(self):
        """Return the rotation angles as a string."""
        return _mplot3d.rotation_coords(self._native)

    #: A callable formatting z values for `format_zdata`, or None.
    fmt_zdata = None

    def format_zdata(self, z):
        """
        Return *z* string formatted.  This function will use the
        :attr:`!fmt_zdata` attribute if it is callable, else will fall back
        on the zaxis major formatter
        """
        try:
            return self.fmt_zdata(z)
        except (AttributeError, TypeError):
            return _mplot3d.format_data(self._native, 'z', float(z)) or ""

    # -- decorations -------------------------------------------------------

    def axis(self, arg=None, /, *, emit=True, **kwargs):
        # docstring inherited
        if isinstance(arg, (str, bool)) and str(arg).lower() in (
                'on', 'off', 'true', 'false'):
            if str(arg).lower() in ('on', 'true'):
                self.set_axis_on()
            else:
                self.set_axis_off()
            arg = None
        return super().axis(arg, emit=emit, **kwargs)

    def set_axis_off(self):
        """Hide the panes, axis lines, ticks and gridlines."""
        _mplot3d.set_axis_on(self._native, False)

    def set_axis_on(self):
        """Show the panes, axis lines, ticks and gridlines again."""
        _mplot3d.set_axis_on(self._native, True)

    @property
    def axison(self):
        """Whether the panes, axes and gridlines are drawn."""
        return _mplot3d.get_axis_on(self._native)

    def grid(self, visible=True, **kwargs):
        """
        Set / unset 3D grid.

        .. note::

            Currently, this function does not behave the same as
            `.axes.Axes.grid`, but it is intended to eventually support that
            behavior.
        """
        if len(kwargs):
            visible = True
        _mplot3d.grid(self._native, bool(visible))

    def _set_axis_label(self, name, label, fontdict, labelpad, kwargs):
        """Label the axis *name* (3-D axes place their labels themselves, so
        no *loc* applies)."""
        axis = getattr(self, f'{name}axis')
        if labelpad is not None:
            axis.labelpad = labelpad
        return axis.set_label_text(label, fontdict, **kwargs)

    def set_xlabel(self, xlabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        """Set the label for the x-axis."""
        return self._set_axis_label('x', xlabel, fontdict, labelpad, kwargs)

    def set_ylabel(self, ylabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        """Set the label for the y-axis."""
        return self._set_axis_label('y', ylabel, fontdict, labelpad, kwargs)

    def set_zlabel(self, zlabel, fontdict=None, labelpad=None, **kwargs):
        """Set zlabel.  See doc for `.set_ylabel` for description."""
        return self._set_axis_label('z', zlabel, fontdict, labelpad, kwargs)

    def tick_params(self, axis='both', **kwargs):
        """
        Convenience method for changing the appearance of ticks and
        tick labels.

        See `.Axes.tick_params` for full documentation.  Because this function
        applies to 3D Axes, *axis* can also be set to 'z', and setting *axis*
        to 'both' autoscales all three axes.

        Also, because of how Axes3D objects are drawn very differently
        from regular 2D Axes, some of these settings may have
        ambiguous meaning.  For simplicity, the 'z' axis will
        accept settings as if it was like the 'y' axis.

        .. note::
           Axes3D currently ignores some of these settings.
        """
        _api.check_in_list(['x', 'y', 'z', 'both'], axis=axis)
        if axis in ['x', 'y', 'both']:
            super().tick_params(axis, **kwargs)
        if axis in ['z', 'both']:
            zkw = dict(kwargs)
            zkw.pop('top', None)
            zkw.pop('bottom', None)
            zkw.pop('labeltop', None)
            zkw.pop('labelbottom', None)
            self.zaxis.set_tick_params(**zkw)

    def get_xlabel(self):
        """Get the xlabel text string."""
        return _mplot3d.get_label(self._native, 'x')

    def get_ylabel(self):
        """Get the ylabel text string."""
        return _mplot3d.get_label(self._native, 'y')

    def get_zlabel(self):
        """Get the zlabel text string."""
        return _mplot3d.get_label(self._native, 'z')

    def set_title(self, label, fontdict=None, loc='center', **kwargs):
        # docstring inherited
        # Centred whatever axes.titlelocation says, as matplotlib's is.
        # matplotlib also lowers the title to 0.92 of its height, which
        # its first draw replaces with the automatic position; pltrs
        # (Rust) places 3-D titles automatically from the start.
        return super().set_title(label, fontdict=fontdict, loc=loc, **kwargs)

    # -- navigation --------------------------------------------------------

    @property
    def _pseudo_w(self):
        """The width of the view window in projected coordinates."""
        lo, hi = _axes.get_xlim(self._native)
        return hi - lo

    @property
    def _pseudo_h(self):
        """The height of the view window in projected coordinates."""
        lo, hi = _axes.get_ylim(self._native)
        return hi - lo

    def mouse_init(self, rotate_btn=1, pan_btn=2, zoom_btn=3):
        """
        Set the mouse buttons for 3D rotation and zooming.

        Parameters
        ----------
        rotate_btn : int or list of int, default: 1
            The mouse button or buttons to use for 3D rotation of the Axes.
        pan_btn : int or list of int, default: 2
            The mouse button or buttons to use to pan the 3D Axes.
        zoom_btn : int or list of int, default: 3
            The mouse button or buttons to use to zoom the 3D Axes.
        """
        self.button_pressed = None
        # coerce scalars into array-like, then convert into
        # a regular list to avoid comparisons against None
        # which breaks in recent versions of numpy.
        self._rotate_btn = np.atleast_1d(rotate_btn).tolist()
        self._pan_btn = np.atleast_1d(pan_btn).tolist()
        self._zoom_btn = np.atleast_1d(zoom_btn).tolist()

    def _button_press(self, event):
        if event.inaxes == self:
            self.button_pressed = event.button
            self._sx, self._sy = event.xdata, event.ydata
            toolbar = self.get_figure(root=True).canvas.toolbar
            if toolbar and toolbar._nav_stack() is None:
                toolbar.push_current()
            if toolbar:
                toolbar.set_message(toolbar._mouse_event_to_message(event))

    def _button_release(self, event):
        self.button_pressed = None
        toolbar = self.get_figure(root=True).canvas.toolbar
        # backend_bases.release_zoom and backend_bases.release_pan call
        # push_current, so check the navigation mode so we don't call it twice
        if toolbar and self.get_navigate_mode() is None:
            toolbar.push_current()
        if toolbar:
            toolbar.set_message(toolbar._mouse_event_to_message(event))

    def _on_move(self, event):
        """
        Mouse moving.

        By default, button-1 rotates, button-2 pans, and button-3 zooms;
        these buttons can be modified via `mouse_init`.
        """
        if not self.button_pressed or self.get_navigate_mode() is not None:
            return
        x, y = event.xdata, event.ydata
        # In case the mouse is out of bounds.
        if x is None or event.inaxes != self:
            return
        if self.button_pressed in self._rotate_btn:
            if x == self._sx and y == self._sy:
                return
            _mplot3d.drag_rotate(self._native, (float(self._sx), float(self._sy)),
                                 (float(x), float(y)),
                                 getattr(event, "key", None) == "control")
        elif self.button_pressed in self._pan_btn:
            px, py = self.transData.transform([self._sx, self._sy])
            self.start_pan(px, py, 2)
            self.drag_pan(2, None, event.x, event.y)
            self.end_pan()
        elif self.button_pressed in self._zoom_btn:
            _mplot3d.drag_zoom(self._native, float(self._sy), float(y))
        self._sx, self._sy = x, y
        self.get_figure(root=True).canvas.draw_idle()

    def start_pan(self, x, y, button):
        # docstring inherited
        self._pan_start = (x, y, self.transData.inverted().frozen())

    def end_pan(self):
        # docstring inherited
        del self._pan_start

    def drag_pan(self, button, key, x, y):
        # docstring inherited
        px, py, inverse = self._pan_start
        (xdata, ydata), (xdata_start, ydata_start) = inverse.transform(
            [(x, y), (px, py)])
        self._sx, self._sy = xdata, ydata
        self.start_pan(x, y, button)
        du, dv = xdata - xdata_start, ydata - ydata_start
        if key == 'x':
            dv = 0
        elif key == 'y':
            du = 0
        _mplot3d.pan_view(self._native, float(du), float(dv))

    def _set_view_from_bbox(self, bbox, direction='in',
                            mode=None, twinx=False, twiny=False):
        # docstring inherited
        _mplot3d.zoom_to_rect(self._native, tuple(map(float, bbox)),
                              direction == 'out', mode)

    def _get_view(self):
        # docstring inherited
        return {
            "xlim": self.get_xlim(), "autoscalex_on": self.get_autoscalex_on(),
            "ylim": self.get_ylim(), "autoscaley_on": self.get_autoscaley_on(),
            "zlim": self.get_zlim(), "autoscalez_on": self.get_autoscalez_on(),
        }, (self.elev, self.azim, self.roll)

    def _set_view(self, view):
        # docstring inherited
        props, (elev, azim, roll) = view
        self.set(**props)
        _mplot3d.view_init(self._native, (elev, azim, roll),
                           self._axis_names[self._vertical_axis], False)

    # -- collections -------------------------------------------------------

    def add_collection3d(self, col, zs=0, zdir='z', autolim=True, *,
                         axlim_clip=False):
        """
        Add a 3D collection object to the plot.

        2D collection types are converted to a 3D version by
        modifying the object and adding z coordinate information,
        *zs* and *zdir*.

        Supported 2D collection types are:

        - `.PolyCollection`
        - `.LineCollection`

        Parameters
        ----------
        col : `.Collection`
            A 2D collection object.
        zs : float or array-like, default: 0
            The z-positions to be used for the 2D objects.
        zdir : {'x', 'y', 'z'}, default: 'z'
            The direction to use for the z-positions.
        autolim : bool, default: True
            Whether to update the data limits.
        axlim_clip : bool, default: False
            Whether to hide the scatter points outside the axes view limits.
        """
        if not isinstance(col, art3d._Loose3D):
            _api.check_in_list(self._axis_names, zdir=zdir)
            zs = np.ma.filled(np.ma.asarray(zs, dtype=float), np.nan)
            if isinstance(col, mcoll.PolyCollection):
                art3d.poly_collection_2d_to_3d(col, zs=zs, zdir=zdir)
            elif isinstance(col, mcoll.LineCollection):
                art3d.line_collection_2d_to_3d(col, zs=zs, zdir=zdir)
            elif isinstance(col, mcoll.PatchCollection):
                art3d.patch_collection_2d_to_3d(col, zs=zs, zdir=zdir)
            else:
                # Any other collection is added as it is, as matplotlib
                # adds it.
                return self.add_collection(col)
        return col._adopt_into(self, autolim, axlim_clip)

    def add_collection(self, collection, autolim=True):
        # docstring inherited
        if isinstance(collection, art3d._Loose3D):
            return collection._adopt_into(self, bool(autolim), False)
        return super().add_collection(collection, autolim)

    # -- plotting ----------------------------------------------------------

    def plot(self, xs, ys, *args, zdir='z', axlim_clip=False, **kwargs):
        """
        Plot 2D or 3D data.

        Parameters
        ----------
        xs : 1D array-like
            x coordinates of vertices.
        ys : 1D array-like
            y coordinates of vertices.
        zs : float or 1D array-like
            z coordinates of vertices; either one for all points or one for
            each point.
        zdir : {'x', 'y', 'z'}, default: 'z'
            When plotting 2D data, the direction to use as z.
        axlim_clip : bool, default: False
            Whether to hide data that is outside the axes view limits.
        **kwargs
            Other arguments are forwarded to `matplotlib.axes.Axes.plot`.
        """
        # `zs` can be passed positionally or as keyword; checking whether
        # args[0] is a string matches the behavior of 2D `plot`.
        if args and not isinstance(args[0], str):
            zs, *args = args
            if 'zs' in kwargs:
                raise TypeError("plot() for multiple values for argument 'zs'")
        else:
            zs = kwargs.pop('zs', 0)
        _api.check_in_list(self._axis_names, zdir=zdir)
        kwargs = cbook.normalize_kwargs(kwargs, mlines.Line2D)
        xs, ys, zs = cbook._broadcast_with_masks(xs, ys, zs)
        if len(args) > 1 or args and not isinstance(args[0], str):
            # Several datasets, as 2D `plot` splits them, each at *zs*
            # (matplotlib's `line_2d_to_3d` of every line).
            zs = np.ma.filled(np.ma.asarray(zs, dtype=float), np.nan).ravel()
            datasets = [
                (x, y, np.broadcast_to(zs, len(x)), fmt, label)
                for x, y, fmt, label in _plot_datasets([xs, ys, *args], None,
                                                       kwargs)]
        else:
            datasets = [(xs, ys, zs, args[0] if args else None,
                         kwargs.get("label"))]
        lines = []
        for x, y, z, fmt, label in datasets:
            props = {k: v for k, v in kwargs.items() if v is not None}
            if label is not None:
                props["label"] = label
            native = _mplot3d.plot(self._native, _floats(x, y, z), zdir, fmt,
                                   mlines._native_kwargs(props),
                                   bool(axlim_clip))
            lines.append(keep_program_picker(wrap(self, native, art3d.Line3D),
                                             props))
        return lines

    plot3D = plot

    @_preprocess_data(replace_names=["xs", "ys", "zs", "s",
                                     "edgecolors", "c", "facecolor",
                                     "facecolors", "color"])
    def scatter(self, xs, ys, zs=0, zdir='z', s=20, c=None, depthshade=None,
                *args, depthshade_minalpha=None, axlim_clip=False, **kwargs):
        """
        Create a scatter plot.

        Parameters
        ----------
        xs, ys : array-like
            The data positions.
        zs : float or array-like, default: 0
            The z-positions.
        zdir : {'x', 'y', 'z', '-x', '-y', '-z'}, default: 'z'
            The axis direction for the *zs*.
        s : float or array-like, default: 20
            The marker size in points**2.
        c : :mpltype:`color`, sequence, or sequence of colors, optional
            The marker color.
        depthshade : bool, default: :rc:`axes3d.depthshade`
            Whether to shade the scatter markers to give the appearance of
            depth.
        depthshade_minalpha : float, default: :rc:`axes3d.depthshade_minalpha`
            The lowest alpha value applied by depth-shading.
        axlim_clip : bool, default: False
            Whether to hide the scatter points outside the axes view limits.

        Returns
        -------
        paths : `~matplotlib.collections.PathCollection`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        if args:
            # matplotlib hands them to 2D `scatter` after *s*, given as a
            # keyword.
            raise TypeError("Axes.scatter() got multiple values for argument 's'")
        xs, ys, zs = cbook._broadcast_with_masks(xs, ys, zs)
        s = np.ma.ravel(s)
        xs, ys, zs, s, c, color = cbook.delete_masked_points(
            xs, ys, zs, s, c, kwargs.get('color', None))
        if kwargs.get("color") is not None:
            kwargs['color'] = color
        kwargs = cbook.normalize_kwargs(kwargs, mcoll.Collection)
        linewidths = kwargs.pop('linewidth', None)
        edgecolors = kwargs.pop('edgecolor', None)
        marker = kwargs.pop('marker', None) or rcParams['scatter.marker']
        cmap, norm = kwargs.pop('cmap', None), kwargs.pop('norm', None)
        vmin, vmax = kwargs.pop('vmin', None), kwargs.pop('vmax', None)
        alpha = kwargs.pop('alpha', None)
        c, colors, edgecolors = self._parse_scatter_color_args(
            c, edgecolors, kwargs, xs.size,
            get_next_color_func=self._next_patch_color)
        marker_obj = marker if isinstance(marker, MarkerStyle) else MarkerStyle(marker)
        made = None
        if colors is None:
            made, mapping = _mapping(cmap, norm, vmin, vmax)
            faces = (None, (np.ma.filled(np.ma.asarray(c, dtype=float),
                                         np.nan).ravel(), mapping))
        else:
            faces = (np.asarray(colors, dtype=float).reshape(-1, 4), None)
        if edgecolors is not None and not cbook._str_equal(edgecolors, 'face'):
            edgecolors = mcolors.to_rgba_array(edgecolors)
        if linewidths is not None:
            linewidths = [float(lw) for lw in np.atleast_1d(linewidths)]
        alphas = None
        if np.iterable(alpha):
            alphas, alpha = [float(a) for a in alpha], None
        s = np.broadcast_to(np.ma.filled(np.asarray(s, dtype=float), np.nan),
                            xs.shape if s.size != 1 else (1,))
        native = _mplot3d.scatter(
            self._native, _floats(xs, ys, zs),
            (np.ascontiguousarray(s, dtype=float).ravel(),
             mlines._marker_spec(marker_obj)), faces, (edgecolors, linewidths),
            (None if alpha is None else float(alpha), alphas),
            (zdir, None if depthshade is None else bool(depthshade),
             None if depthshade_minalpha is None
             else float(depthshade_minalpha), bool(axlim_clip)))
        collection = wrap(self, native, art3d.Path3DCollection)
        # matplotlib's scatter gives its marker path as a tuple.
        collection._paths_container = tuple
        if made is not None:
            collection._attach_native_mapping(colorizer=made)
        collection._internal_update(kwargs)
        return collection

    scatter3D = scatter

    def _next_patch_color(self):
        """The next colour of the patch property cycle."""
        from pltrs._native import plotting as _plotting
        return _plotting.next_patch_color(self._native)

    def plot_surface(self, X, Y, Z, *, norm=None, vmin=None,
                     vmax=None, lightsource=None, axlim_clip=False, **kwargs):
        """
        Create a surface plot.

        Parameters
        ----------
        X, Y, Z : 2D arrays
            Data values.
        rcount, ccount : int
            Maximum number of samples used in each direction.
        rstride, cstride : int
            Downsampling stride in each direction.
        color : :mpltype:`color`
            Color of the surface patches.
        cmap : Colormap, optional
            Colormap of the surface patches.
        shade : bool, default: True
            Whether to shade the facecolors.
        axlim_clip : bool, default: False
            Whether to hide patches with a vertex outside the axes view limits.

        Returns
        -------
        `.Poly3DCollection`
        """
        if Z.ndim != 2:
            raise ValueError("Argument Z must be 2-dimensional.")
        Z = cbook._to_unmasked_float_array(Z)
        X, Y, Z = np.broadcast_arrays(X, Y, Z)
        sampling = _sampling(kwargs.pop('rstride', None),
                             kwargs.pop('cstride', None),
                             kwargs.pop('rcount', None),
                             kwargs.pop('ccount', None))
        color = kwargs.pop('color', None)
        cmap = kwargs.pop('cmap', None)
        fcolors = kwargs.pop('facecolors', None)
        if 'shade' in kwargs and kwargs['shade'] is None:
            raise ValueError("shade cannot be None.")
        shade = kwargs.pop('shade', None)
        if fcolors is not None:
            fcolors = _face_colors(fcolors, Z.shape)
        made, mapping = _surface_mapping(cmap, norm, vmin, vmax)
        grids = tuple(np.ascontiguousarray(np.ma.filled(
            np.ma.asarray(a, dtype=float), np.nan)) for a in (X, Y, Z))
        native = _mplot3d.plot_surface(
            self._native, grids, sampling, (color, mapping, fcolors),
            _shading(shade, lightsource), bool(axlim_clip))
        collection = wrap(self, native, art3d.Poly3DCollection)
        if made is not None:
            collection._attach_native_mapping(colorizer=made)
        collection._internal_update(kwargs)
        return collection

    def plot_wireframe(self, X, Y, Z, *, axlim_clip=False, **kwargs):
        """
        Plot a 3D wireframe.

        Parameters
        ----------
        X, Y, Z : 2D arrays
            Data values.
        rcount, ccount : int
            Maximum number of samples used in each direction.
        rstride, cstride : int
            Downsampling stride in each direction.
        axlim_clip : bool, default: False
            Whether to hide lines and patches with vertices outside the axes
            view limits.

        Returns
        -------
        `.Line3DCollection`
        """
        if Z.ndim != 2:
            raise ValueError("Argument Z must be 2-dimensional.")
        X, Y, Z = np.broadcast_arrays(X, Y, Z)
        rstride, cstride = kwargs.pop('rstride', None), kwargs.pop('cstride', None)
        rcount, ccount = kwargs.pop('rcount', None), kwargs.pop('ccount', None)
        if (rstride == 0 and cstride == 0) or (rcount == 0 and ccount == 0):
            raise ValueError("Either rstride or cstride must be non zero")
        sampling = _sampling(rstride, cstride, rcount, ccount)
        if sampling is None and rcParams['_internal.classic_mode']:
            sampling = ("stride", 1, 1)
        kwargs = cbook.normalize_kwargs(kwargs, mcoll.Collection)
        color = kwargs.pop('color', None)
        grids = tuple(np.ascontiguousarray(np.ma.filled(
            np.ma.asarray(a, dtype=float), np.nan)) for a in (X, Y, Z))
        native = _mplot3d.plot_wireframe(self._native, grids, sampling, color,
                                         bool(axlim_clip))
        collection = wrap(self, native, art3d.Line3DCollection)
        collection._internal_update(kwargs)
        return collection

    def text(self, x, y, z, s, zdir=None, *, axlim_clip=False, **kwargs):
        """
        Add the text *s* to the 3D Axes at location *x*, *y*, *z* in data
        coordinates.

        Parameters
        ----------
        x, y, z : float
            The position to place the text.
        s : str
            The text.
        zdir : {'x', 'y', 'z', 3-tuple}, optional
            The direction to be used as the z-direction.
        axlim_clip : bool, default: False
            Whether to hide text that is outside the axes view limits.

        Returns
        -------
        `.Text3D`
        """
        direction = tuple(map(float, art3d.get_dir_vector(zdir)))
        native = _mplot3d.text(self._native, (float(x), float(y), float(z)),
                               str(s), direction, bool(axlim_clip))
        text = wrap(self, native, art3d.Text3D)
        text._internal_update(kwargs)
        return text

    text3D = text
    text2D = Axes.text

    @_preprocess_data()
    def bar3d(self, x, y, z, dx, dy, dz, color=None,
              zsort='average', shade=True, lightsource=None, *args,
              axlim_clip=False, **kwargs):
        """
        Generate a 3D barplot.

        Parameters
        ----------
        x, y, z : array-like
            The coordinates of the anchor point of the bars.
        dx, dy, dz : float or array-like
            The width, depth, and height of the bars, respectively.
        color : sequence of colors, optional
            The color of the bars can be specified globally or
            individually.
        zsort : {'average', 'min', 'max'}, default: 'average'
            The z-axis sorting scheme.
        shade : bool, default: True
            When true, this shades the dark sides of the bars (relative
            to the plot's source of light).
        axlim_clip : bool, default: False
            Whether to hide the bars with points outside the axes view limits.
        **kwargs
            Any additional keyword arguments are passed onto
            `~.art3d.Poly3DCollection`.

        Returns
        -------
        collection : `~.art3d.Poly3DCollection`
            A collection of three-dimensional polygons representing the bars.

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        # The positional arguments go to the `.Poly3DCollection` after its
        # vertices: its *sizes*.
        if len(args) > 1:
            raise TypeError(f"PolyCollection.__init__() takes from 2 to 3 "
                            f"positional arguments but {len(args) + 2} were "
                            f"given")
        if args:
            if 'sizes' in kwargs:
                raise TypeError("PolyCollection.__init__() got multiple "
                                "values for argument 'sizes'")
            kwargs['sizes'] = args[0]
        _api.check_in_list(['average', 'min', 'max'], zsort=zsort)
        columns = tuple(np.atleast_1d(np.asarray(v, dtype=float)).ravel().tolist()
                        for v in (x, y, z, dx, dy, dz))
        colors = None if color is None else mcolors.to_rgba_array(color)
        native = _mplot3d.bar3d(self._native, (columns[:3], columns[3:]),
                                (colors, zsort),
                                _shading(bool(shade), lightsource),
                                bool(axlim_clip))
        collection = wrap(self, native, art3d.Poly3DCollection)
        collection._internal_update(kwargs)
        return collection

    @_preprocess_data()
    def quiver(self, X, Y, Z, U, V, W, *,
               length=1, arrow_length_ratio=.3, pivot='tail', normalize=False,
               axlim_clip=False, **kwargs):
        """
        Plot a 3D field of arrows.

        Parameters
        ----------
        X, Y, Z : array-like
            The x, y and z coordinates of the arrow locations.
        U, V, W : array-like
            The x, y and z components of the arrow vectors.
        length : float, default: 1
            The length of each quiver.
        arrow_length_ratio : float, default: 0.3
            The ratio of the arrow head with respect to the quiver.
        pivot : {'tail', 'middle', 'tip'}, default: 'tail'
            The part of the arrow that is at the grid point.
        normalize : bool, default: False
            Whether all arrows are normalized to have the same length.
        axlim_clip : bool, default: False
            Whether to hide arrows with points outside the axes view limits.
        **kwargs
            Any additional keyword arguments are delegated to
            :class:`.LineCollection`

        Other Parameters
        ----------------
        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER
        """
        _api.check_in_list(['tail', 'middle', 'tip'], pivot=pivot)
        input_args = [X, Y, Z, U, V, W]
        masks = [np.ma.getmaskarray(a) for a in input_args
                 if isinstance(a, np.ma.MaskedArray)]
        broadcast = np.broadcast_arrays(*input_args, *masks)
        arrays = [np.asarray(a, dtype=float).ravel() for a in broadcast[:6]]
        keep = np.ones(arrays[0].size, dtype=bool)
        for mask in broadcast[6:]:
            keep &= ~mask.ravel()
        x, y, z, u, v, w = (a[keep] for a in arrays)
        kwargs = cbook.normalize_kwargs(kwargs, mcoll.Collection)
        color = kwargs.pop('color', None)
        colors = kwargs.pop('colors', None)
        if colors is None and color is not None and np.ndim(color) == 2:
            color, colors = None, color
        native = _mplot3d.quiver(
            self._native,
            (np.column_stack([x, y, z]), np.column_stack([u, v, w])),
            (float(length), float(arrow_length_ratio), pivot, bool(normalize)),
            (color, None if colors is None else mcolors.to_rgba_array(colors)),
            bool(axlim_clip))
        collection = wrap(self, native, art3d.Line3DCollection)
        collection._internal_update(kwargs)
        return collection

    quiver3D = quiver

    def plot_trisurf(self, *args, color=None, norm=None, vmin=None, vmax=None,
                     lightsource=None, axlim_clip=False, **kwargs):
        """
        Plot a triangulated surface.

        Parameters
        ----------
        X, Y, Z : array-like
            Data values as 1D arrays.
        color
            Color of the surface patches.
        cmap
            A colormap for the surface patches.
        norm : `~matplotlib.colors.Normalize`, optional
            An instance of Normalize to map values to colors.
        vmin, vmax : float, optional
            Minimum and maximum value to map.
        shade : bool, default: True
            Whether to shade the facecolors.
        axlim_clip : bool, default: False
            Whether to hide patches with a vertex outside the axes view limits.

        Returns
        -------
        `.Poly3DCollection`
        """
        from pltrs.tri._triangulation import Triangulation
        tri, args, kwargs = Triangulation.get_from_args_and_kwargs(*args, **kwargs)
        try:
            z = kwargs.pop('Z')
        except KeyError:
            # We do this so Z doesn't get passed as an arg to PolyCollection
            z, *args = args
        cmap = kwargs.pop('cmap', None)
        shade = kwargs.pop('shade', None)
        made, mapping = _surface_mapping(cmap, norm, vmin, vmax)
        native = _mplot3d.plot_trisurf(
            self._native,
            (np.asarray(tri.x, dtype=float).tolist(),
             np.asarray(tri.y, dtype=float).tolist(),
             [tuple(map(int, t)) for t in tri.triangles],
             None if tri.mask is None else tri.mask.tolist()),
            np.asarray(z, dtype=float).ravel().tolist(), (color, mapping),
            _shading(shade, lightsource), bool(axlim_clip))
        collection = wrap(self, native, art3d.Poly3DCollection)
        if made is not None:
            collection._attach_native_mapping(colorizer=made)
        collection._internal_update(kwargs)
        return collection


def get_test_data(delta=0.05):
    """Return a tuple X, Y, Z with a test data set."""
    return _mplot3d.test_data(float(delta))


class _Quaternion:
    """
    Quaternions
    consisting of scalar, along 1, and vector, with components along i, j, k
    """

    def __init__(self, scalar, vector):
        self.scalar = scalar
        self.vector = np.array(vector)

    def __neg__(self):
        return self.__class__(-self.scalar, -self.vector)

    def __mul__(self, other):
        """
        Product of two quaternions
        i*i = j*j = k*k = i*j*k = -1
        Quaternion multiplication can be expressed concisely
        using scalar and vector parts,
        see <https://en.wikipedia.org/wiki/Quaternion#Scalar_and_vector_parts>
        """
        return self.__class__(
            self.scalar*other.scalar - np.dot(self.vector, other.vector),
            self.scalar*other.vector + self.vector*other.scalar
            + np.cross(self.vector, other.vector))

    def conjugate(self):
        """The conjugate quaternion -(1/2)*(q+i*q*i+j*q*j+k*q*k)"""
        return self.__class__(self.scalar, -self.vector)

    @property
    def norm(self):
        """The 2-norm, q*q', a scalar"""
        return self.scalar*self.scalar + np.dot(self.vector, self.vector)

    def normalize(self):
        """Scaling such that norm equals 1"""
        n = np.sqrt(self.norm)
        return self.__class__(self.scalar/n, self.vector/n)

    def reciprocal(self):
        """The reciprocal, 1/q = q'/(q*q') = q' / norm(q)"""
        n = self.norm
        return self.__class__(self.scalar/n, -self.vector/n)

    def __div__(self, other):
        return self*other.reciprocal()

    __truediv__ = __div__

    def rotate(self, v):
        # Rotate the vector v by the quaternion q, i.e.,
        # calculate (the vector part of) q*v/q
        v = self.__class__(0, v)
        v = self*v/self
        return v.vector

    def __eq__(self, other):
        return (self.scalar == other.scalar) and (self.vector == other.vector).all

    def __repr__(self):
        return "_Quaternion({}, {})".format(repr(self.scalar), repr(self.vector))

    @classmethod
    def rotate_from_to(cls, r1, r2):
        """
        The quaternion for the shortest rotation from vector r1 to vector r2
        i.e., q = sqrt(r2*r1'), normalized.
        If r1 and r2 are antiparallel, then the result is ambiguous;
        a normal vector will be returned, and a warning will be issued.
        """
        k = np.cross(r1, r2)
        nk = np.linalg.norm(k)
        th = np.arctan2(nk, np.dot(r1, r2))
        th /= 2
        if nk == 0:  # r1 and r2 are parallel or anti-parallel
            if np.dot(r1, r2) < 0:
                warnings.warn("Rotation defined by anti-parallel vectors is ambiguous")
                k = np.zeros(3)
                k[np.argmin(r1*r1)] = 1  # basis vector most perpendicular to r1-r2
                k = np.cross(r1, k)
                k = k / np.linalg.norm(k)  # unit vector normal to r1-r2
                q = cls(0, k)
            else:
                q = cls(1, [0, 0, 0])  # = 1, no rotation
        else:
            q = cls(np.cos(th), k*np.sin(th)/nk)
        return q

    @classmethod
    def from_cardan_angles(cls, elev, azim, roll):
        """
        Converts the angles to a quaternion
            q = exp((roll/2)*e_x)*exp((elev/2)*e_y)*exp((-azim/2)*e_z)
        i.e., the angles are a kind of Tait-Bryan angles, -z,y',x".
        The angles should be given in radians, not degrees.
        """
        ca, sa = np.cos(azim/2), np.sin(azim/2)
        ce, se = np.cos(elev/2), np.sin(elev/2)
        cr, sr = np.cos(roll/2), np.sin(roll/2)

        qw = ca*ce*cr + sa*se*sr
        qx = ca*ce*sr - sa*se*cr
        qy = ca*se*cr + sa*ce*sr
        qz = ca*se*sr - sa*ce*cr
        return cls(qw, [qx, qy, qz])

    def as_cardan_angles(self):
        """
        The inverse of `from_cardan_angles()`.
        Note that the angles returned are in radians, not degrees.
        The angles are not sensitive to the quaternion's norm().
        """
        qw = self.scalar
        qx, qy, qz = self.vector[..., :]
        azim = np.arctan2(2*(-qw*qz+qx*qy), qw*qw+qx*qx-qy*qy-qz*qz)
        elev = np.arcsin(np.clip(2*(qw*qy+qz*qx)/(qw*qw+qx*qx+qy*qy+qz*qz), -1, 1))
        roll = np.arctan2(2*(qw*qx-qy*qz), qw*qw-qx*qx-qy*qy+qz*qz)
        return elev, azim, roll
