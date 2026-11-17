# A Quantity class with a ConversionInterface: axisinfo with label and locators, default_units, convert between metres and feet.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.units as munits
import matplotlib.ticker as mticker

FACTORS = {"m": 1.0, "ft": 0.3048, "km": 1000.0}


class Quantity:
    def __init__(self, value, unit):
        self.value = np.asarray(value, float)
        self.unit = unit

    def __len__(self):
        return self.value.size

    def __getitem__(self, i):
        return Quantity(self.value[i], self.unit)

    def __iter__(self):
        return (Quantity(v, self.unit) for v in self.value)

    @property
    def shape(self):
        return self.value.shape

    @property
    def ndim(self):
        return self.value.ndim

    def to(self, unit):
        return self.value * FACTORS[self.unit] / FACTORS[unit]


class QuantityConverter(munits.ConversionInterface):
    calls = []

    @staticmethod
    def convert(value, unit, axis):
        QuantityConverter.calls.append(("convert", type(value).__name__, unit))
        if isinstance(value, Quantity):
            return value.to(unit)
        return [v.to(unit) if isinstance(v, Quantity) else v for v in value] if np.iterable(value) else value

    @staticmethod
    def axisinfo(unit, axis):
        QuantityConverter.calls.append(("axisinfo", unit))
        return munits.AxisInfo(label=f"distance [{unit}]", majloc=mticker.MaxNLocator(4),
                               majfmt=mticker.FormatStrFormatter(f"%g {unit}"))

    @staticmethod
    def default_units(x, axis):
        QuantityConverter.calls.append(("default_units", type(x).__name__))
        return x.unit if isinstance(x, Quantity) else x[0].unit


munits.registry[Quantity] = QuantityConverter()
fig, (a1, a2) = plt.subplots(1, 2, figsize=(7, 3))
a1.plot(np.arange(5), Quantity(np.arange(5) * 100, "ft"))
a2.plot(np.arange(5), Quantity(np.arange(5) * 100, "ft"), yunits="m")
fig.canvas.draw()
print(a1.yaxis.get_units(), a2.yaxis.get_units(), a1.get_ylabel(), a2.get_ylabel())
print([t.get_text() for t in a1.get_yticklabels()], [t.get_text() for t in a2.get_yticklabels()])
print([round(v, 3) for v in a2.get_ylim()])
a2.yaxis.set_units("km")
fig.canvas.draw()
print([round(v, 5) for v in a2.get_ylim()], a2.get_ylabel())
print(sorted(set(c[0] for c in QuantityConverter.calls)))
print(type(munits.registry.get_converter(Quantity(1, "m"))).__name__)
fig.savefig("lengths.png")
