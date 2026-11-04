r"""
The lines framing an Axes (matplotlib's ``spines``): each `Spine` is a
pltrs (Rust) spine of its Axes, and `Spines` is matplotlib's container of
them (``ax.spines``), kept from matplotlib 3.11.2 under the Matplotlib
License with `SpinesProxy`.
"""

from collections.abc import MutableMapping
import functools

import numpy as np

from . import _api, _handles, _values, rcParams
from .colors import to_rgba
from ._native import artist as _artist, spines as _spines
from .artist import Artist
from .patches import Patch
from .path import Path
from .transforms import Bbox, IdentityTransform

#: matplotlib's paths of the linear spines (``Spine.linear_spine``), in
#: data along their axis and axes fractions across it, until a draw or a
#: measure spans them over the view.
_LINEAR_PATHS = {
    "left": [(0.0, 0.999), (0.0, 0.999)],
    "right": [(1.0, 0.999), (1.0, 0.999)],
    "bottom": [(0.999, 0.0), (0.999, 0.0)],
    "top": [(0.999, 1.0), (0.999, 1.0)],
}


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


class Spine(Patch):
    """
    An axis spine -- the line noting the data area boundaries.

    Spines are the lines connecting the axis tick marks and noting the
    boundaries of the data area. They can be placed at arbitrary
    positions. See `~.Spine.set_position` for more information.
    """

    def __str__(self):
        return "Spine"

    def __new__(cls, axes=None, spine_type=None, path=None, **kwargs):
        # The spine is one pltrs (Rust) draws for *axes* (see `__init__`);
        # pickling makes a bare object.
        if axes is None:
            return super().__new__(cls)
        return cls._of_axes(axes, spine_type)

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

        **kwargs
            `.Patch` properties.

        pltrs: the spine is the one of *axes* on the side *spine_type*
        names, else the one framing its outline (a polar Axes' ``'polar'``
        spine), drawn as *path* through its transform (``transData``
        unless it is given another).
        """
        _api.check_isinstance(Path, path=path)
        self._path = path
        self.set_facecolor('none')
        self.set_edgecolor(rcParams['axes.edgecolor'])
        self.set_linewidth(rcParams['axes.linewidth'])
        self.set_capstyle('projecting')
        self.set_zorder(2.5)
        if spine_type not in _LINEAR_PATHS:
            _spines.set_patch_path(self._native, path)
            self.set_transform(axes.transData)
        self._internal_update(kwargs)

    @classmethod
    def linear_spine(cls, axes, spine_type, **kwargs):
        """Create and return a linear `Spine`: the spine of *axes* on the
        side *spine_type*, as an object of *cls*."""
        if spine_type not in dict(_spines.axes_spines(axes._native)):
            raise ValueError(f'unable to make path for spine "{spine_type}"')
        spine = cls._of_axes(axes, spine_type)
        spine.set(**kwargs)
        return spine

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
        """
        Create and return an arc `Spine`.

        pltrs: the spine of *axes* named *spine_type* (its outline's for a
        name of no side), drawn as the arc.
        """
        result = cls._of_axes(axes, spine_type)
        result.set_patch_arc(center, radius, theta1, theta2)
        result.set(**kwargs)
        return result

    @classmethod
    def circular_spine(cls, axes, center, radius, **kwargs):
        """
        Create and return a circular `Spine`.

        pltrs: the spine framing the outline of *axes*, drawn as the
        circle.
        """
        result = cls._of_axes(axes, 'circle')
        result.set_patch_circle(center, radius)
        result.set(**kwargs)
        return result

    def set_patch_arc(self, center, radius, theta1, theta2):
        """Set the spine to be arc-like."""
        _spines.set_patch_arc(self._native, tuple(map(float, center)),
                              float(radius), (float(theta1), float(theta2)))
        self.stale = True

    def set_patch_circle(self, center, radius):
        """Set the spine to be circular."""
        _spines.set_patch_circle(self._native, tuple(map(float, center)),
                                 float(radius))
        self.stale = True

    def set_patch_line(self):
        """Set the spine to be linear."""
        _spines.set_patch_line(self._native)
        self.stale = True

    @property
    def _patch_type(self):
        """The shape of the spine: 'line', 'arc' or 'circle'."""
        return _spines.patch_type(self._native)

    def clear(self):
        """Clear the current spine."""
        _spines.set_position(self._native, ("outward", 0.0))
        if self.axis is not None:
            self.axis.clear()

    def get_spine_transform(self):
        """Return the spine transform."""
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

    def _adjust_location(self):
        """
        Have the spine's path (``_path``: two points in data coordinates
        along its axis and axes fractions across it) span its axis' view
        interval, as matplotlib's linear spines do; what the projection of
        a program's class asks of a spine class overriding it (bounds set
        on the spine are taken by pltrs (Rust) first).
        """
        vertices = self._path.vertices
        if self.spine_type in ('left', 'right'):
            vertices[:, 1] = self.axes.viewLim.intervaly
        else:
            vertices[:, 0] = self.axes.viewLim.intervalx

    def _span_path(self):
        """Have the spine's path span its bounds, else its axis' view
        interval, as matplotlib's linear spines do when they are drawn or
        measured (``_adjust_location``)."""
        bounds = self.get_bounds()
        if bounds is None:
            self._adjust_location()
        else:
            along = 1 if self.spine_type in ('left', 'right') else 0
            self._path.vertices[:, along] = bounds

    @property
    def _is_linear(self):
        """Whether the spine is a line along a side of its Axes, drawn
        through its transform as matplotlib's linear spines are (not an
        arc, a circle or a spine of polar or geographic Axes, which pltrs
        (Rust) lays out in display space)."""
        return self._patch_type == 'line' and self.spine_type in _LINEAR_PATHS

    def _linear_path(self):
        """The spine's ``_path``, matplotlib's linear one first, which is
        spanned over the view again when the spine or its Axes was drawn
        since (drawing an axis measures the spines its label is placed
        beside, animated ones too)."""
        if "_path" not in self.__dict__:
            self._path = Path(np.array(_LINEAR_PATHS[self.spine_type]))
        draws = (_artist.draw_count(self._native),
                 0 if self.axes is None else _artist.draw_count(self.axes._native))
        if draws != self.__dict__.get("_path_draws", (0, 0)):
            self._path_draws = draws
            self._span_path()
        return self._path

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
        """
        _check_position(position)
        kind, amount = {'center': ('axes', 0.5),
                        'zero': ('data', 0.0)}.get(position, position)
        _spines.set_position(self._native, (kind, float(amount)))
        # The spine takes the transform of its position again.
        self.__dict__.pop("_given_transform", None)

    def get_position(self):
        """Return the spine position."""
        return _spines.get_position(self._native)

    def set_bounds(self, low=None, high=None):
        """
        Set the spine bounds.

        Parameters
        ----------
        low : float or None, optional
            The lower spine bound. Passing *None* leaves the limit unchanged.

            The bounds may also be passed as the tuple (*low*, *high*) as the
            first positional argument.

        high : float or None, optional
            The higher spine bound. Passing *None* leaves the limit unchanged.
        """
        if self.spine_type == 'circle':
            raise ValueError(
                'set_bounds() method incompatible with circular spines')
        if high is None and np.iterable(low):
            low, high = low
        _spines.set_bound_ends(self._native,
                               None if low is None else float(low),
                               None if high is None else float(high))

    def get_bounds(self):
        """Get the bounds of the spine."""
        return _spines.get_bounds(self._native)

    @property
    def axis(self):
        """The `.Axis` the spine belongs to (see `register_axis`)."""
        if "_axis" in self.__dict__:
            return self._axis
        name = {"left": "y", "right": "y", "bottom": "x", "top": "x",
                "polar": "y"}.get(self.spine_type)
        return None if name is None else getattr(self.axes, f"{name}axis")

    def register_axis(self, axis):
        """
        Register an axis.

        An axis should be registered with its corresponding spine from
        the Axes instance. This allows the spine to clear any axis
        properties when needed.
        """
        self._axis = axis

    def get_window_extent(self, renderer=None):
        """
        Return the window extent of the spines in display space, including
        padding for ticks (but not their labels)
        """
        if self._is_linear:
            # Measuring spans the path over the view, as drawing does.
            self._linear_path()
            self._span_path()
        extent = _spines.window_extent(self._native)
        if extent is None:
            return Bbox.null()
        x0, y0, x1, y1 = extent
        return Bbox([[x0, y0], [x1, y1]])

    def get_tightbbox(self, renderer=None):
        """Like `get_window_extent`: a spine is not clipped."""
        return self.get_window_extent(renderer)

    def get_path(self):
        """
        Return the path of the spine: that of a linear spine in data along
        its axis and axes fractions across it (spanning the view where it
        was last drawn or measured), the others in display coordinates,
        where pltrs (Rust) lays them out.
        """
        if self._is_linear:
            return self._linear_path()
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
        if self._is_linear:
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
        _spines.set_color(self._native, c)
        _values.remember(self, "edgecolor", c)

    def get_edgecolor(self):
        """Return the edge color."""
        # ``to_rgba`` of the colour given, as matplotlib keeps it, while
        # pltrs (Rust) holds that colour.
        native = _spines.get_edgecolor(self._native)
        given = self.__dict__.get("_given_values", {}).get("edgecolor")
        if given is None:
            return native
        rgba = to_rgba(given, self.get_alpha())
        return rgba if rgba == tuple(native) else native

    def set_linewidth(self, w):
        """Set the spine width in points."""
        _spines.set_linewidth(self._native, float(w))

    def set_linestyle(self, ls):
        """Set the spine line style."""
        _spines.set_linestyle(self._native, ls)


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
        """The container of the spines of the dict *d*."""
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
        self._dict[key] = value

    def __delitem__(self, key):
        del self._dict[key]

    def __iter__(self):
        return iter(self._dict)

    def __len__(self):
        return len(self._dict)

