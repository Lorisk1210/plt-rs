# Major ticks every 10, minor every 2, with a grid on both, for a calibration plot.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.ticker import MultipleLocator, AutoMinorLocator

x = np.linspace(0, 100, 101)
fig, ax = plt.subplots(figsize=(6, 4))
ax.plot(x, 0.98 * x + 1.5, label="instrument")
ax.plot(x, x, "k--", lw=0.8, label="ideal")
ax.xaxis.set_major_locator(MultipleLocator(10))
ax.xaxis.set_minor_locator(MultipleLocator(2))
ax.yaxis.set_minor_locator(AutoMinorLocator(4))
ax.grid(which="major", color="0.7")
ax.grid(which="minor", color="0.9", lw=0.5)
ax.legend()
print(ax.get_xticks()[:4], len(ax.xaxis.get_minorticklocs()))
fig.savefig("calibration.png")
