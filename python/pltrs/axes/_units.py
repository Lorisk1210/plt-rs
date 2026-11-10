"""
The unit pipeline of an axes (matplotlib's ``_AxesBase._process_unit_info``,
``_unit_change_handler`` and ``Axes._convert_dx``): plotting methods turn
their values in units (dates, categories, the quantities of a registered
converter) into the numbers of the axes' axes here, so that pltrs (Rust)
only sees numbers.
"""

import contextlib
import functools
from numbers import Real

import numpy as np

from .. import _api, cbook
from .._native import axis as _axis
# Register the category and date unit converters as a side effect, as
# matplotlib's axes module does.
from .. import category, dates  # noqa: F401


@functools.cache
def _axis_class():
    """`.Axis`, imported on first use."""
    from ..axis import Axis
    return Axis


class UnitsMixin:
    """The unit methods of `~.axes.Axes`."""

    @property
    def _axis_map(self):
        """
        A mapping of axis names, e.g. 'x', to `.Axis` instances: None for
        an axis that takes no units (those of 3-D axes, whose values pass
        as they are, but in `_units_of_native_axes`).
        """
        axis_class = _axis_class()
        through_python = self.__dict__.get("_native_axes_take_units", False)
        return {name: (axis if isinstance(axis, axis_class)
                       and (through_python or not axis._native_units) else None)
                for name in self._axis_names
                for axis in [getattr(self, f"{name}axis")]}

    @contextlib.contextmanager
    def _units_of_native_axes(self):
        """Have the axes whose values pltrs (Rust) converts (those of 3-D
        axes) take and convert units through the 2-D plotting code run
        meanwhile, as matplotlib's do."""
        self._native_axes_take_units = True
        try:
            yield
        finally:
            del self._native_axes_take_units

    def _process_unit_info(self, datasets=None, kwargs=None, *, convert=True):
        """
        Set axis units based on *datasets* and *kwargs*, and optionally apply
        unit conversions to *datasets*.

        Parameters
        ----------
        datasets : list
            List of (axis_name, dataset) pairs (where the axis name is defined
            as in `._axis_map`).  Individual datasets can also be None
            (which gets passed through).
        kwargs : dict
            Other parameters from which unit info (i.e., the *xunits*,
            *yunits*, *zunits* (for 3D Axes), *runits* and *thetaunits* (for
            polar) entries) is popped, if present.  Note that this dict is
            mutated in-place!
        convert : bool, default: True
            Whether to return the original datasets or the converted ones.

        Returns
        -------
        list
            Either the original datasets if *convert* is False, or the
            converted ones if *convert* is True (the default).
        """
        datasets = datasets or []
        kwargs = kwargs or {}
        axis_map = self._axis_map
        for axis_name, data in datasets:
            try:
                axis = axis_map[axis_name]
            except KeyError:
                raise ValueError(f"Invalid axis name: {axis_name!r}") from None
            # Update from data if axis is already set but no unit is set yet.
            if axis is not None and data is not None and not axis.have_units():
                axis.update_units(data)
        for axis_name, axis in axis_map.items():
            if axis is None:
                continue
            # Check for units in the kwargs, and if present update axis.
            units = kwargs.pop(f"{axis_name}units", axis.units)
            if self.name == "polar":
                # Special case: polar supports "thetaunits"/"runits".
                polar_units = {"x": "thetaunits", "y": "runits"}
                units = kwargs.pop(polar_units[axis_name], units)
            if units != axis.units and units is not None:
                axis.set_units(units)
                # If the units being set imply a different converter,
                # we need to update again.
                for dataset_axis_name, data in datasets:
                    if dataset_axis_name == axis_name and data is not None:
                        axis.update_units(data)
        return [axis_map[axis_name].convert_units(data)
                if convert and data is not None
                and axis_map[axis_name] is not None else data
                for axis_name, data in datasets]

    def _converted_extent(self, extent, kwargs=None):
        """The image extent ``(left, right, bottom, top)`` in axis numbers,
        its units taken by the axes (matplotlib's ``AxesImage.set_extent``)."""
        return tuple(map(float, self._extent_values(extent, kwargs)))

    def _extent_values(self, extent, kwargs=None):
        """The image extent ``[left, right, bottom, top]`` converted from
        its units, the numbers as given (matplotlib's ``AxesImage._extent``),
        refused as matplotlib's ``_validate_converted_limits`` refuses
        limits that are not finite."""
        (xmin, xmax), (ymin, ymax) = self._process_unit_info(
            [("x", [extent[0], extent[1]]), ("y", [extent[2], extent[3]])],
            kwargs)
        values = [xmin, xmax, ymin, ymax]
        if any(isinstance(value, Real) and not np.isfinite(value) for value in values):
            raise ValueError("Axis limits cannot be NaN or Inf")
        return values

    def _connect_unit_change_handler(self):
        """Connect the Axes' `_unit_change_handler` to the units changes
        of each of its axes made so far that pltrs (Rust) has the Axes
        follow: matplotlib's ``__init__`` connects it once the Axes is
        cleared, and a later clear (a polar Axes' own) disconnects it."""
        for name in self._axis_names:
            axis = getattr(self, f"{name}axis", None)
            if axis is not None and _axis.follows_units(self._native, name):
                axis.callbacks._connect_picklable(
                    'units', self._unit_change_handler(name))

    def _unit_change_handler(self, axis_name, event=None):
        """
        Process axis units changes: requests updates to data and view limits.
        """
        if event is None:  # Allow connecting `self._unit_change_handler(name)`
            return functools.partial(
                self._unit_change_handler, axis_name, event=object())
        _api.check_in_list(self._axis_map, axis_name=axis_name)
        for line in self.lines:
            line.recache_always()
        self.relim()
        self._request_autoscale_view(axis_name)

    @staticmethod
    def _convert_dx(dx, x0, xconv, convert):
        """
        Small helper to do logic of width conversion flexibly.

        *dx* and *x0* have units, but *xconv* has already been converted
        to unitless (and is an ndarray).  This allows the *dx* to have units
        that are different from *x0*, but are still accepted by the
        ``__add__`` operator of *x0*.
        """
        if xconv.size == 0:
            # xconv has already been converted, but maybe empty...
            return convert(dx)
        try:
            # Attempt to add the width to x0; this works for
            # datetime+timedelta, for instance.  Only the first element
            # of x and x0 is used: addition need not work across the
            # whole vector (lists would concatenate), and casting to
            # arrays would strip the units of unit packages.
            try:
                x0 = cbook._safe_first_finite(x0)
            except (TypeError, IndexError, KeyError):
                pass
            try:
                x = cbook._safe_first_finite(xconv)
            except (TypeError, IndexError, KeyError):
                x = xconv
            if np.iterable(dx):
                dx = [convert(x0 + ddx) - x for ddx in dx]
            else:
                dx = convert(x0 + dx) - x
        except (ValueError, TypeError, AttributeError):
            # If the above fails (for any reason) just fallback to what
            # we do by default and convert dx by itself.
            dx = convert(dx)
        return dx
