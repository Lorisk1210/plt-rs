"""
The unit pipeline of an axes (matplotlib's ``_AxesBase._process_unit_info``,
``_unit_change_handler`` and ``Axes._convert_dx``): plotting methods turn
their values in units (dates, categories, the quantities of a registered
converter) into the numbers of the axes' axes here, so that pltrs (Rust)
only sees numbers.
"""

import functools

import numpy as np

from .. import cbook
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
        as they are).
        """
        axis_class = _axis_class()
        return {name: (axis if isinstance(axis, axis_class) and not axis._native_units
                       else None)
                for name in self._axis_names
                for axis in [getattr(self, f"{name}axis")]}

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
        its units, the numbers as given (matplotlib's ``AxesImage._extent``)."""
        (xmin, xmax), (ymin, ymax) = self._process_unit_info(
            [("x", [extent[0], extent[1]]), ("y", [extent[2], extent[3]])],
            kwargs)
        return [xmin, xmax, ymin, ymax]

    def _unit_change_handler(self, axis_name):
        """
        Process axis units changes: the lines keeping values in units
        convert them again, and the data and view limits follow.
        """
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
            delist = False
            if not np.iterable(dx):
                dx = [dx]
                delist = True
            dx = [convert(x0 + ddx) - x for ddx in dx]
            if delist:
                dx = dx[0]
        except (ValueError, TypeError, AttributeError):
            # If the above fails (for any reason) just fallback to what
            # we do by default and convert dx by itself.
            dx = convert(dx)
        return dx
