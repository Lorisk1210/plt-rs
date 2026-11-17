# An angle type whose converter shows radians or degrees by unit; set_units switches labels, limits and formatters live.
import math
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.units as munits
import matplotlib.ticker as mticker


class Angle(float):
    """An angle in radians."""


class AngleConverter(munits.ConversionInterface):
    @staticmethod
    def convert(value, unit, axis):
        scale = 180 / math.pi if unit == "deg" else 1.0
        return np.asarray(value, float) * scale

    @staticmethod
    def axisinfo(unit, axis):
        if unit == "deg":
            return munits.AxisInfo(majloc=mticker.MultipleLocator(45), majfmt=mticker.StrMethodFormatter("{x:.0f}°"),
                                   label="angle (degrees)", default_limits=(0, 360))
        return munits.AxisInfo(majloc=mticker.MultipleLocator(math.pi / 2),
                               majfmt=mticker.FuncFormatter(lambda x, p: f"{x / math.pi:.1f}π"), label="angle (rad)")

    @staticmethod
    def default_units(x, axis):
        return "rad"


munits.registry[Angle] = AngleConverter()
theta = [Angle(t) for t in np.linspace(0, 2 * math.pi, 9)]
fig, (a1, a2) = plt.subplots(1, 2, figsize=(7, 3))
a1.plot(theta, np.sin(np.asarray(theta, float)), "o-")
a2.plot(theta, np.cos(np.asarray(theta, float)), "s-", xunits="deg")
fig.canvas.draw()
for a in (a1, a2):
    print(a.xaxis.get_units(), a.get_xlabel(), [t.get_text() for t in a.get_xticklabels()][:4],
          [round(v, 3) for v in a.get_xlim()])
a1.xaxis.set_units("deg")
fig.canvas.draw()
print(a1.get_xlabel(), [t.get_text() for t in a1.get_xticklabels()][:4], [round(v, 3) for v in a1.get_xlim()])
print(np.round(a1.lines[0].get_xdata(orig=False), 2).tolist()[:3])
empty = plt.figure().add_subplot()
empty.xaxis.update_units(Angle(1.0))
empty.xaxis.set_units("deg")
print(empty.get_xlim(), empty.get_xlabel())
fig.savefig("angles.png")
