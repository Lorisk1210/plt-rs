"""Unit classes of matplotlib's ``tests/test_units.py`` for the ``mpl_units``
probes and the ``mpl_tests_units`` figure cases, and a scope for the unit
registry.

``Quantity`` is the test module's numpy facade with units, unchanged.
``QuantityConverter`` is its ``quantity_converter`` fixture without the
``MagicMock`` wrappers that only count calls. ``registered`` installs
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


def jpl_registered():
    """``registered`` with what ``jpl_units.register()`` registers."""
    import matplotlib.testing.jpl_units as units

    return registered({str: units.StrConverter(), units.Epoch: units.EpochConverter(),
                       units.Duration: units.EpochConverter(),
                       units.UnitDbl: units.UnitDblConverter()})
