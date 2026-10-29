r"""
The lines framing an Axes (matplotlib's ``spines``): each `Spine` is a
pltrs (Rust) spine of its Axes, and `Spines` is matplotlib's container of
them (``ax.spines``), kept from matplotlib 3.11.2 under the Matplotlib
License with `SpinesProxy`.
"""

from collections.abc import MutableMapping
import functools

from ._native import spines as _spines
from .patches import Patch


class Spine(Patch):
    """
    An axis spine -- the line noting the data area boundaries.

    Spines are the lines connecting the axis tick marks and noting the
    boundaries of the data area. They can be placed at arbitrary
    positions. See `~.Spine.set_position` for more information.
    """

    def __str__(self):
        return "Spine"

    @property
    def spine_type(self):
        """The side the spine frames: 'left', 'right', 'bottom' or 'top'."""
        return _spines.side(self._native)

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
        if position in ('center', 'zero'):  # special positions
            pass
        else:
            if len(position) != 2:
                raise ValueError("position should be 'center' or 2-tuple")
            if position[0] not in ['outward', 'axes', 'data']:
                raise ValueError("position[0] should be one of 'outward', "
                                 "'axes', or 'data' ")
        kind, amount = {'center': ('axes', 0.5),
                        'zero': ('data', 0.0)}.get(position, position)
        _spines.set_position(self._native, (kind, float(amount)))

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
        if high is None and isinstance(low, tuple):
            low, high = low
        old_low, old_high = self.get_bounds() or (None, None)
        if low is None:
            low = old_low
        if high is None:
            high = old_high
        _spines.set_bounds(self._native, (float(low), float(high)))

    def get_bounds(self):
        """Get the bounds of the spine."""
        return _spines.get_bounds(self._native)

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

    def get_edgecolor(self):
        """Return the edge color."""
        return _spines.get_edgecolor(self._native)

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

