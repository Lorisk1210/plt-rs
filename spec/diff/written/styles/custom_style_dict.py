# A house style kept as a dict and applied with style.use(dict), mixed with a named sheet.
import numpy as np
import matplotlib.pyplot as plt
from cycler import cycler

house = {
    "axes.prop_cycle": cycler(color=["#003f5c", "#bc5090", "#ffa600", "#58508d"]),
    "axes.facecolor": "#fbfbfb",
    "axes.edgecolor": "#444444",
    "axes.titlelocation": "left",
    "axes.titlepad": 10,
    "font.family": "sans-serif",
    "grid.color": "#dddddd",
    "axes.grid": True,
}
plt.style.use(["fast", house])
fig, ax = plt.subplots(figsize=(6, 3.5))
for k in range(4):
    ax.plot(np.arange(12), np.cumsum(np.ones(12) * (k + 1)), label=f"s{k}")
ax.set_title("House style")
ax.legend()
print([l.get_color() for l in ax.lines])
fig.savefig("house.png")
