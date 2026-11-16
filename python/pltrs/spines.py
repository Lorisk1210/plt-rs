r"""
The lines framing an Axes (matplotlib's ``spines``): each `Spine` is a
pltrs (Rust) spine of its Axes, and `Spines` is matplotlib's container of
them (``ax.spines``), kept from matplotlib 3.11.2 under the Matplotlib
License with `SpinesProxy`.
"""

from collections.abc import MutableMapping
import contextlib
import functools
import inspect

import numpy as np

from . import _docstring
from . import (_api, _axis_artists, _handles, _python_drawing, _unusable, _values,
               rcParams)
from .colors import to_rgba
from ._native import artist as _artist, axis as _axis, spines as _spines
from ._program_classes import takes_from_program
from .artist import Artist, allow_rasterization
from .cbook import _str_equal
from .lines import _get_dash_pattern
from .patches import Patch, _kept_linestyle
from .path import Path
from .transforms import Affine2D, Bbox, IdentityTransform

#: matplotlib's paths of the linear spines (``Spine.linear_spine``), in
#: data along their axis and axes fractions across it, until a draw or a
#: measure spans them over the view.
_LINEAR_PATHS = {
    "left": [(0.0, 0.999), (0.0, 0.999)],
    "right": [(1.0, 0.999), (1.0, 0.999)],
    "bottom": [(0.999, 0.0), (0.999, 0.0)],
    "top": [(0.999, 1.0), (0.999, 1.0)],
}


#: Whether the spines' transforms are read rather than asked for
#: (`_transforms_read`).
_reading = False


@contextlib.contextmanager
def _transforms_read():
    """
    Read the transforms of spines as they are: within the block a spine's
    transform does not set its position, so that nothing is placed and no
    ticks are made anew, as when matplotlib reads the transform a tick was
    made with.
    """
    global _reading
    previous, _reading = _reading, True
    try:
        yield
    finally:
        _reading = previous


def _position_pair(position):
    """The ``(kind, amount)`` pair of the spine position *position*
    (``'center'`` and ``'zero'`` spelled out), as pltrs (Rust) keeps it."""
    kind, amount = {'center': ('axes', 0.5),
                    'zero': ('data', 0.0)}.get(position, position)
    return kind, float(amount)


def _check_position(position):
    """Refuse a spine position that is not ``'center'``, ``'zero'`` or a
    ``(kind, amount)`` pair, as matplotlib's `.Spine.set_position` does."""
    if position in ('center', 'zero'):  # special positions
        return
    if len(position) != 2:
        raise ValueError("position should be 'center' or 2-tuple")
    if position[0] not in ['outward', 'axes', 'data']:
        raise ValueError("position[0] should be one of 'outward', "
                         "'axes', or 'data' ")


#: The hooks of a linear spine matplotlib's draws and measures call, which
#: a class of the program's may override: pltrs then lays the spine out
#: through them (`Spine._push_layout`).
_LAYOUT_HOOKS = ("_adjust_location", "get_patch_transform", "get_spine_transform",
                 "_ensure_position_is_set", "get_path", "get_transform", "get_window_extent")


class Spine(Patch):
    """
    An axis spine -- the line noting the data area boundaries.

    Spines are the lines connecting the axis tick marks and noting the
    boundaries of the data area. They can be placed at arbitrary
    positions. See `~.Spine.set_position` for more information.

    The default position is ``('outward', 0)``.

    Spines are subclasses of `.Patch`, and inherit much of their behavior.

    Spines draw a line, a circle, or an arc depending on if
    `~.Spine.set_patch_line`, `~.Spine.set_patch_circle`, or
    `~.Spine.set_patch_arc` has been called. Line-like is the default.

    For examples see :ref:`spines_examples`.
    """

    # The spine's outline is its ``_path``, which pltrs (Rust) draws as a
    # line, an arc or a circle (`_push_path` for the others), not a
    # patch's outline handed over by `.Patch._push_computed`.
    _computed = ()
    _path_hooks = ()

    def __str__(self):
        return "Spine"

    def __new__(cls, *args, **kwargs):
        # A new spine of *axes* (see `__init__`, which refuses arguments it
        # does not take); pickling makes a bare object, and so does a class
        # of the program's naming its type in its own constructor
        # (cartopy's ``GeoSpine(axes)``), which `__init__` binds.
        spine = super().__new__(cls)
        given = {**dict(zip(("axes", "spine_type", "path"), args)), **kwargs}
        axes, spine_type = given.get("axes"), given.get("spine_type")
        if len(args) <= 3 and axes is not None and spine_type is not None:
            spine._bind(axes, spine_type)
        return spine

    @_docstring.interpd
    def __init__(self, axes, spine_type, path, **kwargs):
        """
        Parameters
        ----------
        axes : `~matplotlib.axes.Axes`
            The `~.axes.Axes` instance containing the spine.
        spine_type : str
            The spine type.
        path : `~matplotlib.path.Path`
            The `.Path` instance used to draw the spine.

        Other Parameters
        ----------------
        **kwargs
            Valid keyword arguments are:

            %(Patch:kwdoc)s
        """
        # pltrs: the spine is a new one of *axes* on the side *spine_type*
        # names, else on the side of the one framing its outline (a polar
        # Axes' ``'polar'`` spine), drawn as *path* through its transform
        # (``transData`` unless it is given another) once ``axes.spines``
        # has it (see `_replace_drawn`).
        _api.check_isinstance(Path, path=path)
        if "_native" not in self.__dict__:
            self._bind(axes, spine_type)
        self._kept_path = path
        self.set_facecolor('none')
        self.set_edgecolor(rcParams['axes.edgecolor'])
        self.set_linewidth(rcParams['axes.linewidth'])
        self.set_capstyle('projecting')
        self.set_zorder(2.5)
        if spine_type not in _LINEAR_PATHS:
            self._push_path()
            self.set_transform(axes.transData)
        self._internal_update(kwargs)
        # A class of the program's drawing the spine itself draws it.
        _python_drawing.attach(self)

    def _bind(self, axes, spine_type):
        """Make the bare spine (`__new__`) the object of a new native spine
        of *axes* on the side *spine_type* names, else on that of the
        spine framing its outline."""
        # The figure first, as matplotlib's constructor asks *axes* for it.
        figure = axes.get_figure(root=False)
        natives = dict(_spines.axes_spines(axes._native))
        framing = natives.get(spine_type, natives.get("top", natives.get("polar")))
        _handles.adopt(self, _spines.new_spine(axes._native, _spines.side(framing)),
                       figure)
        # Known without asking pltrs (Rust), which the projection of a
        # program's class cannot while it draws.
        self.__dict__.update(_spine_axes=axes, _spine_type=spine_type, _made=True)

    def _push_path(self):
        """Have pltrs (Rust) draw the spine of a type naming no side as its
        ``_path``, which a class of the program's sets anew before it draws
        (cartopy's ``GeoSpine`` clips its boundary to the view), as
        matplotlib's draw draws ``_path``."""
        vertices = self._kept_path.vertices
        if (self.__dict__.get("_pushed_path") is not self._kept_path
                or not np.array_equal(self.__dict__.get("_pushed_vertices"), vertices)):
            _spines.set_patch_path(self._native, self._kept_path)
            self._pushed_path = self._kept_path
            # (A class may move the vertices of the path it keeps.)
            self._pushed_vertices = np.array(vertices)

    # ``Spine(...)`` takes what the constructor takes: the defaults of
    # ``__new__`` only let pickling make a bare spine.
    __new__.__signature__ = inspect.signature(__init__)

    @classmethod
    def linear_spine(cls, axes, spine_type, **kwargs):
        """Create and return a linear `Spine`."""
        if spine_type not in _LINEAR_PATHS:
            raise ValueError(f'unable to make path for spine "{spine_type}"')
        result = cls(axes, spine_type, Path(_LINEAR_PATHS[spine_type]), **kwargs)
        result.set_visible(rcParams[f'axes.spines.{spine_type}'])
        return result

    @classmethod
    def _of_axes(cls, axes, spine_type):
        """The spine of *axes* named *spine_type* (a side, or a spine of
        polar axes), else the spine framing its outline, as an object of
        *cls*."""
        natives = dict(_spines.axes_spines(axes._native))
        native = natives.get(spine_type, natives.get("top", natives.get("polar")))
        spine = _handles.wrap(native, cls, axes.figure)
        # Known without asking pltrs (Rust), which the projection of a
        # program's class cannot while it draws.
        spine.__dict__.update(_spine_axes=axes, _spine_type=spine_type)
        return spine

    @classmethod
    def arc_spine(cls, axes, spine_type, center, radius, theta1, theta2,
                  **kwargs):
        """Create and return an arc `Spine`."""
        path = Path.arc(theta1, theta2)
        result = cls(axes, spine_type, path, **kwargs)
        result.set_patch_arc(center, radius, theta1, theta2)
        return result

    @classmethod
    def circular_spine(cls, axes, center, radius, **kwargs):
        """Create and return a circular `Spine`."""
        path = Path.unit_circle()
        spine_type = 'circle'
        result = cls(axes, spine_type, path, **kwargs)
        result.set_patch_circle(center, radius)
        return result

    def set_patch_arc(self, center, radius, theta1, theta2):
        """Set the spine to be arc-like."""
        size = radius * 2
        # The arc's path, which the spine keeps as matplotlib's does.
        path = Path.arc(theta1, theta2)
        self._set_shape(center, size, lambda center, radius: _spines.set_patch_arc(
            self._native, center, radius, (float(theta1), float(theta2))))
        self._kept_path = path
        self._watch_location()

    def set_patch_circle(self, center, radius):
        """Set the spine to be circular."""
        self._set_shape(center, radius * 2, lambda center, radius: _spines.set_patch_circle(
            self._native, center, radius))
        self._watch_location()

    def set_patch_line(self):
        """Set the spine to be linear."""
        _spines.set_patch_line(self._native)
        self._watch_location()
        self.stale = True

    def _set_shape(self, center, size, shape):
        """
        Have *shape(center, radius)* draw the spine as an arc or a circle of
        centre *center* and width and height *size*, which the spine keeps
        as given, as matplotlib's does (``_center``, ``_width``): a centre
        or a size pltrs (Rust) cannot hold leaves pltrs (Rust) a circle of
        nothing, and the spine fails as matplotlib's computes with them
        (`_locate`).
        """
        self._shape = (center, size)
        try:
            center = (float(center[0]), float(center[1]))
        except (TypeError, ValueError, LookupError):
            center = (0.0, 0.0)
        try:
            radius = float(size) / 2
        except (TypeError, ValueError):
            radius = 0.0
        shape(center, radius)
        self.stale = True

    def _misshapen(self):
        """
        Whether the spine cannot be laid out, as matplotlib's cannot: a
        spine of a side drawn as a line or a circle whose path is no line
        (the arc's path `set_patch_arc` left, which `set_patch_line` and
        `set_patch_circle` keep).
        """
        return (self._patch_type != 'arc' and self.spine_type in _LINEAR_PATHS
                and "_kept_path" in self.__dict__
                and self._kept_path.vertices.shape != (2, 2))

    def _unlocatable(self):
        """Whether laying the spine out fails as matplotlib's does
        (`_locate`): it is misshapen, its bounds or its arc's or circle's
        centre and size are no numbers, or the axis registered is no
        axis."""
        if self._misshapen() or self.__dict__.get("_unheld_bounds"):
            return True
        axis = self.__dict__.get("_axis")
        if axis is not None and not callable(getattr(axis, "get_visible", None)):
            return True
        if self._patch_type == "line" or "_shape" not in self.__dict__:
            return False
        center, size = self._shape
        if self._patch_type == "arc" and self.spine_type == "bottom":
            # Its draw sizes it to the radial view.
            size = 1.0
        try:
            numbers = (float(center[0]), float(center[1]), float(size))
        except (TypeError, ValueError, LookupError):
            return True
        # A shape of no finite numbers has no extents (`Path.get_extents`).
        return not np.isfinite(numbers).all()

    def _locate(self, _=None):
        """
        Lay the spine out as matplotlib's draw and measure do, from what it
        keeps (its ``_adjust_location``, then for an arc or a circle its
        ``_recompute_transform``, then the visibility of its axis), raising
        as they do (`_unlocatable`).
        """
        bounds = self.get_bounds()
        if self._patch_type == "arc" and self.spine_type in ("bottom", "top"):
            if bounds is not None:
                direction = getattr(self.axes, "get_theta_direction", lambda: 1)()
                offset = getattr(self.axes, "get_theta_offset", lambda: 0)()
                low, high = (end * direction + offset for end in bounds)
                if low > high:
                    low, high = high, low
                Path.arc(np.rad2deg(low), np.rad2deg(high))
        elif self._patch_type != "arc" and self.spine_type in _LINEAR_PATHS:
            assert not self._misshapen(), 'unexpected vertices shape'
            if bounds is not None:
                # matplotlib writes them into the path's vertices.
                vertices = np.empty(2)
                vertices[0], vertices[1] = bounds
        if self._patch_type in ("arc", "circle") and "_shape" in self.__dict__:
            center, size = self._shape
            if self._patch_type == "arc" and self.spine_type == "bottom":
                size = 0.0
            center = (self.convert_xunits(center[0]), self.convert_yunits(center[1]))
            width = self.convert_xunits(size)
            height = self.convert_yunits(size)
            transform = Affine2D().scale(width * 0.5, height * 0.5).translate(*center)
            # Measured: the extents of its path through that transform.
            self._kept_path.get_extents(transform)
        if self.axis is not None:
            self.axis.get_visible()

    def _decouple(self, removed):
        # docstring inherited; pltrs keeps the spine its Axes, which draws
        # it, visible or not: the Axes fails to draw where matplotlib's
        # linear spine without its Axes (an Axes listing it took it out)
        # fails to span its path over the view limits of the Axes.
        axes = self.axes or self.__dict__.get("_owner_axes")
        super()._decouple(removed)
        self._owner_axes = axes
        if axes is not None and self._is_linear and self.get_bounds() is None:
            _unusable.keep(axes, self._without_axes, self,
                           lambda spine: spine.axes.viewLim, measured=False)

    def _recouple(self):
        # docstring inherited
        super()._recouple()
        axes = self.__dict__.pop("_owner_axes", None)
        if axes is not None:
            _unusable.drop(axes, self._without_axes)

    @property
    def _without_axes(self):
        """The name of the problem of the spine's Axes while the spine has
        none (`_decouple`)."""
        return f"{self.spine_type} spine without its Axes"

    def _watch_location(self):
        """Have the spine fail to be drawn and measured as matplotlib's
        while laying it out fails (`_locate`), and its Axes to be drawn,
        whose axes measure the spines beside their labels."""
        if self._unlocatable():
            _unusable.keep(self, "location", None, self._locate)
        else:
            _unusable.drop(self, "location")
        axes = self.axes
        spines = getattr(axes, "spines", {})
        if any(isinstance(spine, Spine) and spine._unlocatable()
               for spine in spines.values()):
            _unusable.keep(axes, "spine locations", axes, _locate_label_spines)
        else:
            _unusable.drop(axes, "spine locations")

    def _adjust_before_draws(self):
        """Have each draw of the spine, one whose class adjusts its location
        itself, run that ``_adjust_location`` first and draw the path it
        leaves, as matplotlib's `draw` does (mpltern's spines of no side
        span the limits of their ternary axis so; a linear spine spans
        what the path it leaves spans along its axis, `_push_span`)."""
        self._adjusts_itself = True
        if self._is_linear and self.__dict__.get("_made") and not self.__dict__.get("_placed"):
            # As matplotlib's new spine, placed where first asked
            # (`_ensure_position_is_set`, which a class may override).
            self._position_set = False
        # A linear spine is measured through its class too (its
        # `get_window_extent` adjusts it first, as matplotlib's).
        _python_drawing.draw_through_python(self, measures=self._is_linear)

    def _push_layout(self):
        """Have pltrs (Rust) lay the linear spine out as its class does,
        as matplotlib's draw and measure: its ``_adjust_location``, then
        its path through its transform (`get_patch_transform` and the
        transform its position gave it, `get_spine_transform`)."""
        self._ensure_position_is_set()
        self._adjust_location()
        path = self.get_transform().transform_path(self.get_path())
        _spines.set_program_path(self._native, path)

    @allow_rasterization
    def draw(self, renderer):
        # docstring inherited
        if self.__dict__.get("_adjusts_itself"):
            if self._is_linear:
                self._push_layout()
            else:
                self._adjust_location()
        if "_pushed_path" in self.__dict__:
            self._push_path()
        return super().draw(renderer)

    @property
    def _patch_type(self):
        """The shape of the spine: 'line', 'arc' or 'circle'."""
        return _spines.patch_type(self._native)

    #: Whether the position of the spine is set, as it is once the Axes is
    #: made: matplotlib's spine leaves it unset (``_position`` None) from a
    #: clear of its Axes until it is asked for (`_ensure_position_is_set`).
    _position_set = True

    @property
    def _position(self):
        # matplotlib's field: the position as given (or set by a class
        # before it places the spine), None while unset.
        if not self._position_set:
            return None
        given = self.__dict__.get("_given_position")
        return self._given_or_placed() if given is None else given

    @_position.setter
    def _position(self, position):
        # A class of the program's sets it before it places the spine
        # (`set_position`), as in its `_ensure_position_is_set`.
        self._position_set = position is not None
        self._given_position = position

    def clear(self):
        """Clear the current spine."""
        _spines.set_position(self._native, ("outward", 0.0))
        self._clear()
        if self.axis is not None:
            self.axis.clear()

    def _clear(self):
        """Forget the position given, as the clear of the Axes leaves the
        spine (pltrs (Rust) places it outward again)."""
        self.__dict__.pop("_given_position", None)
        self._position_set = False

    def _ensure_position_is_set(self):
        """Set the position the spine has until it is given one, where
        matplotlib first asks for it: ('outward', 0), where pltrs (Rust)
        placed it, which makes the ticks of its axis anew as setting a
        position does (see `set_position`). A spine the program made and
        never placed is placed there only now: until then it is drawn
        with the transform it was made with, ``transData``."""
        if _reading:
            return
        if not self._position_set and takes_from_program(type(self), "get_spine_transform"):
            # matplotlib's places it, taking the transform its class gives.
            self.set_position(("outward", 0.0))
            return
        self._placed = True
        if _spines.unplaced(self._native):
            _spines.set_position(self._native, ("outward", 0.0))
        if not self._position_set:
            self._position_set = True
            if self.axis is not None:
                self.axis.reset_ticks()

    def get_spine_transform(self):
        """Return the spine transform."""
        self._ensure_position_is_set()
        from ._live_transforms import from_native
        native = _spines.spine_transform(self._native)
        if native is None:
            raise ValueError(f'unknown spine spine_type: {self.spine_type!r}')
        return from_native(self._figure, native)

    @property
    def axes(self):
        """The Axes the spine frames."""
        axes = self.__dict__.get("_spine_axes")
        return Artist.axes.fget(self) if axes is None else axes

    @axes.setter
    def axes(self, new_axes):
        Artist.axes.fset(self, new_axes)

    @property
    def spine_type(self):
        """The side the spine frames: 'left', 'right', 'bottom' or 'top'."""
        side = self.__dict__.get("_spine_type")
        return _spines.side(self._native) if side is None else side

    @spine_type.setter
    def spine_type(self, spine_type):
        # matplotlib's field, which its later placements read (the side
        # pltrs (Rust) draws stays, as matplotlib's next draw keeps it).
        self.__dict__["_spine_type"] = spine_type

    def _adjust_location(self):
        """
        Have the spine's path (``_path``: two points in data coordinates
        along its axis and axes fractions across it) span its axis' view
        interval, or its bounds when set, as matplotlib's linear spines do;
        what the projection of a program's class asks of a spine class
        overriding it, and what such a class' draws and measures run in a
        plain Axes.
        """
        vertices = self._kept_path.vertices
        # (A projection of the program's lays the spine out while pltrs
        # (Rust) holds the figure, having taken the bounds first.)
        bounds = self.get_bounds() if self.__dict__.get("_adjusts_itself") else None
        if self.spine_type in ('left', 'right'):
            vertices[:, 1] = self.axes.viewLim.intervaly if bounds is None else bounds
        else:
            vertices[:, 0] = self.axes.viewLim.intervalx if bounds is None else bounds

    def _span_path(self):
        """Have the spine's path span its bounds, else its axis' view
        interval, as matplotlib's linear spines do when they are drawn or
        measured (``_adjust_location``)."""
        bounds = self.get_bounds()
        if bounds is None:
            self._adjust_location()
        else:
            along = 1 if self.spine_type in ('left', 'right') else 0
            self._kept_path.vertices[:, along] = bounds

    @property
    def _is_linear(self):
        """Whether the spine is a line along a side of its Axes, drawn
        through its transform as matplotlib's linear spines are (not an
        arc, a circle or a spine of polar or geographic Axes, which pltrs
        (Rust) lays out in display space)."""
        return self._patch_type == 'line' and self.spine_type in _LINEAR_PATHS

    @property
    def _path(self):
        # matplotlib's field: the path the spine was made with or given, a
        # linear one spanning its view as its last draw or measure left it
        # (as it stands, while pltrs (Rust) lays the figure out and runs a
        # projection's code reading it).
        try:
            linear = self._is_linear
        except RuntimeError:
            return self.__dict__.get("_kept_path")
        return self._linear_path() if linear else self.__dict__.get("_kept_path")

    @_path.setter
    def _path(self, path):
        # A path the program gives (cartopy's ``GeoSpine``, a class's
        # ``_adjust_location``) stays as given until the next draw spans it;
        # one given while the figure is laid out, from the next read on.
        self._kept_path = path
        try:
            linear, draws = self._is_linear, self._draws()
        except RuntimeError:
            self._path_draws = None
            return
        if linear:
            self._path_draws = draws

    def _draws(self):
        """The draws of the spine and of its Axes so far."""
        return (_artist.draw_count(self._native),
                0 if self.axes is None else _artist.draw_count(self.axes._native))

    def _linear_path(self):
        """The spine's ``_path``, matplotlib's linear one first, which is
        spanned over the view again when the spine or its Axes was drawn
        since (drawing an axis measures the spines its label is placed
        beside, animated ones too)."""
        if "_kept_path" not in self.__dict__:
            self._kept_path = Path(np.array(_LINEAR_PATHS[self.spine_type]))
        draws = self._draws()
        if self.__dict__.get("_path_draws", (0, 0)) is None:
            # Given while the figure was laid out: as it stands now.
            self._path_draws = draws
        elif draws != self.__dict__.get("_path_draws", (0, 0)) and not self._misshapen():
            self._path_draws = draws
            self._span_path()
        return self._kept_path

    def set_position(self, position):
        """
        Set the position of the spine.

        Spine position is specified by a 2 tuple of (position type,
        amount). The position types are:

        * 'outward': place the spine out from the data area by the specified
          number of points. (Negative values place the spine inwards.)
        * 'axes': place the spine at the specified Axes coordinate (0 to 1).
        * 'data': place the spine at the specified data coordinate.

        Additionally, shorthand notations define a special positions:

        * 'center' -> ``('axes', 0.5)``
        * 'zero' -> ``('data', 0.0)``

        Examples
        --------
        :doc:`/gallery/spines/spine_placement_demo`
        """
        _check_position(position)
        self._placed = True
        _spines.set_position(self._native, _position_pair(position))
        # Read back as given, as matplotlib keeps it.
        self._given_position = position
        # The spine takes the transform of its position again.
        self.__dict__.pop("_given_transform", None)
        self._position_set = True
        if takes_from_program(type(self), "get_spine_transform"):
            # As matplotlib's, the one its class gives.
            self.set_transform(self.get_spine_transform())
        if self.axis is not None:
            # The ticks of its axis are made anew, as matplotlib's.
            self.axis.reset_ticks()
        self.stale = True

    def get_position(self):
        """Return the spine position."""
        self._ensure_position_is_set()
        return self._given_or_placed()

    def _given_or_placed(self):
        """The position the spine is placed at, as given if it was."""
        position = _spines.get_position(self._native)
        given = self.__dict__.get("_given_position")
        return given if given is not None and _position_pair(given) == position else position

    def set_bounds(self, low=None, high=None):
        """
        Set the spine bounds.

        Parameters
        ----------
        low : float or None, optional
            The lower spine bound. Passing *None* leaves the limit unchanged.

            The bounds may also be passed as the tuple (*low*, *high*) as the
            first positional argument.

            .. ACCEPTS: (low: float, high: float)

        high : float or None, optional
            The higher spine bound. Passing *None* leaves the limit unchanged.
        """
        if self.spine_type == 'circle':
            raise ValueError(
                'set_bounds() method incompatible with circular spines')
        if high is None and np.iterable(low):
            low, high = low
        old_low, old_high = self._bounds_or_view()
        low = old_low if low is None else low
        high = old_high if high is None else high
        try:
            ends = (float(low), float(high))
        except (TypeError, ValueError):
            # matplotlib keeps them, and fails to lay the spine out with them
            # (`_locate`); pltrs (Rust) keeps the bounds it had.
            ends = None
        else:
            _spines.set_bound_ends(self._native, *ends)
        # Read back as given, an end left unchanged as it was (the view's
        # as a numpy float), as matplotlib keeps them.
        self._given_bounds = (low, high)
        self._unheld_bounds = ends is None
        self._watch_location()
        self.stale = True

    def _bounds_or_view(self):
        """The bounds of the spine, else the view interval along it."""
        bounds = self.get_bounds()
        if bounds is not None:
            return bounds
        if self.spine_type in ('left', 'right'):
            return tuple(self.axes.viewLim.intervaly)
        if self.spine_type in ('top', 'bottom'):
            return tuple(self.axes.viewLim.intervalx)
        raise ValueError(f'spine_type: {self.spine_type} not supported')

    def get_bounds(self):
        """Get the bounds of the spine."""
        bounds = _spines.get_bounds(self._native)
        given = self.__dict__.get("_given_bounds")
        if self.__dict__.get("_unheld_bounds"):
            return given
        if given is not None and bounds is not None and _values._same(given, bounds):
            # (A NaN bound held as given too.)
            return given
        return bounds

    @property
    def axis(self):
        """The `.Axis` the spine belongs to (see `register_axis`)."""
        if "_axis" in self.__dict__:
            return self._axis
        name = {"left": "y", "right": "y", "bottom": "x", "top": "x",
                "polar": "y"}.get(self.spine_type)
        return None if name is None else getattr(self.axes, f"{name}axis")

    @axis.setter
    def axis(self, axis):
        # Registered as matplotlib's `register_axis` assigns it (pltrs's
        # own, whatever a subclass overrides).
        Spine.register_axis(self, axis)

    def register_axis(self, axis):
        """
        Register an axis.

        An axis should be registered with its corresponding spine from
        the Axes instance. This allows the spine to clear any axis
        properties when needed.
        """
        self._axis = axis
        _spines.set_unregistered(self._native, axis is None)
        # matplotlib measures the spine with its axis' ticks while the axis
        # is visible, which an object that is no axis fails (`_locate`).
        self._watch_location()

    def get_window_extent(self, renderer=None):
        """
        Return the window extent of the spines in display space, including
        padding for ticks (but not their labels)

        See Also
        --------
        matplotlib.axes.Axes.get_tightbbox
        matplotlib.axes.Axes.get_window_extent
        """
        _unusable.check(self, measuring=True)
        if self._is_linear and self.__dict__.get("_adjusts_itself"):
            # As matplotlib's, laid out by the class' own hooks.
            self._push_layout()
        elif self._is_linear:
            # Measuring spans the path over the view, as drawing does.
            self._linear_path()
            self._span_path()
        elif "_pushed_path" in self.__dict__:
            self._push_path()
        assigned = self.__dict__.get("_spine_type")
        retyped = (self._is_linear and assigned is not None
                   and assigned != _spines.side(self._native))
        if (self.axis is None or retyped) and self._is_linear:
            # Without an axis, the line alone: no ticks stand out of it.
            drawn = _spines.display_path(self._native)
            if drawn is None:
                return Bbox.null()
            vertices = np.asarray(drawn[0])
            line = Bbox([vertices.min(axis=0), vertices.max(axis=0)])
            return line if self.axis is None else self._padded_as_type(line, assigned)
        extent = _spines.window_extent(self._native)
        if extent is None:
            return Bbox.null()
        x0, y0, x1, y1 = extent
        return Bbox([[x0, y0], [x1, y1]])

    def _padded_as_type(self, bb, spine_type):
        """The box *bb* of the spine's line padded by its axis' drawn ticks
        on the sides *spine_type* names, which a program assigned unlike
        the side the spine is drawn on (matplotlib's ``get_window_extent``
        pads by ``spine_type``)."""
        # matplotlib 3.11.2's, under the Matplotlib License.
        if not self.axis.get_visible():
            return bb
        bboxes = [bb]
        drawn_ticks = self.axis._update_ticks()
        major_tick = next(iter({*drawn_ticks} & {*self.axis.majorTicks}), None)
        minor_tick = next(iter({*drawn_ticks} & {*self.axis.minorTicks}), None)
        for tick in [major_tick, minor_tick]:
            if tick is None:
                continue
            bb0 = bb.frozen()
            tickl = tick._size
            tickdir = tick._tickdir
            if tickdir == 'out':
                padout, padin = 1, 0
            elif tickdir == 'in':
                padout, padin = 0, 1
            else:
                padout = padin = 0.5
            dpi = self.get_figure(root=True).dpi
            padout = padout * tickl / 72 * dpi
            padin = padin * tickl / 72 * dpi
            if tick.tick1line.get_visible():
                if spine_type == 'left':
                    bb0.x0, bb0.x1 = bb0.x0 - padout, bb0.x1 + padin
                elif spine_type == 'bottom':
                    bb0.y0, bb0.y1 = bb0.y0 - padout, bb0.y1 + padin
            if tick.tick2line.get_visible():
                if spine_type == 'right':
                    bb0.x1, bb0.x0 = bb0.x1 + padout, bb0.x0 - padin
                elif spine_type == 'top':
                    bb0.y1, bb0.y0 = bb0.y1 + padout, bb0.y0 - padout
            bboxes.append(bb0)
        return Bbox.union(bboxes)

    def get_tightbbox(self, renderer=None):
        """Like `get_window_extent`: a spine is not clipped."""
        return self.get_window_extent(renderer)

    def get_patch_transform(self):
        # docstring inherited
        return super().get_patch_transform()

    def _keeps_given_path(self):
        """Whether the spine is drawn as the path given it, through its
        transform: a spine of a type naming no side, drawn as a line
        (cartopy's ``GeoSpine``, which sets its ``_path`` anew before it
        draws), whose path is the one given, as matplotlib's is."""
        return "_pushed_path" in self.__dict__ and self._patch_type == "line"

    def get_path(self):
        # Return the path of the spine: that of a linear spine in data along
        # its axis and axes fractions across it (spanning the view where it
        # was last drawn or measured), the one given a spine drawn as it,
        # the others in display coordinates, where pltrs (Rust) lays them
        # out.
        if self._is_linear:
            return self._linear_path()
        if self._keeps_given_path():
            return self._kept_path
        vertices, codes = _spines.display_path(self._native) or ([[0, 0]], None)
        # One path, its vertices moved as the spine is laid out again, as
        # matplotlib moves those of its ``_path``.
        path = self.__dict__.get("_path_kept")
        if path is None:
            path = self._path_kept = Path(vertices, codes)
        elif not (np.array_equal(path.vertices, vertices)
                  and np.array_equal(path.codes, codes)):
            moved = Path(vertices, codes)
            path.vertices, path.codes = moved.vertices, moved.codes
        return path

    def get_transform(self):
        """Return the transform of the spine's path: the identity for the
        spines whose `get_path` is in display coordinates."""
        if self._is_linear or self._keeps_given_path():
            if ("_given_transform" not in self.__dict__ and self.axes is not None
                    and _spines.unplaced(self._native)):
                # matplotlib's new spine keeps the transform it was made
                # with until something places it.
                return self.axes.transData
            return super().get_transform()
        return IdentityTransform()

    def set_color(self, c):
        """
        Set the edgecolor.

        Parameters
        ----------
        c : :mpltype:`color`

        Notes
        -----
        This method does not modify the facecolor (which defaults to "none"),
        unlike the `.Patch.set_color` method defined in the parent class.  Use
        `.Patch.set_facecolor` to set the facecolor.
        """
        if c is not None:
            # matplotlib's patch converts the colour at once.
            to_rgba(c)
        _spines.set_color(self._native, self._edge_or_default(c))
        _values.remember(self, "edgecolor", c)

    def _edge_or_default(self, color):
        """*color*, else the edge colour matplotlib's patches take for
        None: :rc:`patch.edgecolor` for a patch of no face (a spine's),
        else none."""
        if color is not None:
            return color
        face = self.__dict__.get("_given_values", {}).get("facecolor")
        if rcParams['patch.force_edgecolor'] or _str_equal(face, 'none'):
            return rcParams['patch.edgecolor']
        return 'none'

    def get_edgecolor(self):
        """Return the edge color."""
        # ``to_rgba`` of the colour given (:rc:`axes.edgecolor`, which
        # matplotlib's spine is made with, for none), as matplotlib keeps
        # it, while pltrs (Rust) holds that colour.
        native = _spines.get_edgecolor(self._native)
        given = self.__dict__.get("_given_values", {}).get("edgecolor")
        if given is None:
            given = rcParams['axes.edgecolor']
        rgba = to_rgba(given, self.get_alpha())
        return rgba if rgba == tuple(native) else native

    def set_linewidth(self, w):
        # pltrs (Rust) holds the spine's width in points.
        if w is None:
            w = rcParams['patch.linewidth']
        _spines.set_linewidth(self._native, float(w))

    set_linewidth.__doc__ = Patch.set_linewidth.__doc__

    def set_linestyle(self, ls):
        # pltrs (Rust) holds the spine's line style, kept as matplotlib's
        # patches keep it before they make its dash pattern.
        self._linestyle = _kept_linestyle(ls)
        _get_dash_pattern(self._linestyle)
        _spines.set_linestyle(self._native, self._linestyle)

    set_linestyle.__doc__ = Patch.set_linestyle.__doc__


#: The spines matplotlib's ticks read by name as they are made
#: (``get_xaxis_transform('tick1')``, ...), in the order they read them.
_TICK_SPINES = ("bottom", "top", "left", "right")


def _replace_drawn(old, spine):
    """Have the Axes of *old*, a spine it draws, draw *spine*, another
    spine of it, in its place, or beside the spines it draws when there is
    no *old* (a spine of a new name), as matplotlib's Axes draws the
    spines of its ``spines``."""
    if not isinstance(spine, Spine) or spine is old or spine.axes is None:
        return
    if old is None:
        _spines.add_spine(spine.axes._native, spine._native)
    elif isinstance(old, Spine) and spine.axes is old.axes:
        _keep_tick_spine(old)
        _spines.replace_spine(old.axes._native, old._native, spine._native)


def _keep_tick_spine(old):
    """Have the ticks made on the spine *old* of a side, which the program
    replaces, keep its transform, as matplotlib's ticks keep the one their
    spine gave them until they are made anew (``reset_ticks``)."""
    side = old.spine_type
    axis = {"bottom": "xaxis", "top": "xaxis", "left": "yaxis", "right": "yaxis"}.get(side)
    axis = getattr(old.axes, axis, None) if axis else None
    if axis is not None and any(axis.__dict__.get("_ticks_seen", {}).values()):
        high = side in ("top", "right")
        _axis.keep_tick_spine(*axis._args(), high)
        _axis_artists.keep_tick_side(axis, high, old.get_spine_transform())


def _locate_label_spines(axes):
    """Lay out the spines beside the labels of the visible axes of *axes*
    as matplotlib's axes measure them as they place their labels
    (``_update_label_position``), failing as they do (`Spine._locate`)."""
    for axis in (axes.xaxis, axes.yaxis):
        spine = axes.spines.get(axis.get_label_position())
        if axis.get_visible() and isinstance(spine, Spine):
            spine._locate()


def _read_tick_spines(axes):
    """Read the spines of *axes* as matplotlib's new ticks read them: by
    name, which fails for one deleted from ``axes.spines``."""
    for name in _TICK_SPINES:
        getattr(axes.spines, name)


def _watch_tick_spines(spines, spine):
    """
    Have the Axes of *spine*, if *spines* is that Axes' container, fail to
    draw as matplotlib's does while a spine its ticks read is missing from
    it (see `._unusable`): its ticks, made as it first draws, read them by
    name. An Axes drawn before already has its ticks.
    """
    axes = getattr(spine, "axes", None)
    if axes is None or axes.__dict__.get("_spines_kept") is not spines:
        return
    if (not set(_TICK_SPINES) <= set(spines)
            and set(_TICK_SPINES) <= set(dict(_spines.axes_spines(axes._native)))
            and not _artist.draw_count(axes._native)):
        _unusable.keep(axes, "spines", axes, _read_tick_spines)
    else:
        _unusable.drop(axes, "spines")


def _in_program_projection(spine):
    """Whether *spine* frames Axes of a projection of the program's."""
    from .projections import _program
    axes = spine.__dict__.get("_spine_axes")
    return axes is not None and _program.is_program_class(type(axes))


class SpinesProxy:
    """
    A proxy to broadcast ``set_*()`` and ``set()`` method calls to contained `.Spines`.

    The proxy cannot be used for any other operations on its members.

    The supported methods are determined dynamically based on the contained
    spines. If not all spines support a given method, it's executed only on
    the subset of spines that support it.
    """
    def __init__(self, spine_dict):
        self._spine_dict = spine_dict

    def __getattr__(self, name):
        broadcast_targets = [spine for spine in self._spine_dict.values()
                             if hasattr(spine, name)]
        if (name != 'set' and not name.startswith('set_')) or not broadcast_targets:
            raise AttributeError(
                f"'SpinesProxy' object has no attribute '{name}'")

        def x(_targets, _funcname, *args, **kwargs):
            for spine in _targets:
                getattr(spine, _funcname)(*args, **kwargs)
        x = functools.partial(x, broadcast_targets, name)
        x.__doc__ = broadcast_targets[0].__doc__
        return x

    def __dir__(self):
        names = []
        for spine in self._spine_dict.values():
            names.extend(name
                         for name in dir(spine) if name.startswith('set_'))
        return list(sorted(set(names)))


class Spines(MutableMapping):
    r"""
    The container of all `.Spine`\s in an Axes.

    The interface is dict-like mapping names (e.g. 'left') to `.Spine` objects.
    Additionally, it implements some pandas.Series-like features like accessing
    elements by attribute::

        spines['top'].set_visible(False)
        spines.top.set_visible(False)

    Multiple spines can be addressed simultaneously by passing a list::

        spines[['top', 'right']].set_visible(False)

    Use an open slice to address all spines::

        spines[:].set_visible(False)

    The latter two indexing methods will return a `SpinesProxy` that broadcasts all
    ``set_*()`` and ``set()`` calls to its members, but cannot be used for any other
    operation.
    """
    def __init__(self, **kwargs):
        self._dict = kwargs

    @classmethod
    def from_dict(cls, d):
        return cls(**d)

    def __getstate__(self):
        return self._dict

    def __setstate__(self, state):
        self.__init__(**state)

    def __getattr__(self, name):
        try:
            return self._dict[name]
        except KeyError:
            raise AttributeError(
                f"'Spines' object does not contain a '{name}' spine")

    def __getitem__(self, key):
        if isinstance(key, list):
            unknown_keys = [k for k in key if k not in self._dict]
            if unknown_keys:
                raise KeyError(', '.join(unknown_keys))
            return SpinesProxy({k: v for k, v in self._dict.items()
                                if k in key})
        if isinstance(key, tuple):
            raise ValueError('Multiple spines must be passed as a single list')
        if isinstance(key, slice):
            if key.start is None and key.stop is None and key.step is None:
                return SpinesProxy(self._dict)
            else:
                raise ValueError(
                    'Spines does not support slicing except for the fully '
                    'open slice [:] to access all spines.')
        return self._dict[key]

    def __setitem__(self, key, value):
        if isinstance(value, Spine) and value.__dict__.get("_made"):
            # A spine the program made has no axis until one is registered
            # (`Spine.register_axis`), as matplotlib's new spine has none,
            # and the transform it was made with, ``transData``, until
            # something places it.
            if value.__dict__.setdefault("_axis", None) is None:
                # It measures as its line alone (`get_window_extent`).
                _spines.set_unregistered(value._native, True)
            if not value.__dict__.get("_placed"):
                _spines.leave_unplaced(value._native)
        _replace_drawn(self._dict.get(key), value)
        if (isinstance(value, Spine) and value.__dict__.get("_made")
                and not value.__dict__.get("_placed")
                and takes_from_program(type(value), "_ensure_position_is_set")):
            # Placed only if its class' `_ensure_position_is_set`, which
            # matplotlib's axes call as they make their ticks, places it
            # (`set_position`): a class that only assigns `_position`
            # leaves it drawn in data coordinates, as matplotlib's.
            _spines.leave_unplaced(value._native)
        if (isinstance(value, Spine) and value._is_linear
                and any(takes_from_program(type(value), hook) for hook in _LAYOUT_HOOKS)
                and not _in_program_projection(value)):
            # Laid out by its class' own hooks when drawn and measured, as
            # matplotlib's Axes draws and measures it (a projection of the
            # program's asks them itself, `.projections._program`).
            value._adjust_before_draws()
        if not isinstance(value, Artist):
            # Among the children of the Axes, on which its draws fail
            # (`.axes._foreign_children.check_drawable`).
            from ._computed_transforms import follow
            axes = next((spine.axes for spine in self._dict.values()
                         if isinstance(spine, Spine) and spine.axes is not None), None)
            if axes is not None:
                follow(axes._figure)
        self._join(key, value)

    def _join(self, key, value):
        """Name *value*, a spine its Axes draws already, *key*."""
        self._dict[key] = value
        _watch_tick_spines(self, value)

    def __delitem__(self, key):
        spine = self._dict.pop(key)
        _watch_tick_spines(self, spine)

    def __iter__(self):
        return iter(self._dict)

    def __len__(self):
        return len(self._dict)


# What matplotlib's module also has from its imports, which programs
# import from it.
from . import patches as mpatches, path as mpath, transforms as mtransforms  # noqa: E402, F401
mpl = __import__(__name__.partition('.')[0])  # matplotlib's ``mpl``: the package
