"""
The property cycles of an Axes as objects (matplotlib's ``_get_lines``
and ``_get_patches_for_fill``, its ``_process_plot_var_args``
instances), which pltrs (Rust) keeps: giving one Axes another's cycle
makes both draw from it (``twin._get_lines = ax._get_lines``, as pandas
does for a secondary y axis). Each holds the cycle's entries and position
(``_prop_cycle``, matplotlib's ``_PropCycle``), which programs read and
advance (mplhep's ``_prop_cycle.getdefaults(kwargs)``).
"""

from .. import _values
from .._native import axes_base as _axes_base


def _owner(axes, kind):
    """The Axes whose cycle *kind* *axes* draws from (see
    `PropCycleMixin._share_cycle`)."""
    seen = set()
    while id(axes) not in seen:
        seen.add(id(axes))
        other = axes.__dict__.get("_cycles_from", {}).get(kind)
        if other is None:
            break
        axes = other
    return axes


def _cycler(axes, kind):
    """The cycler of the property cycle *kind* of *axes*, as the program
    gave it: to the cycle alone, else to the Axes, else
    :rc:`axes.prop_cycle`."""
    axes = _owner(axes, kind)
    cycler = axes.__dict__.get("_kind_cycles", {}).get(kind)
    return _values.prop_cycle(axes) if cycler is None else cycler


class _PropCycle:
    """
    A class that holds the property cycle information.

    It expands the iterator-based Cycler into an explicit indexed list to support
    conditional advancing.
    """

    def __init__(self, axes, kind):
        self._axes = axes
        self._kind = kind

    @property
    def _cycler_items(self):
        """The entries of the cycle, one dict of properties each."""
        return [*_cycler(self._axes, self._kind)]

    @property
    def _idx(self):
        """The entry the cycle gives next."""
        return _axes_base.cycle_position(self._axes._native, self._kind)

    @_idx.setter
    def _idx(self, index):
        _axes_base.set_cycle_position(self._axes._native, self._kind,
                                      index % len(self._cycler_items))

    def get_next_color(self):
        """Return the next color in the cycle."""
        entry = self._cycler_items[self._idx]
        if "color" in entry:
            _axes_base.next_cycle_color(self._axes._native, self._kind)
            return entry["color"]
        else:
            return "k"

    def getdefaults(self, kw, ignore=frozenset()):
        """
        If some keys in the property cycle (excluding those in the set
        *ignore*) are absent or set to None in the dict *kw*, return a copy
        of the next entry in the property cycle, excluding keys in *ignore*.
        Otherwise, don't advance the property cycle, and return an empty dict.
        """
        items = self._cycler_items
        index = self._idx
        defaults = items[index]
        if any(kw.get(k, None) is None for k in {*defaults} - ignore):
            self._idx = (index + 1) % len(items)  # Advance cycler.
            # Return a new dict to avoid exposing _cycler_items entries to mutation.
            return {k: v for k, v in defaults.items() if k not in ignore}
        else:
            return {}


class _process_plot_var_args:
    """
    Process variable length arguments to `~.Axes.plot`, to support ::

      plot(t, s)
      plot(t1, s1, 'ko', t2, s2)
      plot(t1, s1, 'ko', t2, s2, 'r--', t3, e3)

    an arbitrary number of *x*, *y*, *fmt* are allowed
    """

    def __init__(self, axes, kind):
        self._axes = axes
        self._kind = kind
        self.output = "Line2D" if kind == "lines" else "Polygon"

    @property
    def _prop_cycle(self):
        """The entries of the cycle and the one it gives next."""
        return _PropCycle(self._axes, self._kind)

    def set_prop_cycle(self, cycler):
        """Cycle over *cycler*, from its first entry (``None``:
        :rc:`axes.prop_cycle`)."""
        from ..rcsetup import _rc_value
        axes = _owner(self._axes, self._kind)
        _axes_base.set_prop_cycle_of(axes._native, self._kind,
                                     None if cycler is None else _rc_value(cycler))
        axes.__dict__.setdefault("_kind_cycles", {})[self._kind] = cycler

    def get_next_color(self):
        """Return the next color in the cycle."""
        return self._prop_cycle.get_next_color()

    @staticmethod
    def _setdefaults(defaults, kw):
        """
        Add to the dict *kw* the entries in the dict *default* that are absent
        or set to None in *kw*.
        """
        for k in defaults:
            if kw.get(k, None) is None:
                kw[k] = defaults[k]


class PropCycleMixin:
    """``_get_lines`` and ``_get_patches_for_fill`` of `~.axes.Axes`."""

    def _share_cycle(self, kind, cycle):
        """Draw from *cycle*, the cycle of another Axes."""
        if not isinstance(cycle, _process_plot_var_args) or cycle._kind != kind:
            raise TypeError(f"pltrs: {type(cycle).__name__} is not a property "
                            f"cycle of {kind} of an Axes")
        _axes_base.share_prop_cycle(self._native, kind, cycle._axes._native)
        self.__dict__.setdefault("_cycles_from", {})[kind] = _owner(cycle._axes, kind)

    def _cycles_set(self, cycler):
        """Note *cycler* (`None`: :rc:`axes.prop_cycle`) as the cycler of
        the cycles the Axes draws from (`set_prop_cycle`)."""
        if cycler is None:
            from .. import rcParams
            cycler = rcParams['axes.prop_cycle']
        for kind in ("lines", "patches"):
            _owner(self, kind).__dict__.setdefault("_kind_cycles", {})[kind] = cycler

    def _forget_cycles(self):
        """Forget the cycles given to the Axes' lines and patches and the
        Axes they were shared from, as matplotlib's clear does."""
        self.__dict__.pop("_kind_cycles", None)
        self.__dict__.pop("_cycles_from", None)
        self.__dict__.get("_given_values", {}).pop("prop_cycle", None)

    @property
    def _get_lines(self):
        """The property cycle lines are drawn from."""
        return _process_plot_var_args(self, "lines")

    @_get_lines.setter
    def _get_lines(self, cycle):
        self._share_cycle("lines", cycle)

    @property
    def _get_patches_for_fill(self):
        """The property cycle patches are drawn from."""
        return _process_plot_var_args(self, "patches")

    @_get_patches_for_fill.setter
    def _get_patches_for_fill(self, cycle):
        self._share_cycle("patches", cycle)
