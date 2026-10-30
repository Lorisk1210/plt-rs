"""
Unit conversion through an artist's axes (matplotlib's
``Artist.have_units``, ``Artist.convert_xunits`` and
``Artist.convert_yunits``): what an artist holding values in units (dates,
categories, the quantities of a registered converter) converts them with.
"""


class UnitsMixin:
    """The unit methods of `.Artist`."""

    def _convert_units_for(self, axes):
        """
        Convert the values in units the artist was made with outside any
        axes, now that it joins *axes* (matplotlib's artists convert them
        when drawn): the artists that can hold such values override this.
        """

    def have_units(self):
        """Return whether units are set on any axis."""
        ax = self.axes
        return bool(ax) and any(axis.have_units()
                                for axis in ax._axis_map.values()
                                if axis is not None)

    def convert_xunits(self, x):
        """
        Convert *x* using the unit type of the xaxis.

        If the artist is not contained in an Axes or if the xaxis does not
        have units, *x* itself is returned.
        """
        return self._convert_units("x", x)

    def convert_yunits(self, y):
        """
        Convert *y* using the unit type of the yaxis.

        If the artist is not contained in an Axes or if the yaxis does not
        have units, *y* itself is returned.
        """
        return self._convert_units("y", y)

    def _convert_units(self, name, values):
        """*values* converted by the axis *name* of the artist's axes, if
        that axis converts units."""
        ax = getattr(self, 'axes', None)
        axis = None if ax is None else ax._axis_map.get(name)
        return values if axis is None else axis.convert_units(values)
