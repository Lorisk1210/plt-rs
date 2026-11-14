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

from numbers import Real
import textwrap
import threading
import warnings
import weakref

import numpy as np

from pltrs import _docstring
from pltrs import _api, _handles, _lazy_imports, _values, cbook, rcParams
from pltrs import collections as mcoll, colors as mcolors, lines as mlines
from pltrs import patches as mpatches, text as mtext
from pltrs._native import artist as _artist, axes as _axes, axis as _axis
from pltrs._native import mplot3d as _mplot3d
from pltrs.axes import Axes
from pltrs.axes._base import _SharedAxes
from pltrs.axes._construction import native_figure
from pltrs.axes._ticks import _axis_method_wrapper
from pltrs._artist_kwargs import keep_python_kwargs
from pltrs._preprocess import _preprocess_data
from pltrs._program_classes import takes_from_program
from pltrs.axes._plotting._helpers import wrap
from pltrs.axes._plotting.lines import _plot_datasets
from pltrs.axes._plotting.scatter import _mapping
from pltrs.markers import MarkerStyle
from pltrs._text_methods import _init_added
from pltrs.text import _measured_by_given
from pltrs.transforms import Bbox

from . import _program_projection, _properties, art3d, axis3d, proj3d
from ._charts import ChartsMixin
from ._contours import ContoursMixin
from ._limit_boxes import LimitBoxesMixin
from ._voxels import VoxelsMixin

_draw_problems = _lazy_imports.module(globals(), "._draw_problems")


def _check_margin(m):
    """matplotlib's check of a margin."""
    if m <= -0.5:
        raise ValueError("margin must be greater than -0.5")
    return float(m)


# matplotlib's 3-D axes add their z and view groups to those of every
# `Axes`, whose x and y groups they share with 2-D axes.
Axes._shared_axes["z"] = _SharedAxes("z")
Axes._shared_axes["view"] = _SharedAxes("view")


def _native_share(other, kind):
    """The native axes a new 3-D axes shares an axis (*kind* ``'x'``,
    ``'y'``, ``'z'``) or, ``'3'``, its view with, None for none: matplotlib
    checks those of a 2-D axes to be axes, and joins the others to its
    groups by weak reference."""
    if other is None:
        return None
    if kind in 'xy':
        from pltrs.axes._base import _AxesBase
        _api.check_isinstance(_AxesBase, other=other)
    else:
        weakref.ref(other)
    return other._native


def _margined(lower, upper, view_margin):
    """
    The limits *lower* and *upper* as matplotlib's ``_set_lim3d`` hands them
    to the axis, typed as it types them: widened by *view_margin* in the
    coordinates of the scale when it is positive, which makes them numpy
    floats, else as given (adding the margin of 0 keeps their types).
    """
    if view_margin > 0:
        return np.float64(lower), np.float64(upper)
    delta = (upper - lower) * view_margin
    return lower - delta, upper + delta


def _floats(*arrays):
    """The arrays broadcast together and flattened to floats, masked values
    as NaN."""
    arrays = np.broadcast_arrays(*[np.ma.filled(np.ma.asarray(a, dtype=float),
                                                np.nan) for a in arrays])
    return tuple(np.ascontiguousarray(a, dtype=float).ravel() for a in arrays)


def _plain_angles(view):
    """Whether the view angles *view* are all numbers (pltrs (Rust) holds
    them; matplotlib keeps any other, failing where it projects)."""
    return all(isinstance(angle, Real) for angle in view)


def _failure_of(problem):
    """The exception *problem* (None for none) as pltrs (Rust) records the
    failure of a draw: the name of its class and its message."""
    return None if problem is None else (type(problem).__name__, str(problem))


#: The faces of the unit cube as matplotlib's ``bar3d`` lists them, each
#: seen counterclockwise from outside: -z, +z, -y, +y, -x, +x.
_CUBOID = np.array([
    ((0, 0, 0), (0, 1, 0), (1, 1, 0), (1, 0, 0)),
    ((0, 0, 1), (1, 0, 1), (1, 1, 1), (0, 1, 1)),
    ((0, 0, 0), (1, 0, 0), (1, 0, 1), (0, 0, 1)),
    ((0, 1, 0), (0, 1, 1), (1, 1, 1), (1, 1, 0)),
    ((0, 0, 0), (0, 0, 1), (0, 1, 1), (0, 1, 0)),
    ((1, 0, 0), (1, 1, 0), (1, 1, 1), (1, 0, 1)),
])


def _cuboids(x, y, z, dx, dy, dz):
    """The faces of the bars of ``bar3d`` as matplotlib builds them, with
    numpy, whose arithmetic warns of bars that are no numbers (pltrs
    (Rust) builds those it draws): indexed by bar and face, vertex,
    coordinate."""
    polys = np.empty(x.shape + _CUBOID.shape)
    for i, p, dp in [(0, x, dx), (1, y, dy), (2, z, dz)]:
        p = p[..., np.newaxis, np.newaxis]
        dp = dp[..., np.newaxis, np.newaxis]
        polys[..., i] = p + dp * _CUBOID[..., i]
    return polys.reshape((-1,) + polys.shape[2:])


def _shading(shade, lightsource):
    """The native shading: whether to shade (None: the method's default)
    and the light of a `~matplotlib.colors.LightSource` (None: mplot3d's)."""
    return (None if shade is None else bool(shade), art3d._native_light(lightsource))


def _check_quiver(arrows, length, arrow_length_ratio, pivot):
    """Raise what matplotlib's ``quiver`` raises computing the shafts of
    the *arrows* (its arguments broadcast together, the masked ones left
    out) from *length*, *arrow_length_ratio* and *pivot*: numpy's errors on
    values of no numbers. Return whether there is an arrow (matplotlib
    reads none of the three for none)."""
    if any(len(v) == 0 for v in arrows):
        return False
    shaft_dt = np.array([0., length], dtype=float)
    shaft_dt * arrow_length_ratio
    _api.check_in_list(['tail', 'middle', 'tip'], pivot=pivot)
    if pivot == 'tail':
        shaft_dt -= length
    elif pivot == 'middle':
        shaft_dt -= length / 2
    XYZ = np.column_stack(arrows[:3])
    UVW = np.column_stack(arrows[3:]).astype(float)
    XYZ - np.multiply.outer(shaft_dt, UVW)
    return True


def _surface_mapping(cmap, norm, vmin, vmax):
    """The colour mapping of a surface, ``(colorizer, native)`` as
    `._mapping` gives it, or ``(None, None)`` for a surface coloured
    without one: matplotlib maps the faces only through a *cmap* given,
    and ignores *norm*, *vmin* and *vmax* otherwise."""
    if not cmap:
        return None, None
    return _mapping(cmap, norm, vmin, vmax)


def _sampling(rstride, cstride, rcount, ccount, default_stride):
    """The native sampling of a surface or wireframe (strides or counts,
    which matplotlib refuses together); a stride left out of strides given
    is *default_stride* (a surface's 10, a wireframe's 1)."""
    has_stride = rstride is not None or cstride is not None
    has_count = rcount is not None or ccount is not None
    if has_stride and has_count:
        raise ValueError("Cannot specify both stride and count arguments")
    if has_stride:
        return ("stride", default_stride if rstride is None else int(rstride),
                default_stride if cstride is None else int(cstride))
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

#: What the 3-D axes a thread makes next was given that its native axes
#: does not keep: the view angles as given (``view``) and the axes it
#: shares its view with (``shareview``).
_made = threading.local()


@_docstring.interpd
@_api.define_aliases({
    "xlim": ["xlim3d"], "ylim": ["ylim3d"], "zlim": ["zlim3d"]})
class Axes3D(ChartsMixin, ContoursMixin, VoxelsMixin, LimitBoxesMixin, Axes):
    """
    3D Axes object.

    .. note::

        As a user, you do not instantiate Axes directly, but use Axes creation
        methods instead; e.g. from `.pyplot` or `.Figure`:
        `~.pyplot.subplots`, `~.pyplot.subplot_mosaic` or `.Figure.add_axes`.

    .. note::

        Some of the inherited behavior of Axes is not applicable to 3d and will raise
        an `.UnsupportedError`.
    """
    name = '3d'

    _axis_names = ("x", "y", "z")
    _native_axis_names = _axis_names

    def __init__(self, fig, rect=None, *args, elev=30, azim=-60, roll=0,
                 shareview=None, sharez=None, proj_type='persp',
                 focal_length=None, box_aspect=None, computed_zorder=True,
                 **kwargs):
        """
        Parameters
        ----------
        fig : Figure
            The parent figure.
        rect : tuple (left, bottom, width, height), default: (0, 0, 1, 1)
            The ``(left, bottom, width, height)`` Axes position.
        elev : float, default: 30
            The elevation angle in degrees rotates the camera above and below
            the x-y plane, with a positive angle corresponding to a location
            above the plane.
        azim : float, default: -60
            The azimuthal angle in degrees rotates the camera about the z axis,
            with a positive angle corresponding to a right-handed rotation. In
            other words, a positive azimuth rotates the camera about the origin
            from its location along the +x axis towards the +y axis.
        roll : float, default: 0
            The roll angle in degrees rotates the camera about the viewing
            axis. A positive angle spins the camera clockwise, causing the
            scene to rotate counter-clockwise.
        shareview : Axes3D, optional
            Other Axes to share view angles with.  Note that it is not possible
            to unshare axes.
        sharez : Axes3D, optional
            Other Axes to share z-limits with.  Note that it is not possible to
            unshare axes.
        proj_type : {'persp', 'ortho'}
            The projection type, default 'persp'.
        focal_length : float, default: None
            For a projection type of 'persp', the focal length of the virtual
            camera. Must be > 0. If None, defaults to 1.
            For a projection type of 'ortho', must be set to either None
            or infinity (numpy.inf). If None, defaults to infinity.
            The focal length can be computed from a desired Field Of View via
            the equation: focal_length = 1/tan(FOV/2)
        box_aspect : 3-tuple of floats, default: None
            Changes the physical dimensions of the Axes3D, such that the ratio
            of the axis lengths in display units is x:y:z.
            If None, defaults to 4:4:3
        computed_zorder : bool, default: True
            If True, the draw order is computed based on the average position
            along the view direction for supported artist types (currently
            Collections and Patches only).
            Set to False if you want to manually control the order in which
            Artists are drawn on top of each other using their *zorder*
            attribute. This can be used for fine-tuning if the automatic order
            does not produce the desired result. Note however, that a manual
            order will only be correct for a limited view angle. If the figure
            is rotated by the user, it will look wrong from certain angles.

        **kwargs
            Other optional keyword arguments:

            %(Axes3D:kwdoc)s
        """
        if rect is None:
            rect = [0.0, 0.0, 1.0, 1.0]
        options, kwargs = self._native_options({
            **kwargs, 'elev': elev, 'azim': azim, 'roll': roll,
            'shareview': shareview, 'sharez': sharez, 'proj_type': proj_type,
            'focal_length': focal_length, 'box_aspect': box_aspect,
            'computed_zorder': computed_zorder})
        args = (rect, *args)
        unplaced = None
        if len(args) == 1 and (isinstance(rect, Bbox) or np.iterable(rect)):
            bounds = tuple(map(float, rect.bounds if isinstance(rect, Bbox)
                               else Bbox.from_bounds(*rect).bounds))
            if not np.isfinite(bounds).all():
                # matplotlib keeps the rectangle (only `add_axes` refuses
                # it): the axes is made at the unit one, then moved there.
                bounds, unplaced = (0., 0., 1., 1.), bounds
            native = _mplot3d.removed_axes(native_figure(fig), bounds, options)
        else:
            from pltrs.gridspec import SubplotSpec
            spec = SubplotSpec._from_subplot_args(fig, args)
            native = _mplot3d.removed_subplot(native_figure(fig),
                                              spec._native_spec(), options)
        _handles.adopt(self, native, fig.figure)
        if unplaced is not None:
            _mplot3d.set_position(native, unplaced)
        self._init_3d_state(fig.figure)
        self._internal_update(kwargs)

    @classmethod
    def _native_options(cls, kwargs):
        """
        The native options of a new 3-D axes from matplotlib's `Axes3D`
        keyword arguments in *kwargs*, and the other keyword arguments.
        """
        kwargs = dict(kwargs)
        given = (kwargs.pop('elev', 30), kwargs.pop('azim', -60),
                 kwargs.pop('roll', 0))
        # For `_init_3d_state`, on this thread: the axes made next.
        _made.view = given
        # Angles of no number start from the default view, and are kept
        # once the axes is made (see `_init_3d_state`).
        initial = (tuple(map(float, given)) if _plain_angles(given)
                   else (30., -60., 0.))
        proj_type = kwargs.pop('proj_type', 'persp')
        focal_length = cls._checked_focal_length(
            proj_type, kwargs.pop('focal_length', None))
        box_aspect = kwargs.pop('box_aspect', None)
        if box_aspect is not None:
            # Checked as matplotlib's `set_box_aspect` checks it.
            box_aspect = np.asarray(box_aspect, dtype=float)
            _api.check_shape((3,), aspect=box_aspect)
        _made.computed_zorder = kwargs.pop('computed_zorder', True)
        computed_zorder = bool(_made.computed_zorder)
        given_shares = (kwargs.pop('sharex', None), kwargs.pop('sharey', None),
                        kwargs.pop('sharez', None), kwargs.pop('shareview', None))
        _made.shareview = given_shares[3]
        shares = tuple(map(_native_share, given_shares, 'xyz3'))
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
        ax._connect_unit_change_handler()
        return ax

    def _init_3d_state(self, figure):
        """Set up the Python state of the axes around its native handle in
        the root *figure*."""
        self.xaxis = axis3d.XAxis(self)
        self.yaxis = axis3d.YAxis(self)
        self.zaxis = axis3d.ZAxis(self)
        # A new axes looks from its initial view, whose angles matplotlib
        # keeps as given (its defaults are int literals, ``elev=30``).
        native = _mplot3d.get_view(self._native)
        given = _made.__dict__.pop('view', None)
        kept = given is not None and not _plain_angles(given)
        if not kept and (given is None or tuple(map(float, given)) != native):
            given = tuple(default if angle == default else angle
                          for angle, default in zip(native, (30, -60, 0)))
        self.initial_elev, self.initial_azim, self.initial_roll = given
        _made.__dict__.pop('shareview', None)
        # As given, as matplotlib keeps it.
        self._computed_zorder = _made.__dict__.pop('computed_zorder', True)
        _values.remember_all(self, dict(zip(_VIEW_ANGLES, (
            self.initial_elev, self.initial_azim, self.initial_roll))), _VIEW_ANGLES)
        if kept:
            # matplotlib keeps them, its projection failing on them.
            self._turn(given, 'z', False)
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

    def _native_block(self):
        # The block of the 3-D axes, of which pltrs (Rust) holds its own.
        return _mplot3d.subplot_spec(self._native)

    def set_subplotspec(self, subplotspec):
        # docstring inherited
        # matplotlib places the Axes at the position the spec gives, which
        # fails first for what is no spec.
        subplotspec.get_position(self.get_figure(root=False))
        _mplot3d.set_subplot_spec(self._native, subplotspec._native_spec())
        self._place_on(subplotspec)

    def clear(self):
        # docstring inherited.
        self._decouple_children()
        _mplot3d.clear(self._native)
        # matplotlib's clear makes a new registry of the limit callbacks
        # (pltrs (Rust) disconnected those of the axes).
        self.__dict__.pop("_limit_callbacks", None)
        self.__dict__.pop("_limit_callbacks_connected", None)
        for axis in (self.xaxis, self.yaxis, self.zaxis):
            axis._given_scale = None
            # matplotlib's clear clears each axis, which leaves it stale and
            # with new callbacks (the Axes' handler of its units gone).
            axis.callbacks = cbook.CallbackRegistry(signals=["units"])
            axis.stale = True

    def _on_limits_changed(self, name, notify):
        # docstring inherited: the limits of the three axes are pltrs
        # (Rust)'s 3-D axes', not those of its projection plane.
        _mplot3d.on_limits_changed(self._native, name, notify)

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

    twinx = _api.unsupported_method()
    twiny = _api.unsupported_method()
    secondary_xaxis = _api.unsupported_method()
    secondary_yaxis = _api.unsupported_method()

    _msg = "Use ax.set_xscale/set_yscale/set_zscale and ax.plot(...) instead."
    semilogx = _api.unsupported_method(append_message=_msg)
    semilogy = _api.unsupported_method(append_message=_msg)
    semilogz = _api.unsupported_method(append_message=_msg)
    loglog = _api.unsupported_method(append_message=_msg)

    def convert_zunits(self, z):
        """
        For artists in an Axes, if the zaxis has units support,
        convert *z* using zaxis unit type
        """
        return self.zaxis.convert_units(z)

    def set_top_view(self):
        _mplot3d.set_top_view(self._native)
        self.stale = True

    def update_datalim(self, xys, **kwargs):
        """
        Not implemented in `~mpl_toolkits.mplot3d.axes3d.Axes3D`.
        """
        pass

    get_zticks = _axis_method_wrapper("zaxis", "get_ticklocs")
    set_zticks = _axis_method_wrapper("zaxis", "set_ticks")
    set_zticklabels = _axis_method_wrapper(
        "zaxis", "set_ticklabels",
        doc_sub={"Axis.set_ticks": "Axes3D.set_zticks"})

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
        if np.isinf(lower) or np.isinf(upper):
            raise ValueError(f"Axis limits {lower}, {upper} cannot be infinite")
        margin = (view_margin if view_margin is not None
                  else self._view_margin if rcParams['axes3d.automargin'] else 0)
        given = _margined(lower, upper, margin)
        if not all(np.isfinite(given)):
            # The limits matplotlib's axis is handed, NaN, or made infinite
            # by a margin that is not positive.
            raise ValueError("Axis limits cannot be NaN or Inf")
        lower, upper = float(lower), float(upper)
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
        if lower == upper:
            _api.warn_external(
                f"Attempting to set identical low and high {name}lims "
                f"makes transformation singular; automatically expanding.")
        _mplot3d.set_lim(self._native, name, (lower, upper),
                         None if view_margin is None else float(view_margin),
                         None if auto is None else bool(auto))
        if auto is not None:
            # matplotlib's axis keeps the flag as the bool of *auto*.
            _values.remember(getattr(self, f"{name}axis"), "autoscale_on", bool(auto))
        return getattr(self, f"{name}axis")._typed_limits(
            given, _mplot3d.get_lim(self._native, name))

    def set_xlim(self, left=None, right=None, *, emit=True, auto=False,
                 view_margin=None, xmin=None, xmax=None):
        """
        Set the 3D x-axis view limits.

        Parameters
        ----------
        left : float, optional
            The left xlim in data coordinates. Passing *None* leaves the
            limit unchanged.

            The left and right xlims may also be passed as the tuple
            (*left*, *right*) as the first positional argument (or as
            the *left* keyword argument).

            .. ACCEPTS: (left: float, right: float)

        right : float, optional
            The right xlim in data coordinates. Passing *None* leaves the
            limit unchanged.

        emit : bool, default: True
            Whether to notify observers of limit change.

        auto : bool or None, default: False
            Whether to turn on autoscaling of the x-axis. *True* turns on,
            *False* turns off, *None* leaves unchanged.

        view_margin : float, optional
            The additional margin to apply to the limits.

        xmin, xmax : float, optional
            They are equivalent to left and right respectively, and it is an
            error to pass both *xmin* and *left* or *xmax* and *right*.

        Returns
        -------
        left, right : (float, float)
            The new x-axis limits in data coordinates.

        See Also
        --------
        get_xlim
        set_xbound, get_xbound
        invert_xaxis, xaxis_inverted

        Notes
        -----
        The *left* value may be greater than the *right* value, in which
        case the x-axis values will decrease from *left* to *right*.

        Examples
        --------
        >>> set_xlim(left, right)
        >>> set_xlim((left, right))
        >>> left, right = set_xlim(left, right)

        One limit may be left unchanged.

        >>> set_xlim(right=right_lim)

        Limits may be passed in reverse order to flip the direction of
        the x-axis. For example, suppose ``x`` represents depth of the
        ocean in m. The x-axis limits might be set like the following
        so 5000 m depth is at the left of the plot and the surface,
        0 m, is at the right.

        >>> set_xlim(5000, 0)
        """
        return self._set_lim3d('x', left, right, emit=emit, auto=auto,
                               view_margin=view_margin, axmin=xmin, axmax=xmax)

    def set_ylim(self, bottom=None, top=None, *, emit=True, auto=False,
                 view_margin=None, ymin=None, ymax=None):
        """
        Set the 3D y-axis view limits.

        Parameters
        ----------
        bottom : float, optional
            The bottom ylim in data coordinates. Passing *None* leaves the
            limit unchanged.

            The bottom and top ylims may also be passed as the tuple
            (*bottom*, *top*) as the first positional argument (or as
            the *bottom* keyword argument).

            .. ACCEPTS: (bottom: float, top: float)

        top : float, optional
            The top ylim in data coordinates. Passing *None* leaves the
            limit unchanged.

        emit : bool, default: True
            Whether to notify observers of limit change.

        auto : bool or None, default: False
            Whether to turn on autoscaling of the y-axis. *True* turns on,
            *False* turns off, *None* leaves unchanged.

        view_margin : float, optional
            The additional margin to apply to the limits.

        ymin, ymax : float, optional
            They are equivalent to bottom and top respectively, and it is an
            error to pass both *ymin* and *bottom* or *ymax* and *top*.

        Returns
        -------
        bottom, top : (float, float)
            The new y-axis limits in data coordinates.

        See Also
        --------
        get_ylim
        set_ybound, get_ybound
        invert_yaxis, yaxis_inverted

        Notes
        -----
        The *bottom* value may be greater than the *top* value, in which
        case the y-axis values will decrease from *bottom* to *top*.

        Examples
        --------
        >>> set_ylim(bottom, top)
        >>> set_ylim((bottom, top))
        >>> bottom, top = set_ylim(bottom, top)

        One limit may be left unchanged.

        >>> set_ylim(top=top_lim)

        Limits may be passed in reverse order to flip the direction of
        the y-axis. For example, suppose ``y`` represents depth of the
        ocean in m. The y-axis limits might be set like the following
        so 5000 m depth is at the bottom of the plot and the surface,
        0 m, is at the top.

        >>> set_ylim(5000, 0)
        """
        return self._set_lim3d('y', bottom, top, emit=emit, auto=auto,
                               view_margin=view_margin, axmin=ymin, axmax=ymax)

    def set_zlim(self, bottom=None, top=None, *, emit=True, auto=False,
                 view_margin=None, zmin=None, zmax=None):
        """
        Set the 3D z-axis view limits.

        Parameters
        ----------
        bottom : float, optional
            The bottom zlim in data coordinates. Passing *None* leaves the
            limit unchanged.

            The bottom and top zlims may also be passed as the tuple
            (*bottom*, *top*) as the first positional argument (or as
            the *bottom* keyword argument).

            .. ACCEPTS: (bottom: float, top: float)

        top : float, optional
            The top zlim in data coordinates. Passing *None* leaves the
            limit unchanged.

        emit : bool, default: True
            Whether to notify observers of limit change.

        auto : bool or None, default: False
            Whether to turn on autoscaling of the z-axis. *True* turns on,
            *False* turns off, *None* leaves unchanged.

        view_margin : float, optional
            The additional margin to apply to the limits.

        zmin, zmax : float, optional
            They are equivalent to bottom and top respectively, and it is an
            error to pass both *zmin* and *bottom* or *zmax* and *top*.

        Returns
        -------
        bottom, top : (float, float)
            The new z-axis limits in data coordinates.

        See Also
        --------
        get_zlim
        set_zbound, get_zbound
        invert_zaxis, zaxis_inverted

        Notes
        -----
        The *bottom* value may be greater than the *top* value, in which
        case the z-axis values will decrease from *bottom* to *top*.

        Examples
        --------
        >>> set_zlim(bottom, top)
        >>> set_zlim((bottom, top))
        >>> bottom, top = set_zlim(bottom, top)

        One limit may be left unchanged.

        >>> set_zlim(top=top_lim)

        Limits may be passed in reverse order to flip the direction of
        the z-axis. For example, suppose ``z`` represents depth of the
        ocean in m. The z-axis limits might be set like the following
        so 5000 m depth is at the bottom of the plot and the surface,
        0 m, is at the top.

        >>> set_zlim(5000, 0)
        """
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
        """
        Return the 3D z-axis view limits.

        Returns
        -------
        left, right : (float, float)
            The current z-axis limits in data coordinates.

        See Also
        --------
        set_zlim
        set_zbound, get_zbound
        invert_zaxis, zaxis_inverted

        Notes
        -----
        The z-axis may be inverted, in which case the *left* value will
        be greater than the *right* value.
        """
        return _values.float64s(_mplot3d.get_lim(self._native, 'z'))

    def get_w_lims(self):
        """Get 3D world limits."""
        minx, maxx = self.get_xlim3d()
        miny, maxy = self.get_ylim3d()
        minz, maxz = self.get_zlim3d()
        return minx, maxx, miny, maxy, minz, maxz

    def _set_bound3d(self, name, lower, upper, view_margin):
        """Set the bounds of axis *name* through its limits, ordered as the
        axis is (matplotlib's ``_set_bound3d``)."""
        if upper is None and np.iterable(lower):
            lower, upper = lower
        old_lower, old_upper = getattr(self, f"get_{name}bound")()
        if lower is None:
            lower = old_lower
        if upper is None:
            upper = old_upper
        inverted = _mplot3d.get_inverted(self._native, name)
        getattr(self, f"set_{name}lim")(sorted((lower, upper), reverse=inverted),
                                        auto=None, view_margin=view_margin)

    def set_xbound(self, lower=None, upper=None, view_margin=None):
        """
        Set the lower and upper numerical bounds of the x-axis.

        This method will honor axis inversion regardless of parameter order.
        It will not change the autoscaling setting (`.get_autoscalex_on()`).

        Parameters
        ----------
        lower, upper : float or None
            The lower and upper bounds. If *None*, the respective axis bound
            is not modified.
        view_margin : float or None
            The margin to apply to the bounds. If *None*, the margin is handled
            by `.set_xlim`.

        See Also
        --------
        get_xbound
        get_xlim, set_xlim
        invert_xaxis, xaxis_inverted
        """
        self._set_bound3d('x', lower, upper, view_margin)

    def set_ybound(self, lower=None, upper=None, view_margin=None):
        """
        Set the lower and upper numerical bounds of the y-axis.

        This method will honor axis inversion regardless of parameter order.
        It will not change the autoscaling setting (`.get_autoscaley_on()`).

        Parameters
        ----------
        lower, upper : float or None
            The lower and upper bounds. If *None*, the respective axis bound
            is not modified.
        view_margin : float or None
            The margin to apply to the bounds. If *None*, the margin is handled
            by `.set_ylim`.

        See Also
        --------
        get_ybound
        get_ylim, set_ylim
        invert_yaxis, yaxis_inverted
        """
        self._set_bound3d('y', lower, upper, view_margin)

    def set_zbound(self, lower=None, upper=None, view_margin=None):
        """
        Set the lower and upper numerical bounds of the z-axis.
        This method will honor axis inversion regardless of parameter order.
        It will not change the autoscaling setting (`.get_autoscaley_on()`).

        Parameters
        ----------
        lower, upper : float or None
            The lower and upper bounds. If *None*, the respective axis bound
            is not modified.
        view_margin : float or None
            The margin to apply to the bounds. If *None*, the margin is handled
            by `.set_zlim`.

        See Also
        --------
        get_zbound
        get_zlim, set_zlim
        invert_zaxis, zaxis_inverted
        """
        self._set_bound3d('z', lower, upper, view_margin)

    def get_xbound(self):
        """Return the lower and upper x-axis bounds, in increasing order."""
        return _values.float64s(_mplot3d.get_bound(self._native, 'x'))

    def get_ybound(self):
        """Return the lower and upper y-axis bounds, in increasing order."""
        return _values.float64s(_mplot3d.get_bound(self._native, 'y'))

    def get_zbound(self):
        """
        Return the lower and upper z-axis bounds, in increasing order.

        See Also
        --------
        set_zbound
        get_zlim, set_zlim
        invert_zaxis, zaxis_inverted
        """
        return _values.float64s(_mplot3d.get_bound(self._native, 'z'))

    def invert_xaxis(self):
        """[*Discouraged*] Invert the x-axis."""
        _mplot3d.set_inverted(self._native, 'x', not self.xaxis_inverted())

    def invert_yaxis(self):
        """[*Discouraged*] Invert the y-axis."""
        _mplot3d.set_inverted(self._native, 'y', not self.yaxis_inverted())

    def invert_zaxis(self):
        """
        [*Discouraged*] Invert the z-axis.

        .. admonition:: Discouraged

            The use of this method is discouraged.
            Use `.Axes3D.set_zinverted` instead.

        See Also
        --------
        get_zinverted
        get_zlim, set_zlim
        get_zbound, set_zbound
        """
        _mplot3d.set_inverted(self._native, 'z', not self.zaxis_inverted())

    def xaxis_inverted(self):
        """Return whether the xaxis is oriented in the "inverse" direction."""
        return self.xaxis.get_inverted()

    def yaxis_inverted(self):
        """Return whether the yaxis is oriented in the "inverse" direction."""
        return self.yaxis.get_inverted()

    zaxis_inverted = _axis_method_wrapper("zaxis", "get_inverted")
    if zaxis_inverted.__doc__:
        zaxis_inverted.__doc__ = ("[*Discouraged*] " + zaxis_inverted.__doc__ +
                                  textwrap.dedent("""

        .. admonition:: Discouraged

            The use of this method is discouraged.
            Use `.Axes3D.get_zinverted` instead.
        """))

    # The frame_on methods are not available for 3D axes.
    get_frame_on = None
    set_frame_on = None

    def apply_aspect(self, position=None):
        # docstring inherited; matplotlib's makes the box square in the
        # original position (no data aspect is applied to a 3-D axes).
        if position is None:
            _mplot3d.position(self._native, False)
            self.stale = True
            return
        super().apply_aspect(position)

    def can_zoom(self):
        # doc-string inherited
        return super().can_zoom()

    def can_pan(self):
        # doc-string inherited
        return super().can_pan()

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
        # Set whether autoscaling is applied to each axis on the next draw.
        for name in self._axis_names:
            _mplot3d.set_autoscale_on(self._native, name, bool(b))

    set_autoscale_on.__doc__ = Axes.set_autoscale_on.__doc__

    get_autoscalez_on = _axis_method_wrapper("zaxis", "_get_autoscale_on")
    set_autoscalez_on = _axis_method_wrapper("zaxis", "_set_autoscale_on")

    def autoscale(self, enable=True, axis='both', tight=None):
        """
        Convenience method for simple axis view autoscaling.

        See `.Axes.autoscale` for full documentation.  Because this function
        applies to 3D Axes, *axis* can also be set to 'z', and setting *axis*
        to 'both' autoscales all three axes.
        """
        # matplotlib takes any other name for no axis, and *enable* None
        # for all three.
        selected = ((True,) * 3 if enable is None
                    else tuple(axis in (name, 'both') for name in self._axis_names))
        _mplot3d.autoscale(self._native,
                           None if enable is None else bool(enable),
                           selected, None if tight is None else bool(tight))

    def _request_autoscale_view(self, axis="all", tight=None):
        """
        Mark a single axis, or all of them, for autoscaling when the axes
        is next drawn or `_unstale_viewLim` runs (matplotlib's 3-D limit
        getters leave it pending).
        """
        axis_names = _api.getitem_checked(
            {**{k: [k] for k in self._axis_names}, "all": self._axis_names},
            axis=axis)
        _mplot3d.request_autoscale(
            self._native, tuple(name in axis_names for name in self._axis_names),
            None if tight is None else bool(tight))

    def _unstale_viewLim(self):
        # docstring inherited; the 3-D limits as well as the plane's.
        _mplot3d.unstale_view_lim(self._native)
        super()._unstale_viewLim()

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
        """
        Retrieve autoscaling margin of the z-axis.

        .. versionadded:: 3.9

        Returns
        -------
        zmargin : float

        See Also
        --------
        mpl_toolkits.mplot3d.axes3d.Axes3D.set_zmargin
        """
        return self._get_margin('z')

    def _set_margin(self, name, m):
        """Set the margin of axis *name*, which autoscales when the axes is
        next drawn (matplotlib's ``set_xmargin``...)."""
        _mplot3d.set_margin(self._native, name, _check_margin(m))
        _values.remember(self, f"{name}margin", m)
        self.stale = True

    def set_xmargin(self, m):
        # Set padding of X data limits prior to autoscaling.
        self._set_margin('x', m)

    set_xmargin.__doc__ = Axes.set_xmargin.__doc__

    def set_ymargin(self, m):
        # Set padding of Y data limits prior to autoscaling.
        self._set_margin('y', m)

    set_ymargin.__doc__ = Axes.set_ymargin.__doc__

    def set_zmargin(self, m):
        """
        Set padding of Z data limits prior to autoscaling.

        *m* times the data interval will be added to each end of that interval
        before it is used in autoscaling.  If *m* is negative, this will clip
        the data range instead of expanding it.

        For example, if your data is in the range [0, 2], a margin of 0.1 will
        result in a range [-0.2, 2.2]; a margin of -0.1 will result in a range
        of [0.2, 1.8].

        Parameters
        ----------
        m : float greater than -0.5
        """
        self._set_margin('z', m)

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
            _mplot3d.set_python_scale(self._native, name, scale)
            scale.set_default_locators_and_formatters(axis)
            _axis.mark_default_helpers(self._native, name)
        else:
            _mplot3d.set_scale(self._native, name, *native)
        axis._given_scale = scale

    def set_xscale(self, value, **kwargs):
        """
        Set the x-axis scale.

        Parameters
        ----------
        value : {"linear", "log", "symlog", "logit", ...}
            The axis scale type to apply. See `~.scale.ScaleBase` for
            the list of available scales.

        **kwargs
            Keyword arguments are forwarded to the scale class.
            For example, ``base=2`` can be passed when using a log scale.
        """
        self._set_axis_scale('x', value, **kwargs)

    def set_yscale(self, value, **kwargs):
        """
        Set the y-axis scale.

        Parameters
        ----------
        value : {"linear", "log", "symlog", "logit", ...}
            The axis scale type to apply. See `~.scale.ScaleBase` for
            the list of available scales.

        **kwargs
            Keyword arguments are forwarded to the scale class.
            For example, ``base=2`` can be passed when using a log scale.
        """
        self._set_axis_scale('y', value, **kwargs)

    def set_zscale(self, value, **kwargs):
        """
        Set the z-axis scale.

        Parameters
        ----------
        value : {"linear", "log", "symlog", "logit", ...}
            The axis scale type to apply. See `~.scale.ScaleBase` for
            the list of available scales.

        **kwargs
            Keyword arguments are forwarded to the scale class.
            For example, ``base=2`` can be passed when using a log scale.
        """
        self._set_axis_scale('z', value, **kwargs)

    get_zscale = _axis_method_wrapper("zaxis", "get_scale")

    # -- the camera and the box --------------------------------------------

    def view_init(self, elev=None, azim=None, roll=None, vertical_axis="z",
                  share=False):
        """
        Set the elevation and azimuth of the Axes in degrees (not radians).

        This can be used to rotate the Axes programmatically.

        To look normal to the primary planes, the following elevation and
        azimuth angles can be used. A roll angle of 0, 90, 180, or 270 deg
        will rotate these views while keeping the axes at right angles.

        ==========   ====  ====
        view plane   elev  azim
        ==========   ====  ====
        XY           90    -90
        XZ           0     -90
        YZ           0     0
        -XY          -90   90
        -XZ          0     90
        -YZ          0     180
        ==========   ====  ====

        Parameters
        ----------
        elev : float, default: None
            The elevation angle in degrees rotates the camera above the plane
            pierced by the vertical axis, with a positive angle corresponding
            to a location above that plane. For example, with the default
            vertical axis of 'z', the elevation defines the angle of the camera
            location above the x-y plane.
            If None, then the initial value as specified in the `Axes3D`
            constructor is used.
        azim : float, default: None
            The azimuthal angle in degrees rotates the camera about the
            vertical axis, with a positive angle corresponding to a
            right-handed rotation. For example, with the default vertical axis
            of 'z', a positive azimuth rotates the camera about the origin from
            its location along the +x axis towards the +y axis.
            If None, then the initial value as specified in the `Axes3D`
            constructor is used.
        roll : float, default: None
            The roll angle in degrees rotates the camera about the viewing
            axis. A positive angle spins the camera clockwise, causing the
            scene to rotate counter-clockwise.
            If None, then the initial value as specified in the `Axes3D`
            constructor is used.
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
        self._turn(view, vertical_axis, share)

    def _turn(self, view, vertical_axis, share):
        """
        Turn the view to the angles *view* (elev, azim, roll), kept as
        given, with *vertical_axis* up, the axes sharing the view too with
        *share*. matplotlib keeps angles its projection fails on: the view
        stays as it was, and the draws of the axes fail as matplotlib's do.
        """
        problem = _draw_problems.view_problem(self, *view)
        _mplot3d.view_init(
            self._native, tuple(map(float, view)) if problem is None
            else _mplot3d.get_view(self._native), vertical_axis, bool(share))
        keep = _values.remember if problem is None else _values.keep
        for axes in (self._shared_axes["view"].get_siblings(self) if share
                     else [self]):
            for name, value in zip(_VIEW_ANGLES, view):
                keep(axes, name, value)
            axes._view_problem = problem
            axes._record_draw_problems()

    def _record_draw_problems(self):
        """Record with pltrs (Rust) the problems matplotlib's draw of the
        axes meets with values it keeps unchecked, before it draws their
        artists: computing the projection of the view (``get_proj``), then
        the z-order of the collections and patches from those of the axes,
        and drawing the axes (`._draw_problems`)."""
        problem = self.__dict__.get("_view_problem")
        if problem is None and self.computed_zorder:
            problem = _draw_problems.zorder_problem(self)
        _mplot3d.set_view_failure(self._native, _failure_of(problem))
        _mplot3d.set_axes_failure(
            self._native, _failure_of(_draw_problems.axes_problem(self)))

    def _view_angle(index):
        """The property of one of the view angles (as given, as matplotlib
        keeps them, until the view turns)."""
        name = _VIEW_ANGLES[index]

        def get(self):
            return _values.given(self, name, _mplot3d.get_view(self._native)[index])

        def set(self, value):
            view = [getattr(self, angle) for angle in _VIEW_ANGLES]
            view[index] = value
            self._turn(view, self._axis_names[self._vertical_axis], False)
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
            The focal length can be computed from a desired Field Of View via
            the equation: focal_length = 1/tan(FOV/2)
        """
        focal_length = self._checked_focal_length(proj_type, focal_length)
        _mplot3d.set_proj_type(self._native, proj_type, focal_length)

    def set_box_aspect(self, aspect, *, zoom=1):
        """
        Set the Axes box aspect.

        The box aspect is the ratio of height to width in display
        units for each face of the box when viewed perpendicular to
        that face.  This is not to be confused with the data aspect (see
        `~.Axes3D.set_aspect`). The default ratios are 4:4:3 (x:y:z).

        To simulate having equal aspect in data space, set the box
        aspect to match your data range in each dimension.

        *zoom* controls the overall size of the Axes3D in the figure.

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
            Possible values:

            =========   ==================================================
            value       description
            =========   ==================================================
            'auto'      automatic; fill the position rectangle with data.
            'equal'     adapt all the axes to have equal aspect ratios.
            'equalxy'   adapt the x and y axes to have equal aspect ratios.
            'equalxz'   adapt the x and z axes to have equal aspect ratios.
            'equalyz'   adapt the y and z axes to have equal aspect ratios.
            =========   ==================================================

        adjustable : {'box', 'datalim'}, default: 'box'
            Defines which parameter to adjust to meet the aspect ratio.

            - 'box': Change the physical dimensions of the axes bounding box.
            - 'datalim': Change the x, y, or z data limits.

        anchor : None or str or 2-tuple of float, optional
            .. deprecated:: 3.11
                This parameter has no effect.

        share : bool, default: False
            .. deprecated:: 3.11
                This parameter has no effect.

        See Also
        --------
        mpl_toolkits.mplot3d.axes3d.Axes3D.set_box_aspect
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

    def set_adjustable(self, adjustable, share=False):
        # Set what keeps the aspect: the box ('box') or the data limits
        # ('datalim'), as `.Axes.set_adjustable` records it; `set_aspect`
        # applies it.
        _api.check_in_list(["box", "datalim"], adjustable=adjustable)
        for ax in self._aspect_group(share):
            _mplot3d.set_adjustable(ax._native, adjustable)
        self.stale = True

    set_adjustable.__doc__ = Axes.set_adjustable.__doc__

    def get_adjustable(self):
        # Return whether the box ('box') or the data limits ('datalim') keep the
        # aspect.
        return _mplot3d.get_aspect(self._native)[1]

    get_adjustable.__doc__ = Axes.get_adjustable.__doc__

    @property
    def _frameon(self):
        # matplotlib's draw turns it off (it draws the panes in place of a
        # frame), until it is set again.
        set_at = self.__dict__.get("_frameon_set_at", 0)
        if _artist.draw_count(self._native) > set_at:
            return False
        return Axes._frameon.fget(self)

    @_frameon.setter
    def _frameon(self, b):
        Axes._frameon.fset(self, b)
        self._frameon_set_at = _artist.draw_count(self._native)

    def get_proj(self):
        """Create the projection matrix from the current viewing position."""
        problem = _draw_problems.view_problem(
            self, *(getattr(self, angle) for angle in _VIEW_ANGLES))
        if problem is not None:
            raise problem
        return _mplot3d.get_proj(self._native)

    def draw(self, renderer):
        # docstring inherited; matplotlib's keeps the view it draws in as
        # that of its last draw (``M``, ``invM``), as pltrs (Rust) keeps it
        # when it draws the figure.
        if self.get_visible():
            self.M = self.get_proj()
            self.invM = np.linalg.inv(self.M)
        super().draw(renderer)

    @property
    def M(self):
        """The projection matrix of the last draw (None before the first)."""
        return _mplot3d.drawn_proj(self._native)

    @M.setter
    def M(self, matrix):
        _mplot3d.set_drawn_proj(
            self._native, None if matrix is None else np.asarray(matrix, float))

    @property
    def invM(self):
        """The inverse of the projection matrix `M` of the last draw."""
        return _mplot3d.drawn_proj_inverse(self._native)

    @invM.setter
    def invM(self, inverse):
        _mplot3d.set_drawn_proj_inverse(
            self._native, None if inverse is None else np.asarray(inverse, float))

    @property
    def computed_zorder(self):
        """Whether collections and patches are drawn in depth order."""
        return self.__dict__.get('_computed_zorder', True)

    @computed_zorder.setter
    def computed_zorder(self, computed):
        self._computed_zorder = bool(computed)
        _mplot3d.set_computed_zorder(self._native, bool(computed))
        self._record_draw_problems()

    def _shared_from(self, name):
        native = _mplot3d.share_target(self._native, name)
        return None if native is None else _handles.wrap(native, type(self), self.figure)

    def _sharing(self, name):
        # docstring inherited: 3-D axes share their z-axis and their view
        # too.
        return _mplot3d.shared(self._native, name)

    @property
    def _sharez(self):
        """The Axes whose z-axis this one shares (``sharez=``), or None."""
        return self._shared_from("z")

    @property
    def _shareview(self):
        """The Axes whose view angles this one shares (``shareview=``), or
        None."""
        return self._shared_from("view")

    def sharez(self, other):
        """
        Share the z-axis with *other*.

        This is equivalent to passing ``sharez=other`` when constructing the
        Axes, and cannot be used if the z-axis is already being shared with
        another Axes.  Note that it is not possible to unshare axes.
        """
        _api.check_isinstance(Axes3D, other=other)
        _mplot3d.share(self._native, 'z', other._native)
        self.zaxis._take_shared_tickers()

    def shareview(self, other):
        """
        Share the view angles with *other*.

        This is equivalent to passing ``shareview=other`` when constructing the
        Axes, and cannot be used if the view angles are already being shared
        with another Axes.  Note that it is not possible to unshare axes.
        """
        _api.check_isinstance(Axes3D, other=other)
        shared = self._shareview
        if shared is not None and other is not shared:
            raise ValueError("view angles are already shared")
        _mplot3d.share(self._native, 'view', other._native)
        self.view_init(elev=other.elev, azim=other.azim, roll=other.roll,
                       vertical_axis=other._axis_names[other._vertical_axis],
                       share=True)

    @_docstring.copy(Axes.set_anchor)
    def set_anchor(self, anchor, share=False):
        # docstring inherited; the square box of the 3-D axes is anchored
        # there too.
        super().set_anchor(anchor, share)
        named = isinstance(anchor, str)
        if named and anchor not in Bbox.coefs:
            # Kept (see `.Axes.set_anchor`), failing where matplotlib's
            # `apply_aspect` squares the box (`_anchor_box_corner`).
            return
        for ax in self._aspect_group(share):
            _mplot3d.set_anchor(ax._native, anchor if named else None,
                                (0.0, 0.0) if named else tuple(map(float, anchor)))

    def _anchor_box_corner(self, anchor):
        # docstring inherited: matplotlib's squares the box of a 3-D axes
        # at its anchor whenever it applies the aspect.
        return Bbox.coefs[anchor]

    def get_position(self, original=False):
        """
        Return the position of the Axes within the figure as a `.Bbox`:
        its box made square in it (``apply_aspect``), or the original
        position.
        """
        return Bbox.from_extents(*_mplot3d.position(self._native, bool(original)))

    @_docstring.copy(Axes.set_position)
    def set_position(self, pos, which='both'):
        # docstring inherited
        self._set_position(pos, which=which)
        # Because this is being called externally to the library we
        # don't let it be in the layout.
        self.set_in_layout(False)

    def _set_position(self, pos, which='both'):
        """
        Set the position of the axes; its active box is its original one
        made square at the next draw (`apply_aspect`), so that setting the
        active one alone changes nothing that stays.
        """
        if not isinstance(pos, Bbox):
            pos = Bbox.from_bounds(*pos)
        _api.check_in_list(['both', 'active', 'original'], which=which)
        if which in ('both', 'original'):
            _mplot3d.set_position(self._native, tuple(map(float, pos.bounds)))
        self.stale = True

    def get_tightbbox(self, renderer=None, *, call_axes_locator=True,
                      bbox_extra_artists=None, for_layout_only=False):
        # docstring inherited
        self._check_figure_renderer(renderer)
        with _measured_by_given(renderer, self):
            extent = _mplot3d.tightbbox(self._native, bool(for_layout_only))
        if self._axis3don:
            # matplotlib ticks each visible axis to measure it, raising
            # what its locator or formatter raises (the measure above
            # warned where it ticks them).
            for axis in (self.xaxis, self.yaxis, self.zaxis):
                if axis.get_visible():
                    _mplot3d.check_ticks(self._native, axis.axis_name)
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
        position = _mplot3d.get_axis_position(self._native)
        if position is None:
            raise art3d._no_projection_error()
        return tuple(np.bool_(high) for high in position)

    def _calc_coord(self, xv, yv, renderer=None):
        """
        Given the 2D view coordinates, find the point on the nearest axis pane
        that lies directly below those coordinates. Returns a 3D point in data
        coordinates.
        """
        found = _mplot3d.calc_coord(self._native, float(xv), float(yv))
        if found is None:
            raise art3d._no_projection_error()
        point, pane_idx = found
        return np.array(point), np.int64(pane_idx)

    def disable_mouse_rotation(self):
        """Disable mouse buttons for 3D rotation, panning, and zooming."""
        self.mouse_init(rotate_btn=[], pan_btn=[], zoom_btn=[])

    def format_coord(self, xv, yv, renderer=None):
        """
        Return a string giving the current view rotation angles, or the x, y, z
        coordinates of the point on the nearest axis pane underneath the mouse
        cursor, depending on the mouse button pressed.
        """
        if self.button_pressed in self._rotate_btn:
            # ignore xv and yv and display angles instead
            return self._rotation_coords()
        if not all(isinstance(v, Real) and np.isfinite(v) for v in (xv, yv)):
            # matplotlib's arithmetic bringing the point back through the
            # view of the last draw, which fails or warns on these.
            zv = 1 if self._focal_length == np.inf else -1 / self._focal_length
            proj3d.inv_transform(xv, yv, zv, self.invM)
        return _mplot3d.format_coord(self._native, float(xv), float(yv))

    def _rotation_coords(self):
        """Return the rotation angles as a string."""
        return _mplot3d.rotation_coords(self._native)

    #: A callable formatting z values for `format_zdata`, or None.
    fmt_zdata = None

    def format_zdata(self, z):
        """
        Return *z* string formatted.  This function will use the
        :attr:`!fmt_zdata` attribute if it is callable, else will fall
        back on the zaxis major formatter
        """
        try:
            return self.fmt_zdata(z)
        except (AttributeError, TypeError):
            return self.zaxis.get_major_formatter().format_data_short(z)

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
        self._axis3don = False

    def set_axis_on(self):
        self._axis3don = True

    #: Whether the 2-D axes are drawn: never for a 3-D axes, whose own
    #: are (`_axis3don`), as matplotlib's `Axes3D.__init__` turns them off.
    axison = False

    @property
    def _axis3don(self):
        """Whether the panes, axes and gridlines are drawn."""
        return _mplot3d.get_axis_on(self._native)

    @_axis3don.setter
    def _axis3don(self, on):
        _mplot3d.set_axis_on(self._native, bool(on))
        self.stale = True

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
        # The gridlines are drawn first with the image filters of the axes.
        self._record_draw_problems()

    def _set_axis_label(self, name, label, fontdict, labelpad, kwargs):
        """Label the axis *name* (3-D axes place their labels themselves, so
        no *loc* applies)."""
        axis = getattr(self, f'{name}axis')
        if labelpad is not None:
            axis.labelpad = labelpad
        return axis.set_label_text(label, fontdict, **kwargs)

    def set_xlabel(self, xlabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        # Set the label for the x-axis.
        return self._set_axis_label('x', xlabel, fontdict, labelpad, kwargs)

    set_xlabel.__doc__ = Axes.set_xlabel.__doc__

    def set_ylabel(self, ylabel, fontdict=None, labelpad=None, *, loc=None,
                   **kwargs):
        # Set the label for the y-axis.
        return self._set_axis_label('y', ylabel, fontdict, labelpad, kwargs)

    set_ylabel.__doc__ = Axes.set_ylabel.__doc__

    def set_zlabel(self, zlabel, fontdict=None, labelpad=None, **kwargs):
        """
        Set zlabel.  See doc for `.set_ylabel` for description.
        """
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
        """
        Get the z-label text string.
        """
        return _mplot3d.get_label(self._native, 'z')

    def set_title(self, label, fontdict=None, loc='center', **kwargs):
        # docstring inherited
        # Centred whatever axes.titlelocation says, as matplotlib's is, and
        # lowered to 0.92 of its height until a draw places it.
        ret = super().set_title(label, fontdict=fontdict, loc=loc, **kwargs)
        (x, y) = self.title.get_position()
        self.title.set_y(0.92 * y)
        return ret

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
        if (not self.button_pressed or self.get_navigate_mode() is not None
                or self.M is None):
            return
        x, y = event.xdata, event.ydata
        # In case the mouse is out of bounds.
        if x is None or event.inaxes != self:
            return
        if self.button_pressed in self._rotate_btn:
            if x == self._sx and y == self._sy:
                return
            snapped = getattr(event, "key", None) == "control"
            _mplot3d.drag_rotate(self._native, (float(self._sx), float(self._sy)),
                                 (float(x), float(y)), snapped)
            self._remember_rotated_view(
                snapped and rcParams["axes3d.snap_rotation"] > 0)
        elif self.button_pressed in self._pan_btn:
            px, py = self.transData.transform([self._sx, self._sy])
            self.start_pan(px, py, 2)
            self.drag_pan(2, None, event.x, event.y)
            self.end_pan()
        elif self.button_pressed in self._zoom_btn:
            _mplot3d.drag_zoom(self._native, float(self._sy), float(y))
        self._sx, self._sy = x, y
        self.get_figure(root=True).canvas.draw_idle()

    def _remember_rotated_view(self, snapped):
        """
        Keep the view angles a rotation by the mouse gave as matplotlib
        types them: numpy floats as computed (the azimuth-elevation style
        keeps the roll as it was), Python floats once *snapped* to steps.
        """
        view = _mplot3d.get_view(self._native)
        if snapped:
            angles = dict(zip(_VIEW_ANGLES, map(float, view)))
        elif rcParams['axes3d.mouserotationstyle'] == 'azel':
            angles = dict(zip(_VIEW_ANGLES[:2], map(np.float64, view[:2])))
        else:
            angles = dict(zip(_VIEW_ANGLES, map(np.float64, view)))
        _values.remember_all(self, angles, _VIEW_ANGLES)

    def _zoom_data_limits(self, scale_u, scale_v, scale_w):
        """
        Zoom in or out of a 3D plot by the factors *scale_u*, *scale_v*
        and *scale_w* along the screen's horizontal, vertical and depth
        directions (more than 1 zooms out), the axes of an equal aspect
        keeping it.
        """
        _mplot3d.zoom_data_limits(self._native, float(scale_u), float(scale_v),
                                  float(scale_w))

    #: The distance of the camera from the centre of the box.
    _dist = 10

    @property
    def _focal_length(self):
        """The focal length of the camera (infinite: orthographic), the int
        ``1`` matplotlib's perspective camera takes by default."""
        length = _mplot3d.get_focal_length(self._native)
        return 1 if length == 1 else length

    @property
    def _draw_grid(self):
        """Whether the gridlines are drawn on the panes."""
        return _mplot3d.get_grid(self._native)

    @_draw_grid.setter
    def _draw_grid(self, visible):
        _mplot3d.grid(self._native, bool(visible))

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
        - `.PatchCollection` (currently not supporting *autolim*)

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

            .. versionadded:: 3.10
        """
        zvals = np.atleast_1d(zs)
        if zvals.size:
            # matplotlib orders the collection by the lowest position.
            np.min(zvals)
        if not isinstance(col, art3d._Loose3D):
            if not isinstance(col, mcoll.LineCollection):
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
        segments = col.__dict__.get('_segments3d')
        if autolim and isinstance(col, art3d.Line3DCollection) and segments is not None:
            # matplotlib gathers the coordinates of the points of the
            # segments to autoscale, which fails without any.
            points = np.concatenate(segments)
            points[:, 0], points[:, 1], points[:, 2]
        return col._adopt_into(self, autolim, axlim_clip)

    def add_collection(self, collection, autolim=True):
        # docstring inherited
        if isinstance(collection, art3d._Loose3D):
            return collection._adopt_into(self, bool(autolim), False)
        return self._without_projection(super().add_collection(collection, autolim))

    def add_patch(self, p):
        # docstring inherited
        return self._without_projection(super().add_patch(p))

    def add_artist(self, a):
        # docstring inherited
        return self._without_projection(super().add_artist(a))

    def _without_projection(self, artist):
        """Record that the draws of the axes fail on *artist* when it is a
        2-D collection or patch, without the 3-D projection matplotlib's
        draw asks the collections and patches for, and have them call the
        projection of one whose class gives it; return it."""
        if isinstance(artist, (mcoll.Collection, mpatches.Patch)):
            if not hasattr(artist, 'do_3d_projection'):
                _draw_problems.record(artist, _draw_problems.missing_projection(artist))
            elif takes_from_program(type(artist), 'do_3d_projection'):
                _program_projection.follow(self, artist)
        return artist

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

            .. versionadded:: 3.10
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
        # 2D plot's autoscaling flags (pyplot's `plot` passes them), which
        # matplotlib's 3D autoscaling of every axis after it overrides.
        kwargs.pop('scalex', None)
        kwargs.pop('scaley', None)
        kwargs = cbook.normalize_kwargs(kwargs, mlines.Line2D)
        xs, ys, zs = cbook._broadcast_with_masks(xs, ys, zs)
        # matplotlib's 2-D `plot` takes the units of the data first.
        self.xaxis.update_units(xs)
        self.yaxis.update_units(ys)
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
        # matplotlib places the lines along *zdir* (``juggle_axes``), which
        # fails on a direction of no string.
        art3d.juggle_axes(xs, ys, zs, zdir)
        lines = []
        for x, y, z, fmt, label in datasets:
            props = {k: v for k, v in kwargs.items() if v is not None}
            if label is not None:
                props["label"] = label
            native = _mplot3d.plot(self._native, _floats(x, y, z), zdir, fmt,
                                   mlines._native_kwargs(props),
                                   bool(axlim_clip))
            line = keep_python_kwargs(wrap(self, native, art3d.Line3D), props)
            # The 2-D data as matplotlib's `plot` gives them to the line,
            # until a draw projects it.
            line._xorig, line._yorig = np.atleast_1d(x), np.atleast_1d(y)
            # The 3-D data as matplotlib's `set_3d_properties` keeps them:
            # the 2-D data as given, and the z values as floats.
            z = np.broadcast_to(cbook._to_unmasked_float_array(z).ravel(), len(x))
            verts = art3d.juggle_axes(x, y, z, zdir)
            _values.remember(line, "data_3d", verts)
            _draw_problems.record(line, _draw_problems.line_problem(line, verts))
            lines.append(line)
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
            The z-positions. Either an array of the same length as *xs* and
            *ys* or a single value to place all points in the same plane.
        zdir : {'x', 'y', 'z', '-x', '-y', '-z'}, default: 'z'
            The axis direction for the *zs*. This is useful when plotting 2D
            data on a 3D Axes. The data must be passed as *xs*, *ys*. Setting
            *zdir* to 'y' then plots the data to the x-z-plane.

            See also :doc:`/gallery/mplot3d/2dcollections3d`.

        s : float or array-like, default: 20
            The marker size in points**2. Either an array of the same length
            as *xs* and *ys* or a single value to make all markers the same
            size.
        c : :mpltype:`color`, sequence, or sequence of colors, optional
            The marker color. Possible values:

            - A single color format string.
            - A sequence of colors of length n.
            - A sequence of n numbers to be mapped to colors using *cmap* and
              *norm*.
            - A 2D array in which the rows are RGB or RGBA.

            For more details see the *c* argument of `~.axes.Axes.scatter`.
        depthshade : bool, default: :rc:`axes3d.depthshade`
            Whether to shade the scatter markers to give the appearance of
            depth. Each call to ``scatter()`` will perform its depthshading
            independently.

        depthshade_minalpha : float, default: :rc:`axes3d.depthshade_minalpha`
            The lowest alpha value applied by depth-shading.

            .. versionadded:: 3.11

        axlim_clip : bool, default: False
            Whether to hide the scatter points outside the axes view limits.

            .. versionadded:: 3.10

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            All other keyword arguments are passed on to `~.axes.Axes.scatter`.

        Returns
        -------
        paths : `~matplotlib.collections.PathCollection`
        """
        if args:
            # matplotlib hands them to 2D `scatter` after *s*, given as a
            # keyword.
            raise TypeError("Axes.scatter() got multiple values for argument 's'")
        xs, ys, zs = cbook._broadcast_with_masks(xs, ys, zs)
        # matplotlib's 2-D `scatter` takes the units of the data and
        # converts them through them.
        for axis, data in ((self.xaxis, xs), (self.yaxis, ys)):
            if not axis.have_units():
                axis.update_units(data)
            axis.convert_units(data)
        s = np.ma.ravel(s)
        xs, ys, zs, s, c, color = cbook.delete_masked_points(
            xs, ys, zs, s, c, kwargs.get('color', None))
        # As the 2D `scatter` matplotlib's hands the sizes to checks them.
        if not (np.issubdtype(s.dtype, np.floating)
                or np.issubdtype(s.dtype, np.integer)):
            raise ValueError(f"s must be float, but has type {s.dtype}")
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
        # The sizes and the 3-D offsets as matplotlib's collection keeps
        # them: the sizes and the z values as given, x and y in the masked
        # offsets of 2-D `scatter`.
        given_sizes = np.asarray(s)
        given_xs, given_ys = np.ma.column_stack(
            [np.asanyarray(xs, float), np.asanyarray(ys, float)]).T
        given_offsets = art3d.juggle_axes(given_xs, given_ys, np.atleast_1d(zs), zdir)
        s = np.broadcast_to(np.ma.filled(np.asarray(s, dtype=float), np.nan),
                            xs.shape if s.size != 1 else (1,))
        _properties.check_known(mcoll.PathCollection, kwargs)
        native = _mplot3d.scatter(
            self._native, _floats(xs, ys, zs),
            (np.ascontiguousarray(s, dtype=float).ravel(),
             mlines._marker_spec(marker_obj)), faces, (edgecolors, linewidths),
            (None if alpha is None else float(alpha), alphas),
            (zdir, None if depthshade is None else bool(depthshade),
             None if depthshade_minalpha is None
             else float(depthshade_minalpha), bool(axlim_clip)))
        collection = wrap(self, native, art3d.Path3DCollection)
        _values.remember(collection, "sizes", given_sizes)
        _values.remember(collection, "_offsets3d", given_offsets)
        _draw_problems.record(
            collection, _draw_problems.offsets_problem(collection, given_offsets))
        # matplotlib's scatter gives its marker path as a tuple.
        collection._paths_container = tuple
        if made is not None:
            collection._attach_native_mapping(colorizer=made)
        if "rasterized" in kwargs:
            # matplotlib's 2-D `scatter` applies it to a `PathCollection`,
            # before the collection becomes 3-D: no warning that a
            # `Path3DCollection` ignores it.
            _artist.set_rasterized(collection._native, bool(kwargs.pop("rasterized")))
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

        By default, it will be colored in shades of a solid color, but it also
        supports colormapping by supplying the *cmap* argument.

        .. note::

           The *rcount* and *ccount* kwargs, which both default to 50,
           determine the maximum number of samples used in each direction.  If
           the input data is larger, it will be downsampled (by slicing) to
           these numbers of points.

        .. note::

           To maximize rendering speed consider setting *rstride* and *cstride*
           to divisors of the number of rows minus 1 and columns minus 1
           respectively. For example, given 51 rows rstride can be any of the
           divisors of 50.

           Similarly, a setting of *rstride* and *cstride* equal to 1 (or
           *rcount* and *ccount* equal the number of rows and columns) can use
           the optimized path.

        Parameters
        ----------
        X, Y, Z : 2D arrays
            Data values.

        rcount, ccount : int
            Maximum number of samples used in each direction.  If the input
            data is larger, it will be downsampled (by slicing) to these
            numbers of points.  Defaults to 50.

        rstride, cstride : int
            Downsampling stride in each direction.  These arguments are
            mutually exclusive with *rcount* and *ccount*.  If only one of
            *rstride* or *cstride* is set, the other defaults to 10.

            'classic' mode uses a default of ``rstride = cstride = 10`` instead
            of the new default of ``rcount = ccount = 50``.

        color : :mpltype:`color`
            Color of the surface patches.

        cmap : Colormap, optional
            Colormap of the surface patches.

        facecolors : list of :mpltype:`color`
            Colors of each individual patch.

        norm : `~matplotlib.colors.Normalize`, optional
            Normalization for the colormap.

        vmin, vmax : float, optional
            Bounds for the normalization.

        shade : bool, default: True
            Whether to shade the facecolors.  Shading is always disabled when
            *cmap* is specified.

        lightsource : `~matplotlib.colors.LightSource`, optional
            The lightsource to use when *shade* is True.

        axlim_clip : bool, default: False
            Whether to hide patches with a vertex outside the axes view limits.

            .. versionadded:: 3.10

        **kwargs
            Other keyword arguments are forwarded to `.Poly3DCollection`.
        """
        if Z.ndim != 2:
            raise ValueError("Argument Z must be 2-dimensional.")
        Z = cbook._to_unmasked_float_array(Z)
        X, Y, Z = np.broadcast_arrays(X, Y, Z)
        # matplotlib's check of the polygons' vertices, which refuses what
        # is not numbers ("ufunc 'isfinite' not supported for the input
        # types").
        np.isfinite(np.stack((X, Y, Z)))
        sampling = _sampling(kwargs.pop('rstride', None),
                             kwargs.pop('cstride', None),
                             kwargs.pop('rcount', None),
                             kwargs.pop('ccount', None), 10)
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
        _properties.check_known(art3d.Poly3DCollection, kwargs)
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

        .. note::

           The *rcount* and *ccount* kwargs, which both default to 50,
           determine the maximum number of samples used in each direction.  If
           the input data is larger, it will be downsampled (by slicing) to
           these numbers of points.

        Parameters
        ----------
        X, Y, Z : 2D arrays
            Data values.

        axlim_clip : bool, default: False
            Whether to hide lines and patches with vertices outside the axes
            view limits.

            .. versionadded:: 3.10

        rcount, ccount : int
            Maximum number of samples used in each direction.  If the input
            data is larger, it will be downsampled (by slicing) to these
            numbers of points.  Setting a count to zero causes the data to be
            not sampled in the corresponding direction, producing a 3D line
            plot rather than a wireframe plot.  Defaults to 50.

        rstride, cstride : int
            Downsampling stride in each direction.  These arguments are
            mutually exclusive with *rcount* and *ccount*.  If only one of
            *rstride* or *cstride* is set, the other defaults to 1.  Setting a
            stride to zero causes the data to be not sampled in the
            corresponding direction, producing a 3D line plot rather than a
            wireframe plot.

            'classic' mode uses a default of ``rstride = cstride = 1`` instead
            of the new default of ``rcount = ccount = 50``.

        **kwargs
            Other keyword arguments are forwarded to `.Line3DCollection`.
        """
        if Z.ndim != 2:
            raise ValueError("Argument Z must be 2-dimensional.")
        X, Y, Z = np.broadcast_arrays(X, Y, Z)
        rstride, cstride = kwargs.pop('rstride', None), kwargs.pop('cstride', None)
        rcount, ccount = kwargs.pop('rcount', None), kwargs.pop('ccount', None)
        if (rstride == 0 and cstride == 0) or (rcount == 0 and ccount == 0):
            raise ValueError("Either rstride or cstride must be non zero")
        sampling = _sampling(rstride, cstride, rcount, ccount, 1)
        if sampling is None and rcParams['_internal.classic_mode']:
            sampling = ("stride", 1, 1)
        kwargs = cbook.normalize_kwargs(kwargs, mcoll.Collection)
        color = kwargs.pop('color', None)
        grids = tuple(np.ascontiguousarray(np.ma.filled(
            np.ma.asarray(a, dtype=float), np.nan)) for a in (X, Y, Z))
        _properties.check_known(art3d.Line3DCollection, kwargs)
        native = _mplot3d.plot_wireframe(self._native, grids, sampling, color,
                                         bool(axlim_clip))
        collection = wrap(self, native, art3d.Line3DCollection)
        if any(a.dtype.kind not in 'biuf' for a in (X, Y, Z)):
            # Values of no number, on which matplotlib's draws fail: its
            # lines through them, of the rows and the columns.
            lines = ([list(zip(*row)) for row in zip(X, Y, Z)]
                     + [list(zip(*col)) for col in zip(X.T, Y.T, Z.T)])
            _draw_problems.record(
                collection, _draw_problems.segments_problem(collection, lines))
        collection._internal_update(kwargs)
        return collection

    def text(self, x, y, z, s, zdir=None, *, axlim_clip=False, **kwargs):
        """
        Add the text *s* to the 3D Axes at location *x*, *y*, *z* in data coordinates.

        Parameters
        ----------
        x, y, z : float
            The position to place the text.
        s : str
            The text.
        zdir : {'x', 'y', 'z', 3-tuple}, optional
            The direction to be used as the z-direction. Default: 'z'.
            See `.get_dir_vector` for a description of the values.
        axlim_clip : bool, default: False
            Whether to hide text that is outside the axes view limits.

            .. versionadded:: 3.10
        **kwargs
            Other arguments are forwarded to `matplotlib.axes.Axes.text`.

        Returns
        -------
        `.Text3D`
            The created `.Text3D` instance.
        """
        direction = tuple(map(float, art3d.get_dir_vector(zdir)))
        native = _mplot3d.text(self._native, art3d._text_floats((x, y, z)),
                               '' if s is None else str(s), direction,
                               bool(axlim_clip))
        # A 2-D text given its properties, then made 3-D (with the position
        # as given), as matplotlib's is.
        text = wrap(self, native, mtext.Text)
        art3d._keep_text_position(text, (x, y, z))
        _init_added(text, kwargs)
        art3d.text_2d_to_3d(text, z, zdir, axlim_clip)
        return text

    text3D = text
    text2D = Axes.text

    @_preprocess_data()
    def bar3d(self, x, y, z, dx, dy, dz, color=None,
              zsort='average', shade=True, lightsource=None, *args,
              axlim_clip=False, **kwargs):
        """
        Generate a 3D barplot.

        This method creates three-dimensional barplot where the width,
        depth, height, and color of the bars can all be uniquely set.

        Parameters
        ----------
        x, y, z : array-like
            The coordinates of the anchor point of the bars.

        dx, dy, dz : float or array-like
            The width, depth, and height of the bars, respectively.

        color : sequence of colors, optional
            The color of the bars can be specified globally or
            individually. This parameter can be:

            - A single color, to color all bars the same color.
            - An array of colors of length N bars, to color each bar
              independently.
            - An array of colors of length 6, to color the faces of the
              bars similarly.
            - An array of colors of length 6 * N bars, to color each face
              independently.

            When coloring the faces of the boxes specifically, this is
            the order of the coloring:

            1. -Z (bottom of box)
            2. +Z (top of box)
            3. -Y
            4. +Y
            5. -X
            6. +X

        zsort : {'average', 'min', 'max'}, default: 'average'
            The z-axis sorting scheme passed onto `~.art3d.Poly3DCollection`

        shade : bool, default: True
            When true, this shades the dark sides of the bars (relative
            to the plot's source of light).

        lightsource : `~matplotlib.colors.LightSource`, optional
            The lightsource to use when *shade* is True.

        axlim_clip : bool, default: False
            Whether to hide the bars with points outside the axes view limits.

            .. versionadded:: 3.10

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            Any additional keyword arguments are passed onto
            `~.art3d.Poly3DCollection`.

        Returns
        -------
        collection : `~.art3d.Poly3DCollection`
            A collection of three-dimensional polygons representing the bars.
        """
        # matplotlib broadcasts the bars and takes their extent first,
        # failing as numpy does on bars of none or of no numbers.
        x, y, z, dx, dy, dz = np.broadcast_arrays(np.atleast_1d(x), y, z, dx, dy, dz)
        for p, dp in ((x, dx), (y, dy), (z, dz)):
            np.min(p), np.max(p + dp)
        finite = all(v.dtype.kind in "biuf" and np.isfinite(v).all()
                     for v in (x, y, z, dx, dy, dz))
        polys = None if finite else _cuboids(x, y, z, dx, dy, dz)
        colors = None if color is None else mcolors.to_rgba_array(color)
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
        if zsort not in ('average', 'min', 'max'):
            # matplotlib's `.Poly3DCollection` looks the sorting up.
            raise KeyError(zsort)
        columns = tuple(np.atleast_1d(np.asarray(v, dtype=float)).ravel().tolist()
                        for v in (x, y, z, dx, dy, dz))
        _properties.check_known(art3d.Poly3DCollection, kwargs)
        if polys is not None and shade:
            # ... and the normals of the faces it shades.
            art3d._generate_normals(polys)
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

        The arguments can be array-like or scalars, so long as they can be
        broadcast together. The arguments can also be masked arrays. If an
        element in any of argument is masked, then that corresponding quiver
        element will not be plotted.

        Parameters
        ----------
        X, Y, Z : array-like
            The x, y and z coordinates of the arrow locations (default is
            tail of arrow; see *pivot* kwarg).

        U, V, W : array-like
            The x, y and z components of the arrow vectors.

        length : float, default: 1
            The length of each quiver.

        arrow_length_ratio : float, default: 0.3
            The ratio of the arrow head with respect to the quiver.

        pivot : {'tail', 'middle', 'tip'}, default: 'tail'
            The part of the arrow that is at the grid point; the arrow
            rotates about this point, hence the name *pivot*.

        normalize : bool, default: False
            Whether all arrows are normalized to have the same length, or keep
            the lengths defined by *u*, *v*, and *w*.

        axlim_clip : bool, default: False
            Whether to hide arrows with points outside the axes view limits.

            .. versionadded:: 3.10

        data : indexable object, optional
            DATA_PARAMETER_PLACEHOLDER

        **kwargs
            Any additional keyword arguments are delegated to
            :class:`.Line3DCollection`
        """
        if not _check_quiver(
                cbook._broadcast_with_masks(X, Y, Z, U, V, W, compress=True),
                length, arrow_length_ratio, pivot):
            length, arrow_length_ratio, pivot = 1, .3, 'tail'
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
        _properties.check_known(art3d.Line3DCollection, kwargs)
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

        The (optional) triangulation can be specified in one of two ways;
        either::

          plot_trisurf(triangulation, ...)

        where triangulation is a `~matplotlib.tri.Triangulation` object, or::

          plot_trisurf(X, Y, ...)
          plot_trisurf(X, Y, triangles, ...)
          plot_trisurf(X, Y, triangles=triangles, ...)

        in which case a Triangulation object will be created.  See
        `.Triangulation` for an explanation of these possibilities.

        The remaining arguments are::

          plot_trisurf(..., Z)

        where *Z* is the array of values to contour, one per point
        in the triangulation.

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
            Whether to shade the facecolors.  Shading is always disabled when
            *cmap* is specified.
        lightsource : `~matplotlib.colors.LightSource`, optional
            The lightsource to use when *shade* is True.
        axlim_clip : bool, default: False
            Whether to hide patches with a vertex outside the axes view limits.

            .. versionadded:: 3.10
        **kwargs
            All other keyword arguments are passed on to
            :class:`~mpl_toolkits.mplot3d.art3d.Poly3DCollection`

        Examples
        --------
        .. plot:: gallery/mplot3d/trisurf3d.py
        .. plot:: gallery/mplot3d/trisurf3d_2.py
        """
        from pltrs.tri._triangulation import Triangulation
        if color is not None:
            # Read first, as matplotlib reads it (and refuses what is no
            # colour with its message).
            color = mcolors.to_rgba(color)
        tri, args, kwargs = Triangulation.get_from_args_and_kwargs(*args, **kwargs)
        try:
            z = kwargs.pop('Z')
        except KeyError:
            # We do this so Z doesn't get passed as an arg to PolyCollection
            z, *args = args
        cmap = kwargs.pop('cmap', None)
        shade = kwargs.pop('shade', None)
        made, mapping = _surface_mapping(cmap, norm, vmin, vmax)
        _properties.check_known(art3d.Poly3DCollection, kwargs)
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


cbook._own_mixin_methods(Axes3D, Axes)


def get_test_data(delta=0.05):
    """Return a tuple X, Y, Z with a test data set."""
    # The grid of matplotlib's computation, whose `arange` refuses what is
    # no step (and takes None for a step of one).
    x = np.arange(-3.0, 3.0, delta)
    if isinstance(delta, (np.floating, float)) and not (
            isinstance(delta, (float, np.float64)) and np.isfinite(delta)):
        # Steps of another precision (numpy's `arange` steps in theirs) or
        # infinite ones (a grid of the start alone): matplotlib's numpy.
        return _test_data_on(x)
    return _mplot3d.test_data(1.0 if delta is None else float(delta))


def _test_data_on(x):
    """matplotlib's test data on the grid of *x* along both axes."""
    X, Y = np.meshgrid(x, x)
    Z1 = np.exp(-(X**2 + Y**2) / 2) / (2 * np.pi)
    Z2 = (np.exp(-(((X - 1) / 1.5)**2 + ((Y - 1) / 0.5)**2) / 2) /
          (2 * np.pi * 0.5 * 1.5))
    Z = Z2 - Z1
    X = X * 10
    Y = Y * 10
    Z = Z * 500
    return X, Y, Z


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


# What matplotlib's `Axes3D` inherits, which pltrs' class defines to reach
# the 3-D axes pltrs (Rust) draws.
cbook._inherited_methods(Axes3D, Axes, ["set_subplotspec"])


# What matplotlib's module also has from its imports, which programs
# import from it.
import itertools  # noqa: E402, F401
import math  # noqa: E402, F401
import pltrs as mpl  # noqa: E402, F401
from collections import defaultdict  # noqa: E402, F401
from pltrs import (  # noqa: E402, F401
    artist as martist, container as mcontainer, image as mimage,
    transforms as mtransforms)
from pltrs.tri._triangulation import Triangulation  # noqa: E402, F401
