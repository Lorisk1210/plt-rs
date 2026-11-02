"""
The property cycles of an Axes as objects (matplotlib's ``_get_lines``
and ``_get_patches_for_fill``, its ``_process_plot_var_args``
instances), which pltrs (Rust) keeps: giving one Axes another's cycle
makes both draw from it (``twin._get_lines = ax._get_lines``, as pandas
does for a secondary y axis).
"""

from .._native import axes_base as _axes_base


class _PropCycle:
    """The cycle of an Axes its lines (or patches) take their cycled
    properties from."""

    def __init__(self, axes, kind):
        self._axes = axes
        self._kind = kind

    def get_next_color(self):
        """Return the next color in the cycle."""
        return _axes_base.next_cycle_color(self._axes._native, self._kind)


class PropCycleMixin:
    """``_get_lines`` and ``_get_patches_for_fill`` of `~.axes.Axes`."""

    def _share_cycle(self, kind, cycle):
        """Draw from *cycle*, the cycle of another Axes."""
        if not isinstance(cycle, _PropCycle) or cycle._kind != kind:
            raise TypeError(f"pltrs: {type(cycle).__name__} is not a property "
                            f"cycle of {kind} of an Axes")
        _axes_base.share_prop_cycle(self._native, kind, cycle._axes._native)

    @property
    def _get_lines(self):
        """The property cycle lines are drawn from."""
        return _PropCycle(self, "lines")

    @_get_lines.setter
    def _get_lines(self, cycle):
        self._share_cycle("lines", cycle)

    @property
    def _get_patches_for_fill(self):
        """The property cycle patches are drawn from."""
        return _PropCycle(self, "patches")

    @_get_patches_for_fill.setter
    def _get_patches_for_fill(self, cycle):
        self._share_cycle("patches", cycle)
