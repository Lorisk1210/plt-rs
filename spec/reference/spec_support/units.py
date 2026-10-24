"""Unit classes of matplotlib's ``tests/test_units.py`` for the ``mpl_units``
probes and the ``mpl_tests_units`` figure cases, one of the spec's own for
the ``units`` cases, and a scope for the unit registry.

``Quantity`` is the test module's numpy facade with units, unchanged.
``QuantityConverter`` is its ``quantity_converter`` fixture without the
``MagicMock`` wrappers that only count calls. ``Temperature`` (with
``TemperatureConverter``) is the spec's: values that are their own error
sizes and convert with an offset. ``registered`` installs
converters for the duration of a ``with`` block: every case module is
imported into one process, and matplotlib's test fixture restores the
registry after each test.
"""

import contextlib

import matplotlib.units as munits
import numpy as np


class Quantity:
    """``test_units.Quantity``: an array (or number) in a unit."""

    def __init__(self, data, units):
        self.magnitude = data
        self.units = units

    def to(self, new_units):
        factors = {('hours', 'seconds'): 3600, ('minutes', 'hours'): 1 / 60,
                   ('minutes', 'seconds'): 60, ('feet', 'miles'): 1 / 5280.,
                   ('feet', 'inches'): 12, ('miles', 'inches'): 12 * 5280}
        if self.units != new_units:
            mult = factors[self.units, new_units]
            return Quantity(mult * self.magnitude, new_units)
        else:
            return Quantity(self.magnitude, self.units)

    def __copy__(self):
        return Quantity(self.magnitude, self.units)

    def __getattr__(self, attr):
        return getattr(self.magnitude, attr)

    def __getitem__(self, item):
        if np.iterable(self.magnitude):
            return Quantity(self.magnitude[item], self.units)
        else:
            return Quantity(self.magnitude, self.units)

    def __array__(self):
        return np.asarray(self.magnitude)


class QuantityConverter(munits.ConversionInterface):
    """The ``quantity_converter`` fixture of ``test_units``."""

    @staticmethod
    def convert(value, unit, axis):
        if hasattr(value, 'units'):
            return value.to(unit).magnitude
        elif np.iterable(value):
            try:
                return [v.to(unit).magnitude for v in value]
            except AttributeError:
                return [Quantity(v, axis.get_units()).to(unit).magnitude
                        for v in value]
        else:
            return Quantity(value, axis.get_units()).to(unit).magnitude

    @staticmethod
    def axisinfo(unit, axis):
        return munits.AxisInfo(label=unit, default_limits=(0, 100))

    @staticmethod
    def default_units(value, axis):
        if hasattr(value, 'units'):
            return value.units
        elif np.iterable(value):
            for v in value:
                if hasattr(v, 'units'):
                    return v.units
            return None


class Temperature:
    """A temperature in degrees Celsius, plotted in degrees Celsius
    (``"C"``), degrees Fahrenheit (``"F"``) or kelvin (``"K"``). It adds to
    temperatures and scales by numbers, so that temperatures serve as their
    own error sizes; a temperature converts with an offset, so that its
    error bars differ when they convert by themselves."""

    def __init__(self, celsius):
        self.celsius = celsius

    def __add__(self, other):
        return Temperature(self.celsius + other.celsius)

    def __mul__(self, factor):
        return Temperature(self.celsius * factor)

    __rmul__ = __mul__

    def __neg__(self):
        return Temperature(-self.celsius)

    def __lt__(self, other):
        return self.celsius < other.celsius

    def __eq__(self, other):
        return isinstance(other, Temperature) and self.celsius == other.celsius

    __hash__ = None

    def to(self, unit):
        """The temperature in ``unit``."""
        if unit == "F":
            return self.celsius * 9 / 5 + 32
        if unit == "K":
            return self.celsius + 273.15
        return self.celsius


class TemperatureConverter(munits.ConversionInterface):
    """Temperatures in the axis unit, Celsius by default, labelled with
    it."""

    @staticmethod
    def convert(value, unit, axis):
        if np.iterable(value):
            return [v.to(unit) for v in value]
        return value.to(unit)

    @staticmethod
    def axisinfo(unit, axis):
        return munits.AxisInfo(label=f"temperature ({unit})")

    @staticmethod
    def default_units(value, axis):
        return "C"


@contextlib.contextmanager
def registered(converters):
    """Registers ``converters`` (a mapping of classes to converters) for
    the block, then restores the registry as it was."""
    saved = dict(munits.registry)
    munits.registry.update(converters)
    try:
        yield
    finally:
        munits.registry.clear()
        munits.registry.update(saved)


def quantity_registered():
    """``registered`` with the ``quantity_converter`` fixture."""
    return registered({Quantity: QuantityConverter()})


def temperature_registered():
    """``registered`` with the converter of ``Temperature``."""
    return registered({Temperature: TemperatureConverter()})


def jpl_registered():
    """``registered`` with what ``jpl_units.register()`` registers."""
    import matplotlib.testing.jpl_units as units

    return registered({str: units.StrConverter(), units.Epoch: units.EpochConverter(),
                       units.Duration: units.EpochConverter(),
                       units.UnitDbl: units.UnitDblConverter()})
