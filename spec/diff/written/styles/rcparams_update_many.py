# Publication defaults via rcParams.update (fonts, ticks, legend, savefig), then a figure.
import numpy as np
import matplotlib.pyplot as plt

plt.rcParams.update({
    "figure.figsize": (3.5, 2.6), "figure.dpi": 150, "font.size": 8, "axes.labelsize": 8,
    "axes.titlesize": 9, "legend.fontsize": 7, "xtick.labelsize": 7, "ytick.labelsize": 7,
    "lines.linewidth": 1.0, "lines.markersize": 3, "axes.linewidth": 0.6, "legend.frameon": False,
    "savefig.bbox": "tight", "savefig.pad_inches": 0.02, "axes.formatter.use_mathtext": True,
})
x = np.linspace(0, 1, 30)
fig, ax = plt.subplots()
ax.plot(x, 1e4 * x ** 2, "o-", label="model")
ax.set_xlabel("strain")
ax.set_ylabel("stress [Pa]")
ax.legend()
fig.savefig("pub.png")
print(plt.rcParams["savefig.bbox"], plt.rcParams["figure.figsize"])
